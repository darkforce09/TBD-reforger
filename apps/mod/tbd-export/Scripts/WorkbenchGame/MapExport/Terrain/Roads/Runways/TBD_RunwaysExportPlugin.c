/**
 * TBD_RunwaysExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Airfield Runway & Taxiway network data.
 *
 * Menu: Workbench > Plugins > TBD > "Export Airfield Runways & Taxiways"
 */

[WorkbenchPluginAttribute(
	name: "Export Airfield Runways & Taxiways",
	description: "Standalone runway exporter: extracts airfield runways, airstrips, and taxiways with continuous spline geometry to runways.json.",
	category: "TBD"
)]
class TBD_RunwaysExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportRunways m_Exporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Roads][Runways] Starting isolated runways export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Roads][Runways] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_Exporter)
			m_Exporter = new TBD_MapExportRunways();

		bool ok = m_Exporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Roads][Runways] Runways export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Runways Export",
			"Configure airfield runway and taxiway network export parameters:",
			m_Config
		);
	}
}
