/**
 * TBD_BuildingTraceExtract.c
 *
 * Assembles a building blueprint JSON from TBD_BuildingTraceScanner passes (Phase B):
 * floor bands + roof profile from the vertical march, walls/masses from per-band occupancy
 * rects, doors/windows from the aperture scan with MEASURED sills. Every number in the output
 * comes from a raycast against the building's own Projectile-layer collision - nothing is
 * hardcoded. Fields the traces cannot know (hinge side, swing, prefab refs - the entity pass
 * fills those later) are exported as "unknown"/empty, never guessed.
 *
 * Also hosts the parity sampler: random A/B pairs vs the same collision world, the offline
 * oracle replayed through Rust `evaluate_los` for the agreement report.
 *
 * Menu:   Workbench > Plugins > TBD > "Extract Building Blueprint"
 * Remote: `EMCP_WB_TbdBlueprint` actions "extract" / "parity".
 */

class TBD_BuildingTraceExtract
{
	protected static const string TAG = "[TBD][TraceExtract]";
	//! Wall-vs-mass split: occupancy rects thicker than this are interior masses (stairs,
	//! chimneys), not walls.
	protected static const float WALL_MAX_THICKNESS_M = 0.6;
	//! Band scan height above the floor slab: below typical window sills (~0.85 m) so windows
	//! do not slice walls, above floor clutter. Doors DO cut the walls at this height - the
	//! aperture scan re-detects them and the split wall segments stay honest.
	protected static const float BAND_SCAN_ABOVE_FLOOR_M = 0.45;
	protected static const float DOOR_MAX_SILL_M = 0.3;
	//! WALLS-ONLY iteration (operator directive 2026-08-28): aperture/door/window emission is
	//! muted until the wall picture is verified correct in the viewer - one variable at a time.
	protected static const bool SCAN_APERTURES = false;

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
		Print(TAG + " scanning " + slug + " @ " + root.GetOrigin().ToString());

		// Original (unpadded) local bounds for profile numbers.
		vector obMin, obMax;
		root.GetBounds(obMin, obMax);

		TBD_BuildingTraceScanner scanner = new TBD_BuildingTraceScanner(ctx.m_World, root);

		// -- Pass 0: component-driven trace exclusions (recon-proven) -------------------------
		// Door leaves (DoorComponent) are usually CLOSED in the editor - left in the traces they
		// would seal their own doorways; excluded, each doorway reads as a sill-0 aperture and
		// exports as a measured door. Destructible panes/shutters/boards
		// (SCR_DestructionMultiPhaseComponent, 56 on the FarmHouse) would seal the window
		// apertures the same way.
		int doorChildren = 0;
		int glassChildren = 0;
		ref array<IEntity> furnitureEnts = {};
		ExcludeDressing(scanner, root, doorChildren, glassChildren, furnitureEnts);
		Print(TAG + string.Format(" excluded %1 door + %2 destructible + %3 furniture children from traces",
			doorChildren, glassChildren, furnitureEnts.Count()));

		// -- Pass 1: bands + roof -------------------------------------------------------------
		array<float> slabYs;
		float eaveY, ridgeY, chimneyY;
		scanner.ScanVertical(slabYs, eaveY, ridgeY, chimneyY);

		// Floor slabs only: drop foundation-skirt returns and roof-height returns.
		ref array<float> floors = {};
		foreach (float s : slabYs)
		{
			if (s > -0.5 && s < eaveY - 0.5)
				floors.Insert(s);
		}
		if (floors.Count() == 0)
			floors.Insert(0.0);

		TBD_BuildingBlueprint bp = new TBD_BuildingBlueprint(slug, resName);
		bp.m_sLabel = slug;
		bp.m_sCategory = "scanned";
		bp.m_vBBoxMin = obMin;
		bp.m_vBBoxMax = obMax;
		bp.m_fPivotElevationOffsetM = 0.0;
		bp.m_fFoundationSkirtDepthM = Math.Max(0.0, -obMin[1]);
		bp.m_fTotalHeightM = obMax[1];
		bp.m_fEaveHeightM = eaveY;
		bp.m_fRidgeHeightM = ridgeY;
		bp.m_fChimneyHeightM = chimneyY;
		if (chimneyY > ridgeY + 0.2)
			bp.m_sRoofType = "with_chimney";
		else
			bp.m_sRoofType = "scanned";

