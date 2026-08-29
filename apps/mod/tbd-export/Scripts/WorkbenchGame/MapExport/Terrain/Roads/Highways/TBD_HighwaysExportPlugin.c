/**
 * TBD_HighwaysExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Highway & Major Arterial road network data.
 *
 * Menu: Workbench > Plugins > TBD > "Export Highways & Major Arterials"
 */

[WorkbenchPluginAttribute(
	name: "Export Highways & Major Arterials",
	description: "Standalone highway exporter: extracts major arterial highways and continuous spline geometry to highways.json.",
	category: "TBD"
)]
class TBD_HighwaysExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportHighways m_Exporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Roads][Highways] Starting isolated highway export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Roads][Highways] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_Exporter)
			m_Exporter = new TBD_MapExportHighways();

		bool ok = m_Exporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Roads][Highways] Highway export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Highways Export",
			"Configure highway network export parameters:",
			m_Config
		);
	}
}
