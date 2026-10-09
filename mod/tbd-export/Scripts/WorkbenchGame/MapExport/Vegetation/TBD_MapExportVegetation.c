/**
 * TBD_MapExportVegetation.c
 *
 * Scaffolding coordinator for the natural environment domain:
 *   - Orchestrates Trees (TBD_MapExportTrees)
 *   - Orchestrates Rocks & Cliffs (TBD_MapExportRocks)
 *   - Orchestrates Bushes & Undergrowth (TBD_MapExportBushes)
 *   - Orchestrates Wild Plants & Flora (TBD_MapExportPlants)
 *   - Orchestrates Agricultural Crops & Vegetables (TBD_MapExportCrops)
 *   - Orchestrates Tree Stumps & Forestry Trunks (TBD_MapExportStumps)
 *   - Generates consolidated vegetation_meta.json manifest
 */

class TBD_MapExportVegetation
{
	protected static const string TAG = "[TBD][Vegetation]";

	protected ref TBD_MapExportTrees m_TreesExporter;
	protected ref TBD_MapExportRocks m_RocksExporter;
	protected ref TBD_MapExportBushes m_BushesExporter;
	protected ref TBD_MapExportPlants m_PlantsExporter;
	protected ref TBD_MapExportCrops m_CropsExporter;
	protected ref TBD_MapExportStumps m_StumpsExporter;

	//------------------------------------------------------------------------------------------------
	//! Static execution entry point
	static bool Export(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		if (!ctx || !ctx.m_bValid || !ctx.m_World)
		{
			Print(TAG + " Invalid export context", LogLevel.ERROR);
			return false;
		}

		TBD_MapExportVegetation exporter = new TBD_MapExportVegetation();
		return exporter.Execute(ctx, cfg);
	}

	//------------------------------------------------------------------------------------------------
	bool Execute(TBD_MapExportContext ctx, TBD_MapExportConfig cfg)
	{
		int tickStart = System.GetTickCount();
		string mapName = ctx.GetMapName(cfg);
		string outMeta = TBD_MapExportPaths.BuildCategoryPath(cfg.m_sDestinationDir, mapName, "vegetation", "vegetation_meta.json");

		Print(TAG + " Starting coordinated vegetation export suite...", LogLevel.NORMAL);

		if (!m_TreesExporter) m_TreesExporter = new TBD_MapExportTrees();
		if (!m_RocksExporter) m_RocksExporter = new TBD_MapExportRocks();
		if (!m_BushesExporter) m_BushesExporter = new TBD_MapExportBushes();
		if (!m_PlantsExporter) m_PlantsExporter = new TBD_MapExportPlants();
		if (!m_CropsExporter) m_CropsExporter = new TBD_MapExportCrops();
		if (!m_StumpsExporter) m_StumpsExporter = new TBD_MapExportStumps();

		bool treesOk = true;
		if (!cfg || cfg.m_bExportTrees)
			treesOk = m_TreesExporter.Export(ctx, cfg);

		bool rocksOk = true;
		if (!cfg || cfg.m_bExportRocks)
			rocksOk = m_RocksExporter.Export(ctx, cfg);
		
		bool bushesOk = true;
		if (!cfg || cfg.m_bExportBushes)
			bushesOk = m_BushesExporter.Export(ctx, cfg);

		bool plantsOk = true;
		if (!cfg || cfg.m_bExportPlants)
			plantsOk = m_PlantsExporter.Export(ctx, cfg);

		bool cropsOk = true;
		if (!cfg || cfg.m_bExportCrops)
			cropsOk = m_CropsExporter.Export(ctx, cfg);

		bool stumpsOk = true;
		if (!cfg || cfg.m_bExportStumps)
			stumpsOk = m_StumpsExporter.Export(ctx, cfg);

		int elapsedMs = System.GetTickCount() - tickStart;
		bool allOk = (treesOk && rocksOk && bushesOk && plantsOk && cropsOk && stumpsOk);

		if (allOk)
		{
			FileHandle mh = FileIO.OpenFile(outMeta, FileMode.WRITE);
			if (mh)
			{
				string mj = "{\n";
				mj += "  \"method\": \"mod-vegetation-suite-export\",\n";
				mj += "  \"mapName\": \"" + TBD_MapExportJson.Escape(mapName) + "\",\n";
				mj += "  \"worldSizeM\": " + ctx.m_fWorldSize.ToString() + ",\n";
				mj += "  \"layers\": {\n";
				mj += "    \"trees\": " + (cfg.m_bExportTrees).ToString() + ",\n";
				mj += "    \"rocks\": " + (cfg.m_bExportRocks).ToString() + ",\n";
				mj += "    \"bushes\": " + (cfg.m_bExportBushes).ToString() + ",\n";
				mj += "    \"plants\": " + (cfg.m_bExportPlants).ToString() + ",\n";
				mj += "    \"crops\": " + (cfg.m_bExportCrops).ToString() + ",\n";
				mj += "    \"stumps\": " + (cfg.m_bExportStumps).ToString() + "\n";
				mj += "  },\n";
				mj += "  \"elapsedMs\": " + elapsedMs.ToString() + "\n";
				mj += "}\n";
				TBD_MapExportJson.Write(mh, mj, TAG);
				mh.Close();
			}
		}

		Print(string.Format("%1 Vegetation export finished in %2 ms (success=%3) -> %4",
			TAG, elapsedMs, allOk, outMeta), LogLevel.NORMAL);

		return allOk;
	}
}
