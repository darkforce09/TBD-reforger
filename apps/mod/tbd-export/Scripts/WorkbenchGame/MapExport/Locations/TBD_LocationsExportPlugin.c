/**
 * TBD_LocationsExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Named Locations:
 *   - Towns, villages, and landmarks from World/Locations composition entities
 *
 * Menu: Workbench > Plugins > TBD > "Export Named Locations"
 */

[WorkbenchPluginAttribute(
	name: "Export Named Locations",
	description: "Standalone locations exporter: extracts named towns, villages, and landmarks to JSON.",
	category: "TBD"
)]
class TBD_LocationsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportLocations m_LocationsExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Locations] Starting isolated locations export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Locations] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_LocationsExporter)
			m_LocationsExporter = new TBD_MapExportLocations();

		bool ok = m_LocationsExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Locations] Locations export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Locations Export",
			"Configure output destination directory for named locations export:",
			m_Config
		);
	}
}
