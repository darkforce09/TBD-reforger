/**
 * TBD_SatelliteExportPlugin.c - T-090.1 Everon satellite basemap export.
 *
 * Closes spike gate K3. There is NO one-call Workbench export of a satellite tile
 * (spike S3), and the per-terrain satellite is a generated SAP/super-texture set
 * (worlds/Eden/Eden/.Data/Eden_*_supertexture.edds), not a single loose .edds we can
 * decode on the CLI. The clean, scriptable path is the engine's own map rasterizer:
 *
 *   MapDataExporter.ExportRasterization(exportPath, worldPath, scaleLand, scaleOcean,
 *     heightScale, depthScale, depthLerpMeters, shadeIntensity, heightIntensity,
 *     bIncludeGeneratorAreas, forestAreaIntensity, otherAreaIntensity)
 *
 * This is the same engine call the official World Editor "Export Map Data ->
 * Rasterization" toolbar tool (SCR_WorldMapExportTool) drives. It writes a north-up...
 * (actually upside-down per the BI "2D Map Creation" wiki) <world>.tga that the Node
 * post-process (build-tile-pyramid) V-flips, crops to world bounds and slices into the
 * XYZ WebP pyramid under packages/map-assets/everon/tiles/satellite/{z}/{x}/{y}.webp.
 *
 * worldPath = worlds/Eden/Eden.ent - Everon's base world (codename "Eden"); rasterizing
 * the base world gives clean terrain + towns/forests with no TBD game-mode entities.
 *
 * SPIKE gate (mirrors TBD_TerrainExportPlugin.c): with SPIKE = true the plugin confirms
 * the world is loaded (bounds + a known land anchor), instantiates MapDataExporter, runs
 * ExportRasterization once, and writes _spike.json with the params + DataExportErrorType
 * result. Inspect the .tga + result, then set SPIKE = false for the recorded full run
 * (same call - the rasterization is a single atomic engine op, no per-pixel loop).
 *
 * Run: Workbench > Plugins > TBD > "Export TBD Satellite"
 *   (or NetAPI: wb_execute_action menuPath "Plugins,TBD,Export TBD Satellite").
 */

[WorkbenchPluginAttribute(name: "Export TBD Satellite", description: "Rasterize the Everon map (terrain + forests/areas) to a .tga via MapDataExporter for the satellite tile pyramid.", category: "TBD")]
class TBD_SatelliteExportPlugin : WorkbenchPlugin
{
	// Flip to false for the recorded full export run (the call is identical; SPIKE only
	// adds the world-loaded sanity guard + verbose _spike.json).
	protected static const bool SPIKE = true;

	// Everon base world (terrain codename "Eden"). Rasterize the base world for a clean
	// satellite (no TBD entities). If this path fails to resolve the exporter returns an
	// error code and writes nothing (non-destructive) - adjust and re-run.
	protected static const string WORLD_PATH = "worlds/Eden/Eden.ent";

	// ExportRasterization needs a REAL OS path - it does NOT resolve the Enfusion `$profile:` VFS
	// prefix (FileIO does; that is why the JSON below writes fine but rc=32 "Could not open output
	// file" earlier). The Proton-prefix path lives in TBD_ExportPaths.PROFILE_WIN (single source,
	// T-130.4 F1-20). We try several candidate forms in one run and keep the first that returns rc=0.
	protected static const string OUT_SPIKE = "$profile:TBD_SatExport_spike.json"; // FileIO VFS - works
	protected static const string OUT_META  = "$profile:TBD_SatExport_meta.json";

