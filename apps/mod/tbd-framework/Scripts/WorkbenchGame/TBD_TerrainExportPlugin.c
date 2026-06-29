/**
 * TBD_TerrainExportPlugin.c - T-091.0 Everon DEM heightmap export.
 *
 * Manual World Editor "Export Height Map" is dead on vanilla Everon (packed-world
 * lock: GenericTerrainEntity is grey/unselectable, Terrain Tool disabled), and the
 * Enfusion API exposes no bulk heightmap raster read (api_search: only
 * WorldEditorAPI.GetTerrainSurfaceY point reads + WorldEditor.GetTerrainBounds).
 *
 * This plugin resamples WorldEditorAPI.GetTerrainSurfaceY(x,z) ("Very fast method")
 * over a W x H grid that exactly matches the verify sampler's worldToPixel inverse
 *   worldX = px * (WORLD / (W-1)),  worldZ = py * (WORLD / (H-1))
 * and writes an ASCII uint16 raster (rows of space-separated values) plus a meta
 * JSON to $profile:. A Node post-process (raw-u16-to-dem-png.mjs) packs the raster
 * into the committed 16-bit grayscale PNG.
 *
 * Encoding is uint16-linear over the FIXED Everon range (manifest V4 gate asserts
 * these, so the encoder must use them regardless of sampled extremes):
 *   u16  = round((y - HMIN) / (HMAX - HMIN) * 65535),  clamped [0, 65535]
 *   elevM = HMIN + (u16/65535) * (HMAX - HMIN)
 *
 * GetTerrainSurfaceY is the same call wb_terrain getHeight uses (it returned real
 * Everon elevations live: 157.875 @ 6400,6400), and reads the same heightfield the
 * runtime BaseWorld.GetSurfaceY (T-092 spawn authority) reads.
 *
 * SPIKE gate: with SPIKE = true the plugin only probes the anchor coords + runs a
 * 200x200 timing benchmark and writes _spike.json (no full loop). Confirm plausible
 * elevations, then set SPIKE = false for the full export.
 *
 * Run: Workbench > Plugins > "Export TBD Terrain DEM"
 *   (or NetAPI: wb_execute_action menuPath "Plugins,Export TBD Terrain DEM").
 */

class TBD_TerrainAnchor
{
	string id;
	float x;
	float z;
	float surfaceY;
}

[WorkbenchPluginAttribute(name: "Export TBD Terrain DEM", description: "Resample GetTerrainSurfaceY over the heightmap grid and write a uint16 raster + meta JSON.", category: "TBD")]
class TBD_TerrainExportPlugin : WorkbenchPlugin
{
	// Flip to false for the full export run (after the spike confirms plausible elevations).
	protected static const bool   SPIKE = false;

	protected static const int    W      = 6400;       // heightmap width  (px) = 12800 / 2m
	protected static const int    H      = 6400;       // heightmap height (px)
	protected static const float  WORLD  = 12800.0;    // Everon world extent (m), bounds [0..12800]
	protected static const float  HMIN   = -204.78;    // manifest dem.heightRangeMinM (fixed, V4)
	protected static const float  HMAX   = 375.53;     // manifest dem.heightRangeMaxM (fixed, V4)
	protected static const int    FLUSH  = 8000;       // write-buffer flush threshold (chars)

	protected static const string OUT_RASTER = "$profile:TBD_TerrainExport_heightmap.txt";
	protected static const string OUT_META   = "$profile:TBD_TerrainExport_meta.json";
	protected static const string OUT_SPIKE  = "$profile:TBD_TerrainExport_spike.json";

