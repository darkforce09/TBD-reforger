/**
 * TBD_MapExportHighways.c
 *
 * Dedicated Highway & Major Arterial road network extraction engine for Bohemia Reforger.
 * Queries Enfusion Engine's native RoadNetworkManager and BaseRoad splines,
 * extracts authentic 100% continuous 3D centerline curve vertices (GetPoints),
 * classifies asphalt highway materials and primary road widths,
 * establishes topological endpoint graph connectivity, and stream-writes highways.json.
 *
 * Outputs:
 *   - $profile:TBD_Export/<mapName>/roads/highways.json
 */

class TBD_HighwayRecord
{
	int m_iId;
	string m_sId;
	string m_sName;
	string m_sRoadClass;
	float m_fWidthM;
	float m_fTotalLengthM;
	vector m_vBoundsMin;
	vector m_vBoundsMax;
	ref array<vector> m_aPoints;
	string m_sPrefab;
	string m_sMaterial;

	string m_sStartNodeId;
	vector m_vStartNodePos;
	ref array<string> m_aStartConnectedSegments;

	string m_sEndNodeId;
	vector m_vEndNodePos;
	ref array<string> m_aEndConnectedSegments;

	ref array<string> m_aConnectedSegments;

	void TBD_HighwayRecord(int id, string name, float widthM, string prefab, string mat = "")
	{
		m_iId = id;
		m_sId = "highway_" + id.ToString();
		m_sName = name;
		m_sRoadClass = "highway_paved";
		m_fWidthM = widthM;
		m_fTotalLengthM = 0.0;
		m_vBoundsMin = Vector(100000, 100000, 100000);
		m_vBoundsMax = Vector(-100000, -100000, -100000);
		m_aPoints = {};
		m_sPrefab = prefab;
		m_sMaterial = mat;
		m_sStartNodeId = "";
		m_vStartNodePos = Vector(0, 0, 0);
		m_aStartConnectedSegments = {};
		m_sEndNodeId = "";
		m_vEndNodePos = Vector(0, 0, 0);
		m_aEndConnectedSegments = {};
		m_aConnectedSegments = {};
	}

	void AddPoint(vector ptWS)
	{
		if (m_aPoints.Count() > 0)
		{
			vector prev = m_aPoints[m_aPoints.Count() - 1];
			m_fTotalLengthM += vector.Distance(prev, ptWS);
		}

		m_aPoints.Insert(ptWS);

		if (ptWS[0] < m_vBoundsMin[0]) m_vBoundsMin[0] = ptWS[0];
		if (ptWS[1] < m_vBoundsMin[1]) m_vBoundsMin[1] = ptWS[1];
		if (ptWS[2] < m_vBoundsMin[2]) m_vBoundsMin[2] = ptWS[2];

		if (ptWS[0] > m_vBoundsMax[0]) m_vBoundsMax[0] = ptWS[0];
		if (ptWS[1] > m_vBoundsMax[1]) m_vBoundsMax[1] = ptWS[1];
		if (ptWS[2] > m_vBoundsMax[2]) m_vBoundsMax[2] = ptWS[2];
	}
}

class TBD_HighwayRawEntityInfo
{
	IEntity m_Entity;
	vector m_vPos;
	float m_fWidthM;
	string m_sPrefab;
	string m_sMaterial;
	bool m_bVisited;

	void TBD_HighwayRawEntityInfo(IEntity ent, vector pos, float widthM, string prefab, string mat)
	{
		m_Entity = ent;
		m_vPos = pos;
		m_fWidthM = widthM;
		m_sPrefab = prefab;
		m_sMaterial = mat;
		m_bVisited = false;
	}
}

class TBD_MapExportHighways
{
	protected static const string TAG = "[TBD][Roads][Highways]";
	protected static const float Y_MIN = -500.0;
	protected static const float Y_MAX = 1500.0;
	protected static const int FLUSH = 8000;
	protected static const float CONNECTION_TOLERANCE_M = 5.0;
	protected static const float MAX_CHAIN_STEP_M = 200.0;

	protected ref array<IEntity> m_aHits;
	protected ref array<ref TBD_HighwayRecord> m_aRecords;
	protected ref array<ref TBD_HighwayRawEntityInfo> m_aHighwayEntities;
	protected ref array<BaseRoad> m_aProcessedRoads;
	protected ChimeraAIWorld m_CachedAIWorld;