		MeshObject mo = root.GetVObject().ToMeshObject();
		if (mo)
			bp.m_sModelMesh = mo.GetResourceName();

		// -- Per band: occupancy -> walls/masses, outline, apertures ---------------------------
		int wallSeq = 0;
		for (int li = 0; li < floors.Count(); li++)
		{
			float bandMin = floors[li];
			float bandMax;
			if (li + 1 < floors.Count())
				bandMax = floors[li + 1];
			else
				bandMax = Math.Max(eaveY, bandMin + 2.0);
			wallSeq = EmitBand(scanner, bp, li, bandMin, bandMax, wallSeq);
		}

		// -- Furniture records (excluded from traces above; real bounds, assigned by height) --
		EmitFurniture(scanner, bp, furnitureEnts);

		// -- Save -----------------------------------------------------------------------------
		string mapName = ctx.GetMapName(null);
		TBD_MapExportConfig cfg = new TBD_MapExportConfig();
		string outPath = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName,
			"prefabs/buildings", slug + ".json");
		FileHandle f = FileIO.OpenFile(outPath, FileMode.WRITE);
		if (!f)
			return "ERROR: cannot open " + outPath;
		TBD_MapExportJson.Write(f, bp.ToJson(), TAG);
		f.Close();

		int ms = System.GetTickCount() - tick0;
		int nLvls = bp.m_aLevels.Count();
		string summary = string.Format("OK %1: %2 levels in %3 ms -> %4", slug, nLvls, ms, outPath);
		Print(TAG + " " + summary);
		return summary;
	}

	//------------------------------------------------------------------------------------------------
	//! One vertical band -> one level record: occupancy walls/masses, floor-plate outline,
	//! apertures. Split out of Execute because EnforceScript caps a function at 64 local variables
	//! and the one-body version exceeded it ("Maximum of 64 local variables exceeded", WB
	//! 2026-08-28) - do not re-inline. Returns the advanced wall sequence counter.
	protected static int EmitBand(TBD_BuildingTraceScanner scanner, TBD_BuildingBlueprint bp,
		int li, float bandMin, float bandMax, int wallSeq)
	{
		// Operator floor-naming scheme: ground = 1st, then 2nd ... (basement would be Floor 0).
		string lvlName = OrdinalFloorName(li + 1);
		TBD_BuildingLevel lvl = new TBD_BuildingLevel(li, lvlName, bandMin, bandMax,
			bandMin + BAND_SCAN_ABOVE_FLOOR_M);

		array<bool> grid;
		int nx, nz;
		scanner.ScanBandOccupancy(bandMin + BAND_SCAN_ABOVE_FLOOR_M,
			bandMin + BAND_SCAN_ABOVE_FLOOR_M + 0.35, grid, nx, nz);

		// Footprint = the FLOOR PLATE, not the wall occupancy: a mezzanine void or a
		// floor-to-ridge room has no plate and falls outside the level polygon.
		array<bool> plate;
		int pnx, pnz;
		scanner.ScanFloorPlate(bandMin, plate, pnx, pnz);
		array<ref array<float>> outline;
		scanner.OutlineFromGrid(plate, pnx, pnz, outline);
		int occupied = 0;
		for (int gi = 0; gi < grid.Count(); gi++)
		{
			if (grid[gi])
				occupied++;
		}
		int plateCells = 0;
		for (int pi = 0; pi < plate.Count(); pi++)
		{
			if (plate[pi])
				plateCells++;
		}
		foreach (array<float> pt : outline)
			lvl.m_aFootprintPolygon.Insert(Vector(pt[0], 0, pt[1]));

		if (li == 0)
		{
			bp.m_aOverallFootprint2D = lvl.m_aFootprintPolygon;
			bp.m_fFootprintSqM = plateCells * 0.01; // 0.1 m plate cells - true walkable area
		}

		// Rects -> walls / masses.
		array<ref array<float>> rects;
		scanner.RectsFromGrid(grid, nx, nz, rects);
		float outMinX = 1000000;
		float outMinZ = 1000000;
		float outMaxX = -1000000;
		float outMaxZ = -1000000;
		foreach (array<float> r0 : rects)
		{
			if (r0[0] < outMinX) outMinX = r0[0];
			if (r0[1] < outMinZ) outMinZ = r0[1];
			if (r0[2] > outMaxX) outMaxX = r0[2];
			if (r0[3] > outMaxZ) outMaxZ = r0[3];
		}

		// Phase 1: split rects into wall candidates vs interior masses.
		ref array<ref array<float>> wallRects = {};
		foreach (array<float> r : rects)
		{
			float w = r[2] - r[0];
			float d = r[3] - r[1];
			float thickness = Math.Min(w, d);
			float cx = (r[0] + r[2]) * 0.5;
			float cz = (r[1] + r[3]) * 0.5;

			if (thickness > WALL_MAX_THICKNESS_M)
			{
				// Interior mass (stair block, chimney, pillar) - cover, not a wall.
				TBD_BuildingFurniture mass = new TBD_BuildingFurniture(
					"mass_" + li.ToString() + "_" + wallSeq.ToString(),
					"scanned mass", "scan_mass", "",
					cx, cz, 0.0, w, d, bandMax - bandMin,
					true, "full_cover");
				lvl.m_aFurniture.Insert(mass);
				wallSeq++;
				continue;
			}
			// Isolated slivers (both dims tiny) are collision noise, not architecture.
			if (Math.Max(w, d) < 0.5)
				continue;
			wallRects.Insert(r);
		}

		// Phase 2: merge collinear neighbors - the greedy decomposition splits one wall
		// into dozens of grid slivers (236 "walls" on the first live scan).
		MergeWallRects(wallRects);

		// Phase 3: walls + apertures on the MERGED rects.
		foreach (array<float> mr : wallRects)
		{
			float mw = mr[2] - mr[0];
			float md = mr[3] - mr[1];
			float mthick = Math.Min(mw, md);
			float mcx = (mr[0] + mr[2]) * 0.5;
			float mcz = (mr[1] + mr[3]) * 0.5;
			bool alongX = mw >= md;
			bool isExt = (mr[0] - outMinX < 0.3) || (outMaxX - mr[2] < 0.3)
				|| (mr[1] - outMinZ < 0.3) || (outMaxZ - mr[3] < 0.3);
			string wid = "w_scan_" + li.ToString() + "_" + wallSeq.ToString();
			TBD_BuildingWall wall;
			if (alongX)
				wall = new TBD_BuildingWall(wid, mr[0], mcz, mr[2], mcz, mthick, isExt, "scanned");
			else
				wall = new TBD_BuildingWall(wid, mcx, mr[1], mcx, mr[3], mthick, isExt, "scanned");
			lvl.m_aWalls.Insert(wall);
			wallSeq++;

			if (SCAN_APERTURES)
				wallSeq = EmitApertures(scanner, lvl, li, mr, bandMin, bandMax, alongX, mcx, mcz,
					wid, isExt, outMinX, outMinZ, outMaxX, outMaxZ, wallSeq);
		}

		Print(TAG + string.Format(" band %1 [%2..%3]: occ=%4 plate=%5 rects=%6 wallRects=%7 walls=%8",
			li, bandMin, bandMax, occupied, plateCells, rects.Count(), wallRects.Count(),
			lvl.m_aWalls.Count()));

		bp.m_aLevels.Insert(lvl);
		return wallSeq;
	}

	//------------------------------------------------------------------------------------------------
	//! Aperture scan for one merged wall rect: doors (sill < DOOR_MAX_SILL_M) vs windows, window
	//! facing normal pointing away from the footprint-interior bbox center. Same 64-local split as
	//! EmitBand. Returns the advanced wall sequence counter.
	protected static int EmitApertures(TBD_BuildingTraceScanner scanner, TBD_BuildingLevel lvl,
		int li, array<float> mr, float bandMin, float bandMax, bool alongX, float mcx, float mcz,
		string wid, bool isExt, float outMinX, float outMinZ, float outMaxX, float outMaxZ,
		int wallSeq)
	{
		array<ref array<float>> apertures;
		scanner.ScanApertures(mr, bandMin, bandMax, alongX, apertures);
		foreach (array<float> ap : apertures)
		{
			float alongStart = ap[0];
			float openLow = ap[1];
			float openHigh = ap[2];
			float alongLen = ap[3];
			// Human-relevant openings only.
			if (alongLen < 0.5 || openHigh - openLow < 0.5)
				continue;
			float apMidAlong = alongStart + alongLen * 0.5;
			float px, pz;
			if (alongX)
			{
				px = apMidAlong;
				pz = mcz;
			}
			else
			{
				px = mcx;
				pz = apMidAlong;
			}
			float sill = openLow - bandMin;
			if (sill < DOOR_MAX_SILL_M)
			{
				TBD_BuildingDoor door = new TBD_BuildingDoor(
					"door_scan_" + li.ToString() + "_" + wallSeq.ToString(),
					"", wid, px, pz, alongLen, openHigh - bandMin,
					"unknown", "unknown", isExt, false, "closed");
				lvl.m_aDoors.Insert(door);
			}
			else
			{
				// Facing normal: perpendicular to the wall, pointing away from the
				// footprint interior (approximated by the outline bbox center).
				float nxn = 0;
				float nzn = 0;
				if (alongX)
				{
					if (mcz > (outMinZ + outMaxZ) * 0.5)
						nzn = 1;
					else
						nzn = -1;
				}
				else
				{
					if (mcx > (outMinX + outMaxX) * 0.5)
						nxn = 1;
					else
						nxn = -1;
				}
				TBD_BuildingWindow win = new TBD_BuildingWindow(
					"win_scan_" + li.ToString() + "_" + wallSeq.ToString(),
					"", wid, px, pz, alongLen, sill, openHigh - openLow,
					nxn, nzn, 140.0, true, 0);
				lvl.m_aWindows.Insert(win);
			}
			wallSeq++;
		}
		return wallSeq;
	}

	//------------------------------------------------------------------------------------------------
	//! Furniture records for the entities excluded from the traces (real bounds, assigned to the
	//! level whose band holds the prop's base).
	protected static void EmitFurniture(TBD_BuildingTraceScanner scanner, TBD_BuildingBlueprint bp,
		array<IEntity> furnitureEnts)
	{
		int furnSeq = 0;
		foreach (IEntity fe : furnitureEnts)
		{
			vector fl = scanner.WorldToLocal(fe.GetOrigin());
			vector fMin, fMax;
			fe.GetBounds(fMin, fMax);
			float fw = fMax[0] - fMin[0];
			float fh = fMax[1] - fMin[1];
			float fd = fMax[2] - fMin[2];
			// Composition parents span the whole interior - record only prop-sized entities.
			if (Math.Max(fw, fd) > 4.0 || fh > 3.5)
				continue;
			string cover = "low_cover";
			if (fh >= 1.6)
				cover = "full_cover";
			else if (fh < 0.4)
				cover = "none";
			string fname = fe.GetName();
			if (fname.IsEmpty())
				fname = "prop";
			vector fAng = fe.GetAngles();
			TBD_BuildingFurniture fr = new TBD_BuildingFurniture(
				"furn_scan_" + furnSeq.ToString(), fname, "prop",
				TBD_MapExportContext.GetEntityResourceName(fe),
				fl[0], fl[2], fAng[1], Math.Max(0.2, fw), Math.Max(0.2, fd), Math.Max(0.2, fh),
				true, cover);
			// Assign to the level whose band holds the prop's base.
			foreach (TBD_BuildingLevel flvl : bp.m_aLevels)
			{
				if (fl[1] >= flvl.m_fElevationMin - 0.3 && fl[1] < flvl.m_fElevationMax)
				{
					flvl.m_aFurniture.Insert(fr);
					break;
				}
			}
			furnSeq++;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Operator floor-naming: ground = "1st Floor", up = "2nd Floor" ... (Floor 0 = basement, when
	//! basement bands land).
	protected static string OrdinalFloorName(int n)
	{
		if (n == 1)
			return "1st Floor";
		if (n == 2)
			return "2nd Floor";
		if (n == 3)
			return "3rd Floor";
		return n.ToString() + "th Floor";
	}

	//------------------------------------------------------------------------------------------------
	//! Merge axis-aligned rects that continue the same wall: overlapping cross-extents, gap along
	//! the wall axis <= 0.15 m. Iterates to a fixed point (grid slivers chain-merge into runs).
	protected static void MergeWallRects(array<ref array<float>> rects)
	{
		bool merged = true;
		int guard = 0;
		while (merged && guard < 64)
		{
			merged = false;
			guard++;
			for (int i = 0; i < rects.Count() && !merged; i++)
			{
				for (int j = i + 1; j < rects.Count() && !merged; j++)
				{
					array<float> a = rects[i];
					array<float> b = rects[j];
					// Cross-extent overlap in both axes >= -gap tolerance?
					float gapX = Math.Max(a[0], b[0]) - Math.Min(a[2], b[2]);
					float gapZ = Math.Max(a[1], b[1]) - Math.Min(a[3], b[3]);
					// One axis must overlap substantially (shared cross-section), the other may
					// have a small gap (continuation along the wall).
					bool overlapX = gapX < -0.04;
					bool overlapZ = gapZ < -0.04;
					bool joinable = (overlapX && gapZ <= 0.15) || (overlapZ && gapX <= 0.15);
					if (!joinable)
						continue;
					ref array<float> u = {};
					u.Insert(Math.Min(a[0], b[0]));
					u.Insert(Math.Min(a[1], b[1]));
					u.Insert(Math.Max(a[2], b[2]));
					u.Insert(Math.Max(a[3], b[3]));
					// Never merge into a blob: the union must stay wall-shaped.
					if (Math.Min(u[2] - u[0], u[3] - u[1]) > WALL_MAX_THICKNESS_M)
						continue;
					rects[i] = u;
					rects.RemoveOrdered(j);
					merged = true;
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Absolute polygon area (shoelace) over footprint points stored as (x, _, z) vectors.
	protected static float ShoelaceArea(array<ref vector> polygon)
	{
		int n = polygon.Count();
		if (n < 3)
			return 0.0;
		float sum = 0.0;
		for (int i = 0; i < n; i++)
		{
			vector p = polygon[i];
			vector q = polygon[(i + 1) % n];
			sum += p[0] * q[2] - q[0] * p[2];
		}
		return Math.AbsFloat(sum * 0.5);
	}

	//------------------------------------------------------------------------------------------------
	//! Recursive exclusion sweep: door leaves, destructible dressing, and FURNITURE (children
	//! carrying their own prefab resource - the POC-placed FarmHouse ships a full interior
	//! composition: radiators, wardrobes, crates...) out of the scan traces. Furniture entities are
	//! returned so the extract can emit them as cover records instead. Public: the voxel dumper
	//! (TBD_BuildingVoxelDump) applies the identical exclusion set.
	static void ExcludeDressing(TBD_BuildingTraceScanner scanner, IEntity ent,
		inout int doorCount, inout int glassCount, array<IEntity> furnitureOut)
	{
		IEntity child = ent.GetChildren();
		while (child)
		{
			if (TBD_BuildingTraceScanner.HasComponentOfClass(child, "DoorComponent"))
			{
				scanner.ExcludeEntity(child);
				doorCount++;
			}
			else if (TBD_BuildingTraceScanner.HasComponentOfClass(child, "SCR_DestructionMultiPhaseComponent"))
			{
				scanner.ExcludeEntity(child);
				glassCount++;
			}
			else
			{
				string rn = TBD_MapExportContext.GetEntityResourceName(child);
				if (!rn.IsEmpty())
				{
					// Own-prefab child = placed prop, not base-mesh architecture.
					scanner.ExcludeEntity(child);
					furnitureOut.Insert(child);
				}
			}
			ExcludeDressing(scanner, child, doorCount, glassCount, furnitureOut);
			child = child.GetSibling();
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Diagnostic single-segment probe under multiple flag/mask combos (action "probe").
	static string ExecuteProbe(string filter, vector localA, vector localB)
	{
		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
			return "ERROR: context init failed";
		string resName;
		IEntity root = TBD_BuildingTraceScanner.FindTargetByFilter(ctx, filter, resName);
		if (!root)
			return "ERROR: no instance matching '" + filter + "'";
		TBD_BuildingTraceScanner scanner = new TBD_BuildingTraceScanner(ctx.m_World, root);
		string res = scanner.ProbeVariants(localA, localB);
		Print(TAG + " probe " + localA.ToString() + " -> " + localB.ToString() + ": " + res);
		return "OK " + res;
	}

	//------------------------------------------------------------------------------------------------
	//! Glass-only exclusion for the parity oracle (doors stay - closed doors block vision).
	protected static void ExcludeGlassOnly(TBD_BuildingTraceScanner scanner, IEntity ent,
		inout int glassCount)
	{
		IEntity child = ent.GetChildren();
		while (child)
		{
			if (TBD_BuildingTraceScanner.HasComponentOfClass(child, "SCR_DestructionMultiPhaseComponent"))
			{
				scanner.ExcludeEntity(child);
				glassCount++;
			}
			ExcludeGlassOnly(scanner, child, glassCount);
			child = child.GetSibling();
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Parity oracle: N random LOCAL-frame A/B pairs, engine TraceMove verdicts, JSON rows the
	//! Rust side replays through `evaluate_los`.
	static string ExecuteParity(string filter, int samples)
	{
		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
			return "ERROR: context init failed";

		string resName;
		IEntity root = TBD_BuildingTraceScanner.FindTargetByFilter(ctx, filter, resName);
		if (!root)
			return "ERROR: no instance matching '" + filter + "'";
		string slug = TBD_BuildingArchitectExtractor.DerivePrefabSlug(resName);

		vector obMin, obMax;
		root.GetBounds(obMin, obMax);
		TBD_BuildingTraceScanner scanner = new TBD_BuildingTraceScanner(ctx.m_World, root);
		// Parity models VISION: glass panes pass sight (exclude), closed doors block it (keep).
		int glassCount = 0;
		ExcludeGlassOnly(scanner, root, glassCount);
		Print(TAG + string.Format(" parity: %1 destructible children excluded (vision passes glass)",
			glassCount));

		if (samples <= 0)
			samples = 200;

		string json = "{\n  \"slug\": \"" + TBD_MapExportJson.Escape(slug) + "\",\n  \"pairs\": [\n";
		int clearCount = 0;
		for (int i = 0; i < samples; i++)
		{
			float ox = Math.RandomFloat(obMin[0] - 3.0, obMax[0] + 3.0);
			float oz = Math.RandomFloat(obMin[2] - 3.0, obMax[2] + 3.0);
			float oy = Math.RandomFloat(0.4, Math.Max(1.0, obMax[1] * 0.8));
			float tx = Math.RandomFloat(obMin[0] - 3.0, obMax[0] + 3.0);
			float tz = Math.RandomFloat(obMin[2] - 3.0, obMax[2] + 3.0);
			float ty = Math.RandomFloat(0.4, Math.Max(1.0, obMax[1] * 0.8));

			vector hit;
			bool clear = scanner.TraceLocal(Vector(ox, oy, oz), Vector(tx, ty, tz), hit) >= 1.0;
			if (clear)
				clearCount++;

			json += "    [" + ox.ToString() + "," + oy.ToString() + "," + oz.ToString() + ","
				+ tx.ToString() + "," + ty.ToString() + "," + tz.ToString() + ",";
			if (clear)
				json += "true]";
			else
				json += "false]";
			if (i < samples - 1)
				json += ",";
			json += "\n";
		}
		json += "  ]\n}\n";

		string mapName = ctx.GetMapName(null);
		TBD_MapExportConfig cfg = new TBD_MapExportConfig();
		string outPath = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName,
			"prefabs/debug", slug + "_parity.json");
		FileHandle f = FileIO.OpenFile(outPath, FileMode.WRITE);
		if (!f)
			return "ERROR: cannot open " + outPath;
		TBD_MapExportJson.Write(f, json, TAG);
		f.Close();

		string summary = string.Format("OK %1: %2 pairs (%3 clear) -> %4", slug, samples, clearCount, outPath);
		Print(TAG + " " + summary);
		return summary;
	}
}

[WorkbenchPluginAttribute(
	name: "Extract Building Blueprint",
	description: "Trace-scan the first building matching the prefab filter into a blueprint JSON.",
	category: "TBD"
)]
class TBD_BuildingTraceExtractPlugin : WorkbenchPlugin
{
	[Attribute("FarmHouse_E_1L01", UIWidgets.EditBox, desc: "Prefab resource substring to match")]
	string m_sPrefabFilter;

	override void Run()
	{
		Print("[TBD][TraceExtract] " + TBD_BuildingTraceExtract.Execute(m_sPrefabFilter));
	}
}