	//------------------------------------------------------------------------------------------------
	//! Mandatory bridgehead anchors (from golden mission) + named land/sea probes.
	protected ref array<ref TBD_TerrainAnchor> BuildAnchors()
	{
		array<ref TBD_TerrainAnchor> a = {};
		AddAnchor(a, "bridgehead-sl",  4839.2, 6620.8);
		AddAnchor(a, "bridgehead-tl0", 4836.9, 6626.5);
		AddAnchor(a, "bridgehead-tl1", 4831.2, 6628.8);
		AddAnchor(a, "coast-w",        1000.0, 6400.0);
		AddAnchor(a, "valley-inland",  5000.0, 5000.0);
		AddAnchor(a, "hill-north",     9600.0, 3200.0);
		AddAnchor(a, "peak-central",   6400.0, 6400.0);
		AddAnchor(a, "coast-sw",       2000.0, 2000.0);
		AddAnchor(a, "seabed-e",      11000.0, 6400.0);
		AddAnchor(a, "shelf-ne",       8000.0, 8000.0);
		AddAnchor(a, "mid-s",          3200.0, 9600.0);
		return a;
	}

	protected void AddAnchor(array<ref TBD_TerrainAnchor> a, string id, float x, float z)
	{
		TBD_TerrainAnchor t = new TBD_TerrainAnchor();
		t.id = id; t.x = x; t.z = z; t.surfaceY = 0;
		a.Insert(t);
	}

	//------------------------------------------------------------------------------------------------
	protected int EncodeU16(float y)
	{
		float t = (y - HMIN) / (HMAX - HMIN);
		float r = Math.Round(t * 65535.0);
		int u = r;
		if (u < 0) u = 0;
		if (u > 65535) u = 65535;
		return u;
	}

