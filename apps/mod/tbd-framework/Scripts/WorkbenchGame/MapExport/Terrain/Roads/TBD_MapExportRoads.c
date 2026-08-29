/**
 * TBD_MapExportRoads.c
 *
 * Coordinated Road Network & Graph Connectivity export suite for Bohemia Reforger.
 * Orchestrates the modular road extraction pipeline across:
 *   1. Highways & Major Arterials (TBD_MapExportHighways -> highways.json)
 *   2. Secondary Paved Roads      (TBD_MapExportPavedRoads -> roads_paved.json)
 *   3. Dirt & Gravel Roads        (TBD_MapExportDirtRoads  -> roads_dirt.json)
 *   4. Tracks & Tractor Trails    (TBD_MapExportTracks     -> tracks.json)
 *   5. Footpaths & Hiking Trails  (TBD_MapExportFootpaths  -> paths.json)
 *   6. Airfield Runways & Taxiway (TBD_MapExportRunways    -> runways.json)
 *
 * Outputs:
 *   - roads_meta.json (Consolidated metadata manifest with global network totals, layer breakdown, and topological junction catalog)
 */

class TBD_JunctionNode
{
	string m_sId;
	vector m_vPos;
	ref array<string> m_aConnectedSegments;

	void TBD_JunctionNode(string id, vector pos)
	{
		m_sId = id;
		m_vPos = pos;
		m_aConnectedSegments = {};
	}

	void AddSegment(string segId)
	{
		if (m_aConnectedSegments.Find(segId) == -1)
			m_aConnectedSegments.Insert(segId);
	}
}

class TBD_MapExportRoads
{
	protected static const string TAG = "[TBD][Roads]";
	protected static const int FLUSH = 8000;

	protected ref TBD_MapExportHighways m_HighwaysExporter;
	protected ref TBD_MapExportPavedRoads m_PavedRoadsExporter;
	protected ref TBD_MapExportDirtRoads m_DirtRoadsExporter;
	protected ref TBD_MapExportTracks m_TracksExporter;
	protected ref TBD_MapExportFootpaths m_FootpathsExporter;
	protected ref TBD_MapExportRunways m_RunwaysExporter;

