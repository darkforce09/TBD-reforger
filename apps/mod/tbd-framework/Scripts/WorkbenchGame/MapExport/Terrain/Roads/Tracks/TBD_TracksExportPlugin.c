/**
 * TBD_TracksExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Forestry & Agricultural Track network data.
 *
 * Menu: Workbench > Plugins > TBD > "Export Forestry & Agricultural Tracks"
 */

[WorkbenchPluginAttribute(
	name: "Export Forestry & Agricultural Tracks",
	description: "Standalone track exporter: extracts forestry tracks, tractor trails, and rough two-tracks with continuous spline geometry to tracks.json.",
	category: "TBD"
)]
class TBD_TracksExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportTracks m_Exporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Roads][Tracks] Starting isolated tracks export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Roads][Tracks] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_Exporter)
			m_Exporter = new TBD_MapExportTracks();

		bool ok = m_Exporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Roads][Tracks] Tracks export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Tracks Export",
			"Configure forestry and agricultural track network export parameters:",
			m_Config
		);
	}
}
