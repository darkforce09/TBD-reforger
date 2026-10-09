/**
 * TBD_MapExportAnchors.c
 *
 * Authoritative terrain anchor and georeferencing verification oracle.
 * Locates major landmark structures across the terrain and captures sub-millimeter
 * world coordinates, ground elevations, structure heights, and headings.
 *
 * Outputs:
 *   - TBD_AnchorsExport.json
 *   - TBD_AnchorsExport_meta.json
 */

class TBD_AnchorRecord
{
	string m_sId;
	string m_sName;
	string m_sCategory;
	vector m_vPos;
	float m_fGroundElevationYM;
	float m_fApexElevationYM;
	float m_fStructureHeightM;
	float m_fHeadingDeg;
	string m_sPrefab;

	void TBD_AnchorRecord(string id, string name, string category, vector pos, float groundY, float apexY, float heightM, float headingDeg, string prefab)
	{
		m_sId = id;
		m_sName = name;
		m_sCategory = category;
		m_vPos = pos;
		m_fGroundElevationYM = groundY;
		m_fApexElevationYM = apexY;
		m_fStructureHeightM = heightM;
		m_fHeadingDeg = headingDeg;
		m_sPrefab = prefab;
	}
}

class TBD_MapExportAnchors
{
	protected static const string TAG = "[TBD][Anchors]";
	protected static const int FLUSH = 8000;

	protected ref array<ref TBD_AnchorRecord> m_aAnchors;
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
		m_aAnchors = {};

		float worldSize = ctx.m_fWorldSize;
		float cellSize = cfg.m_fObjectChunkSizeM;
		if (cellSize <= 10.0)
			cellSize = 512.0;

		int cells = Math.Ceil(worldSize / cellSize);

		string mapName = ctx.GetMapName(cfg);
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "anchors", "verification.json");
		string outMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "anchors", "anchors_meta.json");

		Print(string.Format("%1 Scanning landmark anchors across %2x%2 grid (%3 m cells)...",
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

		Print(string.Format("%1 Cataloged %2 authoritative anchors. Writing to %3...",
			TAG, m_aAnchors.Count(), outJson));

		FileHandle f = FileIO.OpenFile(outJson, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open " + outJson + " for write", LogLevel.ERROR);
			return false;
		}

		string buf = "[\n";
		bool writeOk = true;

		for (int i = 0; i < m_aAnchors.Count(); i++)
		{
			TBD_AnchorRecord a = m_aAnchors[i];
			buf += "  {\n";
			buf += "    \"id\": \"" + TBD_MapExportJson.Escape(a.m_sId) + "\",\n";
			buf += "    \"name\": \"" + TBD_MapExportJson.Escape(a.m_sName) + "\",\n";
			buf += "    \"category\": \"" + TBD_MapExportJson.Escape(a.m_sCategory) + "\",\n";
			buf += "    \"pos\": [" + a.m_vPos[0].ToString() + ", " + a.m_vPos[1].ToString() + ", " + a.m_vPos[2].ToString() + "],\n";
			buf += "    \"groundElevationYM\": " + a.m_fGroundElevationYM.ToString() + ",\n";
			buf += "    \"apexElevationYM\": " + a.m_fApexElevationYM.ToString() + ",\n";
			buf += "    \"structureHeightM\": " + a.m_fStructureHeightM.ToString() + ",\n";
			buf += "    \"headingDeg\": " + a.m_fHeadingDeg.ToString() + ",\n";
			buf += "    \"prefab\": \"" + TBD_MapExportJson.Escape(a.m_sPrefab) + "\"\n";
			buf += "  }";
			if (i < m_aAnchors.Count() - 1)
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
			Print(TAG + " ABORTED: Anchors JSON write failed", LogLevel.ERROR);
			return false;
		}

		// Write meta JSON
		FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (mh)
		{
			string mj;
			mj += "{\n";
			mj += "  \"method\": \"mod-anchors-oracle-export\",\n";
			mj += "  \"anchorCount\": " + m_aAnchors.Count().ToString() + ",\n";
			mj += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
			mj += "  \"dataFile\": \"verification.json\"\n";
			mj += "}\n";
			TBD_MapExportJson.Write(mh, mj, TAG);
			mh.Close();
		}

		Print(string.Format("%1 DONE - Exported %2 landmark anchors -> %3", TAG, m_aAnchors.Count(), outJson));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected bool QueryEntityCallback(IEntity ent)
	{
		if (!ent)
			return true;

		string resName = TBD_MapExportContext.GetEntityResourceName(ent);
		if (resName.IsEmpty())
			return true;

		string lowerRes = resName;
		lowerRes.ToLower();

		bool isLandmark = false;
		string category = "civic_landmark";
		string name = "Landmark";

		if (lowerRes.Contains("church") || lowerRes.Contains("cathedral") || lowerRes.Contains("chapel"))
		{
			isLandmark = true;
			category = "civic_landmark";
			name = "Church Tower";
		}
		else if (lowerRes.Contains("lighthouse"))
		{
			isLandmark = true;
			category = "maritime_landmark";
			name = "Lighthouse";
		}
		else if (lowerRes.Contains("controltower") || lowerRes.Contains("airport_tower") || lowerRes.Contains("hangar_large"))
		{
			isLandmark = true;
			category = "aviation_landmark";
			name = "Airport Control Tower";
		}
		else if (lowerRes.Contains("radiomast") || lowerRes.Contains("antenna_tower") || lowerRes.Contains("radar_dome") || lowerRes.Contains("broadcast"))
		{
			isLandmark = true;
			category = "communications_landmark";
			name = "Radio Broadcast Tower";
		}
		else if (lowerRes.Contains("castle") || lowerRes.Contains("monument") || lowerRes.Contains("fortress"))
		{
			isLandmark = true;
			category = "historic_landmark";
			name = "Castle / Monument";
		}

		if (!isLandmark)
			return true;

		vector mat[4];
		ent.GetWorldTransform(mat);
		vector origin = mat[3];

		vector mins, maxs;
		ent.GetBounds(mins, maxs);

		float groundY = m_Ctx.m_API.GetTerrainSurfaceY(origin[0], origin[2]);
		float apexY = origin[1] + maxs[1];
		float structHeight = apexY - groundY;
		if (structHeight < 1.0) structHeight = maxs[1] - mins[1];

		vector angles = mat[2].VectorToAngles();
		float heading = angles[0];
		if (heading < 0) heading += 360.0;

		string id = "anchor_" + (m_aAnchors.Count() + 1).ToString();
		m_aAnchors.Insert(new TBD_AnchorRecord(id, name, category, origin, groundY, apexY, structHeight, heading, resName));

		return true;
	}
}
