/**
 * TBD_MapExportRocks.c
 *
 * Dedicated geological rock formations, boulders, and cliff extraction engine for Bohemia Reforger.
 * Queries placed world entities across spatial cells (512m), strictly classifies authentic rocks,
 * performs 5-point terrain surface elevation sampling (GetTerrainSurfaceY) to calculate vertical
 * penetration, burial depth, exposed peak height, and exposure ratios, and stream-writes a valid
 * JSON document (rocks.json) with census breakdown and instance array.
 *
 * Outputs:
 *   - $profile:TBD_Export/<mapName>/vegetation/rocks.json
 *   - $profile:TBD_Export/<mapName>/vegetation/rocks_meta.json
 */

class TBD_RockRecord
{
	int m_iId;
	string m_sResourceName;
	string m_sRockClass;
	string m_sMaterial;
	string m_sVariant;
	vector m_vPosition;
	vector m_vRotation;
	float m_fScale;
	float m_fWidth;
	float m_fHeight;
	float m_fDepth;
	float m_fWorldMinY;
	float m_fWorldMaxY;
	float m_fSurfaceYAtOrigin;
	float m_fSurfaceYMin;
	float m_fSurfaceYMax;
	float m_fExposedPeakHeightM;
	float m_fBurialDepthM;
	float m_fExposureRatio;
	string m_sVisibility;
	vector m_vApex;
}

