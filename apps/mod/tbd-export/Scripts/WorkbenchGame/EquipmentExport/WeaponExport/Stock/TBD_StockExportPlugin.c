/**
 * TBD_StockExportPlugin.c
 *
 * Dedicated Workbench plugin for deep intrinsic export of all weapon buttstocks and stock assemblies
 * across loaded addons:
 *   - Fixed, Folding, and Retractable / Collapsible Buttstocks (AK wooden/folding, Vz.58, M16 fixed, M4 carbine stocks)
 *   - Pure Relational Keys: attachment_type, compatible_attachment_types, obstructed_attachment_types
 *   - Nested Attachment Host: child slots for cheek pads, cheek risers, sling swivels, etc.
 *   - Handling & Recoil Modifiers: recoil angular/linear factors, turn factors, extra obstruction length
 *   - Physical Dimensions, Weights, Volumes, Inventory Sizes, and 3D Model Meshes.
 *
 * Writes category JSON catalog and metadata sidecar to:
 *   $profile:TBD_Export/equipment/stocks/
 *
 * Menu: Workbench > Plugins > TBD > "Export All Buttstocks"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export All Buttstocks",
// 	description: "Pure ground-truth export of all buttstocks to $profile:TBD_Export/equipment/stocks/",
// 	category: "TBD"
// )]
class TBD_StockExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][StockExport]";

	[Attribute("$profile:TBD_Export/equipment/", UIWidgets.EditBox, "Base export directory for equipment data")]
	protected string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Include abstract / base prefabs (*_base.et) for ancestor schema linkage")]
	protected bool m_bIncludeAbstract;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Buttstocks Export...", LogLevel.NORMAL);

		TBD_EquipmentExportConfig config = new TBD_EquipmentExportConfig();
		if (!m_sDestinationDir.IsEmpty())
			config.m_sDestinationDir = m_sDestinationDir;
		config.m_bIncludeAbstract = m_bIncludeAbstract;

		TBD_StockScanner scanner = new TBD_StockScanner(config);
		int count = scanner.RunScan();
		if (count == 0)
		{
			Print(TAG + " WARNING: Scanner discovered 0 buttstocks.", LogLevel.WARNING);
			return;
		}

		int totalElapsed = System.GetTickCount() - startMs;
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 BUTTSTOCKS EXPORT COMPLETE (%2 ms)", TAG, totalElapsed), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Buttstocks Exported: %2", TAG, count), LogLevel.NORMAL);
		Print(string.Format("%1 Destination: %2stocks/stocks.json", TAG, config.m_sDestinationDir), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
