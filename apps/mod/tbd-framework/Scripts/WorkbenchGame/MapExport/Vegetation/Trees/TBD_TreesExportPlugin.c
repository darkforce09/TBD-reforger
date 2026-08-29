/**
 * TBD_TreesExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting placed natural trees:
 *   - Queries world entities for authentic living trees (conifers, deciduous, palms)
 *   - Discards stumps, fallen wood, bushes, wild plants, crops, rocks, and descriptors
 *   - Writes valid JSON document to $profile:TBD_Export/<mapName>/vegetation/trees.json
 *
 * Menu: Workbench > Plugins > TBD > "Export Trees (trees.json)"
 */

[WorkbenchPluginAttribute(
	name: "Export Trees (trees.json)",
	description: "Standalone tree exporter: extracts natural conifer and deciduous trees to trees.json.",
	category: "TBD"
)]
class TBD_TreesExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportTrees m_TreesExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Trees] Starting isolated tree export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Trees] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_TreesExporter)
			m_TreesExporter = new TBD_MapExportTrees();

		bool ok = m_TreesExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Trees] Tree export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Tree Export",
			"Configure destination directory and spatial chunk parameters for tree export:",
			m_Config
		);
	}
}
