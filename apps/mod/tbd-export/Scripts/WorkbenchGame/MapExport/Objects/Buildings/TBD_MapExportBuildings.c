/**
 * TBD_MapExportBuildings.c
 *
 * Placed building and architectural structure extractor:
 *   - Deeply inspects building prefabs using TBD_BuildingArchitectExtractor
 *   - Extracts unique multi-floor blueprints (walls, doors, windows, glass, furniture, stairs)
 *   - Generates per-instance spatial records linking world placements to prefab blueprints
 *   - Partitions into residential, military, commercial, industrial, civic, and sheds
 *
 * Outputs:
 *   - prefabs/buildings/<prefabSlug>.json (Archetype blueprints)
 *   - objects/buildings/all_buildings.jsonl (World instances)
 *   - objects/buildings/residential.json, military.json, commercial.json, industrial.json, civic.json, sheds_garages.json
 *   - objects/buildings/buildings_meta.json
 */

class TBD_BuildingInstanceRecord
{
	string m_sId;
	string m_sPrefabId;
	string m_sResourceName;
	string m_sSubtype;
	vector m_vPos;
	vector m_vAngles;
	vector m_vHalfExtents;

	void TBD_BuildingInstanceRecord(string id, string prefabId, string resName, string subtype, vector pos, vector angles, vector halfExt)
	{
		m_sId = id;
		m_sPrefabId = prefabId;
		m_sResourceName = resName;
		m_sSubtype = subtype;
		m_vPos = pos;
		m_vAngles = angles;
		m_vHalfExtents = halfExt;
	}

	string ToJsonLine()
	{
		string json = "{";
		json += "\"id\":\"" + TBD_MapExportJson.Escape(m_sId) + "\",";
		json += "\"prefabId\":\"" + TBD_MapExportJson.Escape(m_sPrefabId) + "\",";
		json += "\"resourceName\":\"" + TBD_MapExportJson.Escape(m_sResourceName) + "\",";
		json += "\"subtype\":\"" + TBD_MapExportJson.Escape(m_sSubtype) + "\",";
		json += "\"x\":" + m_vPos[0].ToString() + ",";
		json += "\"y\":" + m_vPos[1].ToString() + ",";
		json += "\"z\":" + m_vPos[2].ToString() + ",";
		json += "\"headingDeg\":" + m_vAngles[1].ToString() + ",";
		json += "\"pitchDeg\":" + m_vAngles[0].ToString() + ",";
		json += "\"rollDeg\":" + m_vAngles[2].ToString() + ",";
		json += "\"halfExtentsM\":[" + m_vHalfExtents[0].ToString() + "," + m_vHalfExtents[1].ToString() + "," + m_vHalfExtents[2].ToString() + "]";
		json += "}\n";
		return json;
	}
}

class TBD_MapExportBuildings
{
	protected static const string TAG = "[TBD][Objects][Buildings]";
	protected static const float Y_MIN = -1000.0;
	protected static const float Y_MAX = 2000.0;
	protected static const int FLUSH = 8000;

	protected ref array<IEntity> m_aHits;
	protected ref map<string, ref TBD_BuildingBlueprint> m_mCatalogedBlueprints;