	//------------------------------------------------------------------------------------------------
	//! Primary static export execution method.
	static bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		TBD_MapExportRoads exporter = new TBD_MapExportRoads();
		return exporter.Execute(ctx, cfg);
	}

	//------------------------------------------------------------------------------------------------
	//! Instance coordinator execution method.
	bool Execute(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		int tickStart = System.GetTickCount();
		string mapName = ctx.GetMapName(cfg);
		float worldSize = ctx.m_fWorldSize;
		string outMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "roads", "roads_meta.json");

		Print(TAG + " Starting coordinated continuous road network export suite with topology...", LogLevel.NORMAL);

		if (!m_HighwaysExporter)   m_HighwaysExporter = new TBD_MapExportHighways();
		if (!m_PavedRoadsExporter) m_PavedRoadsExporter = new TBD_MapExportPavedRoads();
		if (!m_DirtRoadsExporter)  m_DirtRoadsExporter = new TBD_MapExportDirtRoads();
		if (!m_TracksExporter)     m_TracksExporter = new TBD_MapExportTracks();
		if (!m_FootpathsExporter)  m_FootpathsExporter = new TBD_MapExportFootpaths();
		if (!m_RunwaysExporter)    m_RunwaysExporter = new TBD_MapExportRunways();

		// 1. Highways & Major Arterials
		bool hwOk = true;
		array<ref TBD_HighwayRecord> hwRecs = {};
		if (!cfg || cfg.m_bExportHighways)
			hwOk = m_HighwaysExporter.Export(ctx, cfg, hwRecs);

		// 2. Secondary Paved Roads
		bool pavedOk = true;
		array<ref TBD_PavedRoadRecord> pavedRecs = {};
		if (!cfg || cfg.m_bExportPavedRoads)
			pavedOk = m_PavedRoadsExporter.Export(ctx, cfg, pavedRecs);

		// 3. Dirt & Gravel Roads
		bool dirtOk = true;
		array<ref TBD_DirtRoadRecord> dirtRecs = {};
		if (!cfg || cfg.m_bExportDirtRoads)
			dirtOk = m_DirtRoadsExporter.Export(ctx, cfg, dirtRecs);

		// 4. Forestry & Agricultural Tracks
		bool tracksOk = true;
		array<ref TBD_TrackRecord> trackRecs = {};
		if (!cfg || cfg.m_bExportTracks)
			tracksOk = m_TracksExporter.Export(ctx, cfg, trackRecs);

		// 5. Footpaths & Trails
		bool pathsOk = true;
		array<ref TBD_FootpathRecord> pathRecs = {};
		if (!cfg || cfg.m_bExportPaths)
			pathsOk = m_FootpathsExporter.Export(ctx, cfg, pathRecs);

		// 6. Airfield Runways & Taxiways
		bool runwaysOk = true;
		array<ref TBD_RunwayRoadRecord> runwayRecs = {};
		if (!cfg || cfg.m_bExportRunways)
			runwaysOk = m_RunwaysExporter.Export(ctx, cfg, runwayRecs);

		int elapsedMs = System.GetTickCount() - tickStart;
		bool allOk = (hwOk && pavedOk && dirtOk && tracksOk && pathsOk && runwaysOk);

		// Build Master Global Junction Graph across all road classes
		array<ref TBD_JunctionNode> globalJunctions = {};
		BuildGlobalJunctionGraph(hwRecs, pavedRecs, dirtRecs, trackRecs, pathRecs, runwayRecs, globalJunctions);

		// Write consolidated metadata manifest
		WriteRoadsMeta(outMeta, mapName, worldSize, hwRecs, pavedRecs, dirtRecs, trackRecs, pathRecs, runwayRecs, globalJunctions, elapsedMs, cfg);

		int totalSegs = 0;
		float totalLenM = 0.0;
		if (hwRecs)     { totalSegs += hwRecs.Count();     for (int i0 = 0; i0 < hwRecs.Count(); i0++)     totalLenM += hwRecs[i0].m_fTotalLengthM; }
		if (pavedRecs)  { totalSegs += pavedRecs.Count();  for (int i1 = 0; i1 < pavedRecs.Count(); i1++)  totalLenM += pavedRecs[i1].m_fTotalLengthM; }
		if (dirtRecs)   { totalSegs += dirtRecs.Count();   for (int i2 = 0; i2 < dirtRecs.Count(); i2++)   totalLenM += dirtRecs[i2].m_fTotalLengthM; }
		if (trackRecs)  { totalSegs += trackRecs.Count();  for (int i3 = 0; i3 < trackRecs.Count(); i3++)  totalLenM += trackRecs[i3].m_fTotalLengthM; }
		if (pathRecs)   { totalSegs += pathRecs.Count();   for (int i4 = 0; i4 < pathRecs.Count(); i4++)   totalLenM += pathRecs[i4].m_fTotalLengthM; }
		if (runwayRecs) { totalSegs += runwayRecs.Count(); for (int i5 = 0; i5 < runwayRecs.Count(); i5++) totalLenM += runwayRecs[i5].m_fTotalLengthM; }

		Print(string.Format("%1 CONTINUOUS ROAD NETWORK EXPORT FINISHED in %2 ms (success=%3) — Continuous Routes=%4, Total Length=%5 km, Intersections=%6 -> %7",
			TAG, elapsedMs, allOk, totalSegs, (totalLenM / 1000.0).ToString(2), globalJunctions.Count(), outMeta), LogLevel.NORMAL);

		return allOk;
	}

	//------------------------------------------------------------------------------------------------
	protected void BuildGlobalJunctionGraph(
		array<ref TBD_HighwayRecord> hwRecs,
		array<ref TBD_PavedRoadRecord> pavedRecs,
		array<ref TBD_DirtRoadRecord> dirtRecs,
		array<ref TBD_TrackRecord> trackRecs,
		array<ref TBD_FootpathRecord> pathRecs,
		array<ref TBD_RunwayRoadRecord> runwayRecs,
		out array<ref TBD_JunctionNode> outJunctions)
	{
		outJunctions = new array<ref TBD_JunctionNode>();
		int juncCounter = 0;

		ref array<vector> endPositions = {};
		ref array<string> endSegIds = {};

		if (hwRecs)
		{
			for (int h = 0; h < hwRecs.Count(); h++)
			{
				TBD_HighwayRecord hr = hwRecs[h];
				endPositions.Insert(hr.m_vStartNodePos); endSegIds.Insert(hr.m_sId);
				endPositions.Insert(hr.m_vEndNodePos); endSegIds.Insert(hr.m_sId);
			}
		}
		if (pavedRecs)
		{
			for (int pv = 0; pv < pavedRecs.Count(); pv++)
			{
				TBD_PavedRoadRecord pvr = pavedRecs[pv];
				endPositions.Insert(pvr.m_vStartNodePos); endSegIds.Insert(pvr.m_sId);
				endPositions.Insert(pvr.m_vEndNodePos); endSegIds.Insert(pvr.m_sId);
			}
		}
		if (dirtRecs)
		{
			for (int d = 0; d < dirtRecs.Count(); d++)
			{
				TBD_DirtRoadRecord dr = dirtRecs[d];
				endPositions.Insert(dr.m_vStartNodePos); endSegIds.Insert(dr.m_sId);
				endPositions.Insert(dr.m_vEndNodePos); endSegIds.Insert(dr.m_sId);
			}
		}
		if (trackRecs)
		{
			for (int t = 0; t < trackRecs.Count(); t++)
			{
				TBD_TrackRecord tr = trackRecs[t];
				endPositions.Insert(tr.m_vStartNodePos); endSegIds.Insert(tr.m_sId);
				endPositions.Insert(tr.m_vEndNodePos); endSegIds.Insert(tr.m_sId);
			}
		}
		if (pathRecs)
		{
			for (int p = 0; p < pathRecs.Count(); p++)
			{
				TBD_FootpathRecord fpr = pathRecs[p];
				endPositions.Insert(fpr.m_vStartNodePos); endSegIds.Insert(fpr.m_sId);
				endPositions.Insert(fpr.m_vEndNodePos); endSegIds.Insert(fpr.m_sId);
			}
		}
		if (runwayRecs)
		{
			for (int r = 0; r < runwayRecs.Count(); r++)
			{
				TBD_RunwayRoadRecord rwr = runwayRecs[r];
				endPositions.Insert(rwr.m_vStartNodePos); endSegIds.Insert(rwr.m_sId);
				endPositions.Insert(rwr.m_vEndNodePos); endSegIds.Insert(rwr.m_sId);
			}
		}

		int totalPts = endPositions.Count();
		for (int i = 0; i < totalPts; i++)
		{
			vector pt = endPositions[i];
			string segId = endSegIds[i];

			TBD_JunctionNode matched = null;
			for (int j = 0; j < outJunctions.Count(); j++)
			{
				if (vector.Distance(pt, outJunctions[j].m_vPos) <= 2.5)
				{
					matched = outJunctions[j];
					break;
				}
			}

			if (!matched)
			{
				juncCounter++;
				matched = new TBD_JunctionNode("junction_" + juncCounter.ToString(), pt);
				outJunctions.Insert(matched);
			}

			matched.AddSegment(segId);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteRoadsMeta(
		string path,
		string mapName,
		float worldSize,
		array<ref TBD_HighwayRecord> hwRecs,
		array<ref TBD_PavedRoadRecord> pavedRecs,
		array<ref TBD_DirtRoadRecord> dirtRecs,
		array<ref TBD_TrackRecord> trackRecs,
		array<ref TBD_FootpathRecord> pathRecs,
		array<ref TBD_RunwayRoadRecord> runwayRecs,
		array<ref TBD_JunctionNode> junctions,
		int elapsedMs,
		TBD_MapExportConfig cfg)
	{
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open metadata file: " + path, LogLevel.ERROR);
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
		buf += "  \"mapName\": \"" + TBD_MapExportJson.Escape(mapName) + "\",\n";
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

		// 1. Highways
		buf += "    \"highways\": {\n";
		buf += "      \"file\": \"highways.json\",\n";
		buf += "      \"roadClass\": \"highway_paved\",\n";
		buf += "      \"enabled\": " + (cfg && cfg.m_bExportHighways).ToString() + ",\n";
		buf += "      \"segmentsCount\": " + hwCount.ToString() + ",\n";
		buf += "      \"totalLengthM\": " + hwLenM.ToString() + "\n";
		buf += "    },\n";

		// 2. Paved Roads
		buf += "    \"roads_paved\": {\n";
		buf += "      \"file\": \"roads_paved.json\",\n";
		buf += "      \"roadClass\": \"road_paved\",\n";
		buf += "      \"enabled\": " + (cfg && cfg.m_bExportPavedRoads).ToString() + ",\n";
		buf += "      \"segmentsCount\": " + pavedCount.ToString() + ",\n";
		buf += "      \"totalLengthM\": " + pavedLenM.ToString() + "\n";
		buf += "    },\n";

		// 3. Dirt Roads
		buf += "    \"roads_dirt\": {\n";
		buf += "      \"file\": \"roads_dirt.json\",\n";
		buf += "      \"roadClass\": \"road_dirt\",\n";
		buf += "      \"enabled\": " + (cfg && cfg.m_bExportDirtRoads).ToString() + ",\n";
		buf += "      \"segmentsCount\": " + dirtCount.ToString() + ",\n";
		buf += "      \"totalLengthM\": " + dirtLenM.ToString() + "\n";
		buf += "    },\n";

		// 4. Tracks
		buf += "    \"tracks\": {\n";
		buf += "      \"file\": \"tracks.json\",\n";
		buf += "      \"roadClass\": \"track\",\n";
		buf += "      \"enabled\": " + (cfg && cfg.m_bExportTracks).ToString() + ",\n";
		buf += "      \"segmentsCount\": " + trackCount.ToString() + ",\n";
		buf += "      \"totalLengthM\": " + trackLenM.ToString() + "\n";
		buf += "    },\n";

		// 5. Paths
		buf += "    \"paths\": {\n";
		buf += "      \"file\": \"paths.json\",\n";
		buf += "      \"roadClass\": \"path\",\n";
		buf += "      \"enabled\": " + (cfg && cfg.m_bExportPaths).ToString() + ",\n";
		buf += "      \"segmentsCount\": " + pathCount.ToString() + ",\n";
		buf += "      \"totalLengthM\": " + pathLenM.ToString() + "\n";
		buf += "    },\n";

		// 6. Runways
		buf += "    \"runways\": {\n";
		buf += "      \"file\": \"runways.json\",\n";
		buf += "      \"roadClass\": \"runway\",\n";
		buf += "      \"enabled\": " + (cfg && cfg.m_bExportRunways).ToString() + ",\n";
		buf += "      \"segmentsCount\": " + runwayCount.ToString() + ",\n";
		buf += "      \"totalLengthM\": " + runwayLenM.ToString() + "\n";
		buf += "    }\n";

		buf += "  },\n";

		// Output only true intersections (degree >= 3 or transitions) in junctions catalog
		buf += "  \"junctions\": [\n";
		bool firstJunc = true;
		for (int ji = 0; ji < totalJuncs; ji++)
		{
			TBD_JunctionNode jn = junctions[ji];
			if (jn.m_aConnectedSegments.Count() < 2)
				continue;

			if (!firstJunc) buf += ",\n";
			firstJunc = false;

			buf += "    {\n";
			buf += "      \"id\": \"" + TBD_MapExportJson.Escape(jn.m_sId) + "\",\n";
			buf += "      \"pos\": [" + jn.m_vPos[0].ToString() + ", " + jn.m_vPos[1].ToString() + ", " + jn.m_vPos[2].ToString() + "],\n";
			buf += "      \"degree\": " + jn.m_aConnectedSegments.Count().ToString() + ",\n";
			buf += "      \"connectedSegments\": [";
			for (int cs = 0; cs < jn.m_aConnectedSegments.Count(); cs++)
			{
				buf += "\"" + TBD_MapExportJson.Escape(jn.m_aConnectedSegments[cs]) + "\"";
				if (cs < jn.m_aConnectedSegments.Count() - 1) buf += ", ";
			}
			buf += "]\n";
			buf += "    }";

			if (buf.Length() > FLUSH)
			{
				TBD_MapExportJson.Write(f, buf, TAG);
				buf = "";
			}
		}

		buf += "\n  ],\n";
		buf += "  \"elapsedMs\": " + elapsedMs.ToString() + "\n";
		buf += "}\n";

		TBD_MapExportJson.Write(f, buf, TAG);
		f.Close();
	}
}
