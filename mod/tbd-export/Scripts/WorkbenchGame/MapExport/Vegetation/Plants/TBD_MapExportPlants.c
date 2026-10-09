/**
 * TBD_MapExportPlants.c
 *
 * Dedicated wild plant and undergrowth extraction engine for Bohemia Reforger.
 * Queries placed world entities across spatial cells (512m), strictly classifies
 * wild herbaceous flora, marine undergrowth (Fucus), and curbside weed strips,
 * and stream-writes a valid JSON document (plants.json) with census breakdown and instance array.
 *
 * Outputs:
 *   - $profile:TBD_Export/<mapName>/vegetation/plants.json
 */

class TBD_PlantRecord
{
	int m_iId;
	string m_sResourceName;
	string m_sSpecies;
	string m_sVariant;
	string m_sEnvironment;
	vector m_vPosition;
	vector m_vRotation;
	float m_fScale;
	float m_fWidth;
	float m_fHeight;
	float m_fDepth;

	void TBD_PlantRecord(int id, string resName, string species, string variant, string env, vector pos, vector rot, float scale, float w, float h, float d)
	{
		m_iId = id;
		m_sResourceName = resName;
		m_sSpecies = species;
		m_sVariant = variant;
		m_sEnvironment = env;
		m_vPosition = pos;
		m_vRotation = rot;
		m_fScale = scale;
		m_fWidth = w;
		m_fHeight = h;
		m_fDepth = d;
	}
}

class TBD_MapExportPlants
{
	protected static const string TAG = "[TBD][Vegetation][Plants]";
	protected static const float Y_MIN = -1000.0;
	protected static const float Y_MAX = 2000.0;
	protected static const int FLUSH = 8000;

