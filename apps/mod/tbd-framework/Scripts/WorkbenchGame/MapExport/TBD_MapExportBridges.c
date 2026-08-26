/**
 * TBD_MapExportBridges.c
 *
 * Multi-level 3D bridge deck and oriented pier/dock quay extractor.
 * Captures deck elevations, vertical clearances over water/terrain, and pier strip vectors.
 *
 * Outputs:
 *   - TBD_BridgesExport.json
 *   - TBD_BridgesExport_meta.json
 */

class TBD_BridgeRecord
{
	string m_sId;
	vector m_vDeckCenter;
	float m_fDeckYM;
	float m_fUnderneathYM;
	float m_fClearanceM;
	float m_fLengthM;
	float m_fWidthM;
	float m_fHeadingDeg;
	string m_sPrefab;

	void TBD_BridgeRecord(string id, vector deckCenter, float deckY, float underY, float clearanceM, float lengthM, float widthM, float headingDeg, string prefab)
	{
		m_sId = id;
		m_vDeckCenter = deckCenter;
		m_fDeckYM = deckY;
		m_fUnderneathYM = underY;
		m_fClearanceM = clearanceM;
		m_fLengthM = lengthM;
		m_fWidthM = widthM;
		m_fHeadingDeg = headingDeg;
		m_sPrefab = prefab;
	}
}

class TBD_PierRecord
{
	string m_sId;
	string m_sType; // "pier", "dock"
	vector m_vP1;
	vector m_vP2;
	float m_fWidthM;
	float m_fDeckYM;
	float m_fTipWaterDepthM;
	string m_sPrefab;

	void TBD_PierRecord(string id, string typeName, vector p1, vector p2, float widthM, float deckY, float tipDepthM, string prefab)
	{
		m_sId = id;
		m_sType = typeName;
		m_vP1 = p1;
		m_vP2 = p2;
		m_fWidthM = widthM;
		m_fDeckYM = deckY;
		m_fTipWaterDepthM = tipDepthM;
		m_sPrefab = prefab;
	}
}

class TBD_MapExportBridges
{
	protected static const string TAG = "[TBD][Bridges]";
	protected static const int FLUSH = 8000;

	protected ref array<ref TBD_BridgeRecord> m_aBridges;
	protected ref array<ref TBD_PierRecord> m_aPiers;

	protected TBD_MapExportContext m_Ctx;

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		m_Ctx = ctx;
		m_aBridges = {};
		m_aPiers = {};

		float worldSize = ctx.m_fWorldSize;
		float cellSize = cfg.m_fObjectChunkSizeM;
		if (cellSize <= 10.0)
			cellSize = 512.0;

		int cells = Math.Ceil(worldSize / cellSize);

		string outJson = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_BridgesExport.json");
		string outMeta = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_BridgesExport_meta.json");

