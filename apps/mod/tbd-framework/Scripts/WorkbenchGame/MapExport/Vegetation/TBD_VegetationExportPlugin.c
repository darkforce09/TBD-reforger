/**
 * TBD_VegetationExportPlugin.c
 *
 * Standalone Workbench plugin for extracting natural vegetation & geological features:
 *   - Trees & Forest Canopies
 *   - Rock Formations & Cliffs
 *   - Bushes & Undergrowth
 *
 * Menu: Workbench > Plugins > TBD > "Export Vegetation (Trees, Rocks, Bushes)"
 */

[WorkbenchPluginAttribute(
	name: "Export Vegetation (Trees, Rocks, Bushes)",
	description: "Standalone vegetation exporter: extracts trees, rock formations, bushes, and natural ground clutter.",
	category: "TBD"
)]
class TBD_VegetationExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Vegetation] Starting isolated vegetation export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Vegetation] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		bool ok = TBD_MapExportVegetation.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Vegetation] Vegetation export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Vegetation Export",
			"Configure vegetation export parameters (trees, rocks, bushes):",
			m_Config
		);
	}
}
