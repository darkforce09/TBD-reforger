/**
 * TBD_EngineOrthoExportPlugin.c - T-090.1.2.4 engine render ortho feasibility spike.
 *
 * GOAL (spec t090_1_2_4_engine_render_ortho_spike.md): find a CONTINUOUS terrain-color
 * top-down capture to replace the 2500-cell SAP stitch (whose baked cell aprons show a
 * ~256 m grid). PASS bar (operator-locked): grid-free AND sat-class (photographic /
 * real engine surface color) - a stylized landcover/shaded-relief raster is FAIL.
 *
 * P0 API SEARCH RESULT (enfusion-mcp api_search, 2026-07-01 - see the spike JSON
 * .ai/artifacts/t090_1_2_4_engine_render_spike.json for the full apisTried list):
 *   - Per-point terrain COLOR read: ABSENT. Only WorldEditorAPI.GetTerrainSurfaceY(x,z)
 *     (height) + BaseWorld.GetSurfaceY exist; GetTerrainSurfaceColor / GetSurfaceProperties
 *     / GetTerrainTextureLayers / SampleSurface / SurfaceColor all return no results. So the
 *     TBD_TerrainExportPlugin GetSurfaceY-resample trick has NO color analog.
 *   - Orthographic camera projection: ABSENT. No Orthographic / SetOrthographic / Frustum;
 *     only perspective CameraBase.SetVerticalFOV. The editor viewport is perspective.
 *   - Scriptable editor-camera positioning: NOT CONFIRMED (no WorldEditorAPI camera setter
 *     surfaced). The operator must frame the shot by hand.
 *   - MapDataExporter: ExportData / SetupColors / ExportRasterization only - all STYLIZED
 *     cartographic (BI's "2D Map Creation" wiki confirms rasterization IS the documented
 *     "Satellite Background Image" method: forests + shaded relief). Operator FAIL tier.
 *   - The one capture primitive that writes real rendered pixels to disk:
 *         System.MakeScreenshot(string path)  -> BMP of the CURRENT viewport.
 *     (also System.MakeScreenshotRawData / MakeScreenshotTexture: region + async callback.)
 *
 * Therefore the ONLY Workbench path to a sat-class bitmap is a screenshot of the editor
 * viewport - which is PERSPECTIVE (edge distortion), at screen resolution, with distance
 * LOD + baked lighting/shadows. A true 12800x12800 @ 1 m/px north-up ORTHO cannot be
 * rendered; it would require a perspective-mosaic + orthorectification that breaks the
 * alignment contract and is unlikely to beat the SAP's native ~1 m/px BC7 detail.
 *
 * This plugin is the honest P0 TEST BITMAP: it screenshots the current editor viewport so
 * the operator can A/B a top-down render against the SAP ortho at the landmark. It does NOT
 * (and cannot, per the search) produce the production ortho - if the A/B confirms the
 * capture is sub-SAP / distorted, P0 is FAIL and SAP stays the fallback (spec: honest FAIL).
 *
 * OPERATOR STEP (Workbench, world TBD_Framework/worlds/TBD_Dev_POC.ent loaded):
 *   1. Fly the editor camera to a top-down (nadir) view centred on world X=4929, Z=5661
 *      (the operator landmark field). Frame it as tight/high as the viewport allows.
 *   2. Run: Plugins > TBD > "Export TBD Engine Ortho (spike)".
 *   3. Copy the written BMP out of the Proton profile dir into
 *      packages/map-assets/everon/staging/engine/ for the Node A/B + tier assessment.
 *
 * Run: Workbench > Plugins > TBD > "Export TBD Engine Ortho (spike)"
 *   (or NetAPI: wb_execute_action menuPath "Plugins,TBD,Export TBD Engine Ortho (spike)").
 */

[WorkbenchPluginAttribute(name: "Export TBD Engine Ortho (spike)", description: "P0 feasibility: screenshot the current top-down editor viewport (System.MakeScreenshot) to A/B a real engine render vs the SAP ortho. NOT the production ortho (no orthographic-projection API exists).", category: "TBD")]
class TBD_EngineOrthoExportPlugin : WorkbenchPlugin
{
	// Landmark the operator should frame top-down before running (spec A/B field patch).
	protected static const float LANDMARK_X = 4929.0;
	protected static const float LANDMARK_Z = 5661.0;

