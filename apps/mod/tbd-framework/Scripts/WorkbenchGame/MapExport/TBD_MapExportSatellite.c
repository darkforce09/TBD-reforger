/**
 * TBD_MapExportSatellite.c
 *
 * Satellite/cartographic rasterization exporter using Enfusion's MapDataExporter.
 */

class TBD_MapExportSatellite
{
	protected static const string TAG = "[TBD][SAT]";
	protected static const string WORLD_PATH = "worlds/Eden/Eden.ent";

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
	static bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		MapDataExporter exporter = new MapDataExporter();

		Color landBright  = Color.FromRGBA(120, 134, 96, 255);
		Color landDark    = Color.FromRGBA(72, 84, 58, 255);
		Color oceanBright = Color.FromRGBA(58, 96, 120, 255);
		Color oceanDark   = Color.FromRGBA(28, 52, 78, 255);
		Color forestArea  = Color.FromRGBA(54, 70, 44, 255);
		Color otherArea   = Color.FromRGBA(110, 104, 88, 255);
		exporter.SetupColors(landBright, landDark, oceanBright, oceanDark, forestArea, otherArea);

		string nativeTga = TBD_MapExportPaths.ResolveNativeOsPath(cfg.m_sDestinationDir, "TBD_SatExport_everon.tga");
		string outMeta = TBD_MapExportPaths.BuildPath(cfg.m_sDestinationDir, "TBD_SatExport_meta.json");

		Print(string.Format("%1 Exporting rasterization to %2", TAG, nativeTga));

		DataExportErrorType err = exporter.ExportRasterization(
			nativeTga, WORLD_PATH,
			SCALE_LAND, SCALE_OCEAN, HEIGHT_SCALE, DEPTH_SCALE, DEPTH_LERP_METERS,
			SHADE_INTENSITY, HEIGHT_INTENSITY, INCLUDE_GEN_AREAS, FOREST_INTENSITY, OTHER_INTENSITY);

		int rc = err;
		string rmsg = SCR_WorldMapExportTool.GetReportMessage(err);
		Print(string.Format("%1 ExportRasterization returned rc=%2 (%3)", TAG, rc, rmsg));

		// Write meta
		FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (mh)
		{
			string genStr = "false";
			if (INCLUDE_GEN_AREAS) genStr = "true";
			string j;
			j += "{\n";
			j += "  \"method\": \"mod-maprasterization-export\",\n";
			j += "  \"worldPath\": \"" + TBD_MapExportJson.Escape(WORLD_PATH) + "\",\n";
			j += "  \"returnCode\": " + rc.ToString() + ",\n";
			j += "  \"returnMessage\": \"" + TBD_MapExportJson.Escape(rmsg) + "\",\n";
			j += "  \"outputPath\": \"" + TBD_MapExportJson.Escape(nativeTga) + "\",\n";
			j += "  \"boundsMin\": \"" + TBD_MapExportJson.Escape(ctx.m_vBoundsMin.ToString()) + "\",\n";
			j += "  \"boundsMax\": \"" + TBD_MapExportJson.Escape(ctx.m_vBoundsMax.ToString()) + "\",\n";
			j += "  \"params\": {\n";
			j += "    \"scaleLand\": " + SCALE_LAND.ToString() + ", \"scaleOcean\": " + SCALE_OCEAN.ToString() + ",\n";
			j += "    \"heightScale\": " + HEIGHT_SCALE.ToString() + ", \"depthScale\": " + DEPTH_SCALE.ToString() + ",\n";
			j += "    \"depthLerpMeters\": " + DEPTH_LERP_METERS.ToString() + ", \"shadeIntensity\": " + SHADE_INTENSITY.ToString() + ",\n";
			j += "    \"heightIntensity\": " + HEIGHT_INTENSITY.ToString() + ", \"includeGeneratorAreas\": " + genStr + ",\n";
			j += "    \"forestAreaIntensity\": " + FOREST_INTENSITY.ToString() + ", \"otherAreaIntensity\": " + OTHER_INTENSITY.ToString() + "\n";
			j += "  }\n";
			j += "}\n";
			TBD_MapExportJson.Write(mh, j, TAG);
			mh.Close();
		}

		return (rc == 0);
	}
}
