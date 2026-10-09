/**
 * TBD_MapExportInlandWater.c
 *
 * Inland water export coordinator for Bohemia Reforger.
 * Orchestrates the modular extraction pipeline across:
 *   1. Rivers (TBD_MapExportRivers -> rivers.json)
 *   2. Lakes  (TBD_MapExportLakes  -> lakes.json)
 *   3. Ponds  (TBD_MapExportPonds  -> ponds.json)
 *
 * Outputs:
 *   - inland_water_meta.json
 */

class TBD_MapExportInlandWater
{
	protected static const string TAG = "[TBD][InlandWater]";
	protected static const int FLUSH = 8000;

	protected ref TBD_MapExportRivers m_RiversExporter;
	protected ref TBD_MapExportLakes m_LakesExporter;
	protected ref TBD_MapExportPonds m_PondsExporter;

	//------------------------------------------------------------------------------------------------
	//! Primary static export execution method.
	static bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World || !ctx.m_API)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		TBD_MapExportInlandWater exporter = new TBD_MapExportInlandWater();
		return exporter.Execute(ctx, cfg);
	}

	//------------------------------------------------------------------------------------------------
	bool Execute(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		int tickStart = System.GetTickCount();
		string mapName = ctx.GetMapName(cfg);
		float worldSize = ctx.m_fWorldSize;
		string outMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "water", "inland_water_meta.json");

		Print(TAG + " Starting coordinated inland water export suite...", LogLevel.NORMAL);

		if (!m_RiversExporter) m_RiversExporter = new TBD_MapExportRivers();
		if (!m_LakesExporter)  m_LakesExporter = new TBD_MapExportLakes();
		if (!m_PondsExporter)  m_PondsExporter = new TBD_MapExportPonds();

		// 1. Rivers
		array<ref TBD_RiverExport> rivers = {};
		bool riversOk = m_RiversExporter.Export(ctx, cfg, rivers);

		// 2. Lakes
		array<ref TBD_LakeRecord> lakes = {};
		bool lakesOk = m_LakesExporter.Export(ctx, cfg, lakes);

		// 3. Ponds
		array<ref TBD_PondRecord> ponds = {};
		bool pondsOk = m_PondsExporter.Export(ctx, cfg, ponds);

		// 4. Consolidated Metadata Manifest
		WriteInlandWaterMeta(outMeta, worldSize, rivers, lakes, ponds);

		int elapsedMs = System.GetTickCount() - tickStart;
		bool allOk = (riversOk && lakesOk && pondsOk);

		int riversCount = 0;
		if (rivers) riversCount = rivers.Count();

		int lakesCount = 0;
		if (lakes) lakesCount = lakes.Count();

		int pondsCount = 0;
		if (ponds) pondsCount = ponds.Count();

		Print(string.Format("%1 INLAND WATER EXPORT FINISHED in %2 ms (success=%3) - Rivers=%4, Lakes=%5, Ponds=%6 -> %7",
			TAG, elapsedMs, allOk, riversCount, lakesCount, pondsCount, outMeta), LogLevel.NORMAL);

		return allOk;
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteInlandWaterMeta(string path, float worldSize, array<ref TBD_RiverExport> rivers, array<ref TBD_LakeRecord> lakes, array<ref TBD_PondRecord> ponds)
	{
		FileHandle f = FileIO.OpenFile(path, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open metadata file: " + path, LogLevel.ERROR);
			return;
		}

		int riversCount = 0;
		float totalRiversLengthM = 0.0;
		int totalRiverParts = 0;
		if (rivers)
		{
			riversCount = rivers.Count();
			for (int r = 0; r < rivers.Count(); r++)
			{
				totalRiversLengthM += rivers[r].m_fTotalLengthM;
				totalRiverParts += rivers[r].m_aParts.Count();
			}
		}

		int lakesCount = 0;
		float totalLakeAreaM2 = 0.0;
		if (lakes)
		{
			lakesCount = lakes.Count();
			for (int l = 0; l < lakes.Count(); l++)
				totalLakeAreaM2 += lakes[l].m_fAreaM2;
		}

		int pondsCount = 0;
		float totalPondAreaM2 = 0.0;
		if (ponds)
		{
			pondsCount = ponds.Count();
			for (int p = 0; p < ponds.Count(); p++)
				totalPondAreaM2 += ponds[p].m_fAreaM2;
		}

		string buf = "{\n";
		buf += "  \"type\": \"InlandWaterMetadataManifest\",\n";
		buf += "  \"worldSizeM\": " + worldSize.ToString() + ",\n";
		buf += "  \"riversCount\": " + riversCount.ToString() + ",\n";
		buf += "  \"totalRiversLengthM\": " + totalRiversLengthM.ToString() + ",\n";
		buf += "  \"totalRiverSubpartsCount\": " + totalRiverParts.ToString() + ",\n";
		buf += "  \"lakesCount\": " + lakesCount.ToString() + ",\n";
		buf += "  \"totalLakeAreaM2\": " + totalLakeAreaM2.ToString() + ",\n";
		buf += "  \"pondsCount\": " + pondsCount.ToString() + ",\n";
		buf += "  \"totalPondAreaM2\": " + totalPondAreaM2.ToString() + ",\n";
		buf += "  \"files\": {\n";
		buf += "    \"rivers\": \"rivers.json\",\n";
		buf += "    \"lakes\": \"lakes.json\",\n";
		buf += "    \"ponds\": \"ponds.json\"\n";
		buf += "  }\n";
		buf += "}\n";

		TBD_MapExportJson.Write(f, buf, TAG);
		f.Close();
	}
}
