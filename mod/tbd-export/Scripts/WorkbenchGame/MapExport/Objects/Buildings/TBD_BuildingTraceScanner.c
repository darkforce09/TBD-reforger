/**
 * TBD_BuildingTraceScanner.c
 *
 * Phase-B ground truth: derives a building's architectural blueprint by RAYCASTING its collision
 * geometry (LayerMask = EPhysicsLayerPresets.Projectile -- the surface game LOS raycasts hit, so
 * the blueprint is faithful to gunfire/vision blocking BY CONSTRUCTION, wherever the visual mesh
 * and the fire geometry disagree).
 *
 * All traces run in WORLD space but the scan grid is laid out in the building's LOCAL frame
 * (inverse root transform), so a rotated instance produces the same blueprint as an axis-aligned
 * one. A whitelist trace filter accepts ONLY the target root + its non-furniture descendants:
 * neighboring world objects and interior furniture (operator-flagged) can never pollute a wall.
 *
 * Passes:
 *   1. Vertical march (0.25 m XZ grid): top-surface heights -> eave/ridge/chimney; downward
 *      hit histogram -> floor-slab elevations -> level bands (replaces every hardcoded height).
 *   2. Per-band horizontal march (0.10 m rows, both directions): entry faces from each side pair
 *      into solid intervals -> occupancy grid -> greedy rect decomposition -> walls with MEASURED
 *      thickness; per-row extremes -> the level's footprint outline (axis-aligned polygons).
 *   3. Aperture scan: across every wall rect, a stack of short perpendicular rays (0.10 m along,
 *      0.10 m height steps) -> open cells -> grouped into doors (sill < 0.3 m) and windows with
 *      MEASURED sill/height.
 *   4. Parity sampling: random observer/target pairs vs the same collision world -- the offline
 *      oracle for `evaluate_los` agreement.
 *
 * Driven via `EMCP_WB_TbdBlueprint` (Net API) or the Workbench menu plugin below.
 */

class TBD_BuildingTraceScanner
{
	protected static const string TAG = "[TBD][TraceScan]";
	protected static const float STEP_PAST_M = 0.02;
	protected static const float ROW_STEP_M = 0.10;
	protected static const float COL_STEP_M = 0.25;
	protected static const float APERTURE_STEP_M = 0.10;
	protected static const int MAX_MARCH_HITS = 48;

	BaseWorld m_World;
	IEntity m_Root;
	protected vector m_aMat[4];
	protected ref array<IEntity> m_aExcluded;

	// Local-frame scan bounds.
	vector m_vMin;
	vector m_vMax;

	//------------------------------------------------------------------------------------------------
	void TBD_BuildingTraceScanner(BaseWorld world, IEntity root)
	{
		m_World = world;
		m_Root = root;
		m_aExcluded = {};
		root.GetWorldTransform(m_aMat);
		root.GetBounds(m_vMin, m_vMax);
		// Pad so exterior rays start genuinely outside the collision hull.
		m_vMin = m_vMin - Vector(0.6, 0.6, 0.6);
		m_vMax = m_vMax + Vector(0.6, 1.2, 0.6);
	}

	//------------------------------------------------------------------------------------------------
	void ExcludeEntity(IEntity e)
	{
		if (e)
			m_aExcluded.Insert(e);
	}

	//------------------------------------------------------------------------------------------------
	//! Target resolution, SELECTION FIRST: an operator-selected matching entity wins (the Eden
	//! subscene means a filter sweep finds the northernmost Everon instance, not the one the
	//! operator is looking at); falls back to the world sweep.
	static IEntity FindTargetByFilter(TBD_MapExportContext ctx, string filter, out string resName)
	{
		resName = "";
		int selCount = ctx.m_API.GetSelectedEntitiesCount();
		for (int i = 0; i < selCount; i++)
		{
			IEntitySource src = ctx.m_API.GetSelectedEntity(i);
			if (!src)
				continue;
			IEntity ent = ctx.m_API.SourceToEntity(src);
			if (!ent)
				continue;
			string rn = ctx.ResolvePrefab(ent);
			if (!rn.IsEmpty() && (filter.IsEmpty() || rn.Contains(filter)))
			{
				resName = rn;
				return ent;
			}
		}

		TBD_ScanTargetFinder finder = new TBD_ScanTargetFinder();
		return finder.Find(ctx, filter, resName);
	}

