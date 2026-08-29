/**
 * TBD_ObjectsExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Placed World Objects:
 *   - 512m chunked spatial AABB queries (JSONL)
 *
 * Menu: Workbench > Plugins > TBD > "Export World Objects"
 */

[WorkbenchPluginAttribute(
	name: "Export World Objects",
	description: "Standalone objects exporter: extracts all placed world entities via spatial chunked AABB queries to JSONL.",
	category: "TBD"
)]
class TBD_ObjectsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportObjects m_ObjectsExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Objects] Starting isolated world objects export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Objects] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_ObjectsExporter)
			m_ObjectsExporter = new TBD_MapExportObjects();

		bool ok = m_ObjectsExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Objects] Objects export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure World Objects Export",
			"Configure world objects spatial query and chunk settings:",
			m_Config
		);
	}
}
