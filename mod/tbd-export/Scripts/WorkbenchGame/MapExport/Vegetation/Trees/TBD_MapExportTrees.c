/**
 * TBD_MapExportTrees.c
 *
 * Dedicated natural tree and forest canopy extraction engine for Bohemia Reforger.
 * Queries placed world entities across spatial cells (512m), strictly classifies
 * authentic conifer and deciduous trees via case-sensitive prefab resource inspection (Prefabs/Vegetation/Tree/*),
 * and stream-writes a valid JSON document (trees.json) with census breakdown and instance array.
 *
 * Outputs:
 *   - $profile:TBD_Export/<mapName>/vegetation/trees.json
 */

class TBD_TreeRecord
{
	int m_iId;
	string m_sResourceName;
	string m_sTreeClass;
	string m_sSpecies;
	string m_sVariant;
	vector m_vPosition;
	vector m_vRotation;
	float m_fScale;
	float m_fWidth;
	float m_fHeight;
	float m_fDepth;

	void TBD_TreeRecord(int id, string resName, string treeClass, string species, string variant, vector pos, vector rot, float scale, float w, float h, float d)
	{
		m_iId = id;
		m_sResourceName = resName;
		m_sTreeClass = treeClass;
		m_sSpecies = species;
		m_sVariant = variant;
		m_vPosition = pos;
		m_vRotation = rot;
		m_fScale = scale;
		m_fWidth = w;
		m_fHeight = h;
		m_fDepth = d;
	}
}

