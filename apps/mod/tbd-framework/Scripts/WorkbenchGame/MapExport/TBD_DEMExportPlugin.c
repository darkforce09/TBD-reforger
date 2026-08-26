/**
 * TBD_DEMExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting DEM Elevation Heightmaps:
 *   - 16-bit resampled ground elevation matrix
 *
 * Menu: Workbench > Plugins > TBD > "Export DEM Heightmap"
 */

[WorkbenchPluginAttribute(
	name: "Export DEM Heightmap",
	description: "Standalone DEM exporter: extracts 16-bit terrain heightmap matrix and metadata.",
	category: "TBD"
)]
class TBD_DEMExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][DEM] Starting isolated DEM heightmap export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][DEM] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		bool ok = TBD_MapExportDEM.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][DEM] DEM export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure DEM Export",
			"Configure DEM elevation heightmap export settings and resolution:",
			m_Config
		);
	}
}
