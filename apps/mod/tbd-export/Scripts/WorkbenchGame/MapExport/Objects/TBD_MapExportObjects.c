/**
 * TBD_MapExportObjects.c
 *
 * Full-world entity extractor & classifier:
 * Iterates the world in spatial cell passes (default 512 m), queries BaseWorld.QueryEntitiesByAABB,
 * classifies entities into:
 *   1. Buildings (partitioned by type into buildings/residential.json, military.json, commercial.json,
 *      industrial.json, civic.json, sheds_garages.json + all_buildings.jsonl + buildings_meta.json)
 *   2. Tactical props & clutter (props/props.jsonl + props_meta.json)
 *   3. Natural foliage & terrain formations (vegetation/trees.jsonl, vegetation/rocks.jsonl + vegetation_meta.json)
 */

class TBD_BuildingExportRecord
{
	string m_sResourceName;
	string m_sClassName;
	string m_sSubtype;
	vector m_vPos;
	vector m_vAngles;
	vector m_vHalfExtents;

	void TBD_BuildingExportRecord(string resName, string clsName, string subtype, vector pos, vector angles, vector halfExt)
	{
		m_sResourceName = resName;
		m_sClassName = clsName;
		m_sSubtype = subtype;
		m_vPos = pos;
		m_vAngles = angles;
		m_vHalfExtents = halfExt;
	}
}

class TBD_MapExportObjects
{
	protected static const string TAG = "[TBD][WorldObjects]";
	protected static const float Y_MIN = -1000.0;
	protected static const float Y_MAX = 2000.0;
	protected static const int FLUSH = 8000;

	protected ref array<IEntity> m_aHits;

	// Categorized in-memory building records for type-specific JSON generation
	protected ref array<ref TBD_BuildingExportRecord> m_aResidential;
	protected ref array<ref TBD_BuildingExportRecord> m_aMilitary;
	protected ref array<ref TBD_BuildingExportRecord> m_aCommercial;
	protected ref array<ref TBD_BuildingExportRecord> m_aIndustrial;
	protected ref array<ref TBD_BuildingExportRecord> m_aCivic;
	protected ref array<ref TBD_BuildingExportRecord> m_aSheds;
	protected ref array<ref TBD_BuildingExportRecord> m_aGenericBuildings;

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
	//! Classifies an entity into primary category ("building", "prop", "tree", "rock") and sub-type.
	static void ClassifyEntity(string resName, string className, out string primaryKind, out string subType)
	{
		string lowerRes = resName;
		lowerRes.ToLower();
		string lowerCls = className;
		lowerCls.ToLower();

		// 1. Buildings
		if (lowerCls.Contains("building") || lowerRes.Contains("/structures/") || lowerRes.Contains("/houses/")
			|| lowerRes.Contains("/industrial/") || lowerRes.Contains("/military/") || lowerRes.Contains("/residential/")
			|| lowerRes.Contains("/commercial/") || lowerRes.Contains("/civic/") || lowerRes.Contains("/sheds/")
			|| lowerRes.Contains("/ruins/") || lowerRes.Contains("/churches/") || lowerRes.Contains("/castle")
			|| lowerRes.Contains("house_"))
		{
			primaryKind = "building";
			if (lowerRes.Contains("residential") || lowerRes.Contains("village") || lowerRes.Contains("house") || lowerRes.Contains("town") || lowerRes.Contains("apartment"))
				subType = "residential";
			else if (lowerRes.Contains("military") || lowerRes.Contains("barracks") || lowerRes.Contains("bunker") || lowerRes.Contains("guardtower") || lowerRes.Contains("checkpoint") || lowerRes.Contains("depot"))
				subType = "military";
			else if (lowerRes.Contains("commercial") || lowerRes.Contains("shop") || lowerRes.Contains("hotel") || lowerRes.Contains("office") || lowerRes.Contains("pub") || lowerRes.Contains("market"))
				subType = "commercial";
			else if (lowerRes.Contains("industrial") || lowerRes.Contains("factory") || lowerRes.Contains("warehouse") || lowerRes.Contains("powerplant") || lowerRes.Contains("substation") || lowerRes.Contains("silo") || lowerRes.Contains("crane"))
				subType = "industrial";
			else if (lowerRes.Contains("castle") || lowerRes.Contains("church") || lowerRes.Contains("monument") || lowerRes.Contains("chapel") || lowerRes.Contains("cathedral") || lowerRes.Contains("museum") || lowerRes.Contains("hall"))
				subType = "civic";
			else if (lowerRes.Contains("shed") || lowerRes.Contains("garage") || lowerRes.Contains("barn") || lowerRes.Contains("outhouse"))
				subType = "sheds_garages";
			else
				subType = "generic";
			return;
		}

		// 2. Trees / Foliage
		if (lowerCls.Contains("tree") || lowerRes.Contains("/trees/") || lowerRes.Contains("tree_")
			|| lowerRes.Contains("picea") || lowerRes.Contains("pinus") || lowerRes.Contains("betula")
			|| lowerRes.Contains("fagus") || lowerRes.Contains("quercus") || lowerRes.Contains("/vegetation/"))
		{
			primaryKind = "tree";
			if (lowerRes.Contains("picea") || lowerRes.Contains("pinus") || lowerRes.Contains("conifer"))
				subType = "conifer";
			else if (lowerRes.Contains("betula") || lowerRes.Contains("fagus") || lowerRes.Contains("quercus") || lowerRes.Contains("deciduous"))
				subType = "deciduous";
			else
				subType = "generic_tree";
			return;
		}

		// 3. Rocks / Cliffs
		if (lowerCls.Contains("rock") || lowerRes.Contains("/rocks/") || lowerRes.Contains("rock_") || lowerRes.Contains("boulder") || lowerRes.Contains("cliff"))
		{
			primaryKind = "rock";
			if (lowerRes.Contains("cliff"))
				subType = "cliff";
			else
				subType = "boulder";
			return;
		}

		// 4. Default: Tactical Props & Clutter
		primaryKind = "prop";
		if (lowerRes.Contains("container")) subType = "container";
		else if (lowerRes.Contains("barrier") || lowerRes.Contains("hesco") || lowerRes.Contains("sandbag")) subType = "barrier";
		else if (lowerRes.Contains("crate") || lowerRes.Contains("box")) subType = "crate";
		else if (lowerRes.Contains("sign") || lowerRes.Contains("lamp") || lowerRes.Contains("bench")) subType = "street_furniture";
		else subType = "clutter";
	}