class TBD_MapExportTrees
{
	protected static const string TAG = "[TBD][Vegetation][Trees]";
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
	//! Authoritative Tree classifier.
	//! Returns true strictly if the entity is an authentic living or standing dead tree.
	static bool IsTreePrefab(string resName, string className, IEntity ent, out string treeClass, out string species, out string variant, out bool isDead)
	{
		treeClass = "deciduous";
		species = "unknown_tree";
		variant = "default";
		isDead = false;

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

		// Reject bushes, wild herbaceous plants, marine undergrowth, agricultural crops/vegetables
		if (lowerRes.Contains("/bush/") || lowerRes.Contains("/bushes/"))
			return false;
		if (lowerRes.Contains("/plant/") || lowerRes.Contains("/plants/"))
			return false;
		if (lowerRes.Contains("/vegetables/") || lowerRes.Contains("/crops/"))
			return false;

		// Reject tree stumps, cut trunks, uncut logs, rooted bases, fallen deadwood, and forestry woodpiles (handled exclusively by stumps.json)
		if (lowerRes.Contains("/debris/") || lowerRes.Contains("stump") || lowerRes.Contains("cut_trunk") || lowerRes.Contains("trunk_cut"))
			return false;
		if (lowerRes.Contains("woodlog") || lowerRes.Contains("woodpile") || lowerRes.Contains("tree_base") || lowerRes.Contains("root_base") || lowerRes.Contains("stem_root"))
			return false;
		if (lowerRes.Contains("_fallen") || lowerRes.Contains("fallen_") || lowerRes.Contains("_debris") || lowerRes.Contains("_branch_") || lowerRes.Contains("_stem_"))
			return false;

		// Reject rocks, buildings, fences, furniture, props, vehicles, water bodies
		if (lowerRes.Contains("/rocks/") || lowerRes.Contains("/rock/"))
			return false;
		if (lowerRes.Contains("/structures/") || lowerRes.Contains("/buildings/") || lowerRes.Contains("/props/") || lowerRes.Contains("/furniture/"))
			return false;
		if (lowerRes.Contains("pond") || lowerRes.Contains("lake") || lowerRes.Contains("river") || lowerRes.Contains("ocean"))
			return false;

		// 2. Tree Inclusion:
		bool isTreeDir = lowerRes.Contains("/tree/") || lowerRes.Contains("/trees/") || lowerRes.Contains("prefabs/vegetation/tree");
		bool isVegDir = lowerRes.Contains("/vegetation/");

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

		// Reject bush (b_*), plant (p_*), weed, crop prefixes
		if (lowerLeaf.StartsWith("b_") || lowerLeaf.StartsWith("p_") || lowerLeaf.Contains("curbsideweeds") || lowerLeaf.Contains("crop"))
			return false;

		// Check if matches tree naming pattern or directory
		bool isTreeName = leaf.StartsWith("t_") || lowerLeaf.StartsWith("tree_") || lowerLeaf.Contains("picea") || lowerLeaf.Contains("pinus") || lowerLeaf.Contains("betula") || lowerLeaf.Contains("quercus") || lowerLeaf.Contains("fagus");
		if (!isTreeDir && !isTreeName)
		{
			if (!isVegDir)
				return false;
		}

		variant = leaf;

		// 3. Detect standing dead / snag variants
		if (lowerLeaf.Contains("_d") || lowerLeaf.Contains("_dead") || lowerLeaf.Contains("dead_") || lowerLeaf.Contains("snag") || lowerLeaf.Contains("dry"))
		{
			isDead = true;
		}

		// 4. Extract Species Name
		if (lowerRes.Contains("picea_abies") || lowerLeaf.Contains("picea_abies"))
		{
			species = "picea_abies";
			treeClass = "conifer";
		}
		else if (lowerRes.Contains("pinus_sylvestris") || lowerLeaf.Contains("pinus_sylvestris") || lowerLeaf.Contains("pinus"))
		{
			species = "pinus_sylvestris";
			treeClass = "conifer";
		}
		else if (lowerRes.Contains("larix_decidua") || lowerLeaf.Contains("larix_decidua") || lowerLeaf.Contains("larix"))
		{
			species = "larix_decidua";
			treeClass = "conifer";
		}
		else if (lowerRes.Contains("betula_pendula") || lowerLeaf.Contains("betula_pendula") || lowerLeaf.Contains("betula"))
		{
			species = "betula_pendula";
			treeClass = "deciduous";
		}
		else if (lowerRes.Contains("carpinus_betulus") || lowerLeaf.Contains("carpinus_betulus") || lowerLeaf.Contains("carpinus"))
		{
			species = "carpinus_betulus";
			treeClass = "deciduous";
		}
		else if (lowerRes.Contains("fagus_sylvatica") || lowerLeaf.Contains("fagus_sylvatica") || lowerLeaf.Contains("fagus"))
		{
			species = "fagus_sylvatica";
			treeClass = "deciduous";
		}
		else if (lowerRes.Contains("quercus_robur") || lowerLeaf.Contains("quercus_robur") || lowerRes.Contains("quercus") || lowerLeaf.Contains("quercus"))
		{
			species = "quercus_robur";
			treeClass = "deciduous";
		}
		else if (lowerRes.Contains("alnus_glutinosa") || lowerLeaf.Contains("alnus_glutinosa") || lowerLeaf.Contains("alnus"))
		{
			species = "alnus_glutinosa";
			treeClass = "deciduous";
		}
		else if (lowerRes.Contains("fraxinus_excelsior") || lowerLeaf.Contains("fraxinus_excelsior") || lowerLeaf.Contains("fraxinus"))
		{
			species = "fraxinus_excelsior";
			treeClass = "deciduous";
		}
		else if (lowerRes.Contains("populus_tremula") || lowerLeaf.Contains("populus_tremula") || lowerLeaf.Contains("populus"))
		{
			species = "populus_tremula";
			treeClass = "deciduous";
		}
		else if (lowerRes.Contains("salix_alba") || lowerRes.Contains("salix_cinerea") || lowerLeaf.Contains("salix"))
		{
			species = "salix_cinerea";
			treeClass = "deciduous";
		}
		else if (lowerRes.Contains("sorbus_aucuparia") || lowerLeaf.Contains("sorbus_aucuparia") || lowerLeaf.Contains("sorbus"))
		{
			species = "sorbus_aucuparia";
			treeClass = "deciduous";
		}
		else if (lowerRes.Contains("tilia_cordata") || lowerLeaf.Contains("tilia_cordata") || lowerLeaf.Contains("tilia"))
		{
			species = "tilia_cordata";
			treeClass = "deciduous";
		}
		else if (lowerRes.Contains("malus_sylvestris") || lowerLeaf.Contains("malus_sylvestris") || lowerLeaf.Contains("malus"))
		{
			species = "malus_sylvestris";
			treeClass = "deciduous";
		}
		else if (lowerRes.Contains("prunus_avium") || lowerLeaf.Contains("prunus_avium") || lowerLeaf.Contains("prunus"))
		{
			species = "prunus_avium";
			treeClass = "deciduous";
		}
		else if (lowerRes.Contains("palm") || lowerLeaf.Contains("palm"))
		{
			species = "palm";
			treeClass = "palm";
		}
		else
		{
			// Fallback: parse genus_species from clean name (e.g. t_genus_species_*)
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
				species = p0 + "_" + p1;
			}
			else if (parts.Count() == 1)
			{
				species = parts[0];
				species.ToLower();
			}

			// Determine class by genus keywords
			if (species.Contains("conifer") || species.Contains("spruce") || species.Contains("pine") || species.Contains("fir") || species.Contains("cedar"))
				treeClass = "conifer";
			else if (species.Contains("palm"))
				treeClass = "palm";
			else if (isDead)
				treeClass = "deadwood";
			else
				treeClass = "deciduous";
		}

		// Clean variant extraction
		string cleanName = leaf;
		if (cleanName.StartsWith("t_"))
			cleanName = cleanName.Substring(2, cleanName.Length() - 2);

		array<string> vparts = {};
		cleanName.Split("_", vparts, false);
		if (vparts.Count() >= 3)
		{
			variant = vparts[vparts.Count() - 1];
		}
		else if (vparts.Count() >= 1)
		{
			variant = vparts[vparts.Count() - 1];
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Primary export execution method (Memory-safe two-pass streaming for high instance counts).
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
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "vegetation", "trees.json");

		Print(string.Format("%1 Starting tree extraction for map '%2' (%3x%3 cells @ %4 m) -> %5",
			TAG, mapName, cells, cellM, outJson), LogLevel.NORMAL);