	protected ref array<IEntity> m_aHits;

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
	//! Authoritative case-sensitive Plant classifier.
	//! Returns true strictly if the entity is a wild herbaceous plant, marine undergrowth, or curbside weed.
	static bool IsPlantPrefab(string resName, string className, IEntity ent, out string species, out string variant, out string environment)
	{
		species = "unknown_plant";
		variant = "default";
		environment = "terrestrial";

		if (resName.IsEmpty())
			return false;

		// 1. Strict Exclusions:
		// Reject cartographic descriptors, bays, and landmarks
		if (ent)
		{
			if (ent.FindComponent(SCR_MapDescriptorComponent) || ent.FindComponent(MapDescriptorComponent))
				return false;
			if (ent.FindComponent(SCR_EditableCommentComponent))
				return false;
		}

		string lowerRes = resName;
		lowerRes.ToLower();

		// Reject bushes, trees, agricultural crops/vegetables, rocks, props, decorations
		if (lowerRes.Contains("/bush/") || lowerRes.Contains("/bushes/"))
			return false;
		if (lowerRes.Contains("/tree/") || lowerRes.Contains("/trees/"))
			return false;
		if (lowerRes.Contains("/vegetables/") || lowerRes.Contains("/crops/"))
			return false;
		if (lowerRes.Contains("/rocks/") || lowerRes.Contains("/rock/"))
			return false;
		if (lowerRes.Contains("/props/") || lowerRes.Contains("/decorations/") || lowerRes.Contains("/flowerpots/"))
			return false;
		if (lowerRes.Contains("stump") || lowerRes.Contains("cut_trunk") || lowerRes.Contains("trunk_cut"))
			return false;
		if (lowerRes.Contains("fallen") || lowerRes.Contains("deadwood") || lowerRes.Contains("woodlog") || lowerRes.Contains("woodpile"))
			return false;

		// 2. Strict Plant Inclusion:
		// Must reside under Plant folder OR have explicit p_ prefix under vegetation
		bool isPlantDir = resName.Contains("Prefabs/Vegetation/Plant/") || resName.Contains("Vegetation/Plant/") || lowerRes.Contains("/plant/");
		if (!isPlantDir && !lowerRes.Contains("/vegetation/"))
			return false;

		// Extract filename / leaf
		string leaf = resName;
		int slashIdx = leaf.LastIndexOf("/");
		if (slashIdx >= 0)
			leaf = leaf.Substring(slashIdx + 1, leaf.Length() - slashIdx - 1);

		int dotIdx = leaf.LastIndexOf(".");
		if (dotIdx > 0)
			leaf = leaf.Substring(0, dotIdx);

		// Case-sensitive prefix / pattern check:
		if (!leaf.StartsWith("p_") && !leaf.Contains("CurbsideWeeds"))
		{
			if (!isPlantDir)
				return false;
		}

		// 3. Extract species name, variant, and environment:
		if (leaf.Contains("CurbsideWeeds"))
		{
			species = "curbside_weeds";
			variant = leaf;
			environment = "terrestrial";
			return true;
		}

		string cleanName = leaf;
		if (cleanName.StartsWith("p_"))
			cleanName = cleanName.Substring(2, cleanName.Length() - 2);

		array<string> parts = {};
		cleanName.Split("_", parts, false);
		if (parts.Count() >= 3)
		{
			// Genus + species + variant (e.g. fucus + fesiculosus + 02, urtica + dioica + 0s)
			species = parts[0] + "_" + parts[1];
			variant = parts[parts.Count() - 1];
		}
		else if (parts.Count() == 2)
		{
			species = parts[0] + "_" + parts[1];
			variant = "0";
		}
		else if (parts.Count() == 1)
		{
			species = parts[0];
			variant = "0";
		}

		// Normalize known botanical taxonomy & determine marine vs terrestrial environment
		if (species.Contains("fucus"))
		{
			species = "fucus_vesiculosus"; // Canonical botanical correction
			environment = "marine";
		}
		else
		{
			environment = "terrestrial";
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Primary export execution method.
	bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		int tick0 = System.GetTickCount();
		string mapName = ctx.GetMapName(cfg);
		float worldSize = ctx.m_fWorldSize;
		float cellM = cfg.m_fObjectChunkSizeM;
		if (cellM <= 10.0)
			cellM = 512.0;

		int cells = Math.Ceil(worldSize / cellM);
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "vegetation", "plants.json");

		Print(string.Format("%1 Starting plant extraction for map '%2' (%3x%3 cells @ %4 m) -> %5",
			TAG, mapName, cells, cellM, outJson), LogLevel.NORMAL);

		// First pass: Query world entities, classify plants, and collect records
		map<string, int> speciesCounts = new map<string, int>();
		map<string, int> environmentCounts = new map<string, int>();
		array<ref TBD_PlantRecord> plantRecords = {};
		int plantId = 0;

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
					if (!e)
						continue;

					vector pos = e.GetOrigin();
					if (pos[0] < 0 || pos[0] > worldSize || pos[2] < 0 || pos[2] > worldSize)
						continue;
					if (CellIndex(pos[0], cellM, cells) != ix || CellIndex(pos[2], cellM, cells) != iz)
						continue;

					vector bmin, bmax;
					e.GetWorldBounds(bmin, bmax);
					float w = bmax[0] - bmin[0];
					float h = bmax[1] - bmin[1];
					float d = bmax[2] - bmin[2];

					// Height validation safeguard for herbaceous undergrowth
					if (h < 0.1 || h > 4.0)
						continue;

					string rn = ctx.ResolvePrefab(e);
					string clsName = e.ClassName();

					string species, variant, env;
					if (!IsPlantPrefab(rn, clsName, e, species, variant, env))
						continue;

					plantId++;
					vector ang = e.GetAngles();
					float scale = e.GetScale();
					if (scale <= 0.001) scale = 1.0;

					plantRecords.Insert(new TBD_PlantRecord(plantId, rn, species, variant, env, pos, ang, scale, w, h, d));

					// Track species count
					int curSpeciesCount = 0;
					if (speciesCounts.Find(species, curSpeciesCount))
						speciesCounts.Set(species, curSpeciesCount + 1);
					else
						speciesCounts.Insert(species, 1);

					// Track environment count
					int curEnvCount = 0;
					if (environmentCounts.Find(env, curEnvCount))
						environmentCounts.Set(env, curEnvCount + 1);
					else
						environmentCounts.Insert(env, 1);
				}
			}
		}

		int totalPlants = plantRecords.Count();
		Print(string.Format("%1 Discovered %2 authentic plants across %3 species.", TAG, totalPlants, speciesCounts.Count()), LogLevel.NORMAL);

		// Second pass: Stream write valid JSON document
		FileHandle f = FileIO.OpenFile(outJson, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open output file for write: " + outJson, LogLevel.ERROR);
			return false;
		}

		string buf = "";
		buf += "{\n";
		buf += "  \"mapName\": \"" + TBD_MapExportJson.Escape(mapName) + "\",\n";
		buf += "  \"worldSize\": " + worldSize.ToString() + ",\n";
		buf += "  \"totalPlants\": " + totalPlants.ToString() + ",\n";

		// Species Counts Dictionary
		buf += "  \"speciesCounts\": {\n";
		int scTotal = speciesCounts.Count();
		for (int sc = 0; sc < scTotal; sc++)
		{
			string spKey = speciesCounts.GetKey(sc);
			int spVal = speciesCounts.GetElement(sc);
			buf += "    \"" + TBD_MapExportJson.Escape(spKey) + "\": " + spVal.ToString();
			if (sc < scTotal - 1)
				buf += ",";
			buf += "\n";
		}
		buf += "  },\n";

		// Environment Counts Dictionary
		buf += "  \"environmentCounts\": {\n";
		int ecTotal = environmentCounts.Count();
		for (int ec = 0; ec < ecTotal; ec++)
		{
			string envKey = environmentCounts.GetKey(ec);
			int envVal = environmentCounts.GetElement(ec);
			buf += "    \"" + TBD_MapExportJson.Escape(envKey) + "\": " + envVal.ToString();
			if (ec < ecTotal - 1)
				buf += ",";
			buf += "\n";
		}
		buf += "  },\n";

		// Plants Array
		buf += "  \"plants\": [\n";
		if (!TBD_MapExportJson.Write(f, buf, TAG))
		{
			f.Close();
			return false;
		}
		buf = "";

		for (int p = 0; p < totalPlants; p++)
		{
			TBD_PlantRecord rec = plantRecords[p];
			buf += "    {\n";
			buf += "      \"id\": " + rec.m_iId.ToString() + ",\n";
			buf += "      \"resourceName\": \"" + TBD_MapExportJson.Escape(rec.m_sResourceName) + "\",\n";
			buf += "      \"species\": \"" + TBD_MapExportJson.Escape(rec.m_sSpecies) + "\",\n";
			buf += "      \"variant\": \"" + TBD_MapExportJson.Escape(rec.m_sVariant) + "\",\n";
			buf += "      \"environment\": \"" + TBD_MapExportJson.Escape(rec.m_sEnvironment) + "\",\n";
			buf += "      \"position\": [" + rec.m_vPosition[0].ToString() + ", " + rec.m_vPosition[1].ToString() + ", " + rec.m_vPosition[2].ToString() + "],\n";
			buf += "      \"rotation\": [" + rec.m_vRotation[0].ToString() + ", " + rec.m_vRotation[1].ToString() + ", " + rec.m_vRotation[2].ToString() + "],\n";
			buf += "      \"scale\": " + rec.m_fScale.ToString() + ",\n";
			buf += "      \"bounds\": {\"width\": " + rec.m_fWidth.ToString() + ", \"height\": " + rec.m_fHeight.ToString() + ", \"depth\": " + rec.m_fDepth.ToString() + "}\n";
			buf += "    }";

			if (p < totalPlants - 1)
				buf += ",";
			buf += "\n";

			if (buf.Length() > FLUSH)
			{
				if (!TBD_MapExportJson.Write(f, buf, TAG))
				{
					f.Close();
					return false;
				}
				buf = "";
			}
		}

		buf += "  ]\n";
		buf += "}\n";

		if (!TBD_MapExportJson.Write(f, buf, TAG))
		{
			f.Close();
			return false;
		}

		f.Close();
		int elapsedMs = System.GetTickCount() - tick0;
		Print(string.Format("%1 PLANT EXPORT FINISHED in %2 ms (Total=%3 plants) -> %4",
			TAG, elapsedMs, totalPlants, outJson), LogLevel.NORMAL);

		return true;
	}
}
