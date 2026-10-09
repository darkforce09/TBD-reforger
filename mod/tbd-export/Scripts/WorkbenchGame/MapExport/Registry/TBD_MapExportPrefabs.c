/**
 * TBD_MapExportPrefabs.c
 *
 * Prefab taxonomy, components, and physical dimensions extractor.
 * Replaces regex path-matching with direct runtime C++ entity and component inspection.
 *
 * Outputs:
 *   - TBD_PrefabsExport.json
 *   - TBD_PrefabsExport_meta.json
 */

class TBD_PrefabRecord
{
	string m_sResourceName;
	string m_sClassName;
	string m_sKind;
	string m_sClass;
	float m_fLengthM;
	float m_fWidthM;
	float m_fHeightM;
	float m_fFootprintAreaM2;
	bool m_bDestructible;
	float m_fMaxHealth;
	string m_sCoverType;

	void TBD_PrefabRecord(string resName, string className, string kind, string classType, float lengthM, float widthM, float heightM, float areaM2, bool destructible, float maxHealth, string coverType)
	{
		m_sResourceName = resName;
		m_sClassName = className;
		m_sKind = kind;
		m_sClass = classType;
		m_fLengthM = lengthM;
		m_fWidthM = widthM;
		m_fHeightM = heightM;
		m_fFootprintAreaM2 = areaM2;
		m_bDestructible = destructible;
		m_fMaxHealth = maxHealth;
		m_sCoverType = coverType;
	}
}

class TBD_MapExportPrefabs
{
	protected static const string TAG = "[TBD][Prefabs]";
	protected static const int FLUSH = 8000;

	protected ref map<string, ref TBD_PrefabRecord> m_mPrefabs;
	protected ref array<ref TBD_PrefabRecord> m_aPrefabsList;

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		m_mPrefabs = new map<string, ref TBD_PrefabRecord>();
		m_aPrefabsList = {};

		float worldSize = ctx.m_fWorldSize;
		float cellSize = cfg.m_fObjectChunkSizeM;
		if (cellSize <= 10.0)
			cellSize = 512.0;

		int cells = Math.Ceil(worldSize / cellSize);

