/**
 * TBD_MapExportBushes.c
 *
 * Dedicated bush extraction engine for Bohemia Reforger.
 * Queries placed world entities across spatial cells (512m), strictly classifies
 * authentic bushes via case-sensitive prefab resource inspection (Prefabs/Vegetation/Bush/b_*),
 * and stream-writes a valid JSON document (bush.json) with census breakdown and instance array.
 *
 * Outputs:
 *   - $profile:TBD_Export/<mapName>/vegetation/bush.json
 */

class TBD_BushRecord
{
	int m_iId;
	string m_sResourceName;
	string m_sSpecies;
	string m_sVariant;
	vector m_vPosition;
	vector m_vRotation;
	float m_fScale;
	float m_fWidth;
	float m_fHeight;
	float m_fDepth;

	void TBD_BushRecord(int id, string resName, string species, string variant, vector pos, vector rot, float scale, float w, float h, float d)
	{
		m_iId = id;
		m_sResourceName = resName;
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

class TBD_MapExportBushes
{
	protected static const string TAG = "[TBD][Vegetation][Bushes]";
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
	//! Authoritative case-sensitive Bush classifier.
	//! Returns true strictly if the entity is an authentic bush prefab (Prefabs/Vegetation/Bush/b_*).
	static bool IsBushPrefab(string resName, string className, IEntity ent, out string species, out string variant)
	{
		species = "unknown_bush";
		variant = "default";

		if (resName.IsEmpty())
			return false;

		// 1. Strict Exclusions:
		// Reject cartographic descriptors, bays, and landmarks (e.g. B_CharletBay)
		if (ent)
		{
			if (ent.FindComponent(SCR_MapDescriptorComponent) || ent.FindComponent(MapDescriptorComponent))
				return false;
			if (ent.FindComponent(SCR_EditableCommentComponent))
				return false;
		}

		// Reject plants, ponds, trees, vegetables/crops, stumps, fallen deadwood
		string lowerRes = resName;
		lowerRes.ToLower();

		if (lowerRes.Contains("/plant/") || lowerRes.Contains("/plants/") || lowerRes.Contains("/vegetables/"))
			return false;
		if (lowerRes.Contains("/tree/") || lowerRes.Contains("/trees/"))
			return false;
		if (lowerRes.Contains("stump") || lowerRes.Contains("cut_trunk") || lowerRes.Contains("trunk_cut"))
			return false;
		if (lowerRes.Contains("fallen") || lowerRes.Contains("deadwood") || lowerRes.Contains("woodlog") || lowerRes.Contains("woodpile"))
			return false;
		if (lowerRes.Contains("pond") || lowerRes.Contains("lake") || lowerRes.Contains("river") || lowerRes.Contains("ocean"))
			return false;

		// 2. Strict Bush Inclusion:
		// Must reside under Bush folder OR have explicit b_ prefix under vegetation
		bool isBushDir = resName.Contains("Prefabs/Vegetation/Bush/") || resName.Contains("Vegetation/Bush/") || lowerRes.Contains("/bush/");
		if (!isBushDir && !lowerRes.Contains("/vegetation/"))
			return false;

		// Extract filename / leaf
		string leaf = resName;
		int slashIdx = leaf.LastIndexOf("/");
		if (slashIdx >= 0)
			leaf = leaf.Substring(slashIdx + 1, leaf.Length() - slashIdx - 1);

		int dotIdx = leaf.LastIndexOf(".");
		if (dotIdx > 0)
			leaf = leaf.Substring(0, dotIdx);

		// Case-sensitive prefix verification:
		// MUST start with lowercase "b_" (reject uppercase "B_")
		if (!leaf.StartsWith("b_"))
		{
			if (!isBushDir)
				return false;
		}

		// 3. Extract species name and variant:
		// Examples:
		//   "b_corylus_avellana_1l" -> species: "corylus_avellana", variant: "1l"
		//   "b_salix_cinerea_0"     -> species: "salix_cinerea", variant: "0"
		//   "b_rubus_idaeus_1s"     -> species: "rubus_idaeus", variant: "1s"
		string cleanName = leaf;
		if (cleanName.StartsWith("b_"))
			cleanName = cleanName.Substring(2, cleanName.Length() - 2);

		array<string> parts = {};
		cleanName.Split("_", parts, false);
		if (parts.Count() >= 3)
		{
			// Genus + species + variant (e.g. corylus + avellana + 1l)
			species = parts[0] + "_" + parts[1];
			variant = parts[parts.Count() - 1];
		}
		else if (parts.Count() == 2)
		{
			// Genus + species (or name + variant)
			species = parts[0] + "_" + parts[1];
			variant = "0";
		}
		else if (parts.Count() == 1)
		{
			species = parts[0];
			variant = "0";
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
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "vegetation", "bush.json");

		Print(string.Format("%1 Starting bush extraction for map '%2' (%3x%3 cells @ %4 m) -> %5",
			TAG, mapName, cells, cellM, outJson), LogLevel.NORMAL);

		// First pass: Query world entities, classify bushes, and collect records
		map<string, int> speciesCounts = new map<string, int>();
		array<ref TBD_BushRecord> bushRecords = {};
		int bushId = 0;

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

					// Height validation safeguard
					if (h < 0.2 || h > 6.0)
						continue;

					string rn = ctx.ResolvePrefab(e);
					string clsName = e.ClassName();

					string species, variant;
					if (!IsBushPrefab(rn, clsName, e, species, variant))
						continue;

					bushId++;
					vector ang = e.GetAngles();
					float scale = e.GetScale();
					if (scale <= 0.001) scale = 1.0;

					bushRecords.Insert(new TBD_BushRecord(bushId, rn, species, variant, pos, ang, scale, w, h, d));

					// Track species count
					int curCount = 0;
					if (speciesCounts.Find(species, curCount))
						speciesCounts.Set(species, curCount + 1);
					else
						speciesCounts.Insert(species, 1);
				}
			}
		}

