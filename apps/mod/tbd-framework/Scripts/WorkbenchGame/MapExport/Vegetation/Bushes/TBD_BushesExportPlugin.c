/**
 * TBD_BushesExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting placed bushes:
 *   - Queries world entities for authentic bush prefabs (Prefabs/Vegetation/Bush/b_*)
 *   - Discards trees, plants, stumps, and bay location descriptors
 *   - Writes valid JSON document to $profile:TBD_Export/<mapName>/vegetation/bush.json
 *
 * Menu: Workbench > Plugins > TBD > "Export Bushes (bush.json)"
 */

[WorkbenchPluginAttribute(
	name: "Export Bushes (bush.json)",
	description: "Standalone bush exporter: extracts all authentic natural bushes to bush.json.",
	category: "TBD"
)]
class TBD_BushesExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportBushes m_BushesExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Bushes] Starting isolated bush export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Bushes] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_BushesExporter)
			m_BushesExporter = new TBD_MapExportBushes();

		bool ok = m_BushesExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Bushes] Bush export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Bush Export",
			"Configure destination directory and spatial chunk parameters for bush export:",
			m_Config
		);
	}
}
