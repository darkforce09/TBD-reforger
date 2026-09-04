/**
 * TBD_WorldFullExportPlugin.c - the single-JSONL full-world export the objects pipeline reads.
 *
 * T-090.12.1 resurrects the T-090.3.1 "Export TBD World Objects (full)" plugin (deleted in the
 * MapExport modularisation) — `copy-export-profile --full` + `world build-objects` consume
 * exactly this pair of files:
 *
 *   $profile:TBD_WorldExport_full.jsonl        one entity per line, every entity in the terrain
 *   $profile:TBD_WorldExport_full_meta.json    written LAST — the completion sentinel
 *
 * Row v2 (exportVersion 2): {resourceName, className, x, y, z, headingDeg, pitchDeg, rollDeg,
 * scale, halfExtentsM}. Angles are GetAngles() = (pitch about X, HEADING about Y, roll about Z)
 * — the S6 rule the converter pins. `scale` is the entity's uniform GetScale() (<= 0.001 reads
 * as 1.0, the vegetation exporters' rule); it is the one field the July 2026 export lacked.
 *
 * Menu: Workbench > Plugins > TBD > "Export TBD World Objects (full)"
 */

[WorkbenchPluginAttribute(name: "Export TBD World Objects (full)", description: "T-090.3.1 / T-090.12.1: iterate the whole terrain in 512 m cell passes; write raw-entities JSONL (full transform + scale) + completion-sentinel meta to $profile.", category: "TBD")]
class TBD_WorldFullExportPlugin : WorkbenchPlugin
{
	protected static const float CELL_M = 512.0;
	protected static const float Y_MIN  = -1000.0; // AABB vertical span (covers Everon -204..375 m)
	protected static const float Y_MAX  = 2000.0;
	protected static const int   FLUSH  = 8000;    // buffered-write threshold (chars) — DEM plugin idiom
	protected static const int   EXPORT_VERSION = 2;

	protected static const string TAG = "[TBD][WorldFull]";
	protected static const string OUT_JSONL = "$profile:TBD_WorldExport_full.jsonl";
	protected static const string OUT_META  = "$profile:TBD_WorldExport_full_meta.json";

	protected ref array<IEntity> m_aHits;

	//------------------------------------------------------------------------------------------------
	protected bool CollectEntity(IEntity e)
	{
		if (e)
			m_aHits.Insert(e);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected BaseWorld ResolveWorld(WorldEditorAPI api)
	{
		int rootCount = api.GetEditorEntityCount();
		for (int i = 0; i < rootCount; i++)
		{
			IEntitySource s = api.GetEditorEntity(i);
			if (!s)
				continue;
			IEntity re = api.SourceToEntity(s);
			if (re)
			{
				BaseWorld w = re.GetWorld();
				if (w)
					return w;
			}
		}
		return null;
	}

	//------------------------------------------------------------------------------------------------
	protected string ResolvePrefab(WorldEditorAPI api, IEntity e)
	{
		IEntitySource src = api.EntityToSource(e);
		if (!src)
			return "";
		BaseContainer anc = src.GetAncestor();
		if (!anc)
			return "";
		return anc.GetResourceName();
	}

	//------------------------------------------------------------------------------------------------
	//! Partition formula — MUST agree with the host side (tools/tbd-tools geometry::cell_of):
	//! clamp(floor(coord / 512), 0, cells-1). coord == worldSize lands in the last cell.
	protected int CellIndex(float coord, int cells)
	{
		int c = Math.Floor(coord / CELL_M);
		if (c < 0)
			c = 0;
		if (c > cells - 1)
			c = cells - 1;
		return c;
	}

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		WorldEditor we = Workbench.GetModule(WorldEditor);
		if (!we)
		{
			Print(TAG + " WorldEditor module not available", LogLevel.ERROR);
			return;
		}
		WorldEditorAPI api = we.GetApi();
		if (!api)
		{
			Print(TAG + " WorldEditorAPI not available", LogLevel.ERROR);
			return;
		}
		BaseWorld world = ResolveWorld(api);
		if (!world)
		{
			Print(TAG + " could not resolve BaseWorld from top-level entities", LogLevel.ERROR);
			return;
		}

		vector bMin, bMax;
		if (!we.GetTerrainBounds(bMin, bMax))
		{
			Print(TAG + " GetTerrainBounds failed", LogLevel.ERROR);
			return;
		}
		float worldSize = bMax[0];
		if (bMax[2] > worldSize)
			worldSize = bMax[2];
		int cells = Math.Ceil(worldSize / CELL_M);
		Print(string.Format("%1 terrain %2 m -> %3 x %3 cell passes (exportVersion %4)", TAG, worldSize, cells, EXPORT_VERSION));

		// Stale sentinel must die BEFORE any writing — a crashed run must never look complete.
		FileIO.DeleteFile(OUT_META);

		FileHandle f = FileIO.OpenFile(OUT_JSONL, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " cannot open " + OUT_JSONL, LogLevel.ERROR);
			return;
		}

		int tick0 = System.GetTickCount();
		int aabbHits = 0;
		int kept = 0;
		int withPrefab = 0;
		int withScale = 0;
		int outOfBounds = 0;
		string buf = "";