		int totalBushes = bushRecords.Count();
		Print(string.Format("%1 Discovered %2 authentic bushes across %3 species.", TAG, totalBushes, speciesCounts.Count()), LogLevel.NORMAL);

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
		buf += "  \"totalBushes\": " + totalBushes.ToString() + ",\n";

		// Species Counts Dictionary
		buf += "  \"speciesCounts\": {\n";
		int scIndex = 0;
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

		// Bushes Array
		buf += "  \"bushes\": [\n";
		if (!TBD_MapExportJson.Write(f, buf, TAG))
		{
			f.Close();
			return false;
		}
		buf = "";

		for (int b = 0; b < totalBushes; b++)
		{
			TBD_BushRecord rec = bushRecords[b];
			buf += "    {\n";
			buf += "      \"id\": " + rec.m_iId.ToString() + ",\n";
			buf += "      \"resourceName\": \"" + TBD_MapExportJson.Escape(rec.m_sResourceName) + "\",\n";
			buf += "      \"species\": \"" + TBD_MapExportJson.Escape(rec.m_sSpecies) + "\",\n";
			buf += "      \"variant\": \"" + TBD_MapExportJson.Escape(rec.m_sVariant) + "\",\n";
			buf += "      \"position\": [" + rec.m_vPosition[0].ToString() + ", " + rec.m_vPosition[1].ToString() + ", " + rec.m_vPosition[2].ToString() + "],\n";
			buf += "      \"rotation\": [" + rec.m_vRotation[0].ToString() + ", " + rec.m_vRotation[1].ToString() + ", " + rec.m_vRotation[2].ToString() + "],\n";
			buf += "      \"scale\": " + rec.m_fScale.ToString() + ",\n";
			buf += "      \"bounds\": {\"width\": " + rec.m_fWidth.ToString() + ", \"height\": " + rec.m_fHeight.ToString() + ", \"depth\": " + rec.m_fDepth.ToString() + "}\n";
			buf += "    }";

			if (b < totalBushes - 1)
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
		Print(string.Format("%1 BUSH EXPORT FINISHED in %2 ms (Total=%3 bushes) -> %4",
			TAG, elapsedMs, totalBushes, outJson), LogLevel.NORMAL);

		return true;
	}
}
