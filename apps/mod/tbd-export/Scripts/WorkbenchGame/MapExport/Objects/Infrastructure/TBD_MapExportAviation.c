/**
 * TBD_MapExportAviation.c
 *
 * Aviation infrastructure extractor: runways, helipads, landing zones (LZs), and taxiways.
 * Computes runway magnetic headings, threshold coordinates, dimensions, and LZ clearances.
 *
 * Outputs:
 *   - TBD_AviationExport.json
 *   - TBD_AviationExport_meta.json
 */

class TBD_RunwayRecord
{
	string m_sId;
	string m_sDesignator;
	float m_fHeadingDeg;
	vector m_vThresholdA;
	vector m_vThresholdB;
	float m_fLengthM;
	float m_fWidthM;
	string m_sSurface;
	string m_sPrefab;

	void TBD_RunwayRecord(string id, string designator, float heading, vector tA, vector tB, float length, float width, string surface, string prefab)
	{
		m_sId = id;
		m_sDesignator = designator;
		m_fHeadingDeg = heading;
		m_vThresholdA = tA;
		m_vThresholdB = tB;
		m_fLengthM = length;
		m_fWidthM = width;
		m_sSurface = surface;
		m_sPrefab = prefab;
	}
}

class TBD_HelipadRecord
{
	string m_sId;
	vector m_vCenter;
	float m_fRadiusM;
	string m_sSurface;
	string m_sPrefab;

	void TBD_HelipadRecord(string id, vector center, float radius, string surface, string prefab)
	{
		m_sId = id;
		m_vCenter = center;
		m_fRadiusM = radius;
		m_sSurface = surface;
		m_sPrefab = prefab;
	}
}

class TBD_MapExportAviation
{
	protected static const string TAG = "[TBD][Aviation]";
	protected static const int FLUSH = 8000;

	protected ref array<ref TBD_RunwayRecord> m_aRunways;
	protected ref array<ref TBD_HelipadRecord> m_aHelipads;

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		m_aRunways = {};
		m_aHelipads = {};

		float worldSize = ctx.m_fWorldSize;
		float cellSize = cfg.m_fObjectChunkSizeM;
		if (cellSize <= 10.0)
			cellSize = 512.0;

		int cells = Math.Ceil(worldSize / cellSize);

