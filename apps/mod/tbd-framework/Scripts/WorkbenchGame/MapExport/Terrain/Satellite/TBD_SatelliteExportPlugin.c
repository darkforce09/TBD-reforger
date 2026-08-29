/**
 * TBD_SatelliteExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Satellite rasterization:
 *   - Orthophoto satellite ground textures (.tga)
 *
 * Menu: Workbench > Plugins > TBD > "Export Satellite Imagery"
 */

[WorkbenchPluginAttribute(
	name: "Export Satellite Imagery",
	description: "Standalone satellite exporter: rasterizes top-down orthographic satellite imagery to TGA.",
	category: "TBD"
)]
class TBD_SatelliteExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Satellite] Starting isolated satellite export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Satellite] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		bool ok = TBD_MapExportSatellite.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Satellite] Satellite export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Satellite Export",
			"Configure satellite imagery export settings:",
			m_Config
		);
	}
}
