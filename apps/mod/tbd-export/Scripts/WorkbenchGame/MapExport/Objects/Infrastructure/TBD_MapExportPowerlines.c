/**
 * TBD_MapExportPowerlines.c
 *
 * Electrical power grid and utility network extractor.
 * Captures pylons, wooden distribution poles, transformers, and builds topological wire graphs.
 *
 * Outputs:
 *   - TBD_PowerlinesExport.json
 *   - TBD_PowerlinesExport_meta.json
 */

class TBD_PylonRecord
{
	string m_sId;
	string m_sType; // "high_voltage", "distribution_pole", "transformer"
	vector m_vPos;
	float m_fHeightM;
	string m_sPrefab;

	void TBD_PylonRecord(string id, string typeName, vector pos, float heightM, string prefab)
	{
		m_sId = id;
		m_sType = typeName;
		m_vPos = pos;
		m_fHeightM = heightM;
		m_sPrefab = prefab;
	}
}

class TBD_WireRecord
{
	string m_sFromPylon;
	string m_sToPylon;
	int m_iCableCount;
	float m_fDistanceM;
	float m_fSagM;

	void TBD_WireRecord(string fromPylon, string toPylon, int cableCount, float distM, float sagM)
	{
		m_sFromPylon = fromPylon;
		m_sToPylon = toPylon;
		m_iCableCount = cableCount;
		m_fDistanceM = distM;
		m_fSagM = sagM;
	}
}

class TBD_MapExportPowerlines
{
	protected static const string TAG = "[TBD][Powerlines]";
	protected static const int FLUSH = 8000;

	protected ref array<ref TBD_PylonRecord> m_aPylons;
	protected ref array<ref TBD_WireRecord> m_aWires;

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		m_aPylons = {};
		m_aWires = {};

		float worldSize = ctx.m_fWorldSize;
		float cellSize = cfg.m_fObjectChunkSizeM;
		if (cellSize <= 10.0)
			cellSize = 512.0;

		int cells = Math.Ceil(worldSize / cellSize);

