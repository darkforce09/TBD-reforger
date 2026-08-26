/**
 * TBD_MapExportFences.c
 *
 * Tactical micro-cover extractor: fences, stone walls, hedgerows, and perimeter barriers.
 * Converts 3D OBBs into oriented centerline vector strips with height and thickness.
 *
 * Outputs:
 *   - TBD_FencesExport.jsonl
 *   - TBD_FencesExport_meta.json
 */

class TBD_FenceRecord
{
	string m_sId;
	string m_sWallClass;
	vector m_vP1;
	vector m_vP2;
	float m_fHeightM;
	float m_fThicknessM;
	string m_sPrefab;

	void TBD_FenceRecord(string id, string wallClass, vector p1, vector p2, float heightM, float thicknessM, string prefab)
	{
		m_sId = id;
		m_sWallClass = wallClass;
		m_vP1 = p1;
		m_vP2 = p2;
		m_fHeightM = heightM;
		m_fThicknessM = thicknessM;
		m_sPrefab = prefab;
	}
}

class TBD_MapExportFences
{
	protected static const string TAG = "[TBD][Fences]";
	protected static const int FLUSH = 8000;

	protected ref array<ref TBD_FenceRecord> m_aFences;
	protected int m_iCount;

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		m_aFences = {};
		m_iCount = 0;

		float worldSize = ctx.m_fWorldSize;
		float cellSize = cfg.m_fObjectChunkSizeM;
		if (cellSize <= 10.0)
			cellSize = 512.0;

		int cells = Math.Ceil(worldSize / cellSize);

		string outJsonl = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_FencesExport.jsonl");
		string outMeta = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_FencesExport_meta.json");

		Print(string.Format("%1 Scanning fences/walls across %2x%2 grid (%3 m cells)...",
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

		Print(string.Format("%1 Found %2 fence/wall segments. Writing to %3...", TAG, m_aFences.Count(), outJsonl));

		FileHandle f = FileIO.OpenFile(outJsonl, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open " + outJsonl + " for write", LogLevel.ERROR);
			return false;
		}

		string buf = "";
		bool writeOk = true;

		for (int i = 0; i < m_aFences.Count(); i++)
		{
			TBD_FenceRecord fnc = m_aFences[i];
			buf += "{\"id\":\"" + TBD_MapExportJson.Escape(fnc.m_sId) + "\"";
			buf += ",\"wallClass\":\"" + TBD_MapExportJson.Escape(fnc.m_sWallClass) + "\"";
			buf += ",\"p1\":[" + fnc.m_vP1[0].ToString() + "," + fnc.m_vP1[1].ToString() + "," + fnc.m_vP1[2].ToString() + "]";
			buf += ",\"p2\":[" + fnc.m_vP2[0].ToString() + "," + fnc.m_vP2[1].ToString() + "," + fnc.m_vP2[2].ToString() + "]";
			buf += ",\"heightM\":" + fnc.m_fHeightM.ToString();
			buf += ",\"thicknessM\":" + fnc.m_fThicknessM.ToString();
			buf += ",\"prefab\":\"" + TBD_MapExportJson.Escape(fnc.m_sPrefab) + "\"}\n";

			if (buf.Length() > FLUSH)
			{
				writeOk = TBD_MapExportJson.Write(f, buf, TAG);
				if (!writeOk) break;
				buf = "";
			}
		}

		if (writeOk && buf.Length() > 0)
			writeOk = TBD_MapExportJson.Write(f, buf, TAG);
		f.Close();

		if (!writeOk)
		{
			FileIO.DeleteFile(outJsonl);
			Print(TAG + " ABORTED: Fence JSONL write failed", LogLevel.ERROR);
			return false;
		}

		// Write meta JSON
		FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (mh)
		{
			string mj;
			mj += "{\n";
			mj += "  \"method\": \"mod-fences-walls-export\",\n";
			mj += "  \"segmentCount\": " + m_aFences.Count().ToString() + ",\n";
			mj += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
			mj += "  \"dataFile\": \"TBD_FencesExport.jsonl\"\n";
			mj += "}\n";
			TBD_MapExportJson.Write(mh, mj, TAG);
			mh.Close();
		}

		Print(string.Format("%1 DONE — Exported %2 fence segments -> %3", TAG, m_aFences.Count(), outJsonl));
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

		bool isFence = false;
		string wallClass = "wooden_fence";
		float thickness = 0.35;

		if (lowerRes.Contains("/walls/") || lowerRes.Contains("/fences/") || lowerRes.Contains("/barriers/")
			|| lowerRes.Contains("fence_") || lowerRes.Contains("wall_") || lowerRes.Contains("barrier_") || lowerRes.Contains("sandbag"))
		{
			isFence = true;
			if (lowerRes.Contains("stone") || lowerRes.Contains("rockwall"))
			{
				wallClass = "stone_wall";
				thickness = 0.5;
			}
			else if (lowerRes.Contains("concrete") || lowerRes.Contains("panel"))
			{
				wallClass = "concrete_wall";
				thickness = 0.4;
			}
			else if (lowerRes.Contains("wire") || lowerRes.Contains("mesh") || lowerRes.Contains("chainlink"))
			{
				wallClass = "wire_fence";
				thickness = 0.15;
			}
			else if (lowerRes.Contains("sandbag") || lowerRes.Contains("barrier") || lowerRes.Contains("hesco"))
			{
				wallClass = "barrier";
				thickness = 0.8;
			}
		}

		if (!isFence)
			return true;

		vector mat[4];
		ent.GetWorldTransform(mat);
		vector origin = mat[3];

		vector mins, maxs;
		ent.GetBounds(mins, maxs);
		vector halfExtents = (maxs - mins) * 0.5;

		float heightM = maxs[1] - mins[1];
		if (heightM <= 0.1)
			heightM = 1.2;

		// The long horizontal axis is the wall direction
		vector axis = mat[0]; // local X
		float halfLength = halfExtents[0];
		if (halfExtents[2] > halfExtents[0])
		{
			axis = mat[2]; // local Z
			halfLength = halfExtents[2];
		}

		if (halfLength < 0.2)
			halfLength = 2.0;

		vector p1 = origin - axis * halfLength;
		vector p2 = origin + axis * halfLength;

		m_iCount++;
		string id = "fence_" + m_iCount.ToString();
		m_aFences.Insert(new TBD_FenceRecord(id, wallClass, p1, p2, heightM, thickness, resName));

		return true;
	}
}
