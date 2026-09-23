/**
 * TBD_BayonetExportPlugin.c
 *
 * Dedicated Workbench plugin for deep intrinsic export of all bayonets and rifle-mounted blades
 * across loaded addons:
 *   - Rifle Bayonets (M9, 6Kh4, Vz. 58, L1A4, etc.)
 *   - Pure Relational Keys: attachment_type, compatible_attachment_types, obstructed_attachment_types
 *   - Combat & Handling Properties: extra_obstruction_length, melee_damage
 *   - Physical Dimensions, Weights, Volumes, Inventory Sizes, and 3D Model Meshes.
 *
 * Writes category JSON catalog and metadata sidecar to:
 *   $profile:TBD_Export/equipment/bayonets/
 *
 * Menu: Workbench > Plugins > TBD > "Export All Bayonets"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export All Bayonets",
// 	description: "Pure ground-truth export of all bayonets to $profile:TBD_Export/equipment/bayonets/",
// 	category: "TBD"
// )]
class TBD_BayonetExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][BayonetExport]";

	[Attribute("$profile:TBD_Export/equipment/", UIWidgets.EditBox, "Base export directory for equipment data")]
	protected string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Include abstract / base prefabs (*_base.et) for ancestor schema linkage")]
	protected bool m_bIncludeAbstract;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Bayonets Export...", LogLevel.NORMAL);

		TBD_EquipmentExportConfig config = new TBD_EquipmentExportConfig();
		if (!m_sDestinationDir.IsEmpty())
			config.m_sDestinationDir = m_sDestinationDir;
		config.m_bIncludeAbstract = m_bIncludeAbstract;

		TBD_BayonetScanner scanner = new TBD_BayonetScanner(config);
		int count = scanner.RunScan();
		if (count == 0)
		{
			Print(TAG + " WARNING: Scanner discovered 0 bayonets.", LogLevel.WARNING);
			return;
		}

		int totalElapsed = System.GetTickCount() - startMs;
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 BAYONETS EXPORT COMPLETE (%2 ms)", TAG, totalElapsed), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Bayonets Exported: %2", TAG, count), LogLevel.NORMAL);
		Print(string.Format("%1 Destination: %2bayonets/bayonets.json", TAG, config.m_sDestinationDir), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
