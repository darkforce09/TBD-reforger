/**
 * TBD_MuzzleExportPlugin.c
 *
 * Dedicated Workbench plugin for deep intrinsic export of all muzzle devices across loaded addons:
 *   - Sound Suppressors & Silencers (M16 Suppressor, PBS-4, etc.)
 *   - Flash Hiders, Muzzle Brakes & Compensators (A2 Flash Hider, 6P20 Muzzle Brake, 6P26, etc.)
 *   - Pure Relational Keys: attachment_type, compatible_attachment_types, obstructed_attachment_types
 *   - Ground-Truth Acoustics & Modifiers: is_suppressed, speed coefficients, dispersion factors,
 *     extra obstruction lengths, effect/shot overrides, and recoil/turn factor vectors.
 *   - Physical Dimensions, Weights, Volumes, Inventory Sizes, and 3D Model Meshes.
 *
 * Writes category JSON catalog and metadata sidecar to:
 *   $profile:TBD_Export/equipment/muzzles/
 *
 * Menu: Workbench > Plugins > TBD > "Export All Muzzle Devices"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export All Muzzle Devices",
// 	description: "Pure ground-truth export of all muzzle devices, suppressors, and flash hiders to $profile:TBD_Export/equipment/muzzles/",
// 	category: "TBD"
// )]
class TBD_MuzzleExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][MuzzleExport]";

	[Attribute("$profile:TBD_Export/equipment/", UIWidgets.EditBox, "Base export directory for equipment data")]
	protected string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Include abstract / base prefabs (*_base.et) for ancestor schema linkage")]
	protected bool m_bIncludeAbstract;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Muzzle Devices Export...", LogLevel.NORMAL);

		TBD_EquipmentExportConfig config = new TBD_EquipmentExportConfig();
		if (!m_sDestinationDir.IsEmpty())
			config.m_sDestinationDir = m_sDestinationDir;
		config.m_bIncludeAbstract = m_bIncludeAbstract;

		TBD_MuzzleScanner scanner = new TBD_MuzzleScanner(config);
		int count = scanner.RunScan();
		if (count == 0)
		{
			Print(TAG + " WARNING: Scanner discovered 0 muzzle devices.", LogLevel.WARNING);
			return;
		}

		int totalElapsed = System.GetTickCount() - startMs;
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 MUZZLE DEVICES EXPORT COMPLETE (%2 ms)", TAG, totalElapsed), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Muzzle Devices Exported: %2", TAG, count), LogLevel.NORMAL);
		Print(string.Format("%1 Destination: %2muzzles/muzzles.json", TAG, config.m_sDestinationDir), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
