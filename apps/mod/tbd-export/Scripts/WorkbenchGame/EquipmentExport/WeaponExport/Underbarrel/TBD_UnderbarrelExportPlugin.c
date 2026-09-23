/**
 * TBD_UnderbarrelExportPlugin.c
 *
 * Dedicated Workbench plugin for deep intrinsic export of all underbarrel devices across loaded addons:
 *   - Underbarrel Grenade Launchers (M203, GP-25, etc.)
 *   - Underbarrel Grips, Bipods, and Mounting Accessories
 *   - Pure Relational Keys: attachment_type, compatible_attachment_types, obstructed_attachment_types
 *   - Secondary Weapon System Properties: is_launcher, magazine_wells, chamber_capacity, zeroing_distances
 *   - Physical Dimensions, Weights, Volumes, Inventory Sizes, and 3D Model Meshes.
 *
 * Writes category JSON catalog and metadata sidecar to:
 *   $profile:TBD_Export/equipment/underbarrel/
 *
 * Menu: Workbench > Plugins > TBD > "Export All Underbarrel Devices"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export All Underbarrel Devices",
// 	description: "Pure ground-truth export of all underbarrel devices and grenade launchers to $profile:TBD_Export/equipment/underbarrel/",
// 	category: "TBD"
// )]
class TBD_UnderbarrelExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][UnderbarrelExport]";

	[Attribute("$profile:TBD_Export/equipment/", UIWidgets.EditBox, "Base export directory for equipment data")]
	protected string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Include abstract / base prefabs (*_base.et) for ancestor schema linkage")]
	protected bool m_bIncludeAbstract;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Underbarrel Devices Export...", LogLevel.NORMAL);

		TBD_EquipmentExportConfig config = new TBD_EquipmentExportConfig();
		if (!m_sDestinationDir.IsEmpty())
			config.m_sDestinationDir = m_sDestinationDir;
		config.m_bIncludeAbstract = m_bIncludeAbstract;

		TBD_UnderbarrelScanner scanner = new TBD_UnderbarrelScanner(config);
		int count = scanner.RunScan();
		if (count == 0)
		{
			Print(TAG + " WARNING: Scanner discovered 0 underbarrel devices.", LogLevel.WARNING);
			return;
		}

		int totalElapsed = System.GetTickCount() - startMs;
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 UNDERBARREL DEVICES EXPORT COMPLETE (%2 ms)", TAG, totalElapsed), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Underbarrel Devices Exported: %2", TAG, count), LogLevel.NORMAL);
		Print(string.Format("%1 Destination: %2underbarrel/underbarrel.json", TAG, config.m_sDestinationDir), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
