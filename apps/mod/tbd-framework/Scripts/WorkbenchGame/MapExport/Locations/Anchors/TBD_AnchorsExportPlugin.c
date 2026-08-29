/**
 * TBD_AnchorsExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting Georeferencing Anchors:
 *   - Ground-truth coordinates and orientation oracle
 *
 * Menu: Workbench > Plugins > TBD > "Export Georeferencing Anchors"
 */

[WorkbenchPluginAttribute(
	name: "Export Georeferencing Anchors",
	description: "Standalone anchors exporter: extracts ground-truth georeferencing anchor oracle to JSON.",
	category: "TBD"
)]
class TBD_AnchorsExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportAnchors m_AnchorsExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Anchors] Starting isolated georeferencing anchors export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Anchors] Failed to initialize map export context — aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_AnchorsExporter)
			m_AnchorsExporter = new TBD_MapExportAnchors();

		bool ok = m_AnchorsExporter.Export(ctx, m_Config);
		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Anchors] Anchors export completed in %1 ms (success=%2) -> %3",
			elapsedMs, ok, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Anchors Export",
			"Configure output destination directory for georeferencing anchors export:",
			m_Config
		);
	}
}