	// MakeScreenshot needs a real OS path. Workbench runs under Proton, so `$profile:` maps to
	// this native prefix dir (same lesson as TBD_SatelliteExportPlugin). Try VFS + native + bare;
	// keep the first that returns true.
	protected static const string PROFILE_WIN = "C:/Users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile/";
	protected static const string OUT_META    = "$profile:TBD_EngineOrtho_spike.json";

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			Print("[TBD][ENGO] WorldEditor module not available", LogLevel.ERROR);
			return;
		}
		WorldEditorAPI api = worldEditor.GetApi();
		if (!api)
		{
			Print("[TBD][ENGO] WorldEditorAPI not available", LogLevel.ERROR);
			return;
		}

		// World-loaded sanity (S0 lesson: GetEditorEntityCount false-positives an empty world).
		vector bmin, bmax;
		bool haveBounds = worldEditor.GetTerrainBounds(bmin, bmax);
		float landmarkY = api.GetTerrainSurfaceY(LANDMARK_X, LANDMARK_Z);
		float peakY = api.GetTerrainSurfaceY(6400.0, 6400.0);
		bool plausible = haveBounds && peakY > 50.0 && peakY < 400.0;
		Print(string.Format("[TBD][ENGO] bounds ok=%1 min=%2 max=%3 landmarkY(%4,%5)=%6 peakY=%7 plausible=%8",
			haveBounds, bmin.ToString(), bmax.ToString(), LANDMARK_X, LANDMARK_Z, landmarkY, peakY, plausible));
		if (!plausible)
		{
			Print("[TBD][ENGO] FAIL: world not loaded / implausible terrain - open TBD_Dev_POC.ent and frame the landmark top-down first", LogLevel.ERROR);
			return;
		}

		// --- Capture the current viewport (operator frames it top-down over the landmark) ---
		// Try candidate OS paths until MakeScreenshot returns true. BMP is converted to PNG in Node.
		array<string> candidates = {};
		candidates.Insert(PROFILE_WIN + "TBD_EngineOrtho_everon.bmp");
		candidates.Insert("$profile:TBD_EngineOrtho_everon.bmp"); // VFS - may or may not resolve for MakeScreenshot
		candidates.Insert("TBD_EngineOrtho_everon.bmp");          // bare / working dir

		string attempts = "";
		string winPath = "";
		bool ok = false;
		for (int i = 0; i < candidates.Count(); i++)
		{
			string p = candidates[i];
			bool r = System.MakeScreenshot(p);
			Print(string.Format("[TBD][ENGO] MakeScreenshot try[%1] ok=%2 path=%3", i, r, p));
			if (attempts != "") attempts += " | ";
			attempts += "[" + i.ToString() + "] ok=" + r.ToString() + " :: " + p;
			if (r)
			{
				ok = true;
				winPath = p;
				break;
			}
		}

		WriteMeta(OUT_META, bmin, bmax, landmarkY, peakY, ok, winPath, attempts);
		Print(string.Format("[TBD][ENGO] DONE screenshotOk=%1 winPath=%2 (perspective viewport - NOT a 1 m/px ortho; A/B vs SAP in Node)", ok, winPath));
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteMeta(string outPath, vector bmin, vector bmax, float landmarkY, float peakY, bool ok, string winPath, string attempts)
	{
		FileHandle h = FileIO.OpenFile(outPath, FileMode.WRITE);
		if (!h)
		{
			Print("[TBD][ENGO] could not open " + outPath + " for write", LogLevel.ERROR);
			return;
		}
		string okStr = "false";
		if (ok) okStr = "true";
		string j;
		j += "{\n";
		j += "  \"method\": \"system-makescreenshot-viewport\",\n";
		j += "  \"note\": \"P0 spike: perspective editor-viewport screenshot; NOT a 1 m/px north-up ortho (no orthographic-projection API exists). A/B vs SAP in Node.\",\n";
		j += "  \"screenshotOk\": " + okStr + ",\n";
		j += "  \"winPath\": \"" + winPath + "\",\n";
		j += "  \"attempts\": \"" + attempts + "\",\n";
		j += "  \"landmarkX\": " + LANDMARK_X.ToString() + ", \"landmarkZ\": " + LANDMARK_Z.ToString() + ",\n";
		j += "  \"landmarkSurfaceYM\": " + landmarkY.ToString() + ",\n";
		j += "  \"peakY_6400_6400\": " + peakY.ToString() + ",\n";
		j += "  \"boundsMin\": \"" + bmin.ToString() + "\",\n";
		j += "  \"boundsMax\": \"" + bmax.ToString() + "\"\n";
		j += "}\n";
		h.Write(j);
		h.Close();
		Print("[TBD][ENGO] wrote " + outPath);
	}
}
