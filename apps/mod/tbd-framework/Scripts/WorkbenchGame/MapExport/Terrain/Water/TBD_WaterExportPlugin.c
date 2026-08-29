/**
 * TBD_WaterExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Water data:
 *   - Global ocean bathymetry sampling
 *   - Procedural LakeGeneratorEntity vector extraction
 *   - Procedural RiverEntity / RiverPartEntity vector extraction
 *   - High-resolution raster mask & 3D depth matrix burn-in
 *
 * Menu: Workbench > Plugins > TBD > "Export Water Data"
 */

[WorkbenchPluginAttribute(
	name: "Export Water Data",
	description: "Standalone water exporter: extracts vector lake polygons, river splines, and high-res bathymetry depth matrix.",
	category: "TBD"
)]
class TBD_WaterExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Water] Starting isolated water data export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Water] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		bool ok = TBD_MapExportWater.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Water] Water export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Water Export",
			"Configure water export settings and resolution:",
			m_Config
		);
	}
}