		string mapName = ctx.GetMapName(cfg);
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "infrastructure", "power_grid.json");
		string outMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "infrastructure", "powerlines_meta.json");

		Print(string.Format("%1 Scanning power grid assets across %2x%2 grid (%3 m cells)...",
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

		// Build topological wire connections between nearby pylons of matching type
		BuildWireConnections();

		Print(string.Format("%1 Found %2 pylons and %3 wire spans. Writing to %4...",
			TAG, m_aPylons.Count(), m_aWires.Count(), outJson));

		FileHandle f = FileIO.OpenFile(outJson, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open " + outJson + " for write", LogLevel.ERROR);
			return false;
		}

		string buf = "{\n  \"pylons\": [\n";
		bool writeOk = true;

		for (int i = 0; i < m_aPylons.Count(); i++)
		{
			TBD_PylonRecord py = m_aPylons[i];
			buf += "    {\n";
			buf += "      \"id\": \"" + TBD_MapExportJson.Escape(py.m_sId) + "\",\n";
			buf += "      \"type\": \"" + TBD_MapExportJson.Escape(py.m_sType) + "\",\n";
			buf += "      \"pos\": [" + py.m_vPos[0].ToString() + ", " + py.m_vPos[1].ToString() + ", " + py.m_vPos[2].ToString() + "],\n";
			buf += "      \"heightM\": " + py.m_fHeightM.ToString() + ",\n";
			buf += "      \"prefab\": \"" + TBD_MapExportJson.Escape(py.m_sPrefab) + "\"\n";
			buf += "    }";
			if (i < m_aPylons.Count() - 1)
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
			buf += "  ],\n  \"wires\": [\n";
			writeOk = TBD_MapExportJson.Write(f, buf, TAG);
			buf = "";
		}

		if (writeOk)
		{
			for (int j = 0; j < m_aWires.Count(); j++)
			{
				TBD_WireRecord wr = m_aWires[j];
				buf += "    {\n";
				buf += "      \"from\": \"" + TBD_MapExportJson.Escape(wr.m_sFromPylon) + "\",\n";
				buf += "      \"to\": \"" + TBD_MapExportJson.Escape(wr.m_sToPylon) + "\",\n";
				buf += "      \"cableCount\": " + wr.m_iCableCount.ToString() + ",\n";
				buf += "      \"distanceM\": " + wr.m_fDistanceM.ToString() + ",\n";
				buf += "      \"sagM\": " + wr.m_fSagM.ToString() + "\n";
				buf += "    }";
				if (j < m_aWires.Count() - 1)
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
			Print(TAG + " ABORTED: Powerlines JSON write failed", LogLevel.ERROR);
			return false;
		}

		// Write meta JSON
		FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (mh)
		{
			string mj;
			mj += "{\n";
			mj += "  \"method\": \"mod-powerlines-grid-export\",\n";
			mj += "  \"pylonCount\": " + m_aPylons.Count().ToString() + ",\n";
			mj += "  \"wireSpanCount\": " + m_aWires.Count().ToString() + ",\n";
			mj += "  \"dataFile\": \"power_grid.json\"\n";
			mj += "}\n";
			TBD_MapExportJson.Write(mh, mj, TAG);
			mh.Close();
		}

		Print(string.Format("%1 DONE - %2 pylons, %3 wire spans -> %4",
			TAG, m_aPylons.Count(), m_aWires.Count(), outJson));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected void BuildWireConnections()
	{
		int count = m_aPylons.Count();
		if (count < 2)
			return;

		float maxSpanM = 120.0; // maximum plausible span between consecutive utility poles
		for (int i = 0; i < count; i++)
		{
			TBD_PylonRecord pA = m_aPylons[i];
			float bestDist = maxSpanM;
			int bestIdx = -1;

			for (int j = i + 1; j < count; j++)
			{
				TBD_PylonRecord pB = m_aPylons[j];
				if (pA.m_sType != pB.m_sType)
					continue;

				float d = vector.Distance(pA.m_vPos, pB.m_vPos);
				if (d > 5.0 && d < bestDist)
				{
					bestDist = d;
					bestIdx = j;
				}
			}

			if (bestIdx != -1)
			{
				TBD_PylonRecord pB = m_aPylons[bestIdx];
				int cableCount = 3;
				float sagM = 1.5;
				if (pA.m_sType == "high_voltage")
				{
					cableCount = 6;
					sagM = 3.5;
				}
				m_aWires.Insert(new TBD_WireRecord(pA.m_sId, pB.m_sId, cableCount, bestDist, sagM));
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected bool QueryEntityCallback(IEntity ent)
	{
		if (!ent)
			return true;

		string resName = TBD_MapExportContext.GetEntityResourceName(ent);
		string lowerRes = resName;
		lowerRes.ToLower();

		bool isPylon = false;
		string pylonType = "distribution_pole";

		if (lowerRes.Contains("/powerline") || lowerRes.Contains("powerline_") || lowerRes.Contains("pylon_")
			|| lowerRes.Contains("pole_wood") || lowerRes.Contains("transformer") || lowerRes.Contains("substation"))
		{
			isPylon = true;
			if (lowerRes.Contains("highvoltage") || lowerRes.Contains("pylon_steel") || lowerRes.Contains("metal_tower"))
				pylonType = "high_voltage";
			else if (lowerRes.Contains("transformer") || lowerRes.Contains("substation"))
				pylonType = "transformer";
			else
				pylonType = "distribution_pole";
		}

		if (!isPylon)
			return true;

		vector mat[4];
		ent.GetWorldTransform(mat);
		vector origin = mat[3];

		vector mins, maxs;
		ent.GetBounds(mins, maxs);
		float heightM = maxs[1] - mins[1];
		if (heightM < 1.0)
			heightM = 9.0;

		string pId = "pylon_" + (m_aPylons.Count() + 1).ToString();
		m_aPylons.Insert(new TBD_PylonRecord(pId, pylonType, origin, heightM, resName));

		return true;
	}
}