class TBD_MapExportRocks
{
	protected static const string TAG = "[TBD][Vegetation][Rocks]";
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
	//! Authoritative Rock classifier.
	//! Returns true strictly if the entity is an authentic natural rock, boulder, cliff, outcrop, or scree formation.
	static bool IsRockPrefab(string resName, string className, IEntity ent, out string rockClass, out string material, out string variant)
	{
		rockClass = "boulder";
		material = "generic";
		variant = "default";

		if (resName.IsEmpty())
			return false;

		// 1. Strict Exclusions:
		// Reject map descriptors, bays, settlement markers, and comment annotations
		if (ent)
		{
			if (ent.FindComponent(SCR_MapDescriptorComponent) || ent.FindComponent(MapDescriptorComponent))
				return false;
			if (ent.FindComponent(SCR_EditableCommentComponent))
				return false;
		}

		string lowerRes = resName;
		lowerRes.ToLower();
		string lowerCls = className;
		lowerCls.ToLower();

		// Reject living trees, bushes, wild plants, agricultural crops, and tree stumps/cut logs
		if (lowerRes.Contains("/bush/") || lowerRes.Contains("/bushes/"))
			return false;
		if (lowerRes.Contains("/tree/") || lowerRes.Contains("/trees/"))
			return false;
		if (lowerRes.Contains("/plant/") || lowerRes.Contains("/plants/"))
			return false;
		if (lowerRes.Contains("/vegetables/") || lowerRes.Contains("/crops/"))
			return false;
		if (lowerRes.Contains("/debris/") || lowerRes.Contains("stump") || lowerRes.Contains("cut_trunk") || lowerRes.Contains("trunk_cut"))
			return false;
		if (lowerRes.Contains("woodlog") || lowerRes.Contains("woodpile") || lowerRes.Contains("tree_base") || lowerRes.Contains("root_base"))
			return false;

		// Reject structures, buildings, urban props, furniture, fences, roads, vehicles, powerlines
		if (lowerRes.Contains("/structures/") || lowerRes.Contains("/buildings/") || lowerRes.Contains("/houses/"))
			return false;
		if (lowerRes.Contains("/props/") || lowerRes.Contains("/furniture/") || lowerRes.Contains("/fences/") || lowerRes.Contains("/walls/"))
			return false;
		if (lowerRes.Contains("/vehicles/") || lowerRes.Contains("/roads/") || lowerRes.Contains("/powerline/"))
			return false;
		if (lowerRes.Contains("pond") || lowerRes.Contains("lake") || lowerRes.Contains("river") || lowerRes.Contains("ocean"))
			return false;

		// 2. Rock Inclusion & Detection:
		bool isRockDir = lowerRes.Contains("/rocks/") || lowerRes.Contains("/rock/") || lowerRes.Contains("prefabs/vegetation/rocks") || lowerRes.Contains("prefabs/rocks");
		bool isRockClass = lowerCls.Contains("rock") || lowerCls.Contains("cliff") || lowerCls.Contains("boulder");

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

		bool isRockName = lowerLeaf.StartsWith("rock_") || lowerLeaf.StartsWith("cliff_") || lowerLeaf.StartsWith("boulder_") || lowerLeaf.StartsWith("scree_") || lowerLeaf.StartsWith("pebble_") || lowerLeaf.StartsWith("stone_") || lowerLeaf.Contains("granite") || lowerLeaf.Contains("limestone") || lowerLeaf.Contains("sandstone") || lowerLeaf.Contains("rockface");

		if (!isRockDir && !isRockClass && !isRockName)
			return false;

		variant = leaf;

		// 3. Classify Rock Sub-type:
		if (lowerRes.Contains("cliff") || lowerLeaf.Contains("cliff") || lowerRes.Contains("rockface") || lowerLeaf.Contains("rockface") || lowerLeaf.Contains("rock_wall") || lowerLeaf.Contains("escarpment"))
		{
			rockClass = "cliff";
		}
		else if (lowerRes.Contains("scree") || lowerLeaf.Contains("scree") || lowerRes.Contains("talus") || lowerLeaf.Contains("talus") || lowerRes.Contains("rubble"))
		{
			rockClass = "scree";
		}
		else if (lowerRes.Contains("pebble") || lowerLeaf.Contains("pebble") || lowerLeaf.Contains("stone_small") || lowerLeaf.Contains("gravel"))
		{
			rockClass = "pebble";
		}
		else if (lowerRes.Contains("outcrop") || lowerLeaf.Contains("outcrop") || lowerLeaf.Contains("shelf") || lowerLeaf.Contains("spine") || lowerLeaf.Contains("formation"))
		{
			rockClass = "outcrop";
		}
		else if (lowerRes.Contains("coastal") || lowerLeaf.Contains("coastal") || lowerLeaf.Contains("searock") || lowerLeaf.Contains("sea_rock") || lowerLeaf.Contains("stack"))
		{
			rockClass = "sea_rock";
		}
		else
		{
			rockClass = "boulder";
		}

		// 4. Classify Geological Material:
		if (lowerRes.Contains("granite") || lowerLeaf.Contains("granite"))
			material = "granite";
		else if (lowerRes.Contains("limestone") || lowerLeaf.Contains("limestone"))
			material = "limestone";
		else if (lowerRes.Contains("sandstone") || lowerLeaf.Contains("sandstone"))
			material = "sandstone";
		else if (lowerRes.Contains("slate") || lowerLeaf.Contains("slate"))
			material = "slate";
		else if (lowerRes.Contains("basalt") || lowerLeaf.Contains("basalt"))
			material = "basalt";
		else
			material = "generic";

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Primary export execution method (Memory-safe two-pass streaming with 5-point terrain sampling).
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
		string outJson = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "vegetation", "rocks.json");
		string outMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "vegetation", "rocks_meta.json");

		bool cullBuried = false;
		if (cfg)
			cullBuried = cfg.m_bCullBuriedRocks;

		Print(string.Format("%1 Starting rock extraction for map '%2' (%3x%3 cells @ %4 m, cullBuried=%5) -> %6",
			TAG, mapName, cells, cellM, cullBuried, outJson), LogLevel.NORMAL);

		// Pass 1: Census pass across spatial cells (counts and aggregations without per-record heap storage)
		map<string, int> classCounts = new map<string, int>();
		map<string, int> materialCounts = new map<string, int>();
		map<string, int> visibilityCounts = new map<string, int>();
		int totalRocks = 0;
		int totalExposed = 0;
		int totalBuried = 0;

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

					// Discard degenerate or anomalously huge objects
					if (h < 0.05 || w < 0.05 || d < 0.05 || h > 300.0 || w > 300.0 || d > 300.0)
						continue;

					string rn = ctx.ResolvePrefab(e);
					string clsName = e.ClassName();

					string rClass, rMaterial, rVariant;
					if (!IsRockPrefab(rn, clsName, e, rClass, rMaterial, rVariant))
						continue;

					// 5-Point Terrain Surface Elevation Sampling
					float origX = pos[0];
					float origZ = pos[2];
					float tOrigin = ctx.m_API.GetTerrainSurfaceY(origX, origZ);
					float tC1 = ctx.m_API.GetTerrainSurfaceY(bmin[0], bmin[2]);
					float tC2 = ctx.m_API.GetTerrainSurfaceY(bmax[0], bmin[2]);
					float tC3 = ctx.m_API.GetTerrainSurfaceY(bmin[0], bmax[2]);
					float tC4 = ctx.m_API.GetTerrainSurfaceY(bmax[0], bmax[2]);

					float tMin = Math.Min(tOrigin, Math.Min(Math.Min(tC1, tC2), Math.Min(tC3, tC4)));
					float tMax = Math.Max(tOrigin, Math.Max(Math.Max(tC1, tC2), Math.Max(tC3, tC4)));
					float tAvg = (tOrigin + tC1 + tC2 + tC3 + tC4) * 0.2;

					float worldMinY = bmin[1];
					float worldMaxY = bmax[1];

					float expPeakH = worldMaxY - tMin;
					if (expPeakH < 0.0) expPeakH = 0.0;

					float totalH = h;
					if (totalH <= 0.001) totalH = 1.0;

					float expRatio = (worldMaxY - tAvg) / totalH;
					if (expRatio < 0.0) expRatio = 0.0;
					if (expRatio > 1.0) expRatio = 1.0;

					string vis;
					if (worldMaxY <= tMin + 0.1 || expPeakH <= 0.1)
					{
						vis = "fully_buried";
						totalBuried++;
					}
					else if (expRatio < 0.20)
					{
						vis = "mostly_buried";
						totalExposed++;
					}
					else if (expRatio < 0.70)
					{
						vis = "partially_exposed";
						totalExposed++;
					}
					else
					{
						vis = "fully_exposed";
						totalExposed++;
					}

					if (cullBuried && vis == "fully_buried")
						continue;

					totalRocks++;

					// Track class counts
					int curClsCount = 0;
					if (classCounts.Find(rClass, curClsCount))
						classCounts.Set(rClass, curClsCount + 1);
					else
						classCounts.Insert(rClass, 1);

					// Track material counts
					int curMatCount = 0;
					if (materialCounts.Find(rMaterial, curMatCount))
						materialCounts.Set(rMaterial, curMatCount + 1);
					else
						materialCounts.Insert(rMaterial, 1);

					// Track visibility counts
					int curVisCount = 0;
					if (visibilityCounts.Find(vis, curVisCount))
						visibilityCounts.Set(vis, curVisCount + 1);
					else
						visibilityCounts.Insert(vis, 1);
				}
			}
		}

		Print(string.Format("%1 Census complete: %2 total rocks (Exposed=%3, Buried=%4) across %5 classes and %6 materials.",
			TAG, totalRocks, totalExposed, totalBuried, classCounts.Count(), materialCounts.Count()), LogLevel.NORMAL);

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
		buf += "  \"totalRocks\": " + totalRocks.ToString() + ",\n";
		buf += "  \"totalExposed\": " + totalExposed.ToString() + ",\n";
		buf += "  \"totalBuried\": " + totalBuried.ToString() + ",\n";

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

		// Material Counts Dictionary
		buf += "  \"materialCounts\": {\n";
		int mtTotal = materialCounts.Count();
		for (int mt = 0; mt < mtTotal; mt++)
		{
			string mtKey = materialCounts.GetKey(mt);
			int mtVal = materialCounts.GetElement(mt);
			buf += "    \"" + TBD_MapExportJson.Escape(mtKey) + "\": " + mtVal.ToString();
			if (mt < mtTotal - 1)
				buf += ",";
			buf += "\n";
		}
		buf += "  },\n";

		// Visibility Counts Dictionary
		buf += "  \"visibilityCounts\": {\n";
		int vsTotal = visibilityCounts.Count();
		for (int vs = 0; vs < vsTotal; vs++)
		{
			string vsKey = visibilityCounts.GetKey(vs);
			int vsVal = visibilityCounts.GetElement(vs);
			buf += "    \"" + TBD_MapExportJson.Escape(vsKey) + "\": " + vsVal.ToString();
			if (vs < vsTotal - 1)
				buf += ",";
			buf += "\n";
		}
		buf += "  },\n";

		// Rocks Array
		buf += "  \"rocks\": [\n";
		if (!TBD_MapExportJson.Write(f, buf, TAG))
		{
			f.Close();
			return false;
		}
		buf = "";

		int writtenRocks = 0;

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

					vector pos2 = e2.GetOrigin();
					if (pos2[0] < 0 || pos2[0] > worldSize || pos2[2] < 0 || pos2[2] > worldSize)
						continue;
					if (CellIndex(pos2[0], cellM, cells) != ix2 || CellIndex(pos2[2], cellM, cells) != iz2)
						continue;

					vector bmin2, bmax2;
					e2.GetWorldBounds(bmin2, bmax2);
					float w2 = bmax2[0] - bmin2[0];
					float h2 = bmax2[1] - bmin2[1];
					float d2 = bmax2[2] - bmin2[2];

					if (h2 < 0.05 || w2 < 0.05 || d2 < 0.05 || h2 > 300.0 || w2 > 300.0 || d2 > 300.0)
						continue;

					string rn2 = ctx.ResolvePrefab(e2);
					string clsName2 = e2.ClassName();

					string rClass2, rMaterial2, rVariant2;
					if (!IsRockPrefab(rn2, clsName2, e2, rClass2, rMaterial2, rVariant2))
						continue;

					// 5-Point Terrain Surface Elevation Sampling
					float origX2 = pos2[0];
					float origZ2 = pos2[2];
					float tOrigin2 = ctx.m_API.GetTerrainSurfaceY(origX2, origZ2);
					float tC12 = ctx.m_API.GetTerrainSurfaceY(bmin2[0], bmin2[2]);
					float tC22 = ctx.m_API.GetTerrainSurfaceY(bmax2[0], bmin2[2]);
					float tC32 = ctx.m_API.GetTerrainSurfaceY(bmin2[0], bmax2[2]);
					float tC42 = ctx.m_API.GetTerrainSurfaceY(bmax2[0], bmax2[2]);

					float tMin2 = Math.Min(tOrigin2, Math.Min(Math.Min(tC12, tC22), Math.Min(tC32, tC42)));
					float tMax2 = Math.Max(tOrigin2, Math.Max(Math.Max(tC12, tC22), Math.Max(tC32, tC42)));
					float tAvg2 = (tOrigin2 + tC12 + tC22 + tC32 + tC42) * 0.2;

					float worldMinY2 = bmin2[1];
					float worldMaxY2 = bmax2[1];

					float expPeakH2 = worldMaxY2 - tMin2;
					if (expPeakH2 < 0.0) expPeakH2 = 0.0;

					float burialD2 = tMax2 - worldMinY2;
					if (burialD2 < 0.0) burialD2 = 0.0;

					float totalH2 = h2;
					if (totalH2 <= 0.001) totalH2 = 1.0;

					float expRatio2 = (worldMaxY2 - tAvg2) / totalH2;
					if (expRatio2 < 0.0) expRatio2 = 0.0;
					if (expRatio2 > 1.0) expRatio2 = 1.0;

					string vis2;
					if (worldMaxY2 <= tMin2 + 0.1 || expPeakH2 <= 0.1)
						vis2 = "fully_buried";
					else if (expRatio2 < 0.20)
						vis2 = "mostly_buried";
					else if (expRatio2 < 0.70)
						vis2 = "partially_exposed";
					else
						vis2 = "fully_exposed";

					if (cullBuried && vis2 == "fully_buried")
						continue;

					writtenRocks++;
					vector ang2 = e2.GetAngles();
					float scale2 = e2.GetScale();
					if (scale2 <= 0.001)
						scale2 = 1.0;

					vector apex2 = Vector(origX2, worldMaxY2, origZ2);

					buf += "    {\n";
					buf += "      \"id\": " + writtenRocks.ToString() + ",\n";
					buf += "      \"resourceName\": \"" + TBD_MapExportJson.Escape(rn2) + "\",\n";
					buf += "      \"rockClass\": \"" + TBD_MapExportJson.Escape(rClass2) + "\",\n";
					buf += "      \"material\": \"" + TBD_MapExportJson.Escape(rMaterial2) + "\",\n";
					buf += "      \"variant\": \"" + TBD_MapExportJson.Escape(rVariant2) + "\",\n";
					buf += "      \"position\": [" + pos2[0].ToString() + ", " + pos2[1].ToString() + ", " + pos2[2].ToString() + "],\n";
					buf += "      \"rotation\": [" + ang2[0].ToString() + ", " + ang2[1].ToString() + ", " + ang2[2].ToString() + "],\n";
					buf += "      \"scale\": " + scale2.ToString() + ",\n";
					buf += "      \"bounds\": {\n";
					buf += "        \"width\": " + w2.ToString() + ",\n";
					buf += "        \"height\": " + h2.ToString() + ",\n";
					buf += "        \"depth\": " + d2.ToString() + ",\n";
					buf += "        \"worldMinY\": " + worldMinY2.ToString() + ",\n";
					buf += "        \"worldMaxY\": " + worldMaxY2.ToString() + "\n";
					buf += "      },\n";
					buf += "      \"terrain\": {\n";
					buf += "        \"surfaceYAtOrigin\": " + tOrigin2.ToString() + ",\n";
					buf += "        \"surfaceYMin\": " + tMin2.ToString() + ",\n";
					buf += "        \"surfaceYMax\": " + tMax2.ToString() + ",\n";
					buf += "        \"exposedPeakHeightM\": " + expPeakH2.ToString() + ",\n";
					buf += "        \"burialDepthM\": " + burialD2.ToString() + ",\n";
					buf += "        \"exposureRatio\": " + expRatio2.ToString() + ",\n";
					buf += "        \"visibility\": \"" + TBD_MapExportJson.Escape(vis2) + "\",\n";
					buf += "        \"apex\": [" + apex2[0].ToString() + ", " + apex2[1].ToString() + ", " + apex2[2].ToString() + "]\n";
					buf += "      }\n";
					buf += "    }";

					if (writtenRocks < totalRocks)
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

		// Write rocks_meta.json
		FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (mh)
		{
			string mj = "";
			mj += "{\n";
			mj += "  \"method\": \"mod-rocks-terrain-sampling-export\",\n";
			mj += "  \"totalRocks\": " + totalRocks.ToString() + ",\n";
			mj += "  \"totalExposed\": " + totalExposed.ToString() + ",\n";
			mj += "  \"totalBuried\": " + totalBuried.ToString() + ",\n";
			mj += "  \"cullBuriedApplied\": " + cullBuried.ToString() + ",\n";
			mj += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
			mj += "  \"dataFile\": \"rocks.json\"\n";
			mj += "}\n";
			TBD_MapExportJson.Write(mh, mj, TAG);
			mh.Close();
		}

		int elapsedMs = System.GetTickCount() - tick0;
		Print(string.Format("%1 ROCK EXPORT FINISHED in %2 ms (Total=%3 rocks, Exposed=%4, Buried=%5) -> %6",
			TAG, elapsedMs, totalRocks, totalExposed, totalBuried, outJson), LogLevel.NORMAL);

		return true;
	}
}
