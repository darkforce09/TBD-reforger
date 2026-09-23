/**
 * TBD_IlluminatorExportPlugin.c
 *
 * Dedicated Workbench plugin for deep intrinsic export of all weapon-mounted tactical lights,
 * IR illuminators, and laser aiming modules across loaded addons:
 *   - Tactical weapon flashlights, searchlights, and headlamps
 *   - Visible and infrared (IR) laser pointers and aiming modules (PEQ, DBAL, Klesh, etc.)
 *   - Multi-mode combo light/laser illumination modules
 *   - Pure Relational Keys: attachment_type, compatible_attachment_types, obstructed_attachment_types
 *   - Ground-Truth Illumination Parameters: emissive_intensity, light_near_plane, adjust_offset,
 *     lenses (description, color, light_value)
 *   - Ground-Truth Laser Parameters: has_laser, is_ir, laser_color
 *   - Physical Dimensions, Weights, Volumes, Inventory Sizes, and 3D Model Meshes.
 *
 * Writes category JSON catalog and metadata sidecar to:
 *   $profile:TBD_Export/equipment/illuminators/
 *
 * Menu: Workbench > Plugins > TBD > "Export All Tactical Lights & Pointers"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export All Tactical Lights & Pointers",
// 	description: "Pure ground-truth export of all tactical weapon lights, IR illuminators, and laser aiming modules to $profile:TBD_Export/equipment/illuminators/",
// 	category: "TBD"
// )]
class TBD_IlluminatorExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][IlluminatorExport]";

	[Attribute("$profile:TBD_Export/equipment/", UIWidgets.EditBox, "Base export directory for equipment data")]
	protected string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Include abstract / base prefabs (*_base.et) for ancestor schema linkage")]
	protected bool m_bIncludeAbstract;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Tactical Lights & Pointers Export...", LogLevel.NORMAL);

		TBD_EquipmentExportConfig config = new TBD_EquipmentExportConfig();
		if (!m_sDestinationDir.IsEmpty())
			config.m_sDestinationDir = m_sDestinationDir;
		config.m_bIncludeAbstract = m_bIncludeAbstract;

		TBD_IlluminatorScanner scanner = new TBD_IlluminatorScanner(config);
		int count = scanner.RunScan();
		if (count == 0)
		{
			Print(TAG + " WARNING: Scanner discovered 0 tactical lights or pointers.", LogLevel.WARNING);
			return;
		}

		int totalElapsed = System.GetTickCount() - startMs;
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 TACTICAL LIGHTS & POINTERS EXPORT COMPLETE (%2 ms)", TAG, totalElapsed), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Illuminators Exported: %2", TAG, count), LogLevel.NORMAL);
		Print(string.Format("%1 Destination: %2illuminators/illuminators.json", TAG, config.m_sDestinationDir), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
