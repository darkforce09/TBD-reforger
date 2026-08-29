/**
 * TBD_PathsExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Footpath & Walking Trail network data.
 *
 * Menu: Workbench > Plugins > TBD > "Export Footpaths & Trails"
 */

[WorkbenchPluginAttribute(
	name: "Export Footpaths & Trails",
	description: "Standalone footpath exporter: extracts pedestrian hiking trails and footpaths with continuous spline geometry to paths.json.",
	category: "TBD"
)]
class TBD_PathsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportFootpaths m_Exporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Roads][Paths] Starting isolated footpaths export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Roads][Paths] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_Exporter)
			m_Exporter = new TBD_MapExportFootpaths();

		bool ok = m_Exporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Roads][Paths] Footpaths export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Footpaths Export",
			"Configure footpath and walking trail network export parameters:",
			m_Config
		);
	}
}