	//------------------------------------------------------------------------------------------------
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		string mapName = ctx.GetMapName(cfg);
		float worldSize = ctx.m_fWorldSize;
		float cellM = cfg.m_fObjectChunkSizeM;
		if (cellM <= 10.0)
			cellM = 512.0;

		int cells = Math.Ceil(worldSize / cellM);

		// Buildings paths
		string outAllBuildings = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "buildings", "all_buildings.jsonl");
		string outBuildingsMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "buildings", "buildings_meta.json");

		// Props paths
		string outProps = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "props", "props.jsonl");
		string outPropsMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "props", "props_meta.json");

		// Vegetation paths
		string outTrees = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "vegetation", "trees.jsonl");
		string outRocks = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "vegetation", "rocks.jsonl");
		string outVegMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "vegetation", "vegetation_meta.json");

		Print(string.Format("%1 Exporting classified world objects across %2 m (%3x%3 cells) for map '%4'...",
			TAG, worldSize, cells, mapName));

		m_aResidential = {};
		m_aMilitary = {};
		m_aCommercial = {};
		m_aIndustrial = {};
		m_aCivic = {};
		m_aSheds = {};
		m_aGenericBuildings = {};

		FileHandle fBld = FileIO.OpenFile(outAllBuildings, FileMode.WRITE);
		FileHandle fProp = FileIO.OpenFile(outProps, FileMode.WRITE);
		FileHandle fTree = FileIO.OpenFile(outTrees, FileMode.WRITE);
		FileHandle fRock = FileIO.OpenFile(outRocks, FileMode.WRITE);

		if (!fBld || !fProp || !fTree || !fRock)
		{
			if (fBld) fBld.Close();
			if (fProp) fProp.Close();
			if (fTree) fTree.Close();
			if (fRock) fRock.Close();
			Print(TAG + " Failed to open destination files for write", LogLevel.ERROR);
			return false;
		}

		int tick0 = System.GetTickCount();
		int totalHits = 0;
		int buildingsKept = 0;
		int propsKept = 0;
		int treesKept = 0;
		int rocksKept = 0;

		string bufBld = "";
		string bufProp = "";
		string bufTree = "";
		string bufRock = "";
		bool writeOk = true;

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
				totalHits += m_aHits.Count();

				foreach (IEntity e : m_aHits)
				{
					vector pos = e.GetOrigin();
					if (pos[0] < 0 || pos[0] > worldSize || pos[2] < 0 || pos[2] > worldSize)
						continue;
					if (CellIndex(pos[0], cellM, cells) != ix || CellIndex(pos[2], cellM, cells) != iz)
						continue;

					vector ang = e.GetAngles();
					vector bmin, bmax;
					e.GetWorldBounds(bmin, bmax);
					float hx = (bmax[0] - bmin[0]) * 0.5;
					float hy = (bmax[1] - bmin[1]) * 0.5;
					float hz = (bmax[2] - bmin[2]) * 0.5;

					string rn = ctx.ResolvePrefab(e);
					string clsName = e.ClassName();

					string primaryKind, subType;
					ClassifyEntity(rn, clsName, primaryKind, subType);

					string row = "{";
					row += "\"resourceName\":\"" + TBD_MapExportJson.Escape(rn) + "\",";
					row += "\"className\":\"" + TBD_MapExportJson.Escape(clsName) + "\",";
					row += "\"subtype\":\"" + TBD_MapExportJson.Escape(subType) + "\",";
					row += "\"x\":" + pos[0].ToString() + ",";
					row += "\"y\":" + pos[1].ToString() + ",";
					row += "\"z\":" + pos[2].ToString() + ",";
					row += "\"headingDeg\":" + ang[1].ToString() + ",";
					row += "\"pitchDeg\":" + ang[0].ToString() + ",";
					row += "\"rollDeg\":" + ang[2].ToString() + ",";
					row += "\"halfExtentsM\":[" + hx.ToString() + "," + hy.ToString() + "," + hz.ToString() + "]";
					row += "}\n";

					if (primaryKind == "building")
					{
						bufBld += row;
						buildingsKept++;

						TBD_BuildingExportRecord bRec = new TBD_BuildingExportRecord(rn, clsName, subType, pos, ang, Vector(hx, hy, hz));
						if (subType == "residential") m_aResidential.Insert(bRec);
						else if (subType == "military") m_aMilitary.Insert(bRec);
						else if (subType == "commercial") m_aCommercial.Insert(bRec);
						else if (subType == "industrial") m_aIndustrial.Insert(bRec);
						else if (subType == "civic") m_aCivic.Insert(bRec);
						else if (subType == "sheds_garages") m_aSheds.Insert(bRec);
						else m_aGenericBuildings.Insert(bRec);

						if (bufBld.Length() > FLUSH)
						{
							writeOk = TBD_MapExportJson.Write(fBld, bufBld, TAG);
							if (!writeOk) break;
							bufBld = "";
						}
					}
					else if (primaryKind == "tree")
					{
						bufTree += row;
						treesKept++;
						if (bufTree.Length() > FLUSH)
						{
							writeOk = TBD_MapExportJson.Write(fTree, bufTree, TAG);
							if (!writeOk) break;
							bufTree = "";
						}
					}
					else if (primaryKind == "rock")
					{
						bufRock += row;
						rocksKept++;
						if (bufRock.Length() > FLUSH)
						{
							writeOk = TBD_MapExportJson.Write(fRock, bufRock, TAG);
							if (!writeOk) break;
							bufRock = "";
						}
					}
					else
					{
						bufProp += row;
						propsKept++;
						if (bufProp.Length() > FLUSH)
						{
							writeOk = TBD_MapExportJson.Write(fProp, bufProp, TAG);
							if (!writeOk) break;
							bufProp = "";
						}
					}
				}
				if (!writeOk)
					break;
			}
			if (!writeOk)
				break;
		}

		if (writeOk && bufBld.Length() > 0) writeOk = TBD_MapExportJson.Write(fBld, bufBld, TAG);
		if (writeOk && bufProp.Length() > 0) writeOk = TBD_MapExportJson.Write(fProp, bufProp, TAG);
		if (writeOk && bufTree.Length() > 0) writeOk = TBD_MapExportJson.Write(fTree, bufTree, TAG);
		if (writeOk && bufRock.Length() > 0) writeOk = TBD_MapExportJson.Write(fRock, bufRock, TAG);

		fBld.Close();
		fProp.Close();
		fTree.Close();
		fRock.Close();

		if (!writeOk)
		{
			Print(TAG + " ABORTED: Stream write failed", LogLevel.ERROR);
			return false;
		}

		// Write partitioned building JSON files
		WriteBuildingCategoryJson(TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "buildings", "residential.json"), m_aResidential, "residential");
		WriteBuildingCategoryJson(TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "buildings", "military.json"), m_aMilitary, "military");
		WriteBuildingCategoryJson(TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "buildings", "commercial.json"), m_aCommercial, "commercial");
		WriteBuildingCategoryJson(TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "buildings", "industrial.json"), m_aIndustrial, "industrial");
		WriteBuildingCategoryJson(TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "buildings", "civic.json"), m_aCivic, "civic");
		WriteBuildingCategoryJson(TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "buildings", "sheds_garages.json"), m_aSheds, "sheds_garages");

		int elapsedMs = System.GetTickCount() - tick0;

		// Write Metadata JSONs
		WriteBuildingsMetaJson(outBuildingsMeta, mapName, worldSize, buildingsKept, m_aResidential.Count(), m_aMilitary.Count(), m_aCommercial.Count(), m_aIndustrial.Count(), m_aCivic.Count(), m_aSheds.Count(), m_aGenericBuildings.Count(), elapsedMs);
		WritePropsMetaJson(outPropsMeta, mapName, worldSize, propsKept, elapsedMs);
		WriteVegetationMetaJson(outVegMeta, mapName, worldSize, treesKept, rocksKept, elapsedMs);

		Print(string.Format("%1 DONE - Buildings=%2, Props=%3, Trees=%4, Rocks=%5 in %6 ms -> %7",
			TAG, buildingsKept, propsKept, treesKept, rocksKept, elapsedMs, TBD_MapExportPaths.GetCategoryDir(cfg.m_sDestinationDir, mapName)));

		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteBuildingCategoryJson(string filePath, array<ref TBD_BuildingExportRecord> records, string categoryName)
	{
		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f) return;

		string buf = "[\n";
		bool writeOk = true;

		for (int i = 0; i < records.Count(); i++)
		{
			TBD_BuildingExportRecord b = records[i];
			buf += "  {\n";
			buf += "    \"resourceName\": \"" + TBD_MapExportJson.Escape(b.m_sResourceName) + "\",\n";
			buf += "    \"className\": \"" + TBD_MapExportJson.Escape(b.m_sClassName) + "\",\n";
			buf += "    \"subtype\": \"" + TBD_MapExportJson.Escape(b.m_sSubtype) + "\",\n";
			buf += "    \"pos\": [" + b.m_vPos[0].ToString() + ", " + b.m_vPos[1].ToString() + ", " + b.m_vPos[2].ToString() + "],\n";
			buf += "    \"angles\": [" + b.m_vAngles[0].ToString() + ", " + b.m_vAngles[1].ToString() + ", " + b.m_vAngles[2].ToString() + "],\n";
			buf += "    \"halfExtentsM\": [" + b.m_vHalfExtents[0].ToString() + ", " + b.m_vHalfExtents[1].ToString() + ", " + b.m_vHalfExtents[2].ToString() + "]\n";
			buf += "  }";
			if (i < records.Count() - 1)
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
			TBD_MapExportJson.Write(f, buf, TAG);
		}
		f.Close();
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteBuildingsMetaJson(string path, string mapName, float worldSize, int total, int residential, int military, int commercial, int industrial, int civic, int sheds, int generic, int elapsedMs)
	{
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f) return;

		string j = "{\n";
		j += "  \"mapName\": \"" + TBD_MapExportJson.Escape(mapName) + "\",\n";
		j += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
		j += "  \"totalBuildingCount\": " + total.ToString() + ",\n";
		j += "  \"countsBySubtype\": {\n";
		j += "    \"residential\": " + residential.ToString() + ",\n";
		j += "    \"military\": " + military.ToString() + ",\n";
		j += "    \"commercial\": " + commercial.ToString() + ",\n";
		j += "    \"industrial\": " + industrial.ToString() + ",\n";
		j += "    \"civic\": " + civic.ToString() + ",\n";
		j += "    \"sheds_garages\": " + sheds.ToString() + ",\n";
		j += "    \"generic\": " + generic.ToString() + "\n";
		j += "  },\n";
		j += "  \"elapsedMs\": " + elapsedMs.ToString() + "\n";
		j += "}\n";

		TBD_MapExportJson.Write(f, j, TAG);
		f.Close();
	}

	//------------------------------------------------------------------------------------------------
	protected void WritePropsMetaJson(string path, string mapName, float worldSize, int total, int elapsedMs)
	{
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f) return;

		string j = "{\n";
		j += "  \"mapName\": \"" + TBD_MapExportJson.Escape(mapName) + "\",\n";
		j += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
		j += "  \"totalPropsCount\": " + total.ToString() + ",\n";
		j += "  \"elapsedMs\": " + elapsedMs.ToString() + "\n";
		j += "}\n";

		TBD_MapExportJson.Write(f, j, TAG);
		f.Close();
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteVegetationMetaJson(string path, string mapName, float worldSize, int trees, int rocks, int elapsedMs)
	{
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f) return;

		string j = "{\n";
		j += "  \"mapName\": \"" + TBD_MapExportJson.Escape(mapName) + "\",\n";
		j += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
		j += "  \"treeCount\": " + trees.ToString() + ",\n";
		j += "  \"rockCount\": " + rocks.ToString() + ",\n";
		j += "  \"elapsedMs\": " + elapsedMs.ToString() + "\n";
		j += "}\n";

		TBD_MapExportJson.Write(f, j, TAG);
		f.Close();
	}
}