		// Pass 1: Census pass across spatial cells (counts and aggregations without per-record heap storage)
		map<string, int> classCounts = new map<string, int>();
		map<string, int> speciesCounts = new map<string, int>();
		int totalTrees = 0;

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
					float h = bmax[1] - bmin[1];

					// Height validation safeguard for standing trees (1.2m to 60.0m)
					if (h < 1.2 || h > 60.0)
						continue;

					string rn = ctx.ResolvePrefab(e);
					string clsName = e.ClassName();

					string tClass, species, variant;
					bool isDead;
					if (!IsTreePrefab(rn, clsName, e, tClass, species, variant, isDead))
						continue;

					totalTrees++;

					// Track class count
					int curClassCount = 0;
					if (classCounts.Find(tClass, curClassCount))
						classCounts.Set(tClass, curClassCount + 1);
					else
						classCounts.Insert(tClass, 1);

					// Track species count
					int curSpeciesCount = 0;
					if (speciesCounts.Find(species, curSpeciesCount))
						speciesCounts.Set(species, curSpeciesCount + 1);
					else
						speciesCounts.Insert(species, 1);
				}
			}
		}

		Print(string.Format("%1 Discovered %2 authentic trees across %3 classes and %4 species.",
			TAG, totalTrees, classCounts.Count(), speciesCounts.Count()), LogLevel.NORMAL);

		// Pass 2: Stream write valid JSON document with buffer flushes
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
		buf += "  \"totalTrees\": " + totalTrees.ToString() + ",\n";

		// Class Counts Dictionary
		buf += "  \"classCounts\": {\n";
		int clTotal = classCounts.Count();
		for (int cl = 0; cl < clTotal; cl++)
		{
			string clKey = classCounts.GetKey(cl);
			int clVal = classCounts.GetElement(cl);
			buf += "    \"" + TBD_MapExportJson.Escape(clKey) + "\": " + clVal.ToString();
			if (cl < clTotal - 1)
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

		// Trees Array
		buf += "  \"trees\": [\n";
		if (!TBD_MapExportJson.Write(f, buf, TAG))
		{
			f.Close();
			return false;
		}
		buf = "";

		int writtenTrees = 0;

		for (int iz2 = 0; iz2 < cells; iz2++)
		{
			for (int ix2 = 0; ix2 < cells; ix2++)
			{
				float x02 = ix2 * cellM;
				float z02 = iz2 * cellM;
				m_aHits = {};
				vector mins2 = Vector(x02, Y_MIN, z02);
				vector maxs2 = Vector(x02 + cellM, Y_MAX, z02 + cellM);
				ctx.m_World.QueryEntitiesByAABB(mins2, maxs2, CollectEntity);

				foreach (IEntity e2 : m_aHits)
				{
					if (!e2)
						continue;

					vector pos = e2.GetOrigin();
					if (pos[0] < 0 || pos[0] > worldSize || pos[2] < 0 || pos[2] > worldSize)
						continue;
					if (CellIndex(pos[0], cellM, cells) != ix2 || CellIndex(pos[2], cellM, cells) != iz2)
						continue;

					vector bmin, bmax;
					e2.GetWorldBounds(bmin, bmax);
					float w = bmax[0] - bmin[0];
					float h = bmax[1] - bmin[1];
					float d = bmax[2] - bmin[2];

					// Height validation safeguard for standing trees (1.2m to 60.0m)
					if (h < 1.2 || h > 60.0)
						continue;

					string rn = ctx.ResolvePrefab(e2);
					string clsName = e2.ClassName();

					string tClass, species, variant;
					bool isDead;
					if (!IsTreePrefab(rn, clsName, e2, tClass, species, variant, isDead))
						continue;

					writtenTrees++;
					vector ang = e2.GetAngles();
					float scale = e2.GetScale();
					if (scale <= 0.001)
						scale = 1.0;

					buf += "    {\n";
					buf += "      \"id\": " + writtenTrees.ToString() + ",\n";
					buf += "      \"resourceName\": \"" + TBD_MapExportJson.Escape(rn) + "\",\n";
					buf += "      \"treeClass\": \"" + TBD_MapExportJson.Escape(tClass) + "\",\n";
					buf += "      \"species\": \"" + TBD_MapExportJson.Escape(species) + "\",\n";
					buf += "      \"variant\": \"" + TBD_MapExportJson.Escape(variant) + "\",\n";
					buf += "      \"position\": [" + pos[0].ToString() + ", " + pos[1].ToString() + ", " + pos[2].ToString() + "],\n";
					buf += "      \"rotation\": [" + ang[0].ToString() + ", " + ang[1].ToString() + ", " + ang[2].ToString() + "],\n";
					buf += "      \"scale\": " + scale.ToString() + ",\n";
					buf += "      \"bounds\": {\"width\": " + w.ToString() + ", \"height\": " + h.ToString() + ", \"depth\": " + d.ToString() + "}\n";
					buf += "    }";

					if (writtenTrees < totalTrees)
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
		Print(string.Format("%1 TREE EXPORT FINISHED in %2 ms (Total=%3 trees) -> %4",
			TAG, elapsedMs, totalTrees, outJson), LogLevel.NORMAL);

		return true;
	}
}