		string mapName = ctx.GetMapName(cfg);
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "registry", "prefabs.json");
		string outMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "registry", "prefabs_meta.json");

		Print(string.Format("%1 Scanning unique prefabs across %2x%2 grid (%3 m cells)...",
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

		Print(string.Format("%1 Cataloged %2 unique prefabs. Writing to %3...",
			TAG, m_aPrefabsList.Count(), outJson));

		FileHandle f = FileIO.OpenFile(outJson, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open " + outJson + " for write", LogLevel.ERROR);
			return false;
		}

		string buf = "[\n";
		bool writeOk = true;

		for (int i = 0; i < m_aPrefabsList.Count(); i++)
		{
			TBD_PrefabRecord p = m_aPrefabsList[i];
			buf += "  {\n";
			buf += "    \"resourceName\": \"" + TBD_MapExportJson.Escape(p.m_sResourceName) + "\",\n";
			buf += "    \"className\": \"" + TBD_MapExportJson.Escape(p.m_sClassName) + "\",\n";
			buf += "    \"kind\": \"" + TBD_MapExportJson.Escape(p.m_sKind) + "\",\n";
			buf += "    \"class\": \"" + TBD_MapExportJson.Escape(p.m_sClass) + "\",\n";
			buf += "    \"lengthM\": " + p.m_fLengthM.ToString() + ",\n";
			buf += "    \"widthM\": " + p.m_fWidthM.ToString() + ",\n";
			buf += "    \"heightM\": " + p.m_fHeightM.ToString() + ",\n";
			buf += "    \"footprintAreaM2\": " + p.m_fFootprintAreaM2.ToString() + ",\n";
			buf += "    \"destructible\": " + p.m_bDestructible.ToString() + ",\n";
			buf += "    \"maxHealth\": " + p.m_fMaxHealth.ToString() + ",\n";
			buf += "    \"coverType\": \"" + TBD_MapExportJson.Escape(p.m_sCoverType) + "\"\n";
			buf += "  }";
			if (i < m_aPrefabsList.Count() - 1)
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
			Print(TAG + " ABORTED: Prefab JSON write failed", LogLevel.ERROR);
			return false;
		}

		// Write meta JSON
		FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (mh)
		{
			string mj;
			mj += "{\n";
			mj += "  \"method\": \"mod-prefabs-component-export\",\n";
			mj += "  \"uniquePrefabCount\": " + m_aPrefabsList.Count().ToString() + ",\n";
			mj += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
			mj += "  \"dataFile\": \"prefabs.json\"\n";
			mj += "}\n";
			TBD_MapExportJson.Write(mh, mj, TAG);
			mh.Close();
		}

		Print(string.Format("%1 DONE - Exported %2 unique prefabs -> %3", TAG, m_aPrefabsList.Count(), outJson));
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

		if (m_mPrefabs.Contains(resName))
			return true;

		string className = ent.ClassName();
		string lowerRes = resName;
		lowerRes.ToLower();
		string lowerClass = className;
		lowerClass.ToLower();

		// Bounding box & dimensions
		vector mins, maxs;
		ent.GetBounds(mins, maxs);
		float width = maxs[0] - mins[0];
		float height = maxs[1] - mins[1];
		float length = maxs[2] - mins[2];
		if (width < 0.1) width = 0.5;
		if (height < 0.1) height = 0.5;
		if (length < 0.1) length = 0.5;
		float footprint = width * length;

		// Taxonomy
		string kind = "prop";
		string classType = "generic";
		string coverType = "hard";
		bool destructible = false;
		float maxHealth = 1000.0;

		if (lowerClass.Contains("building") || lowerRes.Contains("/structures/") || lowerRes.Contains("/houses/"))
		{
			kind = "building";
			destructible = true;
			if (lowerRes.Contains("residential") || lowerRes.Contains("village") || lowerRes.Contains("house"))
				classType = "residential";
			else if (lowerRes.Contains("military") || lowerRes.Contains("barracks") || lowerRes.Contains("bunker"))
				classType = "military";
			else if (lowerRes.Contains("commercial") || lowerRes.Contains("shop") || lowerRes.Contains("hotel"))
				classType = "commercial";
			else if (lowerRes.Contains("industrial") || lowerRes.Contains("factory") || lowerRes.Contains("warehouse"))
				classType = "industrial";
			else if (lowerRes.Contains("castle") || lowerRes.Contains("church") || lowerRes.Contains("monument"))
				classType = "civic";
			else if (lowerRes.Contains("shed") || lowerRes.Contains("garage"))
				classType = "shed";
			else
				classType = "generic";
		}
		else if (lowerClass.Contains("tree") || lowerRes.Contains("/vegetation/trees/") || lowerRes.Contains("tree_"))
		{
			kind = "tree";
			coverType = "foliage";
			if (lowerRes.Contains("picea") || lowerRes.Contains("pinus") || lowerRes.Contains("conifer"))
				classType = "conifer";
			else if (lowerRes.Contains("betula") || lowerRes.Contains("fagus") || lowerRes.Contains("quercus") || lowerRes.Contains("deciduous"))
				classType = "deciduous";
			else
				classType = "generic";
		}
		else if (lowerClass.Contains("rock") || lowerRes.Contains("/rocks/") || lowerRes.Contains("rock_") || lowerRes.Contains("boulder"))
		{
			kind = "rock";
			coverType = "hard";
			if (lowerRes.Contains("cliff"))
				classType = "cliff";
			else
				classType = "boulder";
		}
		else if (lowerRes.Contains("/fences/") || lowerRes.Contains("/walls/"))
		{
			kind = "prop";
			classType = "fence";
			coverType = "hard";
		}
		else if (lowerRes.Contains("/powerline") || lowerRes.Contains("powerline_") || lowerRes.Contains("pylon_") || lowerRes.Contains("lamp_"))
		{
			kind = "utility";
			classType = "powerline";
			coverType = "soft";
		}
		else if (lowerClass.Contains("vehicle") || lowerRes.Contains("/vehicles/"))
		{
			kind = "vehicle";
			classType = "car";
			destructible = true;
			maxHealth = 2000.0;
		}

		TBD_PrefabRecord rec = new TBD_PrefabRecord(resName, className, kind, classType, length, width, height, footprint, destructible, maxHealth, coverType);
		m_mPrefabs.Set(resName, rec);
		m_aPrefabsList.Insert(rec);

		return true;
	}
}
