/**
 * TBD_StumpsExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting tree stumps & forestry trunks:
 *   - Queries world entities for authentic stump prefabs, cut trunks, logs, and woodpiles
 *   - Discards standing living trees, bushes, plants, crops, rocks, and non-forestry props
 *   - Writes valid JSON document to $profile:TBD_Export/<mapName>/vegetation/stumps.json
 *
 * Menu: Workbench > Plugins > TBD > "Export Stumps (stumps.json)"
 */

[WorkbenchPluginAttribute(
	name: "Export Stumps (stumps.json)",
	description: "Standalone stump exporter: extracts tree stumps, cut forestry trunks, and wood logs to stumps.json.",
	category: "TBD"
)]
class TBD_StumpsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportStumps m_StumpsExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Stumps] Starting isolated stump export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Stumps] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_StumpsExporter)
			m_StumpsExporter = new TBD_MapExportStumps();

		bool ok = m_StumpsExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Stumps] Stump export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Stump Export",
			"Configure destination directory and spatial chunk parameters for stump export:",
			m_Config
		);
	}
}
