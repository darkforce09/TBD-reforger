/**
 * TBD_OpticExportPlugin.c
 *
 * Dedicated Workbench plugin for deep intrinsic export of all optics & sights across loaded addons:
 *   - Non-magnified reflex, collimator, and holographic sights
 *   - Fixed-power and variable-power telescopic sniper & combat scopes
 *   - Thermal, night vision, and launcher sights
 *   - Pure Relational Keys: attachment_type, compatible_attachment_types, obstructed_attachment_types
 *   - Ground-Truth Optical Parameters: sights_class, magnification steps & min/max, FOV angles,
 *     eye relief, objective diameter, zeroing distances, reticle textures, RGBA colors, rangefinder
 *   - Physical Dimensions, Weights, Volumes, Inventory Sizes, and 3D Model Meshes.
 *
 * Writes category JSON catalog and metadata sidecar to:
 *   $profile:TBD_Export/equipment/optics/
 *
 * Menu: Workbench > Plugins > TBD > "Export All Optics & Sights"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export All Optics & Sights",
// 	description: "Pure ground-truth export of all optics, scopes, collimators, and sights to $profile:TBD_Export/equipment/optics/",
// 	category: "TBD"
// )]
class TBD_OpticExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][OpticExport]";

	[Attribute("$profile:TBD_Export/equipment/", UIWidgets.EditBox, "Base export directory for equipment data")]
	protected string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Include abstract / base prefabs (*_base.et) for ancestor schema linkage")]
	protected bool m_bIncludeAbstract;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Optics & Sights Export...", LogLevel.NORMAL);

		TBD_EquipmentExportConfig config = new TBD_EquipmentExportConfig();
		if (!m_sDestinationDir.IsEmpty())
			config.m_sDestinationDir = m_sDestinationDir;
		config.m_bIncludeAbstract = m_bIncludeAbstract;

		TBD_OpticScanner scanner = new TBD_OpticScanner(config);
		int count = scanner.RunScan();
		if (count == 0)
		{
			Print(TAG + " WARNING: Scanner discovered 0 optics or sights.", LogLevel.WARNING);
			return;
		}

		int totalElapsed = System.GetTickCount() - startMs;
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 OPTICS & SIGHTS EXPORT COMPLETE (%2 ms)", TAG, totalElapsed), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Optics Exported: %2", TAG, count), LogLevel.NORMAL);
		Print(string.Format("%1 Destination: %2optics/optics.json", TAG, config.m_sDestinationDir), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
