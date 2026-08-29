/**
 * TBD_MapExportCrops.c
 *
 * Dedicated agricultural crops and vegetables extraction engine for Bohemia Reforger.
 * Queries placed world entities across spatial cells (512m), strictly classifies
 * cultivated vegetables, garden crops, and agricultural furrow lines (Prefabs/Vegetation/Vegetables/*),
 * and stream-writes a valid JSON document (crops.json) with census breakdown and instance array.
 *
 * Outputs:
 *   - $profile:TBD_Export/<mapName>/vegetation/crops.json
 */

class TBD_CropRecord
{
	int m_iId;
	string m_sResourceName;
	string m_sCropType;
	string m_sVariant;
	string m_sLayout;
	vector m_vPosition;
	vector m_vRotation;
	float m_fScale;
	float m_fWidth;
	float m_fHeight;
	float m_fDepth;

	void TBD_CropRecord(int id, string resName, string cropType, string variant, string layout, vector pos, vector rot, float scale, float w, float h, float d)
	{
		m_iId = id;
		m_sResourceName = resName;
		m_sCropType = cropType;
		m_sVariant = variant;
		m_sLayout = layout;
		m_vPosition = pos;
		m_vRotation = rot;
		m_fScale = scale;
		m_fWidth = w;
		m_fHeight = h;
		m_fDepth = d;
	}
}