	// Rasterization tunables (sane defaults; mirror the SCR_WorldMapExportTool fields).
	protected static const float SCALE_LAND        = 1.0;
	protected static const float SCALE_OCEAN       = 1.0;
	protected static const float HEIGHT_SCALE      = 1.0;
	protected static const float DEPTH_SCALE       = 1.0;
	protected static const float DEPTH_LERP_METERS = 20.0;
	protected static const float SHADE_INTENSITY   = 1.0;
	protected static const float HEIGHT_INTENSITY  = 1.0;
	protected static const bool  INCLUDE_GEN_AREAS = true;
	protected static const float FOREST_INTENSITY  = 1.0;
	protected static const float OTHER_INTENSITY   = 1.0;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			Print("[TBD][SAT] WorldEditor module not available", LogLevel.ERROR);
			return;
		}
		WorldEditorAPI api = worldEditor.GetApi();
		if (!api)
		{
			Print("[TBD][SAT] WorldEditorAPI not available", LogLevel.ERROR);
			return;
		}

		// World-loaded sanity (S0 lesson: confirm a real terrain, not an empty world).
		vector bmin, bmax;
		bool haveBounds = worldEditor.GetTerrainBounds(bmin, bmax);
		float anchorY = api.GetTerrainSurfaceY(4839.2, 6620.8); // bridgehead-sl land anchor
		float peakY = api.GetTerrainSurfaceY(6400.0, 6400.0);
		bool plausible = haveBounds && peakY > 50.0 && peakY < 400.0;
		Print(string.Format("[TBD][SAT] bounds ok=%1 min=%2 max=%3 anchorY=%4 peakY=%5 plausible=%6",
			haveBounds, bmin.ToString(), bmax.ToString(), anchorY, peakY, plausible));
		if (!plausible)
		{
			Print("[TBD][SAT] FAIL: world not loaded / implausible terrain - aborting before export", LogLevel.ERROR);
			return;
		}

		// --- Engine map rasterizer ---
		MapDataExporter exporter = new MapDataExporter();

		// Satellite-ish palette: green land, blue ocean, darker forest. Tuned after first run.
		Color landBright  = Color.FromRGBA(120, 134, 96, 255);
		Color landDark    = Color.FromRGBA(72, 84, 58, 255);
		Color oceanBright = Color.FromRGBA(58, 96, 120, 255);
		Color oceanDark   = Color.FromRGBA(28, 52, 78, 255);
		Color forestArea  = Color.FromRGBA(54, 70, 44, 255);
		Color otherArea   = Color.FromRGBA(110, 104, 88, 255);
		exporter.SetupColors(landBright, landDark, oceanBright, oceanDark, forestArea, otherArea);

		// Try candidate OS paths until one opens (rc=0): Windows-abs file, dir (engine may name it
		// <world>.tga), bare/relative. PROFILE_WIN maps to the native Proton prefix profile dir.
		array<string> candidates = {};
		candidates.Insert(TBD_ExportPaths.PROFILE_WIN + "TBD_SatExport_everon.tga");
		candidates.Insert(TBD_ExportPaths.PROFILE_WIN); // dir -> engine may write <world>.tga (Eden.tga)
		candidates.Insert("TBD_SatExport_everon.tga");  // bare / working dir

		string attempts = "";
		string winPath = "";
		int winRc = 32;
		for (int i = 0; i < candidates.Count(); i++)
		{
			string p = candidates[i];
			DataExportErrorType r = exporter.ExportRasterization(
				p, WORLD_PATH,
				SCALE_LAND, SCALE_OCEAN, HEIGHT_SCALE, DEPTH_SCALE, DEPTH_LERP_METERS,
				SHADE_INTENSITY, HEIGHT_INTENSITY, INCLUDE_GEN_AREAS, FOREST_INTENSITY, OTHER_INTENSITY);
			int rc = r;
			string rmsg = SCR_WorldMapExportTool.GetReportMessage(r);
			Print(string.Format("[TBD][SAT] try[%1] rc=%2 (%3) path=%4", i, rc, rmsg, p));
			if (attempts != "") attempts += " | ";
			attempts += "[" + i.ToString() + "] rc=" + rc.ToString() + " " + rmsg + " :: " + p;
			if (rc == 0)
			{
				winRc = 0;
				winPath = p;
				break;
			}
		}

		string outFile = OUT_META;
		if (SPIKE) outFile = OUT_SPIKE;
		WriteJson(outFile, bmin, bmax, anchorY, peakY, winRc, winPath, attempts);
		Print(string.Format("[TBD][SAT] DONE (spike=%1) winRc=%2 winPath=%3", SPIKE, winRc, winPath));
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteJson(string outPath, vector bmin, vector bmax, float anchorY, float peakY, int winRc, string winPath, string attempts)
	{
		FileHandle h = FileIO.OpenFile(outPath, FileMode.WRITE);
		if (!h)
		{
			Print("[TBD][SAT] could not open " + outPath + " for write", LogLevel.ERROR);
			return;
		}
		// EnfScript has no ternary operator, so render bools -> strings via temps.
		string spikeStr = "false";
		if (SPIKE) spikeStr = "true";
		string genStr = "false";
		if (INCLUDE_GEN_AREAS) genStr = "true";
		string j;
		j += "{\n";
		j += "  \"method\": \"mod-maprasterization-export\",\n";
		j += "  \"spike\": " + spikeStr + ",\n";
		j += "  \"worldPath\": \"" + TBD_ExportJson.Escape(WORLD_PATH) + "\",\n";
		j += "  \"winRc\": " + winRc.ToString() + ",\n";
		j += "  \"winPath\": \"" + TBD_ExportJson.Escape(winPath) + "\",\n";
		j += "  \"attempts\": \"" + TBD_ExportJson.Escape(attempts) + "\",\n";
		j += "  \"boundsMin\": \"" + TBD_ExportJson.Escape(bmin.ToString()) + "\",\n";
		j += "  \"boundsMax\": \"" + TBD_ExportJson.Escape(bmax.ToString()) + "\",\n";
		j += "  \"anchorY_4839_6620\": " + anchorY.ToString() + ",\n";
		j += "  \"peakY_6400_6400\": " + peakY.ToString() + ",\n";
		j += "  \"params\": {\n";
		j += "    \"scaleLand\": " + SCALE_LAND.ToString() + ", \"scaleOcean\": " + SCALE_OCEAN.ToString() + ",\n";
		j += "    \"heightScale\": " + HEIGHT_SCALE.ToString() + ", \"depthScale\": " + DEPTH_SCALE.ToString() + ",\n";
		j += "    \"depthLerpMeters\": " + DEPTH_LERP_METERS.ToString() + ", \"shadeIntensity\": " + SHADE_INTENSITY.ToString() + ",\n";
		j += "    \"heightIntensity\": " + HEIGHT_INTENSITY.ToString() + ", \"includeGeneratorAreas\": " + genStr + ",\n";
		j += "    \"forestAreaIntensity\": " + FOREST_INTENSITY.ToString() + ", \"otherAreaIntensity\": " + OTHER_INTENSITY.ToString() + "\n";
		j += "  }\n";
		j += "}\n";
		bool ok = TBD_ExportJson.Write(h, j, "[TBD][SAT]");
		h.Close();
		if (!ok)
			return;
		Print("[TBD][SAT] wrote " + outPath);
	}
}
