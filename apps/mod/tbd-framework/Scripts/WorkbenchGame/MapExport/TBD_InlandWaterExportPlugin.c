/**
 * TBD_InlandWaterExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for fast inland water export:
 *   - Procedural & placed inland lakes, ponds, rivers, and mountain streams
 *   - Ground-truth multi-altitude water surface & bathymetry probing
 *   - Skips deep ocean cells to provide rapid iteration in seconds
 *
 * Menu: Workbench > Plugins > TBD > "Export Inland Water (Lakes & Rivers)"
 */

[WorkbenchPluginAttribute(
	name: "Export Inland Water (Lakes & Rivers)",
	description: "Fast standalone inland water exporter: extracts ground-truth lakes, ponds, and river networks in seconds.",
	category: "TBD"
)]
class TBD_InlandWaterExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][InlandWater] Starting fast inland water data export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][InlandWater] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		bool ok = TBD_MapExportInlandWater.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][InlandWater] Inland water export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Inland Water Export",
			"Configure inland water export settings and resolution:",
			m_Config
		);
	}
}