class TBD_MapExportCrops
{
	protected static const string TAG = "[TBD][Vegetation][Crops]";
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
	//! Authoritative Crop classifier.
	//! Returns true strictly if the entity is an agricultural crop or cultivated garden vegetable.
	static bool IsCropPrefab(string resName, string className, IEntity ent, out string cropType, out string variant, out string layout)
	{
		cropType = "unknown_crop";
		variant = "default";
		layout = "patch";

		if (resName.IsEmpty())
			return false;

		// 1. Strict Exclusions:
		if (ent)
		{
			if (ent.FindComponent(SCR_MapDescriptorComponent) || ent.FindComponent(MapDescriptorComponent))
				return false;
			if (ent.FindComponent(SCR_EditableCommentComponent))
				return false;
		}

		string lowerRes = resName;
		lowerRes.ToLower();

		// Reject wild plants, bushes, trees, rocks, props, decorations
		if (lowerRes.Contains("/bush/") || lowerRes.Contains("/bushes/"))
			return false;
		if (lowerRes.Contains("/plant/") || lowerRes.Contains("/plants/"))
			return false;
		if (lowerRes.Contains("/tree/") || lowerRes.Contains("/trees/"))
			return false;
		if (lowerRes.Contains("/rocks/") || lowerRes.Contains("/rock/"))
			return false;
		if (lowerRes.Contains("/props/") || lowerRes.Contains("/decorations/") || lowerRes.Contains("/flowerpots/"))
			return false;
		if (lowerRes.Contains("stump") || lowerRes.Contains("cut_trunk") || lowerRes.Contains("trunk_cut"))
			return false;
		if (lowerRes.Contains("fallen") || lowerRes.Contains("deadwood") || lowerRes.Contains("woodlog") || lowerRes.Contains("woodpile"))
			return false;

		// 2. Strict Crop Inclusion:
		bool isVegDir = resName.Contains("Prefabs/Vegetation/Vegetables/") || resName.Contains("Vegetation/Vegetables/") || lowerRes.Contains("/vegetables/");
		if (!isVegDir && !lowerRes.Contains("crop"))
			return false;

		// Extract filename / leaf
		string leaf = resName;
		int slashIdx = leaf.LastIndexOf("/");
		if (slashIdx >= 0)
			leaf = leaf.Substring(slashIdx + 1, leaf.Length() - slashIdx - 1);

		int dotIdx = leaf.LastIndexOf(".");
		if (dotIdx > 0)
			leaf = leaf.Substring(0, dotIdx);

		// Must contain Crop or reside in vegetables folder
		if (!leaf.Contains("Crop") && !isVegDir)
			return false;

		// 3. Determine Layout & Crop Type
		string lowerLeaf = leaf;
		lowerLeaf.ToLower();

		if (lowerLeaf.Contains("shortline"))
			layout = "shortline";
		else if (lowerLeaf.Contains("longline"))
			layout = "longline";
		else if (lowerLeaf.Contains("line"))
			layout = "line";
		else
			layout = "patch";

		variant = leaf;

		if (lowerLeaf.Contains("tomato"))
			cropType = "tomato";
		else if (lowerLeaf.Contains("potato"))
			cropType = "potato";
		else if (lowerLeaf.Contains("cabbage"))
			cropType = "cabbage";
		else if (lowerLeaf.Contains("capsicum") || lowerLeaf.Contains("pepper"))
			cropType = "capsicum";
		else if (lowerLeaf.Contains("pumpkin") || lowerLeaf.Contains("squash"))
			cropType = "pumpkin";
		else
		{
			// Fallback: strip "Crop" and digits
			array<string> parts = {};
			leaf.Split("_", parts, false);
			if (parts.Count() > 0)
			{
				string p0 = parts[0];
				p0.Replace("Crop", "");
				p0.ToLower();
				cropType = p0;
			}
			else
			{
				cropType = "vegetable";
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
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "vegetation", "crops.json");

		Print(string.Format("%1 Starting agricultural crop extraction for map '%2' (%3x%3 cells @ %4 m) -> %5",
			TAG, mapName, cells, cellM, outJson), LogLevel.NORMAL);

		// First pass: Query world entities, classify crops, and collect records
		map<string, int> cropCounts = new map<string, int>();
		map<string, int> layoutCounts = new map<string, int>();
		array<ref TBD_CropRecord> cropRecords = {};
		int cropId = 0;

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

					string rn = ctx.ResolvePrefab(e);
					string clsName = e.ClassName();

					string cType, variant, layout;
					if (!IsCropPrefab(rn, clsName, e, cType, variant, layout))
						continue;

					cropId++;
					vector ang = e.GetAngles();
					float scale = e.GetScale();
					if (scale <= 0.001) scale = 1.0;

					cropRecords.Insert(new TBD_CropRecord(cropId, rn, cType, variant, layout, pos, ang, scale, w, h, d));

					// Track crop count
					int curCropCount = 0;
					if (cropCounts.Find(cType, curCropCount))
						cropCounts.Set(cType, curCropCount + 1);
					else
						cropCounts.Insert(cType, 1);

					// Track layout count
					int curLayoutCount = 0;
					if (layoutCounts.Find(layout, curLayoutCount))
						layoutCounts.Set(layout, curLayoutCount + 1);
					else
						layoutCounts.Insert(layout, 1);
				}
			}
		}

		int totalCrops = cropRecords.Count();
		Print(string.Format("%1 Discovered %2 authentic crops across %3 crop types.", TAG, totalCrops, cropCounts.Count()), LogLevel.NORMAL);

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
		buf += "  \"totalCrops\": " + totalCrops.ToString() + ",\n";

		// Crop Counts Dictionary
		buf += "  \"cropCounts\": {\n";
		int ccTotal = cropCounts.Count();
		for (int cc = 0; cc < ccTotal; cc++)
		{
			string crKey = cropCounts.GetKey(cc);
			int crVal = cropCounts.GetElement(cc);
			buf += "    \"" + TBD_MapExportJson.Escape(crKey) + "\": " + crVal.ToString();
			if (cc < ccTotal - 1)
				buf += ",";
			buf += "\n";
		}
		buf += "  },\n";

		// Layout Counts Dictionary
		buf += "  \"layoutCounts\": {\n";
		int lcTotal = layoutCounts.Count();
		for (int lc = 0; lc < lcTotal; lc++)
		{
			string layKey = layoutCounts.GetKey(lc);
			int layVal = layoutCounts.GetElement(lc);
			buf += "    \"" + TBD_MapExportJson.Escape(layKey) + "\": " + layVal.ToString();
			if (lc < lcTotal - 1)
				buf += ",";
			buf += "\n";
		}
		buf += "  },\n";

		// Crops Array
		buf += "  \"crops\": [\n";
		if (!TBD_MapExportJson.Write(f, buf, TAG))
		{
			f.Close();
			return false;
		}
		buf = "";

		for (int c = 0; c < totalCrops; c++)
		{
			TBD_CropRecord rec = cropRecords[c];
			buf += "    {\n";
			buf += "      \"id\": " + rec.m_iId.ToString() + ",\n";
			buf += "      \"resourceName\": \"" + TBD_MapExportJson.Escape(rec.m_sResourceName) + "\",\n";
			buf += "      \"cropType\": \"" + TBD_MapExportJson.Escape(rec.m_sCropType) + "\",\n";
			buf += "      \"variant\": \"" + TBD_MapExportJson.Escape(rec.m_sVariant) + "\",\n";
			buf += "      \"layout\": \"" + TBD_MapExportJson.Escape(rec.m_sLayout) + "\",\n";
			buf += "      \"position\": [" + rec.m_vPosition[0].ToString() + ", " + rec.m_vPosition[1].ToString() + ", " + rec.m_vPosition[2].ToString() + "],\n";
			buf += "      \"rotation\": [" + rec.m_vRotation[0].ToString() + ", " + rec.m_vRotation[1].ToString() + ", " + rec.m_vRotation[2].ToString() + "],\n";
			buf += "      \"scale\": " + rec.m_fScale.ToString() + ",\n";
			buf += "      \"bounds\": {\"width\": " + rec.m_fWidth.ToString() + ", \"height\": " + rec.m_fHeight.ToString() + ", \"depth\": " + rec.m_fDepth.ToString() + "}\n";
			buf += "    }";

			if (c < totalCrops - 1)
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
		Print(string.Format("%1 CROP EXPORT FINISHED in %2 ms (Total=%3 crops) -> %4",
			TAG, elapsedMs, totalCrops, outJson), LogLevel.NORMAL);

		return true;
	}
}
