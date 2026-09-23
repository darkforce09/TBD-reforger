/**
 * TBD_WearableExportPlugin.c
 *
 * Workbench plugin entry point for exporting all infantry clothing, personal protective
 * equipment, plate carriers, carry rigs, and wearables across all loaded addons.
 *
 * Menu: Workbench > Plugins > TBD > "Export Wearables & Gear"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export Wearables & Gear",
// 	description: "Universal scan: discovers and exports all wearables, clothing, armor, and gear across loaded addons to $profile:TBD_Export/equipment/wearables/",
// 	category: "TBD"
// )]
class TBD_WearableExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][WearableExportPlugin]";

	protected ref TBD_EquipmentExportConfig m_Config;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_EquipmentExportConfig();

		Print(TAG + " Starting universal wearables & gear export...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		TBD_WearableScanner scanner = new TBD_WearableScanner(m_Config);
		int totalExported = scanner.RunScan();

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("%1 EXPORT FINISHED in %2 ms: %3 total items written under %4wearables/",
			TAG, elapsedMs, totalExported, m_Config.m_sDestinationDir), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_EquipmentExportConfig();

		Workbench.ScriptDialog(
			"Configure Wearables & Gear Export",
			"Configure destination directory and scanning parameters:",
			m_Config
		);
	}
}
