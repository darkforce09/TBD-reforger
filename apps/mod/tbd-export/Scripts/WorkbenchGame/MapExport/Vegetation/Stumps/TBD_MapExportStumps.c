/**
 * TBD_MapExportStumps.c
 *
 * Dedicated tree stumps and forestry trunks extraction engine for Bohemia Reforger.
 * Queries placed world entities across spatial cells (512m), strictly classifies
 * authentic tree stumps, cut trunks, uncut logs, rooted tree bases, and forestry woodpiles,
 * and stream-writes a valid JSON document (stumps.json) with census breakdown and instance array.
 *
 * Outputs:
 *   - $profile:TBD_Export/<mapName>/vegetation/stumps.json
 */

class TBD_StumpRecord
{
	int m_iId;
	string m_sResourceName;
	string m_sStumpType;
	string m_sSpecies;
	string m_sVariant;
	vector m_vPosition;
	vector m_vRotation;
	float m_fScale;
	float m_fWidth;
	float m_fHeight;
	float m_fDepth;
	float m_fDiameter;

	void TBD_StumpRecord(int id, string resName, string stumpType, string species, string variant, vector pos, vector rot, float scale, float w, float h, float d, float diam)
	{
		m_iId = id;
		m_sResourceName = resName;
		m_sStumpType = stumpType;
		m_sSpecies = species;
		m_sVariant = variant;
		m_vPosition = pos;
		m_vRotation = rot;
		m_fScale = scale;
		m_fWidth = w;
		m_fHeight = h;
		m_fDepth = d;
		m_fDiameter = diam;
	}
}

