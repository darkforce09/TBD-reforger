/**
 * TBD_WorldTraceParity.c - the cell-scoped LOS oracle (T-090.12.4).
 *
 * `EMCP_WB_TbdBlueprint` action `world-parity` (cx, cy, maxEntities = samples, seed): N seeded
 * observer/target pairs inside ONE 512 m chunk cell, in ABSOLUTE world coordinates (engine frame
 * x, y up, z), each traced twice on the Projectile layer with NO whitelist and nothing excluded:
 *
 *   clearEnts   TraceFlags.ENTS               every entity's colliders (glass, foliage included)
 *   clearWorld  TraceFlags.ENTS | WORLD       the same plus the terrain
 *
 * plus the prefab slug of the ENTS hit (empty when clear). Glass / foliage semantics are resolved
 * OFFLINE by `cargo xtask map world-los --pairs ... [--glass-blocks] [--foliage-blocks]`, so the
 * oracle never guesses what the engine's layer preset does with a pane or a canopy.
 *
 * Strata (pair i uses stratum i % 4):
 *   0 uniform-eye   both ends 1.4-2.0 m above the ground, 10-300 m apart, random bearing
 *   1 entity        both ends inside a random cell entity's padded world bounds (the T-090.11
 *                   per-entity method, entity chosen at random)
 *   2 long          eye height, 300-500 m apart
 *   3 elevated      0.4-8 m above the ground, 10-200 m apart
 *
 * Output: $profile:TBD_Export/<map>/prefabs/debug/world_parity_<cx>_<cy>.json
 *   { version, cell, seed, samples, entityPool, strata, layer, pairs:
 *     [[ox,oy,oz,tx,ty,tz,clearEnts,clearWorld,"hitSlug"], ...], clearEnts, clearWorld }
 */
class TBD_WorldTraceParity
{
	protected static const float CELL_M = 512.0;
	protected static const string VERSION = "world-parity-1";
	protected static const string TAG = "[TBD][WorldParity]";

	protected ref array<IEntity> m_aHits;
	protected BaseWorld m_World;