	//------------------------------------------------------------------------------------------------
	protected bool CollectEntity(IEntity e)
	{
		if (e)
			m_aHits.Insert(e);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected int CellIndex(float coord, float cellM, int cells)
	{
		int c = Math.Floor(coord / cellM);
		if (c < 0) c = 0;
		if (c > cells - 1) c = cells - 1;
		return c;
	}

	//------------------------------------------------------------------------------------------------
	protected ChimeraAIWorld FindAIWorldRecursive(IEntitySource src, WorldEditorAPI api, int depth = 0)
	{
		if (!src || depth > 10)
			return null;

		string cls = src.GetClassName();
		if (cls == "SCR_AIWorld" || cls == "ChimeraAIWorld" || cls == "AIWorld" || cls.Contains("AIWorld"))
		{
			IEntity ent = api.SourceToEntity(src);
			if (ent)
			{
				ChimeraAIWorld cw = ChimeraAIWorld.Cast(ent);
				if (cw)
				{
					Print(string.Format("%1 Found %2 entity in editor tree (depth=%3)", TAG, cls, depth), LogLevel.NORMAL);
					return cw;
				}
			}
		}

		int nChildren = src.GetNumChildren();
		for (int i = 0; i < nChildren; i++)
		{
			IEntitySource child = src.GetChild(i);
			ChimeraAIWorld found = FindAIWorldRecursive(child, api, depth + 1);
			if (found)
				return found;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! Resolves the engine's RoadNetworkManager via ChimeraAIWorld or SCR_AIWorld.
	protected RoadNetworkManager ResolveRoadNetworkManager(TBD_MapExportContext ctx)
	{
		m_CachedAIWorld = null;

		// 1. Try GetGame().GetAIWorld()
		if (GetGame())
		{
			AIWorld gw = GetGame().GetAIWorld();
			if (gw)
			{
				m_CachedAIWorld = ChimeraAIWorld.Cast(gw);
				if (m_CachedAIWorld)
					Print(TAG + " Found ChimeraAIWorld via GetGame().GetAIWorld()", LogLevel.NORMAL);
			}
		}

		// 2. Try recursive editor entity tree search
		if (!m_CachedAIWorld && ctx && ctx.m_API)
		{
			int rootCount = ctx.m_API.GetEditorEntityCount();
			Print(string.Format("%1 Searching %2 root editor entities recursively for AIWorld...", TAG, rootCount), LogLevel.NORMAL);
			for (int i = 0; i < rootCount; i++)
			{
				IEntitySource src = ctx.m_API.GetEditorEntity(i);
				if (!src)
					continue;
				m_CachedAIWorld = FindAIWorldRecursive(src, ctx.m_API);
				if (m_CachedAIWorld)
					break;
			}
		}



		if (m_CachedAIWorld)
		{
			RoadNetworkManager rnm = m_CachedAIWorld.GetRoadNetworkManager();
			if (rnm)
			{
				Print(TAG + " Successfully resolved RoadNetworkManager from ChimeraAIWorld.", LogLevel.NORMAL);
				return rnm;
			}
		}

		Print(TAG + " Notice: Native RoadNetworkManager not active in current edit context; using direct RoadEntity geometric graph.", LogLevel.NORMAL);
		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! Strict Highway classifier matching asphalt materials & major arterial road classes.
	static bool IsHighwayEntity(string resName, string className, IEntity ent, IEntitySource src, out float widthM, out string roadName, out string matName)
	{
		widthM = 8.0;
		roadName = "";
		matName = "asphalt";

		if (!ent)
			return false;

		if (ent.FindComponent(SCR_MapDescriptorComponent) || ent.FindComponent(MapDescriptorComponent))
			return false;
		if (ent.FindComponent(SCR_EditableCommentComponent))
			return false;

		string lowerRes = resName;
		lowerRes.ToLower();
		string lowerClass = className;
		lowerClass.ToLower();

		// Exclude vegetation, rocks, water, props, fences, and utility lines
		if (lowerRes.Contains("/vegetation/") || lowerRes.Contains("/tree/") || lowerRes.Contains("/rocks/") || lowerRes.Contains("/water/"))
			return false;
		if (lowerRes.Contains("/props/") || lowerRes.Contains("/signs/") || lowerRes.Contains("lamp") || lowerRes.Contains("barrier") || lowerRes.Contains("traffic_"))
			return false;
		if (lowerRes.Contains("/fence") || lowerRes.Contains("/powerline") || lowerRes.Contains("/pylon"))
			return false;
		if (lowerRes.Contains("hydrant") || lowerRes.Contains("radiobroadcast"))
			return false;
		if (lowerRes.Contains("/decals/") || lowerRes.Contains("decal_") || lowerRes.Contains("flowerbed") || lowerRes.Contains("naturedebris") || lowerRes.Contains("concretepanelrow"))
			return false;

		// Exclude narrow pavements unless wide 8m asphalt
		if (lowerRes.Contains("/pavements/") && !lowerRes.Contains("wide_8m"))
			return false;

		string rawMat = "";
		float rawWidth = 0.0;
		if (src)
		{
			src.Get("Material", rawMat);
			src.Get("Width", rawWidth);
		}

		string lowerMat = rawMat;
		lowerMat.ToLower();

		if (lowerMat.Contains("/decals/") || lowerMat.Contains("decal_") || lowerMat.Contains("flowerbed") || lowerMat.Contains("beach_naturedebris") || lowerMat.Contains("concretepanelrow"))
			return false;

		if (rawWidth > 0.5)
			widthM = rawWidth;
		if (!rawMat.IsEmpty())
			matName = rawMat;

		// Main asphalt highway with dashed center line
		if (lowerMat.Contains("dashedline") || lowerMat.Contains("road_asphalt_e_01"))
		{
			if (widthM < 8.0) widthM = 8.0;
			roadName = ent.GetName();
			return true;
		}

		// Primary 2-lane asphalt highway (width >= 7.5m)
		if (lowerMat.Contains("road_asphalt_e_02") && widthM >= 7.5)
		{
			roadName = ent.GetName();
			return true;
		}

		// Broad arterial asphalt road (width >= 8.0m)
		if (lowerMat.Contains("road_asphalt_e_03") && widthM >= 8.0)
		{
			roadName = ent.GetName();
			return true;
		}

		// Explicit highway resource naming
		if (lowerRes.Contains("highway") || lowerRes.Contains("mainroad") || lowerRes.Contains("asphalt_wide") || lowerRes.Contains("wide_8m"))
		{
			if (widthM < 8.0) widthM = 8.0;
			roadName = ent.GetName();
			return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Primary highways export execution method.
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg, out array<ref TBD_HighwayRecord> outRecords = null)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		int tick0 = System.GetTickCount();
		string mapName = ctx.GetMapName(cfg);
		float worldSize = ctx.m_fWorldSize;
		float cellM = cfg.m_fObjectChunkSizeM;
		if (cellM <= 10.0)
			cellM = 512.0;

		int cells = Math.Ceil(worldSize / cellM);
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "roads", "highways.json");

		Print(string.Format("%1 Scanning highway network for '%2' (%3x%3 cells @ %4 m)...",
			TAG, mapName, cells, cellM), LogLevel.NORMAL);

		m_aRecords = new array<ref TBD_HighwayRecord>();
		m_aHighwayEntities = new array<ref TBD_HighwayRawEntityInfo>();
		m_aProcessedRoads = new array<BaseRoad>();

		RoadNetworkManager rnm = ResolveRoadNetworkManager(ctx);

		// 1. Gather all authentic highway RoadEntity waypoints
		int probeCount = 0;
		for (int iz = 0; iz < cells; iz++)
		{
			for (int ix = 0; ix < cells; ix++)
			{
				float x0 = ix * cellM;
				float z0 = iz * cellM;
				m_aHits = {};
				vector mins = Vector(x0, Y_MIN, z0);
				vector maxs = Vector(x0 + cellM, Y_MAX, z0 + cellM);
				ctx.m_World.QueryEntitiesByAABB(mins, maxs, CollectEntity);

				foreach (IEntity e : m_aHits)
				{
					if (!e)
						continue;

					vector pos = e.GetOrigin();
					if (pos[0] < 0 || pos[0] > worldSize || pos[2] < 0 || pos[2] > worldSize)
						continue;
					if (CellIndex(pos[0], cellM, cells) != ix || CellIndex(pos[2], cellM, cells) != iz)
						continue;

					string rn = ctx.ResolvePrefab(e);
					string clsName = e.ClassName();
					IEntitySource src = ctx.m_API.EntityToSource(e);

					float widthM;
					string roadName, matName;
					if (!IsHighwayEntity(rn, clsName, e, src, widthM, roadName, matName))
						continue;

					m_aHighwayEntities.Insert(new TBD_HighwayRawEntityInfo(e, pos, widthM, rn, matName));

					// Print diagnostic probe on first 2 highway entities
					if (probeCount < 2 && src)
					{
						probeCount++;
						int numVars = src.GetNumVars();
						string varDump = "";
						for (int v = 0; v < numVars; v++)
						{
							string vn = src.GetVarName(v);
							string vv = "";
							src.Get(vn, vv);
							varDump += vn + "='" + vv + "'; ";
						}
						Print(string.Format("%1 Probe RoadEntity #%2: class=%3, numChildren=%4, vars=[%5]",
							TAG, probeCount, clsName, src.GetNumChildren(), varDump), LogLevel.NORMAL);
					}
				}
			}
		}

		Print(string.Format("%1 Identified %2 authored highway entity waypoints across world.",
			TAG, m_aHighwayEntities.Count()), LogLevel.NORMAL);

		// 2. Extract continuous BaseRoad splines from RoadNetworkManager if available
		if (rnm)
		{
			foreach (TBD_HighwayRawEntityInfo hEnt : m_aHighwayEntities)
			{
				BaseRoad foundRoad = null;
				float dist = 0.0;
				int queryRes = rnm.GetClosestRoad(hEnt.m_vPos, foundRoad, dist, true);

				if (foundRoad && dist <= 25.0)
				{
					if (m_aProcessedRoads.Find(foundRoad) != -1)
						continue;

					m_aProcessedRoads.Insert(foundRoad);

					ref array<vector> roadPoints = {};
					foundRoad.GetPoints(roadPoints);

					if (roadPoints.Count() >= 2)
					{
						float roadWidth = foundRoad.GetWidth();
						if (roadWidth < 1.0)
							roadWidth = hEnt.m_fWidthM;

						int recId = m_aRecords.Count() + 1;
						TBD_HighwayRecord rec = new TBD_HighwayRecord(recId, "Highway_" + recId.ToString(), roadWidth, hEnt.m_sPrefab, hEnt.m_sMaterial);

						for (int pt = 0; pt < roadPoints.Count(); pt++)
						{
							rec.AddPoint(roadPoints[pt]);
						}

						if (rec.m_aPoints.Count() >= 2)
						{
							rec.m_vStartNodePos = rec.m_aPoints[0];
							rec.m_vEndNodePos = rec.m_aPoints[rec.m_aPoints.Count() - 1];
							m_aRecords.Insert(rec);
						}
					}
				}
			}

			Print(string.Format("%1 Extracted %2 BaseRoad segments from RoadNetworkManager.", TAG, m_aRecords.Count()), LogLevel.NORMAL);
		}

		// 3. Fallback geometric polyline construction from authored waypoints if RoadNetworkManager yielded 0
		if (m_aRecords.Count() == 0 && m_aHighwayEntities.Count() > 0)
		{
			Print(TAG + " Building continuous highway polyline ribbons from authored waypoints...", LogLevel.NORMAL);
			BuildHighwayPolylinesFromEntities();
		}

		float totalNetLengthM = 0.0;
		vector netBoundsMin = Vector(100000, 100000, 100000);
		vector netBoundsMax = Vector(-100000, -100000, -100000);

		for (int r = 0; r < m_aRecords.Count(); r++)
		{
			TBD_HighwayRecord rec = m_aRecords[r];
			totalNetLengthM += rec.m_fTotalLengthM;
			if (rec.m_vBoundsMin[0] < netBoundsMin[0]) netBoundsMin[0] = rec.m_vBoundsMin[0];
			if (rec.m_vBoundsMin[1] < netBoundsMin[1]) netBoundsMin[1] = rec.m_vBoundsMin[1];
			if (rec.m_vBoundsMin[2] < netBoundsMin[2]) netBoundsMin[2] = rec.m_vBoundsMin[2];

			if (rec.m_vBoundsMax[0] > netBoundsMax[0]) netBoundsMax[0] = rec.m_vBoundsMax[0];
			if (rec.m_vBoundsMax[1] > netBoundsMax[1]) netBoundsMax[1] = rec.m_vBoundsMax[1];
			if (rec.m_vBoundsMax[2] > netBoundsMax[2]) netBoundsMax[2] = rec.m_vBoundsMax[2];
		}

		int totalRecords = m_aRecords.Count();
		Print(string.Format("%1 Extracted %2 continuous highway segments (Total length: %3 m). Writing -> %4",
			TAG, totalRecords, totalNetLengthM.ToString(1), outJson), LogLevel.NORMAL);

		// Build intra-layer endpoint connectivity
		BuildConnectivity(m_aRecords);

		// Stream write JSON
		FileHandle f = FileIO.OpenFile(outJson, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open output file: " + outJson, LogLevel.ERROR);
			return false;
		}

		string buf = "{\n";
		buf += "  \"type\": \"RoadTypeDataset\",\n";
		buf += "  \"roadClass\": \"highway_paved\",\n";
		buf += "  \"mapName\": \"" + TBD_MapExportJson.Escape(mapName) + "\",\n";
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
			TBD_HighwayRecord hr = m_aRecords[i];
			buf += "    {\n";
			buf += "      \"id\": \"" + TBD_MapExportJson.Escape(hr.m_sId) + "\",\n";
			buf += "      \"name\": \"" + TBD_MapExportJson.Escape(hr.m_sName) + "\",\n";
			buf += "      \"roadClass\": \"" + TBD_MapExportJson.Escape(hr.m_sRoadClass) + "\",\n";
			buf += "      \"widthM\": " + hr.m_fWidthM.ToString() + ",\n";
			buf += "      \"totalLengthM\": " + hr.m_fTotalLengthM.ToString() + ",\n";
			buf += "      \"pointsCount\": " + hr.m_aPoints.Count().ToString() + ",\n";
			buf += "      \"points\": [\n";

			for (int pt = 0; pt < hr.m_aPoints.Count(); pt++)
			{
				vector p = hr.m_aPoints[pt];
				buf += "        [" + p[0].ToString() + ", " + p[1].ToString() + ", " + p[2].ToString() + "]";
				if (pt < hr.m_aPoints.Count() - 1) buf += ",";
				buf += "\n";
			}
			buf += "      ],\n";

			buf += "      \"bounds\": {\n";
			buf += "        \"min\": [" + hr.m_vBoundsMin[0].ToString() + ", " + hr.m_vBoundsMin[0 + 1].ToString() + ", " + hr.m_vBoundsMin[2].ToString() + "],\n";
			buf += "        \"max\": [" + hr.m_vBoundsMax[0].ToString() + ", " + hr.m_vBoundsMax[0 + 1].ToString() + ", " + hr.m_vBoundsMax[2].ToString() + "]\n";
			buf += "      },\n";

			// Graph Connectivity
			buf += "      \"startNode\": {\n";
			buf += "        \"nodeId\": \"" + TBD_MapExportJson.Escape(hr.m_sStartNodeId) + "\",\n";
			buf += "        \"pos\": [" + hr.m_vStartNodePos[0].ToString() + ", " + hr.m_vStartNodePos[1].ToString() + ", " + hr.m_vStartNodePos[2].ToString() + "],\n";
			buf += "        \"connectedSegmentIds\": [";
			for (int s0 = 0; s0 < hr.m_aStartConnectedSegments.Count(); s0++)
			{
				buf += "\"" + TBD_MapExportJson.Escape(hr.m_aStartConnectedSegments[s0]) + "\"";
				if (s0 < hr.m_aStartConnectedSegments.Count() - 1) buf += ", ";
			}
			buf += "]\n";
			buf += "      },\n";

			buf += "      \"endNode\": {\n";
			buf += "        \"nodeId\": \"" + TBD_MapExportJson.Escape(hr.m_sEndNodeId) + "\",\n";
			buf += "        \"pos\": [" + hr.m_vEndNodePos[0].ToString() + ", " + hr.m_vEndNodePos[1].ToString() + ", " + hr.m_vEndNodePos[2].ToString() + "],\n";
			buf += "        \"connectedSegmentIds\": [";
			for (int s1 = 0; s1 < hr.m_aEndConnectedSegments.Count(); s1++)
			{
				buf += "\"" + TBD_MapExportJson.Escape(hr.m_aEndConnectedSegments[s1]) + "\"";
				if (s1 < hr.m_aEndConnectedSegments.Count() - 1) buf += ", ";
			}
			buf += "]\n";
			buf += "      },\n";

			buf += "      \"connectedSegmentIds\": [";
			for (int sc = 0; sc < hr.m_aConnectedSegments.Count(); sc++)
			{
				buf += "\"" + TBD_MapExportJson.Escape(hr.m_aConnectedSegments[sc]) + "\"";
				if (sc < hr.m_aConnectedSegments.Count() - 1) buf += ", ";
			}
			buf += "],\n";

			buf += "      \"prefab\": \"" + TBD_MapExportJson.Escape(hr.m_sPrefab) + "\",\n";
			buf += "      \"material\": \"" + TBD_MapExportJson.Escape(hr.m_sMaterial) + "\"\n";
			buf += "    }";

			if (i < totalRecords - 1)
				buf += ",";
			buf += "\n";

			if (buf.Length() > FLUSH)
			{
				writeOk = TBD_MapExportJson.Write(f, buf, TAG);
				if (!writeOk) break;
				buf = "";
			}
		}

		if (writeOk)
		{
			buf += "  ]\n}\n";
			writeOk = TBD_MapExportJson.Write(f, buf, TAG);
		}

		f.Close();

		if (outRecords)
			outRecords = m_aRecords;

		int elapsedMs = System.GetTickCount() - tick0;
		Print(string.Format("%1 HIGHWAYS EXPORT FINISHED in %2 ms (Total=%3 continuous routes, %4 m) -> %5",
			TAG, elapsedMs, totalRecords, totalNetLengthM.ToString(1), outJson), LogLevel.NORMAL);

		return writeOk;
	}

	//------------------------------------------------------------------------------------------------
	//! Chains unvisited highway waypoints into smooth continuous polyline ribbons (pure highway entities only).
	protected void BuildHighwayPolylinesFromEntities()
	{
		if (!m_aHighwayEntities) return;
		int totalWp = m_aHighwayEntities.Count();

		for (int i = 0; i < totalWp; i++)
		{
			TBD_HighwayRawEntityInfo curWp = m_aHighwayEntities[i];
			if (curWp.m_bVisited) continue;

			int recId = m_aRecords.Count() + 1;
			TBD_HighwayRecord rec = new TBD_HighwayRecord(recId, "Highway_" + recId.ToString(), curWp.m_fWidthM, curWp.m_sPrefab, curWp.m_sMaterial);
			rec.AddPoint(curWp.m_vPos);
			curWp.m_bVisited = true;

			vector lastPos = curWp.m_vPos;
			vector lastDir = Vector(0, 0, 0);
			bool foundNext = true;

			while (foundNext)
			{
				foundNext = false;
				float bestScore = 100000.0;
				int bestIdx = -1;

				for (int j = 0; j < totalWp; j++)
				{
					TBD_HighwayRawEntityInfo cand = m_aHighwayEntities[j];
					if (cand.m_bVisited) continue;

					float d = vector.Distance(lastPos, cand.m_vPos);
					if (d <= MAX_CHAIN_STEP_M)
					{
						vector candDir = cand.m_vPos - lastPos;
						candDir.Normalize();

						float anglePenalty = 0.0;
						if (lastDir.Length() > 0.1)
						{
							float dot = vector.Dot(lastDir, candDir);
							if (dot < -0.2) continue; // Reject sharp reverse hooks
							anglePenalty = (1.0 - dot) * 30.0;
						}

						float score = d + anglePenalty;
						if (score < bestScore)
						{
							bestScore = score;
							bestIdx = j;
						}
					}
				}

				if (bestIdx != -1)
				{
					TBD_HighwayRawEntityInfo nextWp = m_aHighwayEntities[bestIdx];
					rec.AddPoint(nextWp.m_vPos);
					nextWp.m_bVisited = true;
					lastDir = nextWp.m_vPos - lastPos;
					lastDir.Normalize();
					lastPos = nextWp.m_vPos;
					foundNext = true;
				}
			}

			if (rec.m_aPoints.Count() >= 2)
			{
				rec.m_vStartNodePos = rec.m_aPoints[0];
				rec.m_vEndNodePos = rec.m_aPoints[rec.m_aPoints.Count() - 1];
				m_aRecords.Insert(rec);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Establishes endpoint graph connectivity across road segments.
	protected void BuildConnectivity(array<ref TBD_HighwayRecord> records)
	{
		if (!records) return;
		int count = records.Count();
		int nodeCounter = 0;

		for (int i = 0; i < count; i++)
		{
			TBD_HighwayRecord a = records[i];
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
				TBD_HighwayRecord b = records[j];

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
}
