/**
 * TBD_BuildingVoxelDump.c
 *
 * Action "dump": the dumb-sensor half of the blueprint split. Marches the full 0.1 m lattice
 * along all six axis directions and streams RAW entry-face positions to
 * prefabs/dumps/<slug>_voxels.jsonl - zero interpretation in-engine. All wall/slab/roof
 * heuristics live in `cargo xtask map blueprint-from-voxels`, where a tune costs seconds
 * instead of a compile gate + Workbench restart.
 *
 * Wire format (NDJSON, one JSON value per line):
 *   line 1  meta object: origin (PADDED local scan min), cell, dims, span, unpadded bboxMin/Max,
 *           rootYawDeg, excluded counts, tick.
 *   lines   ["x+", j, k, [a, b, ...]]  scanline entry faces; non-empty scanlines only (that IS
 *           the compression). Lattice mapping: x -> j=iy k=iz, y -> j=ix k=iz, z -> j=ix k=iy;
 *           fixed coords at cell centers origin + (idx + 0.5) * cell. Entries are normalized to
 *           the shared axis coordinate (meters from origin[axis]; "-" runs store span - along)
 *           and kept in march order, so "+" lines ascend and "-" lines descend - the interpreter
 *           asserts this to catch convention bugs. Direction encodes face orientation ("x+"
 *           entries are the faces seen marching +X, i.e. -X-facing surfaces). A trailing 1 marks
 *           MAX_MARCH_HITS truncation.
 *   lines   {"furn":{...}} one per excluded furniture entity - LOCAL pos, WORLD yaw (the
 *           interpreter subtracts meta rootYawDeg; the old extract wrote world yaw into a
 *           local-frame record, which was wrong for rotated instances).
 *   last    {"end":{"lines":N,"ms":T}} - an absent end line means a truncated dump.
 */
class TBD_BuildingVoxelDump
{
	protected static const string TAG = "[TBD][VoxelDump]";
	//! Lattice pitch on every axis. Matches the scanner's ROW_STEP_M; running y at 0.1 m too
	//! (not the scanner's 0.25 m vertical columns) is what buys the interpreter its per-slice
	//! wall-persistence signal. The only escape hatch if a monster building blows the budget.
	protected static const float CELL_M = 0.1;
	//! Mirrors TBD_BuildingTraceScanner.MAX_MARCH_HITS (protected there).
	protected static const int TRUNC_HITS = 48;
	protected static const int FLUSH = 8000;

	//------------------------------------------------------------------------------------------------
	static string Execute(string filter)
	{
		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
			return "ERROR: context init failed";

		string resName;
		IEntity root = TBD_BuildingTraceScanner.FindTargetByFilter(ctx, filter, resName);
		if (!root)
			return "ERROR: no instance matching '" + filter + "'";
		string slug = TBD_BuildingArchitectExtractor.DerivePrefabSlug(resName);
		int tick0 = System.GetTickCount();
		Print(TAG + " dumping " + slug + " @ " + root.GetOrigin().ToString());

		TBD_BuildingTraceScanner scanner = new TBD_BuildingTraceScanner(ctx.m_World, root);
		int doorChildren = 0;
		int glassChildren = 0;
		ref array<IEntity> furnitureEnts = {};
		TBD_BuildingTraceExtract.ExcludeDressing(scanner, root, doorChildren, glassChildren, furnitureEnts);

		string mapName = ctx.GetMapName(null);
		TBD_MapExportConfig cfg = new TBD_MapExportConfig();
		string outPath = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName,
			"prefabs/dumps", slug + "_voxels.jsonl");
		FileHandle f = FileIO.OpenFile(outPath, FileMode.WRITE);
		if (!f)
			return "ERROR: cannot open " + outPath;

		WriteMeta(f, scanner, root, slug, resName, doorChildren, glassChildren, furnitureEnts.Count());

		string buf = "";
		int lines = 0;
		lines += DumpAxisX(scanner, f, buf);
		lines += DumpAxisZ(scanner, f, buf);
		lines += DumpAxisY(scanner, f, buf);
		lines += EmitFurnitureLines(scanner, f, furnitureEnts, buf);

		int ms = System.GetTickCount() - tick0;
		buf += "{\"end\":{\"lines\":" + lines.ToString() + ",\"ms\":" + ms.ToString() + "}}\n";
		TBD_MapExportJson.Write(f, buf, TAG);
		f.Close();