	//------------------------------------------------------------------------------------------------
	//! Diagnostic: one segment traced under several flag/mask combinations -- pins down WHY a
	//! surface does or does not register (editor-mode physics layers are not a documented API).
	string ProbeVariants(vector localA, vector localB)
	{
		string outStr = "";
		outStr += "entsProj=" + ProbeOne(localA, localB, TraceFlags.ENTS, true).ToString();
		outStr += " entsWorldProj=" + ProbeOne(localA, localB, TraceFlags.ENTS | TraceFlags.WORLD, true).ToString();
		outStr += " entsNoMask=" + ProbeOne(localA, localB, TraceFlags.ENTS, false).ToString();
		outStr += " entsWorldNoMask=" + ProbeOne(localA, localB, TraceFlags.ENTS | TraceFlags.WORLD, false).ToString();
		outStr += " anyContact=" + ProbeOne(localA, localB, TraceFlags.ENTS | TraceFlags.ANY_CONTACT, true).ToString();
		outStr += " defaultFlags=" + ProbeOne(localA, localB, TraceFlags.DEFAULT, false).ToString();
		return outStr;
	}

	protected float ProbeOne(vector localA, vector localB, TraceFlags flags, bool useMask)
	{
		TraceParam param = new TraceParam();
		param.Start = LocalToWorld(localA);
		param.End = LocalToWorld(localB);
		param.Flags = flags;
		if (useMask)
			param.LayerMask = EPhysicsLayerPresets.Projectile;
		return m_World.TraceMove(param, TraceFilter);
	}