class TBD_MapExportStumps
{
	protected static const string TAG = "[TBD][Vegetation][Stumps]";
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
		if (c < 0)
			c = 0;
		if (c > cells - 1)
			c = cells - 1;
		return c;
	}

	//------------------------------------------------------------------------------------------------
	//! Authoritative Stump and Forestry Trunk classifier.
	//! Returns true strictly if the entity is an authentic tree stump, cut trunk, log, or forestry woodpile.
	static bool IsStumpPrefab(string resName, string className, IEntity ent, out string stumpType, out string species, out string variant)
	{
		stumpType = "weathered_stump";
		species = "unknown";
		variant = "default";

		if (resName.IsEmpty())
			return false;

		// 1. Strict Exclusions:
		// Reject map descriptors, comments, and non-physical metadata entities
		if (ent)
		{
			if (ent.FindComponent(SCR_MapDescriptorComponent) || ent.FindComponent(MapDescriptorComponent))
				return false;
			if (ent.FindComponent(SCR_EditableCommentComponent))
				return false;
		}

		string lowerRes = resName;
		lowerRes.ToLower();

		// Reject living shrubs, undergrowth, wild herbaceous plants, agricultural crops, and rock formations
		if (lowerRes.Contains("/bush/") || lowerRes.Contains("/bushes/"))
			return false;
		if (lowerRes.Contains("/plant/") || lowerRes.Contains("/plants/"))
			return false;
		if (lowerRes.Contains("/vegetables/") || lowerRes.Contains("/crops/"))
			return false;
		if (lowerRes.Contains("/rocks/") || lowerRes.Contains("/rock/"))
			return false;

		// Reject unrelated wooden furniture, crates, garbage, and structural items
		if (lowerRes.Contains("/furniture/") || lowerRes.Contains("chair") || lowerRes.Contains("table"))
			return false;
		if (lowerRes.Contains("/crates/") || lowerRes.Contains("boxwooden"))
			return false;
		if (lowerRes.Contains("/garbage/") || lowerRes.Contains("/construction/"))
			return false;
		if (lowerRes.Contains("/structures/") || lowerRes.Contains("/industrial/"))
			return false;
		if (lowerRes.Contains("pallet"))
			return false;

		// 2. Strict Stump & Forestry Trunk Inclusion:
		bool isForestProp = lowerRes.Contains("prefabs/props/forest/woodpile/") || lowerRes.Contains("props/forest/woodpile/");
		bool isTreeDebris = lowerRes.Contains("prefabs/vegetation/tree/debris/") || lowerRes.Contains("vegetation/tree/debris/");
		bool isVegetationTree = lowerRes.Contains("/vegetation/tree/") || lowerRes.Contains("prefabs/vegetation/");

		bool hasStumpKeyword = (lowerRes.Contains("stump") || lowerRes.Contains("cut_trunk") || lowerRes.Contains("trunk_cut") || lowerRes.Contains("woodlog") || lowerRes.Contains("woodpile") || lowerRes.Contains("tree_base") || lowerRes.Contains("root_base") || lowerRes.Contains("stem_root"));

		if (!isForestProp && !isTreeDebris && !hasStumpKeyword)
			return false;

		// If it is in the general tree folder, it must explicitly match a stump/trunk/debris pattern
		if (isVegetationTree && !isTreeDebris && !hasStumpKeyword)
			return false;

		// Extract filename / leaf
		string leaf = resName;
		int slashIdx = leaf.LastIndexOf("/");
		if (slashIdx >= 0)
			leaf = leaf.Substring(slashIdx + 1, leaf.Length() - slashIdx - 1);

		int dotIdx = leaf.LastIndexOf(".");
		if (dotIdx > 0)
			leaf = leaf.Substring(0, dotIdx);

		string lowerLeaf = leaf;
		lowerLeaf.ToLower();
		variant = leaf;

		// 3. Classify Stump Type:
		if (lowerLeaf.Contains("woodpile") || lowerRes.Contains("woodpile"))
		{
			stumpType = "wood_pile";
		}
		else if (lowerLeaf.Contains("woodlog") || lowerLeaf.Contains("log_") || lowerRes.Contains("woodlog"))
		{
			stumpType = "trunk_segment";
		}
		else if (lowerLeaf.Contains("cut_trunk") || lowerLeaf.Contains("trunk_cut") || lowerLeaf.Contains("stump_cut") || lowerLeaf.Contains("cut_stump") || lowerLeaf.Contains("sawed"))
		{
			stumpType = "cut_stump";
		}
		else if (lowerLeaf.Contains("root") || lowerLeaf.Contains("tree_base") || lowerLeaf.Contains("stem_root"))
		{
			stumpType = "rooted_base";
		}
		else if (lowerLeaf.Contains("fallen") || lowerLeaf.Contains("deadwood") || lowerLeaf.Contains("branch") || lowerLeaf.Contains("stem"))
		{
			stumpType = "fallen_deadwood";
		}
		else if (lowerLeaf.Contains("weathered") || lowerLeaf.Contains("broken") || lowerLeaf.Contains("rotted") || lowerLeaf.Contains("stump"))
		{
			stumpType = "weathered_stump";
		}
		else
		{
			stumpType = "weathered_stump";
		}

		// 4. Extract Species / Wood Type:
		if (lowerRes.Contains("picea_abies") || lowerLeaf.Contains("picea_abies"))
		{
			species = "picea_abies";
		}
		else if (lowerRes.Contains("pinus_sylvestris") || lowerLeaf.Contains("pinus_sylvestris"))
		{
			species = "pinus_sylvestris";
		}
		else if (lowerRes.Contains("betula_pendula") || lowerLeaf.Contains("betula_pendula"))
		{
			species = "betula_pendula";
		}
		else if (lowerRes.Contains("quercus_robur") || lowerLeaf.Contains("quercus_robur") || lowerRes.Contains("quercus"))
		{
			species = "quercus_robur";
		}
		else if (lowerRes.Contains("fagus_sylvatica") || lowerLeaf.Contains("fagus_sylvatica"))
		{
			species = "fagus_sylvatica";
		}
		else if (lowerRes.Contains("alnus_glutinosa") || lowerLeaf.Contains("alnus_glutinosa"))
		{
			species = "alnus_glutinosa";
		}
		else if (lowerRes.Contains("carpinus_betulus") || lowerLeaf.Contains("carpinus_betulus"))
		{
			species = "carpinus_betulus";
		}
		else if (lowerRes.Contains("fraxinus_excelsior") || lowerLeaf.Contains("fraxinus_excelsior"))
		{
			species = "fraxinus_excelsior";
		}
		else if (lowerRes.Contains("populus_tremula") || lowerLeaf.Contains("populus_tremula"))
		{
			species = "populus_tremula";
		}
		else if (lowerRes.Contains("salix_alba") || lowerRes.Contains("salix_cinerea") || lowerLeaf.Contains("salix"))
		{
			species = "salix";
		}
		else if (lowerRes.Contains("sorbus_aucuparia") || lowerLeaf.Contains("sorbus_aucuparia"))
		{
			species = "sorbus_aucuparia";
		}
		else if (lowerRes.Contains("tilia_cordata") || lowerLeaf.Contains("tilia_cordata"))
		{
			species = "tilia_cordata";
		}
		else if (lowerRes.Contains("malus_sylvestris") || lowerLeaf.Contains("malus_sylvestris"))
		{
			species = "malus_sylvestris";
		}
		else if (lowerRes.Contains("prunus_avium") || lowerLeaf.Contains("prunus_avium"))
		{
			species = "prunus_avium";
		}
		else if (isForestProp || lowerLeaf.Contains("woodlog") || lowerLeaf.Contains("woodpile"))
		{
			species = "forestry_timber";
		}
		else
		{
			// Check if genus_species format exists in clean name (e.g. t_genus_species_*)
			string clean = leaf;
			if (clean.StartsWith("t_"))
				clean = clean.Substring(2, clean.Length() - 2);

			array<string> parts = {};
			clean.Split("_", parts, false);
			if (parts.Count() >= 2)
			{
				string p0 = parts[0];
				string p1 = parts[1];
				p0.ToLower();
				p1.ToLower();
				if (p0 != "stump" && p1 != "stump" && p0 != "debris" && p1 != "debris")
				{
					species = p0 + "_" + p1;
				}
				else
				{
					species = "generic";
				}
			}
			else
			{
				species = "generic";
			}
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
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "vegetation", "stumps.json");

		Print(string.Format("%1 Starting stump extraction for map '%2' (%3x%3 cells @ %4 m) -> %5",
			TAG, mapName, cells, cellM, outJson), LogLevel.NORMAL);

		// First pass: Query world entities, classify stumps and trunks, and collect records
		map<string, int> stumpTypeCounts = new map<string, int>();
		map<string, int> speciesCounts = new map<string, int>();
		array<ref TBD_StumpRecord> stumpRecords = {};
		int stumpId = 0;

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

					// Height validation safeguard: stumps and logs are low ground-level features (0.05m to 5.0m)
					if (h < 0.05 || h > 5.0)
						continue;

					string rn = ctx.ResolvePrefab(e);
					string clsName = e.ClassName();

					string stumpType, species, variant;
					if (!IsStumpPrefab(rn, clsName, e, stumpType, species, variant))
						continue;

					stumpId++;
					vector ang = e.GetAngles();
					float scale = e.GetScale();
					if (scale <= 0.001)
						scale = 1.0;

					float diam = w;
					if (d > diam)
						diam = d;

					stumpRecords.Insert(new TBD_StumpRecord(stumpId, rn, stumpType, species, variant, pos, ang, scale, w, h, d, diam));

					// Track stump type count
					int curTypeCount = 0;
					if (stumpTypeCounts.Find(stumpType, curTypeCount))
						stumpTypeCounts.Set(stumpType, curTypeCount + 1);
					else
						stumpTypeCounts.Insert(stumpType, 1);

					// Track species count
					int curSpeciesCount = 0;
					if (speciesCounts.Find(species, curSpeciesCount))
						speciesCounts.Set(species, curSpeciesCount + 1);
					else
						speciesCounts.Insert(species, 1);
				}
			}
		}

		int totalStumps = stumpRecords.Count();
		Print(string.Format("%1 Discovered %2 authentic stumps and forestry trunks across %3 types and %4 species.",
			TAG, totalStumps, stumpTypeCounts.Count(), speciesCounts.Count()), LogLevel.NORMAL);

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
		buf += "  \"totalStumps\": " + totalStumps.ToString() + ",\n";

		// Stump Type Counts Dictionary
		buf += "  \"stumpTypeCounts\": {\n";
		int tcTotal = stumpTypeCounts.Count();
		for (int tc = 0; tc < tcTotal; tc++)
		{
			string stKey = stumpTypeCounts.GetKey(tc);
			int stVal = stumpTypeCounts.GetElement(tc);
			buf += "    \"" + TBD_MapExportJson.Escape(stKey) + "\": " + stVal.ToString();
			if (tc < tcTotal - 1)
				buf += ",";
			buf += "\n";
		}
		buf += "  },\n";

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

		// Stumps Array
		buf += "  \"stumps\": [\n";
		if (!TBD_MapExportJson.Write(f, buf, TAG))
		{
			f.Close();
			return false;
		}
		buf = "";

		for (int s = 0; s < totalStumps; s++)
		{
			TBD_StumpRecord rec = stumpRecords[s];
			buf += "    {\n";
			buf += "      \"id\": " + rec.m_iId.ToString() + ",\n";
			buf += "      \"resourceName\": \"" + TBD_MapExportJson.Escape(rec.m_sResourceName) + "\",\n";
			buf += "      \"stumpType\": \"" + TBD_MapExportJson.Escape(rec.m_sStumpType) + "\",\n";
			buf += "      \"species\": \"" + TBD_MapExportJson.Escape(rec.m_sSpecies) + "\",\n";
			buf += "      \"variant\": \"" + TBD_MapExportJson.Escape(rec.m_sVariant) + "\",\n";
			buf += "      \"position\": [" + rec.m_vPosition[0].ToString() + ", " + rec.m_vPosition[1].ToString() + ", " + rec.m_vPosition[2].ToString() + "],\n";
			buf += "      \"rotation\": [" + rec.m_vRotation[0].ToString() + ", " + rec.m_vRotation[1].ToString() + ", " + rec.m_vRotation[2].ToString() + "],\n";
			buf += "      \"scale\": " + rec.m_fScale.ToString() + ",\n";
			buf += "      \"bounds\": {\"width\": " + rec.m_fWidth.ToString() + ", \"height\": " + rec.m_fHeight.ToString() + ", \"depth\": " + rec.m_fDepth.ToString() + ", \"diameter\": " + rec.m_fDiameter.ToString() + "}\n";
			buf += "    }";

			if (s < totalStumps - 1)
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
		Print(string.Format("%1 STUMP EXPORT FINISHED in %2 ms (Total=%3 stumps) -> %4",
			TAG, elapsedMs, totalStumps, outJson), LogLevel.NORMAL);

		return true;
	}
}
