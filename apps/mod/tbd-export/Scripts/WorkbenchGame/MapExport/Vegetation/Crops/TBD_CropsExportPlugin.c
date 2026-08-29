/**
 * TBD_CropsExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting agricultural crops & vegetables:
 *   - Queries world entities for authentic crop prefabs (Prefabs/Vegetation/Vegetables/*)
 *   - Discards wild plants, bushes, trees, rocks, and map descriptors
 *   - Writes valid JSON document to $profile:TBD_Export/<mapName>/vegetation/crops.json
 *
 * Menu: Workbench > Plugins > TBD > "Export Crops (crops.json)"
 */

[WorkbenchPluginAttribute(
	name: "Export Crops (crops.json)",
	description: "Standalone crop exporter: extracts agricultural crops and cultivated vegetables to crops.json.",
	category: "TBD"
)]
class TBD_CropsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportCrops m_CropsExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Crops] Starting isolated crop export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Crops] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_CropsExporter)
			m_CropsExporter = new TBD_MapExportCrops();

		bool ok = m_CropsExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Crops] Crop export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Crop Export",
			"Configure destination directory and spatial chunk parameters for crop export:",
			m_Config
		);
	}
}
