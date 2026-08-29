/**
 * TBD_PlantsExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting wild plants & undergrowth:
 *   - Queries world entities for authentic plant prefabs (Prefabs/Vegetation/Plant/*)
 *   - Discards bushes, trees, crops, rocks, and map descriptors
 *   - Writes valid JSON document to $profile:TBD_Export/<mapName>/vegetation/plants.json
 *
 * Menu: Workbench > Plugins > TBD > "Export Plants (plants.json)"
 */

[WorkbenchPluginAttribute(
	name: "Export Plants (plants.json)",
	description: "Standalone plant exporter: extracts wild plants, marine undergrowth, and curbside weeds to plants.json.",
	category: "TBD"
)]
class TBD_PlantsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportPlants m_PlantsExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Plants] Starting isolated plant export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Plants] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_PlantsExporter)
			m_PlantsExporter = new TBD_MapExportPlants();

		bool ok = m_PlantsExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Plants] Plant export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Plant Export",
			"Configure destination directory and spatial chunk parameters for plant export:",
			m_Config
		);
	}
}