	// Categorized instances
	protected ref array<ref TBD_BuildingInstanceRecord> m_aResidential;
	protected ref array<ref TBD_BuildingInstanceRecord> m_aMilitary;
	protected ref array<ref TBD_BuildingInstanceRecord> m_aCommercial;
	protected ref array<ref TBD_BuildingInstanceRecord> m_aIndustrial;
	protected ref array<ref TBD_BuildingInstanceRecord> m_aCivic;
	protected ref array<ref TBD_BuildingInstanceRecord> m_aSheds;
	protected ref array<ref TBD_BuildingInstanceRecord> m_aGeneric;

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
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World)
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

		string outAllBuildings = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "objects/buildings", "all_buildings.jsonl");
		string outMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "objects/buildings", "buildings_meta.json");

		Print(string.Format("%1 Starting architectural building export across %2 m (%3x%3 cells)...",
			TAG, worldSize, cells));

		m_mCatalogedBlueprints = new map<string, ref TBD_BuildingBlueprint>();
		m_aResidential = {};
		m_aMilitary = {};
		m_aCommercial = {};
		m_aIndustrial = {};
		m_aCivic = {};
		m_aSheds = {};
		m_aGeneric = {};

		FileHandle fAll = FileIO.OpenFile(outAllBuildings, FileMode.WRITE);
		if (!fAll)
		{
			Print(TAG + " Failed to open " + outAllBuildings + " for write", LogLevel.ERROR);
			return false;
		}

		int tick0 = System.GetTickCount();
		int buildingsKept = 0;
		string bufAll = "";
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

				foreach (IEntity e : m_aHits)
				{
					vector pos = e.GetOrigin();
					if (pos[0] < 0 || pos[0] > worldSize || pos[2] < 0 || pos[2] > worldSize)
						continue;
					if (CellIndex(pos[0], cellM, cells) != ix || CellIndex(pos[2], cellM, cells) != iz)
						continue;

					string rn = ctx.ResolvePrefab(e);
					string clsName = e.ClassName();

					string primaryKind, subType;
					TBD_MapExportObjects.ClassifyEntity(rn, clsName, primaryKind, subType);

					if (primaryKind != "building")
						continue;

					string prefabSlug = TBD_BuildingArchitectExtractor.DerivePrefabSlug(rn);

					// Blueprints only for POSITIVELY classified buildings with a real slug.
					// The classifier's `/structures/` catch-all funnels fences, crash barriers,
					// power poles and signs into subtype "generic" -- the first full-map run
					// wrote 74,198 such junk blueprints plus one empty-slug ".json".
					if (!m_mCatalogedBlueprints.Contains(prefabSlug)
						&& subType != "generic" && !prefabSlug.IsEmpty())
					{
						TBD_BuildingBlueprint bp = TBD_BuildingArchitectExtractor.ExtractBlueprint(e, rn);
						if (bp)
						{
							m_mCatalogedBlueprints.Set(prefabSlug, bp);
							string bpPath = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "prefabs/buildings", prefabSlug + ".json");
							SaveBlueprintJson(bpPath, bp);
						}
					}

					vector ang = e.GetAngles();
					vector bmin, bmax;
					e.GetWorldBounds(bmin, bmax);
					float hx = (bmax[0] - bmin[0]) * 0.5;
					float hy = (bmax[1] - bmin[1]) * 0.5;
					float hz = (bmax[2] - bmin[2]) * 0.5;

					string instName = e.GetName();
					if (instName.IsEmpty())
						instName = string.Format("%1_%2", prefabSlug, buildingsKept);

					TBD_BuildingInstanceRecord rec = new TBD_BuildingInstanceRecord(
						instName, prefabSlug, rn, subType, pos, ang, Vector(hx, hy, hz)
					);

					bufAll += rec.ToJsonLine();
					buildingsKept++;

					if (subType == "residential") m_aResidential.Insert(rec);
					else if (subType == "military") m_aMilitary.Insert(rec);
					else if (subType == "commercial") m_aCommercial.Insert(rec);
					else if (subType == "industrial") m_aIndustrial.Insert(rec);
					else if (subType == "civic") m_aCivic.Insert(rec);
					else if (subType == "sheds_garages") m_aSheds.Insert(rec);
					else m_aGeneric.Insert(rec);

					if (bufAll.Length() > FLUSH)
					{
						writeOk = TBD_MapExportJson.Write(fAll, bufAll, TAG);
						if (!writeOk) break;
						bufAll = "";
					}
				}
				if (!writeOk) break;
			}
			if (!writeOk) break;
		}

		if (writeOk && bufAll.Length() > 0)
			writeOk = TBD_MapExportJson.Write(fAll, bufAll, TAG);

		fAll.Close();

		if (!writeOk)
		{
			Print(TAG + " ABORTED: Building instance write failed", LogLevel.ERROR);
			return false;
		}

		// Write partitioned category JSONs
		WritePartitionJson(TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "objects/buildings", "residential.json"), m_aResidential);
		WritePartitionJson(TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "objects/buildings", "military.json"), m_aMilitary);
		WritePartitionJson(TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "objects/buildings", "commercial.json"), m_aCommercial);
		WritePartitionJson(TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "objects/buildings", "industrial.json"), m_aIndustrial);
		WritePartitionJson(TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "objects/buildings", "civic.json"), m_aCivic);
		WritePartitionJson(TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "objects/buildings", "sheds_garages.json"), m_aSheds);

		int elapsedMs = System.GetTickCount() - tick0;

		// Write metadata
		WriteBuildingsMeta(outMeta, mapName, worldSize, buildingsKept, m_mCatalogedBlueprints.Count(), elapsedMs);

		Print(string.Format("%1 DONE -- Exported %2 buildings across %3 unique blueprints in %4 ms",
			TAG, buildingsKept, m_mCatalogedBlueprints.Count(), elapsedMs));

		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected void SaveBlueprintJson(string path, TBD_BuildingBlueprint bp)
	{
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f) return;

		string json = bp.ToJson();
		TBD_MapExportJson.Write(f, json, TAG);
		f.Close();
		Print(TAG + " Saved blueprint: " + path, LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	protected void WritePartitionJson(string path, array<ref TBD_BuildingInstanceRecord> records)
	{
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f) return;

		string buf = "[\n";
		bool writeOk = true;

		for (int i = 0; i < records.Count(); i++)
		{
			TBD_BuildingInstanceRecord r = records[i];
			buf += "  {\n";
			buf += "    \"id\": \"" + TBD_MapExportJson.Escape(r.m_sId) + "\",\n";
			buf += "    \"prefabId\": \"" + TBD_MapExportJson.Escape(r.m_sPrefabId) + "\",\n";
			buf += "    \"resourceName\": \"" + TBD_MapExportJson.Escape(r.m_sResourceName) + "\",\n";
			buf += "    \"subtype\": \"" + TBD_MapExportJson.Escape(r.m_sSubtype) + "\",\n";
			buf += "    \"pos\": [" + r.m_vPos[0].ToString() + ", " + r.m_vPos[1].ToString() + ", " + r.m_vPos[2].ToString() + "],\n";
			buf += "    \"angles\": [" + r.m_vAngles[0].ToString() + ", " + r.m_vAngles[1].ToString() + ", " + r.m_vAngles[2].ToString() + "],\n";
			buf += "    \"halfExtentsM\": [" + r.m_vHalfExtents[0].ToString() + ", " + r.m_vHalfExtents[1].ToString() + ", " + r.m_vHalfExtents[2].ToString() + "]\n";
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
	protected void WriteBuildingsMeta(string path, string mapName, float worldSize, int totalInstances, int uniqueBlueprints, int elapsedMs)
	{
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f) return;

		string j = "{\n";
		j += "  \"mapName\": \"" + TBD_MapExportJson.Escape(mapName) + "\",\n";
		j += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
		j += "  \"totalBuildingCount\": " + totalInstances.ToString() + ",\n";
		j += "  \"uniqueBlueprintCount\": " + uniqueBlueprints.ToString() + ",\n";
		j += "  \"countsBySubtype\": {\n";
		j += "    \"residential\": " + m_aResidential.Count().ToString() + ",\n";
		j += "    \"military\": " + m_aMilitary.Count().ToString() + ",\n";
		j += "    \"commercial\": " + m_aCommercial.Count().ToString() + ",\n";
		j += "    \"industrial\": " + m_aIndustrial.Count().ToString() + ",\n";
		j += "    \"civic\": " + m_aCivic.Count().ToString() + ",\n";
		j += "    \"sheds_garages\": " + m_aSheds.Count().ToString() + ",\n";
		j += "    \"generic\": " + m_aGeneric.Count().ToString() + "\n";
		j += "  },\n";
		j += "  \"elapsedMs\": " + elapsedMs.ToString() + "\n";
		j += "}\n";

		TBD_MapExportJson.Write(f, j, TAG);
		f.Close();
	}
}