	//------------------------------------------------------------------------------------------------
	//! Does the entity's SOURCE carry a component of this class? (Recon proved the FarmHouse
	//! children are all INLINE -- no prefab resources -- so classification is component-only.)
	static bool HasComponentOfClass(IEntity ent, string className)
	{
		WorldEditor we = Workbench.GetModule(WorldEditor);
		if (!we)
			return false;
		WorldEditorAPI api = we.GetApi();
		if (!api)
			return false;
		IEntitySource src = api.EntityToSource(ent);
		if (!src)
			return false;
		int n = src.GetComponentCount();
		for (int i = 0; i < n; i++)
		{
			IEntityComponentSource comp = src.GetComponent(i);
			if (comp && comp.GetClassName() == className)
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	vector LocalToWorld(vector local)
	{
		return m_aMat[3]
			+ m_aMat[0] * local[0]
			+ m_aMat[1] * local[1]
			+ m_aMat[2] * local[2];
	}

	//------------------------------------------------------------------------------------------------
	vector WorldToLocal(vector world)
	{
		vector d = world - m_aMat[3];
		return Vector(
			vector.Dot(d, m_aMat[0]),
			vector.Dot(d, m_aMat[1]),
			vector.Dot(d, m_aMat[2])
		);
	}

	//------------------------------------------------------------------------------------------------
	//! Whitelist: only the target building's own non-furniture geometry blocks scan rays.
	//! Returning false tells the trace to IGNORE the entity (the vanilla FilterCallback contract).
	protected bool TraceFilter(IEntity e)
	{
		if (!e)
			return false;
		IEntity walk = e;
		int guard = 0;
		while (walk && guard < 16)
		{
			foreach (IEntity ex : m_aExcluded)
			{
				if (walk == ex)
					return false;
			}
			if (walk == m_Root)
				return true;
			walk = walk.GetParent();
			guard++;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Single filtered trace between LOCAL points; returns hit fraction (1 = clear) and hit pos.
	float TraceLocal(vector localA, vector localB, out vector localHit)
	{
		TraceParam param = new TraceParam();
		param.Start = LocalToWorld(localA);
		param.End = LocalToWorld(localB);
		param.Flags = TraceFlags.ENTS;
		param.LayerMask = EPhysicsLayerPresets.Projectile;
		float frac = m_World.TraceMove(param, TraceFilter);
		if (frac < 1.0)
		{
			vector w = param.Start + (param.End - param.Start) * frac;
			localHit = WorldToLocal(w);
		}
		else
		{
			localHit = localB;
		}
		return frac;
	}

	//------------------------------------------------------------------------------------------------
	//! March a segment collecting every ENTRY face along it (first-hit semantics, re-cast past
	//! each hit). Returns the parametric axis value (the coordinate along `axis`) of each entry.
	void MarchEntries(vector localA, vector localB, out array<float> entries)
	{
		entries = {};
		vector cur = localA;
		vector dir = localB - localA;
		float total = dir.Length();
		if (total < 0.001)
			return;
		dir = dir * (1.0 / total);

		int guard = 0;
		while (guard < MAX_MARCH_HITS)
		{
			vector hit;
			float frac = TraceLocal(cur, localB, hit);
			if (frac >= 1.0)
				break;
			// Distance from the ORIGINAL start along the axis of travel.
			float along = vector.Dot(hit - localA, dir);
			entries.Insert(along);
			float remaining = vector.Dot(localB - hit, dir);
			if (remaining <= STEP_PAST_M * 2)
				break;
			cur = hit + dir * STEP_PAST_M;
			guard++;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Pass 1 -- vertical: top surfaces + downward hit histogram -> bands + roof numbers.
	void ScanVertical(out array<float> slabYs, out float eaveY, out float ridgeY, out float chimneyY)
	{
		slabYs = {};
		ridgeY = m_vMin[1];
		chimneyY = m_vMin[1];

		int nx = Math.Ceil((m_vMax[0] - m_vMin[0]) / COL_STEP_M);
		int nz = Math.Ceil((m_vMax[2] - m_vMin[2]) / COL_STEP_M);

		// Histogram of downward-facing hit ys, 0.1 m bins over the local vertical range.
		float binSize = 0.1;
		int nBins = Math.Ceil((m_vMax[1] - m_vMin[1]) / binSize) + 1;
		ref array<int> bins = {};
		for (int b = 0; b < nBins; b++)
			bins.Insert(0);

		ref array<float> topYs = {};
		int columns = 0;
		int columnsHit = 0;

		for (int ix = 0; ix < nx; ix++)
		{
			for (int iz = 0; iz < nz; iz++)
			{
				float x = m_vMin[0] + (ix + 0.5) * COL_STEP_M;
				float z = m_vMin[2] + (iz + 0.5) * COL_STEP_M;
				columns++;

				array<float> entries;
				MarchEntries(Vector(x, m_vMax[1], z), Vector(x, m_vMin[1], z), entries);
				if (entries.Count() == 0)
					continue;
				columnsHit++;

				// First entry from above = top surface (roof/chimney).
				float topY = m_vMax[1] - entries[0];
				topYs.Insert(topY);
				if (topY > ridgeY)
					chimneyY = topY;

				foreach (float along : entries)
				{
					float y = m_vMax[1] - along;
					int bin = (y - m_vMin[1]) / binSize;
					if (bin >= 0 && bin < nBins)
						bins[bin] = bins[bin] + 1;
				}
			}
		}

		if (columnsHit == 0)
		{
			eaveY = m_vMin[1];
			return;
		}

		// Ridge: p95 of top surfaces (chimney spike excluded); eave: p20.
		SortFloats(topYs);
		int p95 = topYs.Count() * 95 / 100;
		if (p95 > topYs.Count() - 1)
			p95 = topYs.Count() - 1;
		int p20 = topYs.Count() * 20 / 100;
		if (p20 > topYs.Count() - 1)
			p20 = topYs.Count() - 1;
		ridgeY = topYs[p95];
		eaveY = topYs[p20];
		if (chimneyY < ridgeY + 0.3)
			chimneyY = ridgeY; // no distinct chimney spike

		// Slabs: histogram peaks with support >= 15% of hit columns, >= 1.8 m apart (bins hold
		// horizontal surfaces: floor plates, landings -- walls contribute almost nothing to
		// DOWNWARD entry faces).
		int support = columnsHit * 15 / 100;
		if (support < 4)
			support = 4;
		int lastSlabBin = -1000;
		for (int b2 = 0; b2 < nBins; b2++)
		{
			if (bins[b2] < support)
				continue;
			// Local maximum over +/-2 bins.
			bool isPeak = true;
			for (int w = -2; w <= 2; w++)
			{
				int nb = b2 + w;
				if (nb >= 0 && nb < nBins && bins[nb] > bins[b2])
					isPeak = false;
			}
			if (!isPeak)
				continue;
			if (b2 - lastSlabBin < 18) // 1.8 m in 0.1 bins
				continue;
			float slabY = m_vMin[1] + b2 * binSize;
			if (slabY > eaveY + 0.5)
				continue; // roof surfaces are not floor slabs
			slabYs.Insert(slabY);
			lastSlabBin = b2;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Pass 2 -- one band's WALL occupancy: bidirectional marches in BOTH plan axes (x-marches see
	//! z-running walls and vice versa), at TWO heights, AND-combined. A wall is vertical -- solid
	//! at both probe heights; the sloped roof plane crossing one scan height is solid at only one
	//! (it moves >= its height-delta horizontally on a <=45? pitch, landing in different cells) --
	//! the AND kills the phantom stripes the roof painted across the FarmHouse's 2nd-floor north.
	void ScanBandOccupancy(float yLow, float yHigh, out array<bool> grid, out int nx, out int nz)
	{
		nx = Math.Ceil((m_vMax[0] - m_vMin[0]) / ROW_STEP_M);
		nz = Math.Ceil((m_vMax[2] - m_vMin[2]) / ROW_STEP_M);
		grid = {};
		ref array<bool> hi = {};
		for (int i = 0; i < nx * nz; i++)
		{
			grid.Insert(false);
			hi.Insert(false);
		}

		ScanAxisX(yLow, grid, nx, nz);
		ScanAxisZ(yLow, grid, nx, nz);
		ScanAxisX(yHigh, hi, nx, nz);
		ScanAxisZ(yHigh, hi, nx, nz);
		for (int j = 0; j < nx * nz; j++)
		{
			if (!hi[j])
				grid[j] = false;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Floor-plate grid for one band: a cell is FLOOR when a short down-ray just above the slab
	//! hits it. This is the level's REAL walkable footprint -- a mezzanine void or a room running
	//! to the ridge has no plate and falls outside the level polygon (operator decision).
	void ScanFloorPlate(float slabY, out array<bool> grid, out int nx, out int nz)
	{
		nx = Math.Ceil((m_vMax[0] - m_vMin[0]) / ROW_STEP_M);
		nz = Math.Ceil((m_vMax[2] - m_vMin[2]) / ROW_STEP_M);
		grid = {};
		for (int i = 0; i < nx * nz; i++)
			grid.Insert(false);

		for (int ix = 0; ix < nx; ix++)
		{
			for (int iz = 0; iz < nz; iz++)
			{
				float x = m_vMin[0] + (ix + 0.5) * ROW_STEP_M;
				float z = m_vMin[2] + (iz + 0.5) * ROW_STEP_M;
				vector hit;
				if (TraceLocal(Vector(x, slabY + 0.4, z), Vector(x, slabY - 0.35, z), hit) < 1.0)
					grid[ix * nz + iz] = true;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void ScanAxisX(float y, array<bool> grid, int nx, int nz)
	{
		for (int iz = 0; iz < nz; iz++)
		{
			float z = m_vMin[2] + (iz + 0.5) * ROW_STEP_M;
			array<float> fwd;
			array<float> bwd;
			MarchEntries(Vector(m_vMin[0], y, z), Vector(m_vMax[0], y, z), fwd);
			MarchEntries(Vector(m_vMax[0], y, z), Vector(m_vMin[0], y, z), bwd);

			// Backward entries are the OPPOSING faces in forward axis coords. Collision meshes
			// are one-sided (live scan: ordered pairing left 118 occupied cells on a whole
			// farmhouse), so faces are matched by PROXIMITY: each forward face pairs with the
			// nearest opposing face within one wall-thickness ahead; an unmatched face marks
			// its own cell (one-sided sliver).
			float spanX = m_vMax[0] - m_vMin[0];
			ref array<float> opposing = {};
			for (int i2 = bwd.Count() - 1; i2 >= 0; i2--)
				opposing.Insert(spanX - bwd[i2]);

			for (int k = 0; k < fwd.Count(); k++)
			{
				float x0 = fwd[k];
				float x1 = -1;
				for (int m = 0; m < opposing.Count(); m++)
				{
					float b = opposing[m];
					if (b > x0 - 0.05 && b - x0 <= 0.7)
					{
						if (x1 < 0 || b < x1)
							x1 = b;
					}
				}
				if (x1 < 0)
					x1 = x0 + ROW_STEP_M * 0.9; // unmatched one-sided face
				int c0 = x0 / ROW_STEP_M;
				int c1 = x1 / ROW_STEP_M;
				for (int c = c0; c <= c1 && c < nx; c++)
				{
					if (c >= 0)
						grid[c * nz + iz] = true;
				}
			}
			// Opposing faces with no forward partner (wall seen only from the far side).
			for (int m2 = 0; m2 < opposing.Count(); m2++)
			{
				float b2 = opposing[m2];
				bool matched = false;
				for (int k2 = 0; k2 < fwd.Count(); k2++)
				{
					if (b2 > fwd[k2] - 0.05 && b2 - fwd[k2] <= 0.7)
					{
						matched = true;
						break;
					}
				}
				if (!matched)
				{
					int cb = b2 / ROW_STEP_M;
					if (cb >= 0 && cb < nx)
						grid[cb * nz + iz] = true;
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Mirror of [`ScanAxisX`]: z-direction marches per x-column, same proximity pairing.
	protected void ScanAxisZ(float y, array<bool> grid, int nx, int nz)
	{
		for (int ix = 0; ix < nx; ix++)
		{
			float x = m_vMin[0] + (ix + 0.5) * ROW_STEP_M;
			array<float> fwd;
			array<float> bwd;
			MarchEntries(Vector(x, y, m_vMin[2]), Vector(x, y, m_vMax[2]), fwd);
			MarchEntries(Vector(x, y, m_vMax[2]), Vector(x, y, m_vMin[2]), bwd);

			float spanZ = m_vMax[2] - m_vMin[2];
			ref array<float> opposing = {};
			for (int i2 = bwd.Count() - 1; i2 >= 0; i2--)
				opposing.Insert(spanZ - bwd[i2]);

			for (int k = 0; k < fwd.Count(); k++)
			{
				float z0 = fwd[k];
				float z1 = -1;
				for (int m = 0; m < opposing.Count(); m++)
				{
					float b = opposing[m];
					if (b > z0 - 0.05 && b - z0 <= 0.7)
					{
						if (z1 < 0 || b < z1)
							z1 = b;
					}
				}
				if (z1 < 0)
					z1 = z0 + ROW_STEP_M * 0.9;
				int c0 = z0 / ROW_STEP_M;
				int c1 = z1 / ROW_STEP_M;
				for (int c = c0; c <= c1 && c < nz; c++)
				{
					if (c >= 0)
						grid[ix * nz + c] = true;
				}
			}
			for (int m2 = 0; m2 < opposing.Count(); m2++)
			{
				float b2 = opposing[m2];
				bool matched = false;
				for (int k2 = 0; k2 < fwd.Count(); k2++)
				{
					if (b2 > fwd[k2] - 0.05 && b2 - fwd[k2] <= 0.7)
					{
						matched = true;
						break;
					}
				}
				if (!matched)
				{
					int cb = b2 / ROW_STEP_M;
					if (cb >= 0 && cb < nz)
						grid[ix * nz + cb] = true;
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Greedy maximal-rect decomposition of the occupancy grid -> wall/mass rects in local meters.
	//! Each rect: [minX, minZ, maxX, maxZ].
	void RectsFromGrid(array<bool> grid, int nx, int nz, out array<ref array<float>> rects)
	{
		rects = {};
		ref array<bool> used = {};
		for (int i = 0; i < nx * nz; i++)
			used.Insert(false);

		for (int ix = 0; ix < nx; ix++)
		{
			for (int iz = 0; iz < nz; iz++)
			{
				int idx = ix * nz + iz;
				if (!grid[idx] || used[idx])
					continue;

				// Grow in x first, then z, keeping the rectangle solid.
				int endX = ix;
				while (endX + 1 < nx && grid[(endX + 1) * nz + iz] && !used[(endX + 1) * nz + iz])
					endX++;

				int endZ = iz;
				bool grow = true;
				while (grow && endZ + 1 < nz)
				{
					for (int cx = ix; cx <= endX; cx++)
					{
						int cidx = cx * nz + endZ + 1;
						if (!grid[cidx] || used[cidx])
						{
							grow = false;
							break;
						}
					}
					if (grow)
						endZ++;
				}

				for (int cx2 = ix; cx2 <= endX; cx2++)
				{
					for (int cz2 = iz; cz2 <= endZ; cz2++)
						used[cx2 * nz + cz2] = true;
				}

				ref array<float> r = {};
				r.Insert(m_vMin[0] + ix * ROW_STEP_M);
				r.Insert(m_vMin[2] + iz * ROW_STEP_M);
				r.Insert(m_vMin[0] + (endX + 1) * ROW_STEP_M);
				r.Insert(m_vMin[2] + (endZ + 1) * ROW_STEP_M);
				rects.Insert(r);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Axis-aligned outline from per-row solid extremes (handles L/T/U shapes, no holes):
	//! right edge walks per-row maxX transitions top->bottom, left edge walks minX bottom->top.
	//! HYSTERESIS: extreme changes of <= 1 cell are collision-jitter, not corners -- a corner needs
	//! a >= 2-cell jump, which keeps the FarmHouse outline at a handful of points instead of 251.
	void OutlineFromGrid(array<bool> grid, int nx, int nz, out array<ref array<float>> polygon)
	{
		polygon = {};
		ref array<int> rowMin = {};
		ref array<int> rowMax = {};
		for (int iz = 0; iz < nz; iz++)
		{
			int mn = -1;
			int mx = -1;
			for (int ix = 0; ix < nx; ix++)
			{
				if (grid[ix * nz + iz])
				{
					if (mn < 0)
						mn = ix;
					mx = ix;
				}
			}
			rowMin.Insert(mn);
			rowMax.Insert(mx);
		}

		// Right side, increasing z.
		int prevMax = -1000;
		float lastZ = 0;
		for (int z1 = 0; z1 < nz; z1++)
		{
			if (rowMax[z1] < 0)
				continue;
			int dm = rowMax[z1] - prevMax;
			if (dm < 0)
				dm = -dm;
			if (dm >= 2 || prevMax == -1000)
			{
				float xr = m_vMin[0] + (rowMax[z1] + 1) * ROW_STEP_M;
				float zz = m_vMin[2] + z1 * ROW_STEP_M;
				PushPoint(polygon, xr, zz);
				prevMax = rowMax[z1];
			}
			lastZ = m_vMin[2] + (z1 + 1) * ROW_STEP_M;
		}
		// Close the right side at the far corner.
		if (prevMax != -1000)
			PushPoint(polygon, m_vMin[0] + (prevMax + 1) * ROW_STEP_M, lastZ);

		// Left side, decreasing z.
		int prevMin = -1000;
		float firstZ = 0;
		for (int z2 = nz - 1; z2 >= 0; z2--)
		{
			if (rowMin[z2] < 0)
				continue;
			int dn = rowMin[z2] - prevMin;
			if (dn < 0)
				dn = -dn;
			if (dn >= 2 || prevMin == -1000)
			{
				float xl = m_vMin[0] + rowMin[z2] * ROW_STEP_M;
				float zz2 = m_vMin[2] + (z2 + 1) * ROW_STEP_M;
				PushPoint(polygon, xl, zz2);
				prevMin = rowMin[z2];
			}
			firstZ = m_vMin[2] + z2 * ROW_STEP_M;
		}
		if (prevMin != -1000)
			PushPoint(polygon, m_vMin[0] + prevMin * ROW_STEP_M, firstZ);
	}

	//------------------------------------------------------------------------------------------------
	protected void PushPoint(array<ref array<float>> polygon, float x, float z)
	{
		ref array<float> p = {};
		p.Insert(x);
		p.Insert(z);
		polygon.Insert(p);
	}

	//------------------------------------------------------------------------------------------------
	//! Pass 3 -- aperture scan across one wall rect within a band. Emits (alongPos, openLowY,
	//! openHighY, alongLen) groups found by probing short perpendicular rays on a 0.10 m lattice.
	void ScanApertures(array<float> rect, float bandMin, float bandMax, bool alongX,
		out array<ref array<float>> apertures)
	{
		apertures = {};
		float alongMin, alongMax, crossMid, crossHalf;
		if (alongX)
		{
			alongMin = rect[0];
			alongMax = rect[2];
			crossMid = (rect[1] + rect[3]) * 0.5;
			crossHalf = (rect[3] - rect[1]) * 0.5 + 0.25;
		}
		else
		{
			alongMin = rect[1];
			alongMax = rect[3];
			crossMid = (rect[0] + rect[2]) * 0.5;
			crossHalf = (rect[2] - rect[0]) * 0.5 + 0.25;
		}

		int nAlong = Math.Ceil((alongMax - alongMin) / APERTURE_STEP_M);
		float yLow = bandMin + 0.05;
		float yHigh = bandMax - 0.05;
		int nY = Math.Ceil((yHigh - yLow) / APERTURE_STEP_M);
		if (nAlong <= 0 || nY <= 0)
			return;

		// Open matrix.
		ref array<bool> open = {};
		for (int i = 0; i < nAlong * nY; i++)
			open.Insert(false);

		for (int ia = 0; ia < nAlong; ia++)
		{
			float along = alongMin + (ia + 0.5) * APERTURE_STEP_M;
			for (int iy = 0; iy < nY; iy++)
			{
				float y = yLow + (iy + 0.5) * APERTURE_STEP_M;
				vector a, b;
				if (alongX)
				{
					a = Vector(along, y, crossMid - crossHalf);
					b = Vector(along, y, crossMid + crossHalf);
				}
				else
				{
					a = Vector(crossMid - crossHalf, y, along);
					b = Vector(crossMid + crossHalf, y, along);
				}
				// One-sided collision: a face can be invisible from one approach -- probe both
				// directions, open only when BOTH pass.
				vector hit;
				if (TraceLocal(a, b, hit) >= 1.0 && TraceLocal(b, a, hit) >= 1.0)
					open[ia * nY + iy] = true;
			}
		}

		// Group consecutive LINTEL-BOUNDED open columns into apertures. A real door/window has
		// solid wall ABOVE its opening; a column whose open run reaches the scan-band top is a
		// missing/low wall (open porch, attic knee-wall under the sloped roof) -- those produced
		// 59 fake attic "windows" and an 11.8 m "door" on the first live scan. The run used per
		// column is the LOWEST one, so a transom pane splitting an opening still reads as one.
		int runStart = -1;
		float runLow = 0;
		float runHigh = 0;
		for (int ia2 = 0; ia2 <= nAlong; ia2++)
		{
			int lowIy = -1;
			int highIy = -1;
			if (ia2 < nAlong)
			{
				// Lowest contiguous open run.
				for (int iy2 = 0; iy2 < nY; iy2++)
				{
					if (open[ia2 * nY + iy2])
					{
						lowIy = iy2;
						highIy = iy2;
						while (highIy + 1 < nY && open[ia2 * nY + highIy + 1])
							highIy++;
						break;
					}
				}
				// Lintel rule: solid above the run, inside the band.
				if (highIy >= nY - 1)
				{
					lowIy = -1;
					highIy = -1;
				}
			}
			bool colOpen = lowIy >= 0;
			if (colOpen)
			{
				float lo = yLow + lowIy * APERTURE_STEP_M;
				float hi = yLow + (highIy + 1) * APERTURE_STEP_M;
				if (runStart < 0)
				{
					runStart = ia2;
					runLow = lo;
					runHigh = hi;
				}
				else
				{
					if (lo < runLow)
						runLow = lo;
					if (hi > runHigh)
						runHigh = hi;
				}
			}
			else if (runStart >= 0)
			{
				ref array<float> ap = {};
				ap.Insert(alongMin + runStart * APERTURE_STEP_M); // start along
				ap.Insert(runLow);
				ap.Insert(runHigh);
				ap.Insert((ia2 - runStart) * APERTURE_STEP_M); // length
				apertures.Insert(ap);
				runStart = -1;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void SortFloats(array<float> a)
	{
		// Insertion sort -- arrays here are small (<= a few thousand).
		for (int i = 1; i < a.Count(); i++)
		{
			float v = a[i];
			int j = i - 1;
			while (j >= 0 && a[j] > v)
			{
				a[j + 1] = a[j];
				j--;
			}
			a[j + 1] = v;
		}
	}
}

//! QueryEntitiesByAABB needs an instance method callback -- tiny finder object.
class TBD_ScanTargetFinder
{
	protected ref array<IEntity> m_aHits;

	protected bool CollectEntity(IEntity e)
	{
		if (e)
			m_aHits.Insert(e);
		return true;
	}

	IEntity Find(TBD_MapExportContext ctx, string filter, out string resName)
	{
		resName = "";
		float worldSize = ctx.m_fWorldSize;
		float cellM = 512.0;
		int cells = Math.Ceil(worldSize / cellM);
		for (int iz = 0; iz < cells; iz++)
		{
			for (int ix = 0; ix < cells; ix++)
			{
				m_aHits = {};
				vector mins = Vector(ix * cellM, -1000.0, iz * cellM);
				vector maxs = Vector(ix * cellM + cellM, 2000.0, iz * cellM + cellM);
				ctx.m_World.QueryEntitiesByAABB(mins, maxs, CollectEntity);
				foreach (IEntity e : m_aHits)
				{
					string rn = ctx.ResolvePrefab(e);
					if (!rn.IsEmpty() && rn.Contains(filter))
					{
						resName = rn;
						return e;
					}
				}
			}
		}
		return null;
	}
}
