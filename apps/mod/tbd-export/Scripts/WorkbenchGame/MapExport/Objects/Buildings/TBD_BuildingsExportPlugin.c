/**
 * TBD_BuildingsExportPlugin.c
 *
 * Standalone Workbench plugin for extracting placed buildings & structures.
 *
 * Menu: Workbench > Plugins > TBD > "Export Buildings & Structures"
 */

[WorkbenchPluginAttribute(
	name: "Export Buildings & Structures",
	description: "Standalone buildings exporter: extracts all architectural structures partitioned by functional subtype.",
	category: "TBD"
)]
class TBD_BuildingsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportBuildings m_BuildingsExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Buildings] Starting isolated buildings export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Buildings] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_BuildingsExporter)
			m_BuildingsExporter = new TBD_MapExportBuildings();

		bool ok = m_BuildingsExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Buildings] Buildings export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Buildings Export",
			"Configure buildings export parameters:",
			m_Config
		);
	}
}
