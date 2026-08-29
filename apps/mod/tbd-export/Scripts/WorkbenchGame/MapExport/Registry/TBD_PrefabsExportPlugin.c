/**
 * TBD_PrefabsExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Prefab Taxonomy:
 *   - Prefab components, taxonomy classes, and physical bounding dimensions
 *
 * Menu: Workbench > Plugins > TBD > "Export Prefab Taxonomy"
 */

[WorkbenchPluginAttribute(
	name: "Export Prefab Taxonomy",
	description: "Standalone prefabs exporter: extracts prefab taxonomy classifications and components to JSON.",
	category: "TBD"
)]
class TBD_PrefabsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportPrefabs m_PrefabsExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Prefabs] Starting isolated prefab taxonomy export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Prefabs] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_PrefabsExporter)
			m_PrefabsExporter = new TBD_MapExportPrefabs();

		bool ok = m_PrefabsExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Prefabs] Prefab taxonomy export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Prefabs Export",
			"Configure output destination directory for prefab taxonomy export:",
			m_Config
		);
	}
}
