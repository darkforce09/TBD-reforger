/**
 * TBD_PavedRoadsExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Secondary Paved Road network data.
 *
 * Menu: Workbench > Plugins > TBD > "Export Secondary Paved Roads"
 */

[WorkbenchPluginAttribute(
	name: "Export Secondary Paved Roads",
	description: "Standalone paved road exporter: extracts secondary paved asphalt/cobblestone roads and continuous spline geometry to roads_paved.json.",
	category: "TBD"
)]
class TBD_PavedRoadsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportPavedRoads m_Exporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Roads][Paved] Starting isolated paved roads export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Roads][Paved] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_Exporter)
			m_Exporter = new TBD_MapExportPavedRoads();

		bool ok = m_Exporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Roads][Paved] Paved roads export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Paved Roads Export",
			"Configure secondary paved road network export parameters:",
			m_Config
		);
	}
}
