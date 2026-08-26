/**
 * TBD_RoadsExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Road network data:
 *   - Road centerlines, spline points, widths, and surface classes
 *
 * Menu: Workbench > Plugins > TBD > "Export Roads & Splines"
 */

[WorkbenchPluginAttribute(
	name: "Export Roads & Splines",
	description: "Standalone road exporter: extracts road centerlines, widths, and surface classifications to JSON.",
	category: "TBD"
)]
class TBD_RoadsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportRoads m_RoadsExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Roads] Starting isolated road network export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Roads] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_RoadsExporter)
			m_RoadsExporter = new TBD_MapExportRoads();

		bool ok = m_RoadsExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Roads] Roads export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Roads Export",
			"Configure road network export settings:",
			m_Config
		);
	}
}
