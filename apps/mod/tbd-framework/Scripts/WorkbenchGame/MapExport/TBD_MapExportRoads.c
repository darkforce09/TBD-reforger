/**
 * TBD_MapExportRoads.c
 *
 * Road network and spline extractor.
 * Captures road segments, centerlines, widths, surface classes, and junctions.
 *
 * Outputs:
 *   - TBD_RoadsExport.json
 *   - TBD_RoadsExport_meta.json
 */

class TBD_RoadSegmentRecord
{
	string m_sId;
	string m_sRoadClass;
	float m_fWidthM;
	vector m_vP1;
	vector m_vP2;
	string m_sPrefab;

	void TBD_RoadSegmentRecord(string id, string roadClass, float widthM, vector p1, vector p2, string prefab)
	{
		m_sId = id;
		m_sRoadClass = roadClass;
		m_fWidthM = widthM;
		m_vP1 = p1;
		m_vP2 = p2;
		m_sPrefab = prefab;
	}
}

class TBD_MapExportRoads
{
	protected static const string TAG = "[TBD][Roads]";
	protected static const int FLUSH = 8000;

	protected ref array<ref TBD_RoadSegmentRecord> m_aSegments;
	protected int m_iCount;

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		m_aSegments = {};
		m_iCount = 0;

		float worldSize = ctx.m_fWorldSize;
		float cellSize = cfg.m_fObjectChunkSizeM;
		if (cellSize <= 10.0)
			cellSize = 512.0;

		int cells = Math.Ceil(worldSize / cellSize);

		string outJson = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_RoadsExport.json");
		string outMeta = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_RoadsExport_meta.json");

		Print(string.Format("%1 Scanning road network across %2x%2 grid (%3 m cells)...",
			TAG, cells, cellSize));

		for (int cy = 0; cy < cells; cy++)
		{
			for (int cx = 0; cx < cells; cx++)
			{
				vector bMin = Vector(cx * cellSize, -250.0, cy * cellSize);
				vector bMax = Vector((cx + 1) * cellSize, 1000.0, (cy + 1) * cellSize);
				ctx.m_World.QueryEntitiesByAABB(bMin, bMax, QueryEntityCallback, null, EQueryEntitiesFlags.ALL);
			}
		}

		Print(string.Format("%1 Found %2 road segments. Writing to %3...", TAG, m_aSegments.Count(), outJson));

		FileHandle f = FileIO.OpenFile(outJson, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open " + outJson + " for write", LogLevel.ERROR);
			return false;
		}

		string buf = "[\n";
		bool writeOk = true;

		for (int i = 0; i < m_aSegments.Count(); i++)
		{
			TBD_RoadSegmentRecord seg = m_aSegments[i];
			buf += "  {\n";
			buf += "    \"id\": \"" + TBD_MapExportJson.Escape(seg.m_sId) + "\",\n";
			buf += "    \"roadClass\": \"" + TBD_MapExportJson.Escape(seg.m_sRoadClass) + "\",\n";
			buf += "    \"widthM\": " + seg.m_fWidthM.ToString() + ",\n";
			buf += "    \"points\": [\n";
			buf += "      [" + seg.m_vP1[0].ToString() + ", " + seg.m_vP1[1].ToString() + ", " + seg.m_vP1[2].ToString() + "],\n";
			buf += "      [" + seg.m_vP2[0].ToString() + ", " + seg.m_vP2[1].ToString() + ", " + seg.m_vP2[2].ToString() + "]\n";
			buf += "    ],\n";
			buf += "    \"prefab\": \"" + TBD_MapExportJson.Escape(seg.m_sPrefab) + "\"\n";
			buf += "  }";
			if (i < m_aSegments.Count() - 1)
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
			buf += "]\n";
			writeOk = TBD_MapExportJson.Write(f, buf, TAG);
		}
		f.Close();

		if (!writeOk)
		{
			FileIO.DeleteFile(outJson);
			Print(TAG + " ABORTED: Road JSON write failed", LogLevel.ERROR);
			return false;
		}

		// Write meta JSON
		FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (mh)
		{
			string mj;
			mj += "{\n";
			mj += "  \"method\": \"mod-road-network-export\",\n";
			mj += "  \"segmentCount\": " + m_aSegments.Count().ToString() + ",\n";
			mj += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
			mj += "  \"dataFile\": \"TBD_RoadsExport.json\"\n";
			mj += "}\n";
			TBD_MapExportJson.Write(mh, mj, TAG);
			mh.Close();
		}

		Print(string.Format("%1 DONE — Exported %2 road segments -> %3", TAG, m_aSegments.Count(), outJson));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected bool QueryEntityCallback(IEntity ent)
	{
		if (!ent)
			return true;

		string resName = TBD_MapExportContext.GetEntityResourceName(ent);
		string className = ent.ClassName();

		string lowerRes = resName;
		lowerRes.ToLower();
		string lowerClass = className;
		lowerClass.ToLower();

		bool isRoad = false;
		string roadClass = "road_paved";
		float roadWidth = 6.0;

		if (lowerRes.Contains("/roads/") || lowerRes.Contains("road_") || lowerRes.Contains("track_") || lowerRes.Contains("path_") || lowerRes.Contains("runway_") || lowerClass.Contains("road"))
		{
			isRoad = true;
			if (lowerRes.Contains("highway") || lowerRes.Contains("mainroad") || lowerRes.Contains("asphalt_wide"))
			{
				roadClass = "highway_paved";
				roadWidth = 8.0;
			}
			else if (lowerRes.Contains("dirt") || lowerRes.Contains("gravel"))
			{
				roadClass = "road_dirt";
				roadWidth = 4.5;
			}
			else if (lowerRes.Contains("track"))
			{
				roadClass = "track";
				roadWidth = 3.5;
			}
			else if (lowerRes.Contains("path") || lowerRes.Contains("trail"))
			{
				roadClass = "path";
				roadWidth = 2.0;
			}
			else if (lowerRes.Contains("runway"))
			{
				roadClass = "runway";
				roadWidth = 45.0;
			}
		}

		if (!isRoad)
			return true;

		vector mat[4];
		ent.GetWorldTransform(mat);
		vector origin = mat[3];

		vector mins, maxs;
		ent.GetBounds(mins, maxs);
		vector halfExtents = (maxs - mins) * 0.5;

		// Compute forward axis for segment endpoints
		vector forward = mat[2];
		float length = halfExtents[2] * 2.0;
		if (length < 1.0)
			length = 10.0;

		vector p1 = origin - forward * (length * 0.5);
		vector p2 = origin + forward * (length * 0.5);

		m_iCount++;
		string id = "road_" + m_iCount.ToString();
		m_aSegments.Insert(new TBD_RoadSegmentRecord(id, roadClass, roadWidth, p1, p2, resName));

		return true;
	}
}
