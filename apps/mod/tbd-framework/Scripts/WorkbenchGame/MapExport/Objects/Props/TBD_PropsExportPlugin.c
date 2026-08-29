/**
 * TBD_PropsExportPlugin.c
 *
 * Standalone Workbench plugin for extracting tactical props and clutter.
 *
 * Menu: Workbench > Plugins > TBD > "Export Tactical Props & Clutter"
 */

[WorkbenchPluginAttribute(
	name: "Export Tactical Props & Clutter",
	description: "Standalone props exporter: extracts placed tactical cover, barricades, containers, and clutter.",
	category: "TBD"
)]
class TBD_PropsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportProps m_PropsExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Props] Starting isolated props export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Props] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_PropsExporter)
			m_PropsExporter = new TBD_MapExportProps();

		bool ok = m_PropsExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Props] Props export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Props Export",
			"Configure tactical props export parameters:",
			m_Config
		);
	}
}