		string summary = string.Format("OK %1: %2 scanlines in %3 ms -> %4", slug, lines, ms, outPath);
		Print(TAG + " " + summary);
		return summary;
	}

	//------------------------------------------------------------------------------------------------
	protected static void WriteMeta(FileHandle f, TBD_BuildingTraceScanner s, IEntity root,
		string slug, string resName, int doors, int glass, int furn)
	{
		vector obMin, obMax;
		root.GetBounds(obMin, obMax);
		vector ang = root.GetAngles();
		vector span = s.m_vMax - s.m_vMin;
		int nx = Math.Ceil(span[0] / CELL_M);
		int ny = Math.Ceil(span[1] / CELL_M);
		int nz = Math.Ceil(span[2] / CELL_M);

		string m = "{\"v\":\"tbd-voxel-dump/1\"";
		m += ",\"slug\":\"" + TBD_MapExportJson.Escape(slug) + "\"";
		m += ",\"resource\":\"" + TBD_MapExportJson.Escape(resName) + "\"";
		m += ",\"origin\":" + Vec3Json(s.m_vMin);
		m += ",\"cell\":" + CELL_M.ToString();
		m += ",\"dims\":[" + nx.ToString() + "," + ny.ToString() + "," + nz.ToString() + "]";
		m += ",\"span\":" + Vec3Json(span);
		m += ",\"bboxMin\":" + Vec3Json(obMin);
		m += ",\"bboxMax\":" + Vec3Json(obMax);
		m += ",\"rootYawDeg\":" + R2(ang[1]).ToString();
		m += ",\"excluded\":{\"doors\":" + doors.ToString() + ",\"glass\":" + glass.ToString()
			+ ",\"furniture\":" + furn.ToString() + "}";
		m += ",\"tick\":" + System.GetTickCount().ToString() + "}\n";
		TBD_MapExportJson.Write(f, m, TAG);
	}

	//------------------------------------------------------------------------------------------------
	//! X marches over the (iy, iz) lattice, both directions. Returns emitted scanline count.
	protected static int DumpAxisX(TBD_BuildingTraceScanner s, FileHandle f, inout string buf)
	{
		vector span = s.m_vMax - s.m_vMin;
		int ny = Math.Ceil(span[1] / CELL_M);
		int nz = Math.Ceil(span[2] / CELL_M);
		int count = 0;
		for (int iy = 0; iy < ny; iy++)
		{
			float y = s.m_vMin[1] + (iy + 0.5) * CELL_M;
			for (int iz = 0; iz < nz; iz++)
			{
				float z = s.m_vMin[2] + (iz + 0.5) * CELL_M;
				array<float> fwd;
				s.MarchEntries(Vector(s.m_vMin[0], y, z), Vector(s.m_vMax[0], y, z), fwd);
				if (fwd.Count() > 0)
				{
					buf += ScanLineJson("x+", iy, iz, fwd, 0.0);
					count++;
				}
				array<float> bwd;
				s.MarchEntries(Vector(s.m_vMax[0], y, z), Vector(s.m_vMin[0], y, z), bwd);
				if (bwd.Count() > 0)
				{
					buf += ScanLineJson("x-", iy, iz, bwd, span[0]);
					count++;
				}
				FlushIf(f, buf);
			}
		}
		return count;
	}

	//------------------------------------------------------------------------------------------------
	//! Z marches over the (ix, iy) lattice, both directions.
	protected static int DumpAxisZ(TBD_BuildingTraceScanner s, FileHandle f, inout string buf)
	{
		vector span = s.m_vMax - s.m_vMin;
		int nx = Math.Ceil(span[0] / CELL_M);
		int ny = Math.Ceil(span[1] / CELL_M);
		int count = 0;
		for (int ix = 0; ix < nx; ix++)
		{
			float x = s.m_vMin[0] + (ix + 0.5) * CELL_M;
			for (int iy = 0; iy < ny; iy++)
			{
				float y = s.m_vMin[1] + (iy + 0.5) * CELL_M;
				array<float> fwd;
				s.MarchEntries(Vector(x, y, s.m_vMin[2]), Vector(x, y, s.m_vMax[2]), fwd);
				if (fwd.Count() > 0)
				{
					buf += ScanLineJson("z+", ix, iy, fwd, 0.0);
					count++;
				}
				array<float> bwd;
				s.MarchEntries(Vector(x, y, s.m_vMax[2]), Vector(x, y, s.m_vMin[2]), bwd);
				if (bwd.Count() > 0)
				{
					buf += ScanLineJson("z-", ix, iy, bwd, span[2]);
					count++;
				}
				FlushIf(f, buf);
			}
		}
		return count;
	}

	//------------------------------------------------------------------------------------------------
	//! Y marches over the (ix, iz) lattice: "y-" top-down first (first entry = top surface, the
	//! interpreter's eave/ridge/roof-slope source), then "y+" bottom-up (undersides).
	protected static int DumpAxisY(TBD_BuildingTraceScanner s, FileHandle f, inout string buf)
	{
		vector span = s.m_vMax - s.m_vMin;
		int nx = Math.Ceil(span[0] / CELL_M);
		int nz = Math.Ceil(span[2] / CELL_M);
		int count = 0;
		for (int ix = 0; ix < nx; ix++)
		{
			float x = s.m_vMin[0] + (ix + 0.5) * CELL_M;
			for (int iz = 0; iz < nz; iz++)
			{
				float z = s.m_vMin[2] + (iz + 0.5) * CELL_M;
				array<float> down;
				s.MarchEntries(Vector(x, s.m_vMax[1], z), Vector(x, s.m_vMin[1], z), down);
				if (down.Count() > 0)
				{
					buf += ScanLineJson("y-", ix, iz, down, span[1]);
					count++;
				}
				array<float> up;
				s.MarchEntries(Vector(x, s.m_vMin[1], z), Vector(x, s.m_vMax[1], z), up);
				if (up.Count() > 0)
				{
					buf += ScanLineJson("y+", ix, iz, up, 0.0);
					count++;
				}
				FlushIf(f, buf);
			}
		}
		return count;
	}

	//------------------------------------------------------------------------------------------------
	//! One scanline as a compact JSON array line. `flipSpan` > 0 converts a "-" march's
	//! distance-from-start into the shared axis coordinate (span - along); 0.0 passes "+" runs
	//! through unchanged.
	protected static string ScanLineJson(string code, int j, int k, array<float> entries, float flipSpan)
	{
		string line = "[\"" + code + "\"," + j.ToString() + "," + k.ToString() + ",[";
		for (int i = 0; i < entries.Count(); i++)
		{
			float v = entries[i];
			if (flipSpan > 0.0)
				v = flipSpan - v;
			if (i > 0)
				line += ",";
			line += R2(v).ToString();
		}
		line += "]";
		if (entries.Count() == TRUNC_HITS)
			line += ",1";
		line += "]\n";
		return line;
	}

	//------------------------------------------------------------------------------------------------
	protected static int EmitFurnitureLines(TBD_BuildingTraceScanner s, FileHandle f,
		array<IEntity> furniture, inout string buf)
	{
		int count = 0;
		foreach (IEntity fe : furniture)
		{
			vector fl = s.WorldToLocal(fe.GetOrigin());
			vector fMin, fMax;
			fe.GetBounds(fMin, fMax);
			vector ang = fe.GetAngles();
			string fname = fe.GetName();
			if (fname.IsEmpty())
				fname = "prop";
			buf += "{\"furn\":{\"name\":\"" + TBD_MapExportJson.Escape(fname)
				+ "\",\"res\":\"" + TBD_MapExportJson.Escape(TBD_MapExportContext.GetEntityResourceName(fe))
				+ "\",\"pos\":" + Vec3Json(fl)
				+ ",\"worldYawDeg\":" + R2(ang[1]).ToString()
				+ ",\"size\":" + Vec3Json(fMax - fMin)
				+ ",\"boundsMinY\":" + R2(fMin[1]).ToString() + "}}\n";
			count++;
			FlushIf(f, buf);
		}
		return count;
	}

	//------------------------------------------------------------------------------------------------
	protected static void FlushIf(FileHandle f, inout string buf)
	{
		if (buf.Length() > FLUSH)
		{
			TBD_MapExportJson.Write(f, buf, TAG);
			buf = "";
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static string Vec3Json(vector v)
	{
		return "[" + R2(v[0]).ToString() + "," + R2(v[1]).ToString() + "," + R2(v[2]).ToString() + "]";
	}

	//------------------------------------------------------------------------------------------------
	protected static float R2(float v)
	{
		return Math.Round(v * 100.0) / 100.0;
	}
}

[WorkbenchPluginAttribute(
	name: "Dump Building Voxels",
	description: "March the full 0.1 m lattice of the first matching building into a raw voxel dump.",
	category: "TBD"
)]
class TBD_BuildingVoxelDumpPlugin : WorkbenchPlugin
{
	[Attribute("FarmHouse_E_1L01", UIWidgets.EditBox, desc: "Prefab resource substring to match")]
	string m_sPrefabFilter;

	override void Run()
	{
		Print("[TBD][VoxelDump] " + TBD_BuildingVoxelDump.Execute(m_sPrefabFilter));
	}
}
