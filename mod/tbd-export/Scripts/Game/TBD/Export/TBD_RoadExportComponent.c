/**
 * TBD_RoadExportComponent.c
 *
 * Dedicated runtime GameMode component for Bohemia Reforger road & highway network extraction.
 * Runs in active mission simulation ("Play" mode in Workbench or dedicated server).
 * Unlocks compiled C++ ChimeraAIWorld & RoadNetworkManager to extract authentic 100% continuous 3D splines.
 *
 * Outputs:
 *   - $profile:TBD_Export/<mapName>/roads/highways.json
 *   - $profile:TBD_Export/<mapName>/roads/roads_paved.json
 *   - $profile:TBD_Export/<mapName>/roads/roads_dirt.json
 *   - $profile:TBD_Export/<mapName>/roads/tracks.json
 *   - $profile:TBD_Export/<mapName>/roads/paths.json
 *   - $profile:TBD_Export/<mapName>/roads/runways.json
 *   - $profile:TBD_Export/<mapName>/roads/roads_meta.json
 */

[ComponentEditorProps(category: "TBD/Export", description: "Standalone Reforger runtime road network exporter component.")]
class TBD_RoadExportComponentClass : SCR_BaseGameModeComponentClass {}

class TBD_RoadExportComponent : SCR_BaseGameModeComponent
{
	protected static const string TAG = "[TBD-EXPORT]";
	protected static const int FLUSH_BUF_SIZE = 8000;
	protected static const float CONNECTION_TOLERANCE_M = 5.0;
	protected static const float JUNCTION_TOLERANCE_M = 2.5;

	protected ref array<IEntity> m_aSceneRoadEntities;
	protected ref array<BaseRoad> m_aProcessedRoads;

	protected ref array<ref TBD_RoadSegmentRecord> m_aHighways;
	protected ref array<ref TBD_RoadSegmentRecord> m_aPaved;
	protected ref array<ref TBD_RoadSegmentRecord> m_aDirt;
	protected ref array<ref TBD_RoadSegmentRecord> m_aTracks;
	protected ref array<ref TBD_RoadSegmentRecord> m_aPaths;
	protected ref array<ref TBD_RoadSegmentRecord> m_aRunways;

	//------------------------------------------------------------------------------------------------
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// Only authority runs export
		if (RplSession.Mode() == RplMode.Client)
			return;

