/**
 * TBD_InfrastructureExportPlugin.c
 *
 * Dedicated standalone Workbench plugin for extracting tactical infrastructure:
 *   - Fences & stone walls micro-cover
 *   - Bridges & pier decks
 *   - Aviation runways & helipads
 *   - Electrical power grid & pylon graphs
 *
 * Menu: Workbench > Plugins > TBD > "Export Infrastructure"
 */

[WorkbenchPluginAttribute(
	name: "Export Infrastructure",
	description: "Standalone infrastructure exporter: extracts fences, bridges, runways, and powerlines.",
	category: "TBD"
)]
class TBD_InfrastructureExportPlugin : WorkbenchPlugin
{
	protected ref TBD_MapExportConfig m_Config;
	protected ref TBD_MapExportFences m_FencesExporter;
	protected ref TBD_MapExportBridges m_BridgesExporter;
	protected ref TBD_MapExportAviation m_AviationExporter;
	protected ref TBD_MapExportPowerlines m_PowerlinesExporter;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		int tickStart = System.GetTickCount();
		Print("[TBD][Infrastructure] Starting isolated infrastructure export...", LogLevel.NORMAL);

		TBD_MapExportPaths.EnsureDestinationDir(m_Config.m_sDestinationDir);

		TBD_MapExportContext ctx = new TBD_MapExportContext();
		if (!ctx.Init())
		{
			Print("[TBD][Infrastructure] Failed to initialize map export context - aborting.", LogLevel.ERROR);
			return;
		}

		if (!m_FencesExporter) m_FencesExporter = new TBD_MapExportFences();
		if (!m_BridgesExporter) m_BridgesExporter = new TBD_MapExportBridges();
		if (!m_AviationExporter) m_AviationExporter = new TBD_MapExportAviation();
		if (!m_PowerlinesExporter) m_PowerlinesExporter = new TBD_MapExportPowerlines();

		bool allOk = true;
		if (!m_FencesExporter.Export(ctx, m_Config)) allOk = false;
		if (!m_BridgesExporter.Export(ctx, m_Config)) allOk = false;
		if (!m_AviationExporter.Export(ctx, m_Config)) allOk = false;
		if (!m_PowerlinesExporter.Export(ctx, m_Config)) allOk = false;

		int elapsedMs = System.GetTickCount() - tickStart;
		Print(string.Format("[TBD][Infrastructure] Infrastructure export completed in %1 ms (success=%2) -> %3",
			elapsedMs, allOk, m_Config.m_sDestinationDir));
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_MapExportConfig();

		Workbench.ScriptDialog(
			"Configure Infrastructure Export",
			"Configure infrastructure export settings (fences, bridges, aviation, powerlines):",
			m_Config
		);
	}
}