		Print(string.Format("%1 Scanning bridges and piers across %2x%2 grid (%3 m cells)...",
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

		Print(string.Format("%1 Found %2 bridges and %3 piers. Writing to %4...",
			TAG, m_aBridges.Count(), m_aPiers.Count(), outJson));

		FileHandle f = FileIO.OpenFile(outJson, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open " + outJson + " for write", LogLevel.ERROR);
			return false;
		}

		string buf = "{\n  \"bridges\": [\n";
		bool writeOk = true;

		for (int i = 0; i < m_aBridges.Count(); i++)
		{
			TBD_BridgeRecord br = m_aBridges[i];
			buf += "    {\n";
			buf += "      \"id\": \"" + TBD_MapExportJson.Escape(br.m_sId) + "\",\n";
			buf += "      \"deckCenter\": [" + br.m_vDeckCenter[0].ToString() + ", " + br.m_vDeckCenter[1].ToString() + ", " + br.m_vDeckCenter[2].ToString() + "],\n";
			buf += "      \"deckElevationYM\": " + br.m_fDeckYM.ToString() + ",\n";
			buf += "      \"underneathElevationYM\": " + br.m_fUnderneathYM.ToString() + ",\n";
			buf += "      \"clearanceM\": " + br.m_fClearanceM.ToString() + ",\n";
			buf += "      \"lengthM\": " + br.m_fLengthM.ToString() + ",\n";
			buf += "      \"widthM\": " + br.m_fWidthM.ToString() + ",\n";
			buf += "      \"headingDeg\": " + br.m_fHeadingDeg.ToString() + ",\n";
			buf += "      \"prefab\": \"" + TBD_MapExportJson.Escape(br.m_sPrefab) + "\"\n";
			buf += "    }";
			if (i < m_aBridges.Count() - 1)
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
			buf += "  ],\n  \"piers\": [\n";
			writeOk = TBD_MapExportJson.Write(f, buf, TAG);
			buf = "";
		}

		if (writeOk)
		{
			for (int j = 0; j < m_aPiers.Count(); j++)
			{
				TBD_PierRecord pr = m_aPiers[j];
				buf += "    {\n";
				buf += "      \"id\": \"" + TBD_MapExportJson.Escape(pr.m_sId) + "\",\n";
				buf += "      \"type\": \"" + TBD_MapExportJson.Escape(pr.m_sType) + "\",\n";
				buf += "      \"p1\": [" + pr.m_vP1[0].ToString() + ", " + pr.m_vP1[1].ToString() + ", " + pr.m_vP1[2].ToString() + "],\n";
				buf += "      \"p2\": [" + pr.m_vP2[0].ToString() + ", " + pr.m_vP2[1].ToString() + ", " + pr.m_vP2[2].ToString() + "],\n";
				buf += "      \"widthM\": " + pr.m_fWidthM.ToString() + ",\n";
				buf += "      \"deckElevationYM\": " + pr.m_fDeckYM.ToString() + ",\n";
				buf += "      \"tipWaterDepthM\": " + pr.m_fTipWaterDepthM.ToString() + ",\n";
				buf += "      \"prefab\": \"" + TBD_MapExportJson.Escape(pr.m_sPrefab) + "\"\n";
				buf += "    }";
				if (j < m_aPiers.Count() - 1)
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
			Print(TAG + " ABORTED: Bridges JSON write failed", LogLevel.ERROR);
			return false;
		}

		// Write meta JSON
		FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (mh)
		{
			string mj;
			mj += "{\n";
			mj += "  \"method\": \"mod-bridges-piers-export\",\n";
			mj += "  \"bridgeCount\": " + m_aBridges.Count().ToString() + ",\n";
			mj += "  \"pierCount\": " + m_aPiers.Count().ToString() + ",\n";
			mj += "  \"dataFile\": \"TBD_BridgesExport.json\"\n";
			mj += "}\n";
			TBD_MapExportJson.Write(mh, mj, TAG);
			mh.Close();
		}

		Print(string.Format("%1 DONE — %2 bridges, %3 piers -> %4",
			TAG, m_aBridges.Count(), m_aPiers.Count(), outJson));
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

		// 1. Bridges
		if (lowerRes.Contains("/bridge") || lowerRes.Contains("bridge_") || lowerRes.Contains("viaduct"))
		{
			float terrainY = m_Ctx.m_API.GetTerrainSurfaceY(origin[0], origin[2]);
			float deckY = origin[1] + maxs[1];
			float clearance = deckY - terrainY;
			if (clearance < 0) clearance = 0;

			float length = halfExtents[2] * 2.0;
			float width = halfExtents[0] * 2.0;
			if (halfExtents[0] > halfExtents[2])
			{
				length = halfExtents[0] * 2.0;
				width = halfExtents[2] * 2.0;
			}

			vector angles = mat[2].VectorToAngles();
			float heading = angles[0];

			string bId = "bridge_" + (m_aBridges.Count() + 1).ToString();
			m_aBridges.Insert(new TBD_BridgeRecord(bId, origin, deckY, terrainY, clearance, length, width, heading, resName));
			return true;
		}

		// 2. Piers & Docks
		if (lowerRes.Contains("/pier") || lowerRes.Contains("pier_") || lowerRes.Contains("/dock") || lowerRes.Contains("dock_") || lowerRes.Contains("quay"))
		{
			float hx = halfExtents[0];
			float hz = halfExtents[2];
			float maxH = hx;
			float minH = hz;
			vector longAxis = mat[0];
			if (hz > hx)
			{
				maxH = hz;
				minH = hx;
				longAxis = mat[2];
			}

			float aspect = 1.0;
			if (minH > 0.01)
				aspect = maxH / minH;

			// Check if oriented pier strip (aspect >= 3.0)
			if (aspect >= 3.0)
			{
				vector p1 = origin - longAxis * maxH;
				vector p2 = origin + longAxis * maxH;
				float deckY = origin[1] + maxs[1];

				// Water depth at seaward tip (p2)
				float terrainY = m_Ctx.m_API.GetTerrainSurfaceY(p2[0], p2[2]);
				float tipDepth = 0.0;
				if (terrainY < 0.0)
					tipDepth = -terrainY;

				string pierType = "pier";
				if (lowerRes.Contains("dock") || lowerRes.Contains("quay"))
					pierType = "dock";

				string pId = "pier_" + (m_aPiers.Count() + 1).ToString();
				m_aPiers.Insert(new TBD_PierRecord(pId, pierType, p1, p2, minH * 2.0, deckY, tipDepth, resName));
			}
		}

		return true;
	}
}