	//------------------------------------------------------------------------------------------------
	protected string JsonAnchors(array<ref TBD_TerrainAnchor> anchors)
	{
		string s = "[\n";
		for (int i = 0; i < anchors.Count(); i++)
		{
			TBD_TerrainAnchor t = anchors[i];
			s += "    { \"id\": \"" + t.id + "\", \"x\": " + t.x.ToString() + ", \"z\": " + t.z.ToString() + ", \"surfaceYM\": " + t.surfaceY.ToString() + " }";
			if (i < anchors.Count() - 1) s += ",";
			s += "\n";
		}
		s += "  ]";
		return s;
	}

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			Print("[TBD][DEM] WorldEditor module not available", LogLevel.ERROR);
			return;
		}
		WorldEditorAPI api = worldEditor.GetApi();
		if (!api)
		{
			Print("[TBD][DEM] WorldEditorAPI not available", LogLevel.ERROR);
			return;
		}

		vector bmin, bmax;
		bool haveBounds = worldEditor.GetTerrainBounds(bmin, bmax);
		Print(string.Format("[TBD][DEM] bounds ok=%1 min=%2 max=%3", haveBounds, bmin.ToString(), bmax.ToString()));

		// --- Anchor probes (spawn-authority surface read) ---
		array<ref TBD_TerrainAnchor> anchors = BuildAnchors();
		float pMin = 100000.0;
		float pMax = -100000.0;
		foreach (TBD_TerrainAnchor t : anchors)
		{
			t.surfaceY = api.GetTerrainSurfaceY(t.x, t.z);
			if (t.surfaceY < pMin) pMin = t.surfaceY;
			if (t.surfaceY > pMax) pMax = t.surfaceY;
			Print(string.Format("[TBD][DEM][anchor] %1 (%2, %3) -> %4", t.id, t.x, t.z, t.surfaceY));
		}

		// Plausibility gate: a known land point must be in a sane Everon range.
		float peak = api.GetTerrainSurfaceY(6400.0, 6400.0);
		bool plausible = (peak > 50.0 && peak < 400.0 && pMax > pMin);
		Print(string.Format("[TBD][DEM] plausibility peak(6400,6400)=%1 spreadMin=%2 spreadMax=%3 plausible=%4", peak, pMin, pMax, plausible));
		if (!plausible)
		{
			Print("[TBD][DEM] SPIKE FAIL: implausible elevations — aborting before grid loop", LogLevel.ERROR);
			return;
		}

		if (SPIKE)
		{
			// Timing benchmark: 200x200 = 40000 samples, no file raster.
			int bn = 0;
			for (int by = 0; by < 200; by++)
				for (int bx = 0; bx < 200; bx++)
				{
					float wx = bx * (WORLD / (W - 1));
					float wz = by * (WORLD / (H - 1));
					api.GetTerrainSurfaceY(wx, wz);
					bn++;
				}
			Print(string.Format("[TBD][DEM] SPIKE benchmark sampled %1 points OK", bn));

			FileHandle sh = FileIO.OpenFile(OUT_SPIKE, FileMode.WRITE);
			if (sh)
			{
				string sj;
				sj += "{\n";
				sj += "  \"method\": \"mod-getsurfacey-resample\",\n";
				sj += "  \"spike\": true,\n";
				sj += "  \"boundsMin\": \"" + bmin.ToString() + "\",\n";
				sj += "  \"boundsMax\": \"" + bmax.ToString() + "\",\n";
				sj += "  \"benchmarkSamples\": 40000,\n";
				sj += "  \"anchors\": " + JsonAnchors(anchors) + "\n";
				sj += "}\n";
				sh.Write(sj);
				sh.Close();
				Print("[TBD][DEM] SPIKE wrote " + OUT_SPIKE);
			}
			Print("[TBD][DEM] SPIKE done — set SPIKE=false to run full export");
			return;
		}

		// --- Full export: W x H uint16 raster, ASCII rows ---
		FileHandle f = FileIO.OpenFile(OUT_RASTER, FileMode.WRITE);
		if (!f)
		{
			Print("[TBD][DEM] could not open " + OUT_RASTER + " for write", LogLevel.ERROR);
			return;
		}

		float sampMin = 100000.0;
		float sampMax = -100000.0;
		string buf = "";
		float sx = WORLD / (W - 1);
		float sz = WORLD / (H - 1);

		for (int py = 0; py < H; py++)
		{
			float wz = py * sz;
			for (int px = 0; px < W; px++)
			{
				float wx = px * sx;
				float y = api.GetTerrainSurfaceY(wx, wz);
				if (y < sampMin) sampMin = y;
				if (y > sampMax) sampMax = y;
				buf += EncodeU16(y).ToString();
				if (px < W - 1) buf += " ";
				if (buf.Length() > FLUSH) { f.Write(buf); buf = ""; }
			}
			buf += "\n";
			if (buf.Length() > FLUSH) { f.Write(buf); buf = ""; }
			if (py % 256 == 0)
				Print(string.Format("[TBD][DEM] row %1 / %2", py, H));
		}
		if (buf.Length() > 0) f.Write(buf);
		f.Close();
		Print(string.Format("[TBD][DEM] raster written: %1 sampledMin=%2 sampledMax=%3", OUT_RASTER, sampMin, sampMax));

		// --- Meta JSON ---
		FileHandle mh = FileIO.OpenFile(OUT_META, FileMode.WRITE);
		if (mh)
		{
			string mj;
			mj += "{\n";
			mj += "  \"method\": \"mod-getsurfacey-resample\",\n";
			mj += "  \"widthPx\": " + W.ToString() + ",\n";
			mj += "  \"heightPx\": " + H.ToString() + ",\n";
			mj += "  \"planarResolutionM\": " + (WORLD / W).ToString() + ",\n";
			mj += "  \"heightRangeMinM\": " + HMIN.ToString() + ",\n";
			mj += "  \"heightRangeMaxM\": " + HMAX.ToString() + ",\n";
			mj += "  \"sampledMinM\": " + sampMin.ToString() + ",\n";
			mj += "  \"sampledMaxM\": " + sampMax.ToString() + ",\n";
			mj += "  \"boundsMin\": \"" + bmin.ToString() + "\",\n";
			mj += "  \"boundsMax\": \"" + bmax.ToString() + "\",\n";
			mj += "  \"rasterFile\": \"TBD_TerrainExport_heightmap.txt\",\n";
			mj += "  \"rasterFormat\": \"ascii-uint16-rows\",\n";
			mj += "  \"anchors\": " + JsonAnchors(anchors) + "\n";
			mj += "}\n";
			mh.Write(mj);
			mh.Close();
			Print("[TBD][DEM] meta written: " + OUT_META);
		}

		Print("[TBD][DEM] DONE full export");
	}
}