	//------------------------------------------------------------------------------------------------
	protected bool CollectEntity(IEntity e)
	{
		if (e)
			m_aHits.Insert(e);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! One engine trace between absolute points; true when nothing on the Projectile layer stands
	//! between them. `withWorld` adds the terrain. `hitEnt` receives the entity hit.
	protected bool TraceClear(vector a, vector b, bool withWorld, out IEntity hitEnt)
	{
		TraceParam param = new TraceParam();
		param.Start = a;
		param.End = b;
		if (withWorld)
			param.Flags = TraceFlags.ENTS | TraceFlags.WORLD;
		else
			param.Flags = TraceFlags.ENTS;
		param.LayerMask = EPhysicsLayerPresets.Projectile;
		float frac = m_World.TraceMove(param, null);
		hitEnt = param.TraceEnt;
		return frac >= 1.0;
	}

	//------------------------------------------------------------------------------------------------
	protected float GroundY(float x, float z)
	{
		return m_World.GetSurfaceY(x, z);
	}

	//------------------------------------------------------------------------------------------------
	//! A point `dist` metres from `from` along a random bearing, clamped to the terrain.
	protected vector Bearing(vector from, float dist, float worldSize)
	{
		float ang = Math.RandomFloat(0.0, Math.PI2);
		float x = from[0] + dist * Math.Cos(ang);
		float z = from[2] + dist * Math.Sin(ang);
		if (x < 1.0) x = 1.0;
		if (z < 1.0) z = 1.0;
		if (x > worldSize - 1.0) x = worldSize - 1.0;
		if (z > worldSize - 1.0) z = worldSize - 1.0;
		return Vector(x, 0, z);
	}

	//------------------------------------------------------------------------------------------------
	static string Execute(int cx, int cy, int samples, int seed)
	{
		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
			return "ERROR: context init failed";
		if (samples <= 0)
			samples = 2000;
		if (cx < 0 || cy < 0)
			return "ERROR: cell must be non-negative (cx, cy)";

		TBD_WorldTraceParity self = new TBD_WorldTraceParity();
		self.m_World = ctx.m_World;
		float worldSize = ctx.m_fWorldSize;
		float x0 = cx * CELL_M;
		float z0 = cy * CELL_M;
		if (x0 >= worldSize || z0 >= worldSize)
			return "ERROR: cell outside the terrain";

		// Every entity whose bounds touch the cell (stratum 1's entity pool).
		self.m_aHits = {};
		self.m_World.QueryEntitiesByAABB(Vector(x0, -1000.0, z0), Vector(x0 + CELL_M, 2000.0, z0 + CELL_M), self.CollectEntity);
		int poolSize = self.m_aHits.Count();

		Math.Randomize(seed);

		string json = "{\n  \"version\": \"" + VERSION + "\",\n";
		json += "  \"cell\": [" + cx.ToString() + ", " + cy.ToString() + "],\n";
		json += "  \"seed\": " + seed.ToString() + ",\n";
		json += "  \"samples\": " + samples.ToString() + ",\n";
		json += "  \"entityPool\": " + poolSize.ToString() + ",\n";
		json += "  \"strata\": [\"uniform-eye\", \"entity\", \"long\", \"elevated\"],\n";
		json += "  \"layer\": \"EPhysicsLayerPresets.Projectile\",\n";
		json += "  \"pairs\": [\n";

		int clearEntsCount = 0;
		int clearWorldCount = 0;
		string buf = "";
		for (int i = 0; i < samples; i++)
		{
			int stratum = i % 4;
			vector a;
			vector b;
			if (stratum == 1 && poolSize > 0)
			{
				IEntity e = self.m_aHits[Math.RandomInt(0, poolSize)];
				vector bmin;
				vector bmax;
				e.GetWorldBounds(bmin, bmax);
				float height = bmax[1] - bmin[1];
				if (height < 1.0)
					height = 1.0;
				a = Vector(Math.RandomFloat(bmin[0] - 3.0, bmax[0] + 3.0), Math.RandomFloat(bmin[1] + 0.4, bmin[1] + height * 0.8), Math.RandomFloat(bmin[2] - 3.0, bmax[2] + 3.0));
				b = Vector(Math.RandomFloat(bmin[0] - 3.0, bmax[0] + 3.0), Math.RandomFloat(bmin[1] + 0.4, bmin[1] + height * 0.8), Math.RandomFloat(bmin[2] - 3.0, bmax[2] + 3.0));
			}
			else
			{
				float ax = Math.RandomFloat(x0, x0 + CELL_M);
				float az = Math.RandomFloat(z0, z0 + CELL_M);
				float dist;
				float lo;
				float hi;
				if (stratum == 2)
				{
					dist = Math.RandomFloat(300.0, 500.0);
					lo = 1.4;
					hi = 2.0;
				}
				else if (stratum == 3)
				{
					dist = Math.RandomFloat(10.0, 200.0);
					lo = 0.4;
					hi = 8.0;
				}
				else
				{
					dist = Math.RandomFloat(10.0, 300.0);
					lo = 1.4;
					hi = 2.0;
				}
				vector bxz = self.Bearing(Vector(ax, 0, az), dist, worldSize);
				a = Vector(ax, self.GroundY(ax, az) + Math.RandomFloat(lo, hi), az);
				b = Vector(bxz[0], self.GroundY(bxz[0], bxz[2]) + Math.RandomFloat(lo, hi), bxz[2]);
			}

			IEntity hitEnt;
			bool clearEnts = self.TraceClear(a, b, false, hitEnt);
			IEntity ignored;
			bool clearWorld = self.TraceClear(a, b, true, ignored);
			if (clearEnts)
				clearEntsCount++;
			if (clearWorld)
				clearWorldCount++;
			string slug = "";
			if (!clearEnts && hitEnt)
			{
				string rn = ctx.ResolvePrefab(hitEnt);
				if (rn != "")
					slug = TBD_BuildingArchitectExtractor.DerivePrefabSlug(rn);
				else
					slug = hitEnt.ClassName();
			}

			buf += "    [" + a[0].ToString() + "," + a[1].ToString() + "," + a[2].ToString() + ","
				+ b[0].ToString() + "," + b[1].ToString() + "," + b[2].ToString() + ",";
			if (clearEnts)
				buf += "true,";
			else
				buf += "false,";
			if (clearWorld)
				buf += "true,";
			else
				buf += "false,";
			buf += "\"" + TBD_MapExportJson.Escape(slug) + "\"]";
			if (i < samples - 1)
				buf += ",";
			buf += "\n";
			if (buf.Length() > 8000)
			{
				json += buf;
				buf = "";
			}
		}
		json += buf;
		json += "  ],\n";
		json += "  \"clearEnts\": " + clearEntsCount.ToString() + ",\n";
		json += "  \"clearWorld\": " + clearWorldCount.ToString() + "\n";
		json += "}\n";

		string mapName = ctx.GetMapName(null);
		TBD_MapExportConfig cfg = new TBD_MapExportConfig();
		string outPath = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName,
			"prefabs/debug", "world_parity_" + cx.ToString() + "_" + cy.ToString() + ".json");
		FileHandle f = FileIO.OpenFile(outPath, FileMode.WRITE);
		if (!f)
			return "ERROR: cannot open " + outPath;
		TBD_MapExportJson.Write(f, json, TAG);
		f.Close();

		string summary = string.Format("OK %1 cell (%2,%3) seed %4: %5 pairs (clearEnts %6, clearWorld %7, entity pool %8) -> %9",
			VERSION, cx, cy, seed, samples, clearEntsCount, clearWorldCount, poolSize, outPath);
		Print(TAG + " " + summary);
		return summary;
	}
}