		for (int iz = 0; iz < cells; iz++)
		{
			for (int ix = 0; ix < cells; ix++)
			{
				float x0 = ix * CELL_M;
				float z0 = iz * CELL_M;
				m_aHits = {};
				vector mins = Vector(x0, Y_MIN, z0);
				vector maxs = Vector(x0 + CELL_M, Y_MAX, z0 + CELL_M);
				world.QueryEntitiesByAABB(mins, maxs, CollectEntity);
				aabbHits += m_aHits.Count();

				int cellKept = 0;
				foreach (IEntity e : m_aHits)
				{
					vector pos = e.GetOrigin();
					if (pos[0] < 0 || pos[0] > worldSize || pos[2] < 0 || pos[2] > worldSize)
					{
						// Counted once: only the (0,0) pass tallies it so the meta counter stays exact.
						if (ix == 0 && iz == 0)
							outOfBounds++;
						continue;
					}
					if (CellIndex(pos[0], cells) != ix || CellIndex(pos[2], cells) != iz)
						continue;

					// S6 MEASURED: GetAngles() = (pitch, HEADING/yaw-about-Y, roll).
					vector ang = e.GetAngles();
					vector bmin;
					vector bmax2;
					e.GetWorldBounds(bmin, bmax2);
					float hx = (bmax2[0] - bmin[0]) * 0.5;
					float hy = (bmax2[1] - bmin[1]) * 0.5;
					float hz = (bmax2[2] - bmin[2]) * 0.5;

					// T-090.12.1 — uniform scale (forest-generator trees are the non-unit case).
					float scale = e.GetScale();
					if (scale <= 0.001)
						scale = 1.0;
					else
						withScale++;

					string rn = ResolvePrefab(api, e);
					if (rn != "")
						withPrefab++;

					string row = "{";
					row += "\"resourceName\":\"" + TBD_MapExportJson.Escape(rn) + "\",";
					row += "\"className\":\"" + TBD_MapExportJson.Escape(e.ClassName()) + "\",";
					row += "\"x\":" + pos[0].ToString() + ",";
					row += "\"y\":" + pos[1].ToString() + ",";
					row += "\"z\":" + pos[2].ToString() + ",";
					row += "\"headingDeg\":" + ang[1].ToString() + ",";
					row += "\"pitchDeg\":" + ang[0].ToString() + ",";
					row += "\"rollDeg\":" + ang[2].ToString() + ",";
					row += "\"scale\":" + scale.ToString() + ",";
					row += "\"halfExtentsM\":[" + hx.ToString() + "," + hy.ToString() + "," + hz.ToString() + "]";
					row += "}\n";
					buf += row;
					kept++;
					cellKept++;
					if (buf.Length() > FLUSH)
					{
						if (!TBD_MapExportJson.Write(f, buf, TAG))
						{
							f.Close();
							FileIO.DeleteFile(OUT_JSONL);
							Print(TAG + " ABORTED: JSONL write failed — partial file deleted.", LogLevel.ERROR);
							return;
						}
						buf = "";
					}
				}
				Print(string.Format("%1 cell (%2,%3) hits %4 kept %5 (total kept %6)", TAG, ix, iz, m_aHits.Count(), cellKept, kept));
			}
		}

		bool jsonlOk = TBD_MapExportJson.Write(f, buf, TAG);
		f.Close();
		if (!jsonlOk)
		{
			FileIO.DeleteFile(OUT_JSONL);
			Print(TAG + " ABORTED: JSONL write failed — partial file deleted.", LogLevel.ERROR);
			return;
		}

		int elapsedMs = System.GetTickCount() - tick0;

		// Meta LAST — completion sentinel for copy-export-profile --full.
		FileHandle mh = FileIO.OpenFile(OUT_META, FileMode.WRITE);
		if (!mh)
		{
			Print(TAG + " cannot open meta " + OUT_META + " — export UNSEALED (copy will refuse)", LogLevel.ERROR);
			return;
		}
		string mj = "{\n";
		mj += "  \"exportVersion\": " + EXPORT_VERSION.ToString() + ",\n";
		mj += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
		mj += "  \"cellSizeM\": " + CELL_M.ToString() + ",\n";
		mj += "  \"cells\": " + cells.ToString() + ",\n";
		mj += "  \"aabbHitCount\": " + aabbHits.ToString() + ",\n";
		mj += "  \"keptCount\": " + kept.ToString() + ",\n";
		mj += "  \"withPrefab\": " + withPrefab.ToString() + ",\n";
		mj += "  \"withScale\": " + withScale.ToString() + ",\n";
		mj += "  \"outOfBounds\": " + outOfBounds.ToString() + ",\n";
		mj += "  \"elapsedMs\": " + elapsedMs.ToString() + ",\n";
		mj += "  \"anglesRule\": \"headingDeg=GetAngles()[1] (S6); pitch=[0], roll=[2]\",\n";
		mj += "  \"scaleRule\": \"scale=GetScale() (<= 0.001 -> 1.0)\",\n";
		mj += "  \"partitionRule\": \"clamp(floor(coord/512), 0, cells-1) on entity origin\"\n";
		mj += "}\n";
		bool metaOk = TBD_MapExportJson.Write(mh, mj, TAG);
		mh.Close();
		if (!metaOk)
		{
			FileIO.DeleteFile(OUT_META);
			Print(TAG + " meta write failed — export UNSEALED (copy will refuse).", LogLevel.ERROR);
			return;
		}
		Print(string.Format("%1 DONE — kept %2 (withPrefab %3, withScale %4, aabbHits %5, oob %6) in %7 ms", TAG, kept, withPrefab, withScale, aabbHits, outOfBounds, elapsedMs));
	}
}
