/**
 * TBD_DirtRoadsExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Dirt & Gravel Road network data.
 *
 * Menu: Workbench > Plugins > TBD > "Export Dirt & Gravel Roads"
 */

[WorkbenchPluginAttribute(
	name: "Export Dirt & Gravel Roads",
	description: "Standalone dirt road exporter: extracts unpaved, gravel, and dirt roads with continuous spline geometry to roads_dirt.json.",
	category: "TBD"
)]
class TBD_DirtRoadsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportDirtRoads m_Exporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Roads][Dirt] Starting isolated dirt roads export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Roads][Dirt] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_Exporter)
			m_Exporter = new TBD_MapExportDirtRoads();

		bool ok = m_Exporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Roads][Dirt] Dirt roads export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Dirt Roads Export",
			"Configure unpaved dirt and gravel road network export parameters:",
			m_Config
		);
	}
}