		Print(TAG + " Registered on GameMode. Scheduling runtime road extraction...", LogLevel.NORMAL);
		// Delay 500ms to ensure the engine's AIWorld and RoadNetworkManager graph are fully initialized
		GetGame().GetCallqueue().CallLater(ExecuteExport, 500, false);
	}

	//------------------------------------------------------------------------------------------------
	protected bool CollectEntity(IEntity e)
	{
		if (e)
			m_aSceneRoadEntities.Insert(e);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Derives a clean canonical lowercase slug for the map from the world path.
	static string DeriveMapName(string worldPath)
	{
		if (worldPath.IsEmpty())
			return "everon";

		worldPath.Replace("\\", "/");
		array<string> parts = {};
		worldPath.Split("/", parts, false);
		if (parts.Count() == 0)
			return "everon";

		string leaf = parts[parts.Count() - 1];
		int dotIdx = leaf.LastIndexOf(".");
		if (dotIdx > 0)
			leaf = leaf.Substring(0, dotIdx);

		string terrainName = leaf;
		if (parts.Count() >= 2)
		{
			string parentFolder = parts[parts.Count() - 2];
			if (parentFolder != "worlds" && parentFolder != "Worlds" && parentFolder != "MP" && !parentFolder.IsEmpty())
				terrainName = parentFolder;
		}

		terrainName.ToLower();
		terrainName.Trim();

		if (terrainName == "eden")
			terrainName = "everon";

		if (terrainName.IsEmpty())
			terrainName = "everon";

		return terrainName;
	}

	//------------------------------------------------------------------------------------------------
	void ExecuteExport()
	{
		int tickStart = System.GetTickCount();
		Print("==================================================================", LogLevel.NORMAL);
		Print(TAG + " >>> STARTING RUNTIME ROAD & HIGHWAY EXTRACTION <<<", LogLevel.NORMAL);
		Print("==================================================================", LogLevel.NORMAL);

		// STEP 1: Context & World
		BaseWorld world = GetGame().GetWorld();
		if (!world)
		{
			Print(TAG + " ERROR: BaseWorld is null! Cannot proceed.", LogLevel.ERROR);
			return;
		}

		string mapName = "everon";
		float worldSize = 12800.0;

		Print(string.Format("%1 Step 1: Active Map = '%2', Assumed World Size = %3 m", TAG, mapName, worldSize), LogLevel.NORMAL);

		// STEP 2: Resolve ChimeraAIWorld & RoadNetworkManager
		ChimeraAIWorld aiWorld = ChimeraAIWorld.Cast(GetGame().GetAIWorld());
		if (!aiWorld)
		{
			Print(TAG + " ERROR: ChimeraAIWorld could not be retrieved from GetGame().GetAIWorld()!", LogLevel.ERROR);
			return;
		}

		RoadNetworkManager rnm = aiWorld.GetRoadNetworkManager();
		if (!rnm)
		{
			Print(TAG + " ERROR: RoadNetworkManager is null on ChimeraAIWorld!", LogLevel.ERROR);
			return;
		}

		Print(TAG + " Step 2: Successfully acquired ChimeraAIWorld and RoadNetworkManager.", LogLevel.NORMAL);

		// STEP 3: Query scene entities & extract compiled BaseRoad splines
		m_aSceneRoadEntities = new array<IEntity>();
		m_aProcessedRoads = new array<BaseRoad>();

		m_aHighways = new array<ref TBD_RoadSegmentRecord>();
		m_aPaved    = new array<ref TBD_RoadSegmentRecord>();
		m_aDirt     = new array<ref TBD_RoadSegmentRecord>();
		m_aTracks   = new array<ref TBD_RoadSegmentRecord>();
		m_aPaths    = new array<ref TBD_RoadSegmentRecord>();
		m_aRunways  = new array<ref TBD_RoadSegmentRecord>();

		Print(TAG + " Step 3: Scanning world for road waypoints...", LogLevel.NORMAL);
		world.QueryEntitiesByAABB(Vector(0, -500, 0), Vector(worldSize, 1500, worldSize), CollectEntity);

		Print(string.Format("%1 QueryEntitiesByAABB found %2 total scene entities.", TAG, m_aSceneRoadEntities.Count()), LogLevel.NORMAL);

		int roadsQueried = 0;
		int roadsExtracted = 0;

		// Probe each entity
		foreach (IEntity ent : m_aSceneRoadEntities)
		{
			if (!ent)
				continue;

			string clsName = ent.ClassName();
			if (clsName != "RoadEntity" && !clsName.Contains("Road"))
				continue;

			vector entPos = ent.GetOrigin();
			roadsQueried++;

			BaseRoad foundRoad = null;
			float dist = 0.0;
			int qRes = rnm.GetClosestRoad(entPos, foundRoad, dist, true);

			if (foundRoad && dist <= 30.0)
			{
				if (m_aProcessedRoads.Find(foundRoad) != -1)
					continue;

				m_aProcessedRoads.Insert(foundRoad);
				roadsExtracted++;

				ref array<vector> pts = {};
				int numPts = foundRoad.GetPoints(pts);
				if (numPts < 2)
					continue;

				float widthM = foundRoad.GetWidth();
				if (widthM < 1.0)
					widthM = 6.0;

				string resName = "";
				EntityPrefabData pd = ent.GetPrefabData();
				if (pd)
					resName = pd.GetPrefabName();

				string matName = "";

				TBD_ERoadLayer layer = TBD_RoadClassifier.Classify(matName, resName, widthM);
				if (layer == TBD_ERoadLayer.NONE)
					layer = TBD_ERoadLayer.PAVED;

				int recId = GetLayerRecordCount(layer) + 1;
				string prefix = TBD_RoadClassifier.LayerToPrefix(layer);
				string roadClass = TBD_RoadClassifier.LayerToSlug(layer);
				string segName = roadClass + "_" + recId.ToString();

				TBD_RoadSegmentRecord rec = new TBD_RoadSegmentRecord(recId, prefix, segName, roadClass, widthM, resName, matName);

				for (int p = 0; p < numPts; p++)
				{
					rec.AddPoint(pts[p]);
				}

				rec.m_vStartNodePos = rec.m_aPoints[0];
				rec.m_vEndNodePos = rec.m_aPoints[rec.m_aPoints.Count() - 1];

				AddRecordToLayer(layer, rec);
			}
		}

		// Also do a grid sweep to capture any road segments that don't have distinct RoadEntities in the query
		float stepM = 200.0;
		int gridSteps = Math.Ceil(worldSize / stepM);
		Print(string.Format("%1 Performing grid probe across world (%2x%2 points @ %3 m)...", TAG, gridSteps, stepM), LogLevel.NORMAL);

		for (int gz = 0; gz < gridSteps; gz++)
		{
			for (int gx = 0; gx < gridSteps; gx++)
			{
				vector probePos = Vector(gx * stepM + (stepM * 0.5), 0, gz * stepM + (stepM * 0.5));
				BaseRoad gRoad = null;
				float gDist = 0.0;
				int gRes = rnm.GetClosestRoad(probePos, gRoad, gDist, true);

				if (gRoad && gDist <= (stepM * 0.75))
				{
					if (m_aProcessedRoads.Find(gRoad) != -1)
						continue;

					m_aProcessedRoads.Insert(gRoad);
					roadsExtracted++;

					ref array<vector> gPts = {};
					int gNumPts = gRoad.GetPoints(gPts);
					if (gNumPts < 2)
						continue;

					float gWidth = gRoad.GetWidth();
					if (gWidth < 1.0)
						gWidth = 6.0;

					TBD_ERoadLayer gLayer = TBD_RoadClassifier.Classify("", "", gWidth);
					int gRecId = GetLayerRecordCount(gLayer) + 1;
					string gPrefix = TBD_RoadClassifier.LayerToPrefix(gLayer);
					string gRoadClass = TBD_RoadClassifier.LayerToSlug(gLayer);
					string gSegName = gRoadClass + "_" + gRecId.ToString();

					TBD_RoadSegmentRecord gRec = new TBD_RoadSegmentRecord(gRecId, gPrefix, gSegName, gRoadClass, gWidth, "", "");

					for (int gp = 0; gp < gNumPts; gp++)
					{
						gRec.AddPoint(gPts[gp]);
					}

					gRec.m_vStartNodePos = gRec.m_aPoints[0];
					gRec.m_vEndNodePos = gRec.m_aPoints[gRec.m_aPoints.Count() - 1];

					AddRecordToLayer(gLayer, gRec);
				}
			}
		}

		Print(string.Format("%1 Step 3 Complete: Extracted %2 total continuous BaseRoad splines.", TAG, roadsExtracted), LogLevel.NORMAL);
		Print(string.Format("%1   - Highways: %2 segments", TAG, m_aHighways.Count()), LogLevel.NORMAL);
		Print(string.Format("%1   - Paved Roads: %2 segments", TAG, m_aPaved.Count()), LogLevel.NORMAL);
		Print(string.Format("%1   - Dirt Roads: %2 segments", TAG, m_aDirt.Count()), LogLevel.NORMAL);
		Print(string.Format("%1   - Tracks: %2 segments", TAG, m_aTracks.Count()), LogLevel.NORMAL);
		Print(string.Format("%1   - Paths: %2 segments", TAG, m_aPaths.Count()), LogLevel.NORMAL);
		Print(string.Format("%1   - Runways: %2 segments", TAG, m_aRunways.Count()), LogLevel.NORMAL);

		// STEP 4: Build Connectivity & Master Junction Graph
		Print(TAG + " Step 4: Building topological endpoint graph...", LogLevel.NORMAL);
		BuildConnectivity(m_aHighways);
		BuildConnectivity(m_aPaved);
		BuildConnectivity(m_aDirt);
		BuildConnectivity(m_aTracks);
		BuildConnectivity(m_aPaths);
		BuildConnectivity(m_aRunways);

		array<ref TBD_RoadJunctionNode> globalJunctions = {};
		BuildGlobalJunctionGraph(m_aHighways, m_aPaved, m_aDirt, m_aTracks, m_aPaths, m_aRunways, globalJunctions);
		Print(string.Format("%1 Identified %2 global intersection junction nodes.", TAG, globalJunctions.Count()), LogLevel.NORMAL);

		// STEP 5: Stream-Write JSON Datasets
		Print(TAG + " Step 5: Stream-writing JSON datasets to $profile...", LogLevel.NORMAL);
		string baseDir = "$profile:TBD_Export/";

		WriteLayerJson(baseDir, mapName, "highways.json", "highway_paved", worldSize, m_aHighways);
		WriteLayerJson(baseDir, mapName, "roads_paved.json", "road_paved", worldSize, m_aPaved);
		WriteLayerJson(baseDir, mapName, "roads_dirt.json", "road_dirt", worldSize, m_aDirt);
		WriteLayerJson(baseDir, mapName, "tracks.json", "track", worldSize, m_aTracks);
		WriteLayerJson(baseDir, mapName, "paths.json", "path", worldSize, m_aPaths);
		WriteLayerJson(baseDir, mapName, "runways.json", "runway", worldSize, m_aRunways);

		int elapsedMs = System.GetTickCount() - tickStart;
		WriteRoadsMetaJson(baseDir, mapName, worldSize, m_aHighways, m_aPaved, m_aDirt, m_aTracks, m_aPaths, m_aRunways, globalJunctions, elapsedMs);

		int totalSegs = m_aHighways.Count() + m_aPaved.Count() + m_aDirt.Count() + m_aTracks.Count() + m_aPaths.Count() + m_aRunways.Count();
		float totalLenM = CalcTotalLength(m_aHighways) + CalcTotalLength(m_aPaved) + CalcTotalLength(m_aDirt) + CalcTotalLength(m_aTracks) + CalcTotalLength(m_aPaths) + CalcTotalLength(m_aRunways);

		Print("==================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 >>> EXTRACTION COMPLETE in %2 ms <<<", TAG, elapsedMs), LogLevel.NORMAL);
		Print(string.Format("%1 Total Continuous Routes: %2", TAG, totalSegs), LogLevel.NORMAL);
		Print(string.Format("%1 Total Network Length: %2 km", TAG, (totalLenM / 1000.0).ToString(2)), LogLevel.NORMAL);
		Print(string.Format("%1 Total Network Intersections: %2", TAG, globalJunctions.Count()), LogLevel.NORMAL);
		Print(string.Format("%1 Output Directory: %2%3/roads/", TAG, baseDir, mapName), LogLevel.NORMAL);
		Print("==================================================================", LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	protected int GetLayerRecordCount(TBD_ERoadLayer layer)
	{
		switch (layer)
		{
			case TBD_ERoadLayer.HIGHWAY: return m_aHighways.Count();
			case TBD_ERoadLayer.PAVED:   return m_aPaved.Count();
			case TBD_ERoadLayer.DIRT:    return m_aDirt.Count();
			case TBD_ERoadLayer.TRACK:   return m_aTracks.Count();
			case TBD_ERoadLayer.PATH:    return m_aPaths.Count();
			case TBD_ERoadLayer.RUNWAY:  return m_aRunways.Count();
		}
		return 0;
	}

	//------------------------------------------------------------------------------------------------
	protected void AddRecordToLayer(TBD_ERoadLayer layer, TBD_RoadSegmentRecord rec)
	{
		switch (layer)
		{
			case TBD_ERoadLayer.HIGHWAY: m_aHighways.Insert(rec); break;
			case TBD_ERoadLayer.PAVED:   m_aPaved.Insert(rec); break;
			case TBD_ERoadLayer.DIRT:    m_aDirt.Insert(rec); break;
			case TBD_ERoadLayer.TRACK:   m_aTracks.Insert(rec); break;
			case TBD_ERoadLayer.PATH:    m_aPaths.Insert(rec); break;
			case TBD_ERoadLayer.RUNWAY:  m_aRunways.Insert(rec); break;
		}
	}

	//------------------------------------------------------------------------------------------------
	protected float CalcTotalLength(array<ref TBD_RoadSegmentRecord> records)
	{
		if (!records) return 0.0;
		float total = 0.0;
		for (int i = 0; i < records.Count(); i++)
		{
			total += records[i].m_fTotalLengthM;
		}
		return total;
	}

	//------------------------------------------------------------------------------------------------
	protected void BuildConnectivity(array<ref TBD_RoadSegmentRecord> records)
	{
		if (!records) return;
		int count = records.Count();
		int nodeCounter = 0;

		for (int i = 0; i < count; i++)
		{
			TBD_RoadSegmentRecord a = records[i];
			if (a.m_sStartNodeId.IsEmpty())
			{
				nodeCounter++;
				a.m_sStartNodeId = "node_" + nodeCounter.ToString();
				a.m_aStartConnectedSegments.Insert(a.m_sId);
			}
			if (a.m_sEndNodeId.IsEmpty())
			{
				nodeCounter++;
				a.m_sEndNodeId = "node_" + nodeCounter.ToString();
				a.m_aEndConnectedSegments.Insert(a.m_sId);
			}

			for (int j = i + 1; j < count; j++)
			{
				TBD_RoadSegmentRecord b = records[j];

				if (vector.Distance(a.m_vStartNodePos, b.m_vStartNodePos) <= CONNECTION_TOLERANCE_M)
				{
					b.m_sStartNodeId = a.m_sStartNodeId;
					if (a.m_aStartConnectedSegments.Find(b.m_sId) == -1) a.m_aStartConnectedSegments.Insert(b.m_sId);
					if (b.m_aStartConnectedSegments.Find(a.m_sId) == -1) b.m_aStartConnectedSegments.Insert(a.m_sId);
					if (a.m_aConnectedSegments.Find(b.m_sId) == -1) a.m_aConnectedSegments.Insert(b.m_sId);
					if (b.m_aConnectedSegments.Find(a.m_sId) == -1) b.m_aConnectedSegments.Insert(a.m_sId);
				}

				if (vector.Distance(a.m_vStartNodePos, b.m_vEndNodePos) <= CONNECTION_TOLERANCE_M)
				{
					b.m_sEndNodeId = a.m_sStartNodeId;
					if (a.m_aStartConnectedSegments.Find(b.m_sId) == -1) a.m_aStartConnectedSegments.Insert(b.m_sId);
					if (b.m_aEndConnectedSegments.Find(a.m_sId) == -1) b.m_aEndConnectedSegments.Insert(a.m_sId);
					if (a.m_aConnectedSegments.Find(b.m_sId) == -1) a.m_aConnectedSegments.Insert(b.m_sId);
					if (b.m_aConnectedSegments.Find(a.m_sId) == -1) b.m_aConnectedSegments.Insert(a.m_sId);
				}

				if (vector.Distance(a.m_vEndNodePos, b.m_vStartNodePos) <= CONNECTION_TOLERANCE_M)
				{
					b.m_sStartNodeId = a.m_sEndNodeId;
					if (a.m_aEndConnectedSegments.Find(b.m_sId) == -1) a.m_aEndConnectedSegments.Insert(b.m_sId);
					if (b.m_aStartConnectedSegments.Find(a.m_sId) == -1) b.m_aStartConnectedSegments.Insert(a.m_sId);
					if (a.m_aConnectedSegments.Find(b.m_sId) == -1) a.m_aConnectedSegments.Insert(b.m_sId);
					if (b.m_aConnectedSegments.Find(a.m_sId) == -1) b.m_aConnectedSegments.Insert(a.m_sId);
				}

				if (vector.Distance(a.m_vEndNodePos, b.m_vEndNodePos) <= CONNECTION_TOLERANCE_M)
				{
					b.m_sEndNodeId = a.m_sEndNodeId;
					if (a.m_aEndConnectedSegments.Find(b.m_sId) == -1) a.m_aEndConnectedSegments.Insert(b.m_sId);
					if (b.m_aEndConnectedSegments.Find(a.m_sId) == -1) b.m_aEndConnectedSegments.Insert(a.m_sId);
					if (a.m_aConnectedSegments.Find(b.m_sId) == -1) a.m_aConnectedSegments.Insert(b.m_sId);
					if (b.m_aConnectedSegments.Find(a.m_sId) == -1) b.m_aConnectedSegments.Insert(a.m_sId);
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void BuildGlobalJunctionGraph(
		array<ref TBD_RoadSegmentRecord> hwRecs,
		array<ref TBD_RoadSegmentRecord> pavedRecs,
		array<ref TBD_RoadSegmentRecord> dirtRecs,
		array<ref TBD_RoadSegmentRecord> trackRecs,
		array<ref TBD_RoadSegmentRecord> pathRecs,
		array<ref TBD_RoadSegmentRecord> runwayRecs,
		out array<ref TBD_RoadJunctionNode> outJunctions)
	{
		outJunctions = new array<ref TBD_RoadJunctionNode>();
		int juncCounter = 0;

		ref array<vector> endPositions = {};
		ref array<string> endSegIds = {};

		AppendEndpoints(hwRecs, endPositions, endSegIds);
		AppendEndpoints(pavedRecs, endPositions, endSegIds);
		AppendEndpoints(dirtRecs, endPositions, endSegIds);
		AppendEndpoints(trackRecs, endPositions, endSegIds);
		AppendEndpoints(pathRecs, endPositions, endSegIds);
		AppendEndpoints(runwayRecs, endPositions, endSegIds);

		int totalPts = endPositions.Count();
		for (int i = 0; i < totalPts; i++)
		{
			vector pt = endPositions[i];
			string segId = endSegIds[i];

			TBD_RoadJunctionNode matched = null;
			for (int j = 0; j < outJunctions.Count(); j++)
			{
				if (vector.Distance(pt, outJunctions[j].m_vPos) <= JUNCTION_TOLERANCE_M)
				{
					matched = outJunctions[j];
					break;
				}
			}

			if (!matched)
			{
				juncCounter++;
				matched = new TBD_RoadJunctionNode("junction_" + juncCounter.ToString(), pt);
				outJunctions.Insert(matched);
			}

			matched.AddSegment(segId);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void AppendEndpoints(array<ref TBD_RoadSegmentRecord> recs, array<vector> endPositions, array<string> endSegIds)
	{
		if (!recs) return;
		for (int i = 0; i < recs.Count(); i++)
		{
			TBD_RoadSegmentRecord r = recs[i];
			endPositions.Insert(r.m_vStartNodePos); endSegIds.Insert(r.m_sId);
			endPositions.Insert(r.m_vEndNodePos); endSegIds.Insert(r.m_sId);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteLayerJson(string baseDir, string mapName, string filename, string roadClass, float worldSize, array<ref TBD_RoadSegmentRecord> records)
	{
		string path = TBD_RoadExportPaths.BuildCategoryPath(baseDir, mapName, "roads", filename);
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("%1 ERROR: Failed to open output file: %2", TAG, path), LogLevel.ERROR);
			return;
		}

		int totalRecords = 0;
		float totalNetLengthM = 0.0;
		vector netBoundsMin = Vector(100000, 100000, 100000);
		vector netBoundsMax = Vector(-100000, -100000, -100000);

		if (records)
		{
			totalRecords = records.Count();
			for (int r = 0; r < totalRecords; r++)
			{
				TBD_RoadSegmentRecord rec = records[r];
				totalNetLengthM += rec.m_fTotalLengthM;
				if (rec.m_vBoundsMin[0] < netBoundsMin[0]) netBoundsMin[0] = rec.m_vBoundsMin[0];
				if (rec.m_vBoundsMin[1] < netBoundsMin[1]) netBoundsMin[1] = rec.m_vBoundsMin[1];
				if (rec.m_vBoundsMin[2] < netBoundsMin[2]) netBoundsMin[2] = rec.m_vBoundsMin[2];

				if (rec.m_vBoundsMax[0] > netBoundsMax[0]) netBoundsMax[0] = rec.m_vBoundsMax[0];
				if (rec.m_vBoundsMax[1] > netBoundsMax[1]) netBoundsMax[1] = rec.m_vBoundsMax[1];
				if (rec.m_vBoundsMax[2] > netBoundsMax[2]) netBoundsMax[2] = rec.m_vBoundsMax[2];
			}
		}

		string buf = "{\n";
		buf += "  \"type\": \"RoadTypeDataset\",\n";
		buf += "  \"roadClass\": \"" + TBD_RoadExportJson.Escape(roadClass) + "\",\n";
		buf += "  \"mapName\": \"" + TBD_RoadExportJson.Escape(mapName) + "\",\n";
		buf += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
		buf += "  \"totalSegments\": " + totalRecords.ToString() + ",\n";
		buf += "  \"totalLengthM\": " + totalNetLengthM.ToString() + ",\n";
		buf += "  \"bounds\": {\n";
		buf += "    \"min\": [" + netBoundsMin[0].ToString() + ", " + netBoundsMin[1].ToString() + ", " + netBoundsMin[2].ToString() + "],\n";
		buf += "    \"max\": [" + netBoundsMax[0].ToString() + ", " + netBoundsMax[1].ToString() + ", " + netBoundsMax[2].ToString() + "]\n";
		buf += "  },\n";
		buf += "  \"segments\": [\n";

		bool writeOk = true;

		for (int i = 0; i < totalRecords; i++)
		{
			TBD_RoadSegmentRecord seg = records[i];
			buf += "    {\n";
			buf += "      \"id\": \"" + TBD_RoadExportJson.Escape(seg.m_sId) + "\",\n";
			buf += "      \"name\": \"" + TBD_RoadExportJson.Escape(seg.m_sName) + "\",\n";
			buf += "      \"roadClass\": \"" + TBD_RoadExportJson.Escape(seg.m_sRoadClass) + "\",\n";
			buf += "      \"widthM\": " + seg.m_fWidthM.ToString() + ",\n";
			buf += "      \"totalLengthM\": " + seg.m_fTotalLengthM.ToString() + ",\n";
			buf += "      \"pointsCount\": " + seg.m_aPoints.Count().ToString() + ",\n";
			buf += "      \"points\": [\n";

			for (int pt = 0; pt < seg.m_aPoints.Count(); pt++)
			{
				vector p = seg.m_aPoints[pt];
				buf += "        [" + p[0].ToString() + ", " + p[1].ToString() + ", " + p[2].ToString() + "]";
				if (pt < seg.m_aPoints.Count() - 1) buf += ",";
				buf += "\n";
			}
			buf += "      ],\n";

			buf += "      \"bounds\": {\n";
			buf += "        \"min\": [" + seg.m_vBoundsMin[0].ToString() + ", " + seg.m_vBoundsMin[1].ToString() + ", " + seg.m_vBoundsMin[2].ToString() + "],\n";
			buf += "        \"max\": [" + seg.m_vBoundsMax[0].ToString() + ", " + seg.m_vBoundsMax[1].ToString() + ", " + seg.m_vBoundsMax[2].ToString() + "]\n";
			buf += "      },\n";

			buf += "      \"startNode\": {\n";
			buf += "        \"nodeId\": \"" + TBD_RoadExportJson.Escape(seg.m_sStartNodeId) + "\",\n";
			buf += "        \"pos\": [" + seg.m_vStartNodePos[0].ToString() + ", " + seg.m_vStartNodePos[1].ToString() + ", " + seg.m_vStartNodePos[2].ToString() + "],\n";
			buf += "        \"connectedSegmentIds\": [";
			for (int s0 = 0; s0 < seg.m_aStartConnectedSegments.Count(); s0++)
			{
				buf += "\"" + TBD_RoadExportJson.Escape(seg.m_aStartConnectedSegments[s0]) + "\"";
				if (s0 < seg.m_aStartConnectedSegments.Count() - 1) buf += ", ";
			}
			buf += "]\n";
			buf += "      },\n";

			buf += "      \"endNode\": {\n";
			buf += "        \"nodeId\": \"" + TBD_RoadExportJson.Escape(seg.m_sEndNodeId) + "\",\n";
			buf += "        \"pos\": [" + seg.m_vEndNodePos[0].ToString() + ", " + seg.m_vEndNodePos[1].ToString() + ", " + seg.m_vEndNodePos[2].ToString() + "],\n";
			buf += "        \"connectedSegmentIds\": [";
			for (int s1 = 0; s1 < seg.m_aEndConnectedSegments.Count(); s1++)
			{
				buf += "\"" + TBD_RoadExportJson.Escape(seg.m_aEndConnectedSegments[s1]) + "\"";
				if (s1 < seg.m_aEndConnectedSegments.Count() - 1) buf += ", ";
			}
			buf += "]\n";
			buf += "      },\n";

			buf += "      \"connectedSegmentIds\": [";
			for (int sc = 0; sc < seg.m_aConnectedSegments.Count(); sc++)
			{
				buf += "\"" + TBD_RoadExportJson.Escape(seg.m_aConnectedSegments[sc]) + "\"";
				if (sc < seg.m_aConnectedSegments.Count() - 1) buf += ", ";
			}
			buf += "],\n";

			buf += "      \"prefab\": \"" + TBD_RoadExportJson.Escape(seg.m_sPrefab) + "\",\n";
			buf += "      \"material\": \"" + TBD_RoadExportJson.Escape(seg.m_sMaterial) + "\"\n";
			buf += "    }";

			if (i < totalRecords - 1)
				buf += ",";
			buf += "\n";

			if (buf.Length() > FLUSH_BUF_SIZE)
			{
				writeOk = TBD_RoadExportJson.Write(f, buf, TAG);
				if (!writeOk) break;
				buf = "";
			}
		}

		if (writeOk)
		{
			buf += "  ]\n}\n";
			writeOk = TBD_RoadExportJson.Write(f, buf, TAG);
		}

		f.Close();
		Print(string.Format("%1 Wrote %2 (%3 segments, %4 m) -> %5", TAG, filename, totalRecords, totalNetLengthM.ToString(1), path), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteRoadsMetaJson(
		string baseDir,
		string mapName,
		float worldSize,
		array<ref TBD_RoadSegmentRecord> hwRecs,
		array<ref TBD_RoadSegmentRecord> pavedRecs,
		array<ref TBD_RoadSegmentRecord> dirtRecs,
		array<ref TBD_RoadSegmentRecord> trackRecs,
		array<ref TBD_RoadSegmentRecord> pathRecs,
		array<ref TBD_RoadSegmentRecord> runwayRecs,
		array<ref TBD_RoadJunctionNode> junctions,
		int elapsedMs)
	{
		string path = TBD_RoadExportPaths.BuildCategoryPath(baseDir, mapName, "roads", "roads_meta.json");
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("%1 ERROR: Failed to open metadata file: %2", TAG, path), LogLevel.ERROR);
			return;
		}

		int hwCount = 0; float hwLenM = 0.0;
		if (hwRecs) { hwCount = hwRecs.Count(); for (int h = 0; h < hwCount; h++) hwLenM += hwRecs[h].m_fTotalLengthM; }

		int pavedCount = 0; float pavedLenM = 0.0;
		if (pavedRecs) { pavedCount = pavedRecs.Count(); for (int pv = 0; pv < pavedCount; pv++) pavedLenM += pavedRecs[pv].m_fTotalLengthM; }

		int dirtCount = 0; float dirtLenM = 0.0;
		if (dirtRecs) { dirtCount = dirtRecs.Count(); for (int d = 0; d < dirtCount; d++) dirtLenM += dirtRecs[d].m_fTotalLengthM; }

		int trackCount = 0; float trackLenM = 0.0;
		if (trackRecs) { trackCount = trackRecs.Count(); for (int t = 0; t < trackCount; t++) trackLenM += trackRecs[t].m_fTotalLengthM; }

		int pathCount = 0; float pathLenM = 0.0;
		if (pathRecs) { pathCount = pathRecs.Count(); for (int p = 0; p < pathCount; p++) pathLenM += pathRecs[p].m_fTotalLengthM; }

		int runwayCount = 0; float runwayLenM = 0.0;
		if (runwayRecs) { runwayCount = runwayRecs.Count(); for (int r = 0; r < runwayCount; r++) runwayLenM += runwayRecs[r].m_fTotalLengthM; }

		int totalSegs = hwCount + pavedCount + dirtCount + trackCount + pathCount + runwayCount;
		float totalLenM = hwLenM + pavedLenM + dirtLenM + trackLenM + pathLenM + runwayLenM;

		int totalJuncs = junctions.Count();
		int deadEnds = 0;
		int intersections2Way = 0;
		int intersections3Way = 0;
		int intersections4Way = 0;
		int complexJunctions = 0;

		for (int j = 0; j < totalJuncs; j++)
		{
			int deg = junctions[j].m_aConnectedSegments.Count();
			if (deg <= 1) deadEnds++;
			else if (deg == 2) intersections2Way++;
			else if (deg == 3) intersections3Way++;
			else if (deg == 4) intersections4Way++;
			else complexJunctions++;
		}

		string buf = "{\n";
		buf += "  \"type\": \"RoadNetworkMetadataManifest\",\n";
		buf += "  \"mapName\": \"" + TBD_RoadExportJson.Escape(mapName) + "\",\n";
		buf += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
		buf += "  \"totalSegments\": " + totalSegs.ToString() + ",\n";
		buf += "  \"totalNetworkLengthM\": " + totalLenM.ToString() + ",\n";
		buf += "  \"totalNetworkLengthKm\": " + (totalLenM / 1000.0).ToString() + ",\n";

		buf += "  \"topology\": {\n";
		buf += "    \"totalNodes\": " + totalJuncs.ToString() + ",\n";
		buf += "    \"terminalDeadEnds\": " + deadEnds.ToString() + ",\n";
		buf += "    \"continuous2WayJoints\": " + intersections2Way.ToString() + ",\n";
		buf += "    \"intersections3Way\": " + intersections3Way.ToString() + ",\n";
		buf += "    \"intersections4Way\": " + intersections4Way.ToString() + ",\n";
		buf += "    \"complexIntersections\": " + complexJunctions.ToString() + "\n";
		buf += "  },\n";

		buf += "  \"layers\": {\n";
		buf += "    \"highways\": {\n      \"file\": \"highways.json\",\n      \"roadClass\": \"highway_paved\",\n      \"segmentsCount\": " + hwCount.ToString() + ",\n      \"totalLengthM\": " + hwLenM.ToString() + "\n    },\n";
		buf += "    \"roads_paved\": {\n      \"file\": \"roads_paved.json\",\n      \"roadClass\": \"road_paved\",\n      \"segmentsCount\": " + pavedCount.ToString() + ",\n      \"totalLengthM\": " + pavedLenM.ToString() + "\n    },\n";
		buf += "    \"roads_dirt\": {\n      \"file\": \"roads_dirt.json\",\n      \"roadClass\": \"road_dirt\",\n      \"segmentsCount\": " + dirtCount.ToString() + ",\n      \"totalLengthM\": " + dirtLenM.ToString() + "\n    },\n";
		buf += "    \"tracks\": {\n      \"file\": \"tracks.json\",\n      \"roadClass\": \"track\",\n      \"segmentsCount\": " + trackCount.ToString() + ",\n      \"totalLengthM\": " + trackLenM.ToString() + "\n    },\n";
		buf += "    \"paths\": {\n      \"file\": \"paths.json\",\n      \"roadClass\": \"path\",\n      \"segmentsCount\": " + pathCount.ToString() + ",\n      \"totalLengthM\": " + pathLenM.ToString() + "\n    },\n";
		buf += "    \"runways\": {\n      \"file\": \"runways.json\",\n      \"roadClass\": \"runway\",\n      \"segmentsCount\": " + runwayCount.ToString() + ",\n      \"totalLengthM\": " + runwayLenM.ToString() + "\n    }\n";
		buf += "  },\n";

		buf += "  \"junctions\": [\n";
		bool firstJunc = true;
		for (int ji = 0; ji < totalJuncs; ji++)
		{
			TBD_RoadJunctionNode jn = junctions[ji];
			if (jn.m_aConnectedSegments.Count() < 2)
				continue;

			if (!firstJunc) buf += ",\n";
			firstJunc = false;

			buf += "    {\n";
			buf += "      \"id\": \"" + TBD_RoadExportJson.Escape(jn.m_sId) + "\",\n";
			buf += "      \"pos\": [" + jn.m_vPos[0].ToString() + ", " + jn.m_vPos[1].ToString() + ", " + jn.m_vPos[2].ToString() + "],\n";
			buf += "      \"degree\": " + jn.m_aConnectedSegments.Count().ToString() + ",\n";
			buf += "      \"connectedSegments\": [";
			for (int cs = 0; cs < jn.m_aConnectedSegments.Count(); cs++)
			{
				buf += "\"" + TBD_RoadExportJson.Escape(jn.m_aConnectedSegments[cs]) + "\"";
				if (cs < jn.m_aConnectedSegments.Count() - 1) buf += ", ";
			}
			buf += "]\n";
			buf += "    }";

			if (buf.Length() > FLUSH_BUF_SIZE)
			{
				TBD_RoadExportJson.Write(f, buf, TAG);
				buf = "";
			}
		}

		buf += "\n  ],\n";
		buf += "  \"elapsedMs\": " + elapsedMs.ToString() + "\n";
		buf += "}\n";

		TBD_RoadExportJson.Write(f, buf, TAG);
		f.Close();
		Print(string.Format("%1 Wrote roads_meta.json -> %2", TAG, path), LogLevel.NORMAL);
	}
}
