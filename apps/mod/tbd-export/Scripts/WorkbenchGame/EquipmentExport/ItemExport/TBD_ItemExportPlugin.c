/**
 * TBD_ItemExportPlugin.c
 *
 * Workbench plugin entry point for exporting all infantry inventory items, medical supplies,
 * communication radios, navigation tools, binoculars, flashlights, tools, explosives,
 * and survival gear across all loaded addons.
 *
 * Menu: Workbench > Plugins > TBD > "Export Inventory Items"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export Inventory Items",
// 	description: "Universal scan: discovers and exports all inventory items across loaded addons to $profile:TBD_Export/equipment/items/",
// 	category: "TBD"
// )]
class TBD_ItemExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][ItemExportPlugin]";

	protected ref TBD_EquipmentExportConfig m_Config;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_EquipmentExportConfig();

		Print(TAG + " Starting universal inventory item export...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		TBD_ItemScanner scanner = new TBD_ItemScanner(m_Config);
		int totalExported = scanner.RunScan();

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("%1 EXPORT FINISHED in %2 ms: %3 total items written under %4items/",
			TAG, elapsedMs, totalExported, m_Config.m_sDestinationDir), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_EquipmentExportConfig();

		Workbench.ScriptDialog(
			"Configure Inventory Item Export",
			"Configure destination directory and scanning parameters:",
			m_Config
		);
	}
}
