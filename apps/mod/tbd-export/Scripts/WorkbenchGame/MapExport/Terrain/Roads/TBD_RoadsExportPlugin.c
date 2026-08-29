/**
 * TBD_RoadsExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting complete Road network data across all types:
 *   - Highways & Major Arterials (highways.json)
 *   - Secondary Paved Roads (roads_paved.json)
 *   - Dirt & Gravel Roads (roads_dirt.json)
 *   - Forestry & Agricultural Tracks (tracks.json)
 *   - Footpaths & Trails (paths.json)
 *   - Airfield Runways & Taxiways (runways.json)
 *   - Consolidated Manifest (roads_meta.json)
 *
 * Menu: Workbench > Plugins > TBD > "Export Roads (All Types)"
 */

[WorkbenchPluginAttribute(
	name: "Export Roads (All Types)",
	description: "Standalone road exporter: extracts highways, paved roads, dirt roads, tracks, footpaths, and runways to modular per-type JSON files with continuous spline geometry.",
	category: "TBD"
)]
class TBD_RoadsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Roads] Starting isolated road network export suite...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Roads] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		bool ok = TBD_MapExportRoads.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Roads] Roads export suite completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Roads Export",
			"Configure road network export settings and per-type layers:",
			m_Config
		);
	}
}
