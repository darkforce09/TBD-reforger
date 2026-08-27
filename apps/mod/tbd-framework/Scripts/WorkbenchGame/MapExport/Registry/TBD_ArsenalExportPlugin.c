/**
 * TBD_ArsenalExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Arsenal Registry:
 *   - Weapons, magazines, attachments, and equipment compatibility graph
 *
 * Menu: Workbench > Plugins > TBD > "Export Arsenal Registry"
 */

[WorkbenchPluginAttribute(
	name: "Export Arsenal Registry",
	description: "Standalone arsenal exporter: extracts weapons, equipment, and magazine compatibility registry to JSON.",
	category: "TBD"
)]
class TBD_ArsenalExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportArsenal m_ArsenalExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Arsenal] Starting isolated arsenal registry export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Arsenal] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_ArsenalExporter)
			m_ArsenalExporter = new TBD_MapExportArsenal();

		bool ok = m_ArsenalExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Arsenal] Arsenal registry export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Arsenal Export",
			"Configure output destination directory for arsenal registry export:",
			m_Config
		);
	}
}
