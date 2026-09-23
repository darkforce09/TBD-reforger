/**
 * TBD_HandguardExportPlugin.c
 *
 * Dedicated Workbench plugin for deep intrinsic export of all weapon handguards, rail systems,
 * and foregrips across loaded addons:
 *   - Standard & Modular Handguards (M16A2 round ribbed, RIS rail systems, M-LOK, Zenitco)
 *   - Foregrips (Vertical grips, angled foregrips)
 *   - Pure Relational Keys: attachment_type, compatible_attachment_types, obstructed_attachment_types
 *   - Nested Attachment Host: child rail slots for optics, UGLs, bipods, and illuminators
 *   - Handling & Recoil Modifiers: angular/linear recoil factors, turn factors, extra obstruction length
 *   - Physical Dimensions, Weights, Volumes, Inventory Sizes, and 3D Model Meshes.
 *
 * Writes category JSON catalog and metadata sidecar to:
 *   $profile:TBD_Export/equipment/handguards/
 *
 * Menu: Workbench > Plugins > TBD > "Export All Handguards & Foregrips"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export All Handguards & Foregrips",
// 	description: "Pure ground-truth export of all handguards & foregrips to $profile:TBD_Export/equipment/handguards/",
// 	category: "TBD"
// )]
class TBD_HandguardExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][HandguardExport]";

	[Attribute("$profile:TBD_Export/equipment/", UIWidgets.EditBox, "Base export directory for equipment data")]
	protected string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Include abstract / base prefabs (*_base.et) for ancestor schema linkage")]
	protected bool m_bIncludeAbstract;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Handguards & Foregrips Export...", LogLevel.NORMAL);

		TBD_EquipmentExportConfig config = new TBD_EquipmentExportConfig();
		if (!m_sDestinationDir.IsEmpty())
			config.m_sDestinationDir = m_sDestinationDir;
		config.m_bIncludeAbstract = m_bIncludeAbstract;

		TBD_HandguardScanner scanner = new TBD_HandguardScanner(config);
		int count = scanner.RunScan();
		if (count == 0)
		{
			Print(TAG + " WARNING: Scanner discovered 0 handguards or foregrips.", LogLevel.WARNING);
			return;
		}

		int totalElapsed = System.GetTickCount() - startMs;
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 HANDGUARDS & FOREGRIPS EXPORT COMPLETE (%2 ms)", TAG, totalElapsed), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Handguards & Foregrips Exported: %2", TAG, count), LogLevel.NORMAL);
		Print(string.Format("%1 Destination: %2handguards/handguards.json", TAG, config.m_sDestinationDir), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