		string mapName = ctx.GetMapName(cfg);
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "infrastructure", "aviation.json");
		string outMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "infrastructure", "aviation_meta.json");

		Print(string.Format("%1 Scanning aviation assets across %2x%2 grid (%3 m cells)...",
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

		Print(string.Format("%1 Found %2 runways and %3 helipads. Writing to %4...",
			TAG, m_aRunways.Count(), m_aHelipads.Count(), outJson));

		FileHandle f = FileIO.OpenFile(outJson, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open " + outJson + " for write", LogLevel.ERROR);
			return false;
		}

		string buf = "{\n  \"runways\": [\n";
		bool writeOk = true;

		for (int i = 0; i < m_aRunways.Count(); i++)
		{
			TBD_RunwayRecord rw = m_aRunways[i];
			buf += "    {\n";
			buf += "      \"id\": \"" + TBD_MapExportJson.Escape(rw.m_sId) + "\",\n";
			buf += "      \"designator\": \"" + TBD_MapExportJson.Escape(rw.m_sDesignator) + "\",\n";
			buf += "      \"headingDeg\": " + rw.m_fHeadingDeg.ToString() + ",\n";
			buf += "      \"thresholdA\": [" + rw.m_vThresholdA[0].ToString() + ", " + rw.m_vThresholdA[1].ToString() + ", " + rw.m_vThresholdA[2].ToString() + "],\n";
			buf += "      \"thresholdB\": [" + rw.m_vThresholdB[0].ToString() + ", " + rw.m_vThresholdB[1].ToString() + ", " + rw.m_vThresholdB[2].ToString() + "],\n";
			buf += "      \"lengthM\": " + rw.m_fLengthM.ToString() + ",\n";
			buf += "      \"widthM\": " + rw.m_fWidthM.ToString() + ",\n";
			buf += "      \"surface\": \"" + TBD_MapExportJson.Escape(rw.m_sSurface) + "\",\n";
			buf += "      \"prefab\": \"" + TBD_MapExportJson.Escape(rw.m_sPrefab) + "\"\n";
			buf += "    }";
			if (i < m_aRunways.Count() - 1)
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
			buf += "  ],\n  \"helipads\": [\n";
			writeOk = TBD_MapExportJson.Write(f, buf, TAG);
			buf = "";
		}

		if (writeOk)
		{
			for (int j = 0; j < m_aHelipads.Count(); j++)
			{
				TBD_HelipadRecord hp = m_aHelipads[j];
				buf += "    {\n";
				buf += "      \"id\": \"" + TBD_MapExportJson.Escape(hp.m_sId) + "\",\n";
				buf += "      \"center\": [" + hp.m_vCenter[0].ToString() + ", " + hp.m_vCenter[1].ToString() + ", " + hp.m_vCenter[2].ToString() + "],\n";
				buf += "      \"radiusM\": " + hp.m_fRadiusM.ToString() + ",\n";
				buf += "      \"surface\": \"" + TBD_MapExportJson.Escape(hp.m_sSurface) + "\",\n";
				buf += "      \"prefab\": \"" + TBD_MapExportJson.Escape(hp.m_sPrefab) + "\"\n";
				buf += "    }";
				if (j < m_aHelipads.Count() - 1)
					buf += ",";
				buf += "\n";

				if (buf.Length() > FLUSH)
				{
					writeOk = TBD_MapExportJson.Write(f, buf, TAG);
					if (!writeOk) break;
					buf = "";
				}
			}
		}

		if (writeOk)
		{
			buf += "  ]\n}\n";
			writeOk = TBD_MapExportJson.Write(f, buf, TAG);
		}
		f.Close();

		if (!writeOk)
		{
			FileIO.DeleteFile(outJson);
			Print(TAG + " ABORTED: Aviation JSON write failed", LogLevel.ERROR);
			return false;
		}

		// Write meta JSON
		FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (mh)
		{
			string mj;
			mj += "{\n";
			mj += "  \"method\": \"mod-aviation-export\",\n";
			mj += "  \"runwayCount\": " + m_aRunways.Count().ToString() + ",\n";
			mj += "  \"helipadCount\": " + m_aHelipads.Count().ToString() + ",\n";
			mj += "  \"dataFile\": \"aviation.json\"\n";
			mj += "}\n";
			TBD_MapExportJson.Write(mh, mj, TAG);
			mh.Close();
		}

		Print(string.Format("%1 DONE - %2 runways, %3 helipads -> %4",
			TAG, m_aRunways.Count(), m_aHelipads.Count(), outJson));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected bool QueryEntityCallback(IEntity ent)
	{
		if (!ent)
			return true;

		string resName = TBD_MapExportContext.GetEntityResourceName(ent);
		string lowerRes = resName;
		lowerRes.ToLower();

		vector mat[4];
		ent.GetWorldTransform(mat);
		vector origin = mat[3];

		vector mins, maxs;
		ent.GetBounds(mins, maxs);
		vector halfExtents = (maxs - mins) * 0.5;

		// 1. Runways
		if (lowerRes.Contains("runway"))
		{
			float length = halfExtents[2] * 2.0;
			float width = halfExtents[0] * 2.0;
			vector forward = mat[2];

			if (halfExtents[0] > halfExtents[2])
			{
				length = halfExtents[0] * 2.0;
				width = halfExtents[2] * 2.0;
				forward = mat[0];
			}

			vector angles = forward.VectorToAngles();
			float heading = angles[0];
			if (heading < 0) heading += 360.0;

			// Compute runway magnetic designator (e.g. 09/27)
			int headNum = Math.Round(heading / 10.0);
			if (headNum <= 0) headNum = 36;
			int oppNum = headNum + 18;
			if (oppNum > 36) oppNum -= 36;

			string sHead = headNum.ToString();
			if (headNum < 10) sHead = "0" + sHead;
			string sOpp = oppNum.ToString();
			if (oppNum < 10) sOpp = "0" + sOpp;
			string designator = sHead + "/" + sOpp;

			vector tA = origin - forward * (length * 0.5);
			vector tB = origin + forward * (length * 0.5);

			string surface = "asphalt";
			if (lowerRes.Contains("concrete")) surface = "concrete";
			else if (lowerRes.Contains("dirt") || lowerRes.Contains("grass")) surface = "grass";

			string rId = "runway_" + (m_aRunways.Count() + 1).ToString();
			m_aRunways.Insert(new TBD_RunwayRecord(rId, designator, heading, tA, tB, length, width, surface, resName));
			return true;
		}

		// 2. Helipads
		if (lowerRes.Contains("helipad") || lowerRes.Contains("heli_pad") || lowerRes.Contains("landingzone"))
		{
			float radius = halfExtents[0];
			if (halfExtents[2] > radius) radius = halfExtents[2];
			if (radius < 4.0) radius = 8.0;

			string surface = "concrete";
			if (lowerRes.Contains("grass") || lowerRes.Contains("dirt")) surface = "grass";
			else if (lowerRes.Contains("metal") || lowerRes.Contains("steel")) surface = "steel";

			string hId = "helipad_" + (m_aHelipads.Count() + 1).ToString();
			m_aHelipads.Insert(new TBD_HelipadRecord(hId, origin, radius, surface, resName));
		}

		return true;
	}
}
