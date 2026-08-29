/**
 * TBD_RocksExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting natural rocks and cliff formations:
 *   - Queries world entities for authentic rock prefabs (Prefabs/Rocks/*, Prefabs/Vegetation/Rocks/*)
 *   - Performs 5-point terrain surface elevation sampling (GetTerrainSurfaceY)
 *   - Computes vertical penetration, exposure ratio, burial depth, and apex coordinates
 *   - Writes valid JSON document to $profile:TBD_Export/<mapName>/vegetation/rocks.json
 *
 * Menu: Workbench > Plugins > TBD > "Export Rocks (rocks.json)"
 */

[WorkbenchPluginAttribute(
	name: "Export Rocks (rocks.json)",
	description: "Standalone rock exporter: extracts boulders, cliffs, outcrops, scree, and pebbles with terrain exposure metrics to rocks.json.",
	category: "TBD"
)]
class TBD_RocksExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportRocks m_RocksExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Rocks] Starting isolated rock export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Rocks] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_RocksExporter)
			m_RocksExporter = new TBD_MapExportRocks();

		bool ok = m_RocksExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Rocks] Rock export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Rock Export",
			"Configure destination directory and spatial chunk parameters for rock export:",
			m_Config
		);
	}
}
