/**
 * TBD_AttachmentExportPlugin.c
 *
 * Dedicated Workbench plugin for deep intrinsic export of all non-optic weapon attachments across loaded addons:
 *   - Muzzle Devices & Suppressors: silencers, flash hiders, compensators, brakes
 *   - Bipods & Grips: deployable bipods, vertical foregrips, angled grips, handstops
 *   - Handguards & Rail Systems: custom handguards, RIS/RAS systems with nested child slots
 *   - Tactical Lights & Lasers: weapon lights, visible lasers, IR designators, combo modules
 *   - Bayonets: M9, 6Kh4, Vz. 58, L1A4 bayonets
 *   - Stocks & Buttstocks: fixed, folding, telescoping, wire stocks
 *   - Mount Adapters: Dovetail side rail adapters, Picatinny risers, cantilever mounts
 *   - Camouflage Wraps: weapon and optic fabric wraps, ghillie concealment covers
 *   - Pure Relational Keys: attachment_type, compatible_attachment_types, obstructed_attachment_types
 *   - Complete Technical Specs: acoustics, flash reduction, recoil scaling, stability, nested slots,
 *     lighting/laser properties, melee blade metrics, length-of-pull, physical mass/volume, 3D meshes.
 *
 * Writes category JSON catalogs, master attachments catalog, and metadata sidecars to:
 *   $profile:TBD_Export/equipment/attachments/
 *
 * Menu: Workbench > Plugins > TBD > "Export All Weapon Attachments"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export All Weapon Attachments",
// 	description: "Deep intrinsic export of all weapon attachments (muzzles, suppressors, bipods, grips, handguards, illuminators, bayonets, stocks, mounts, camouflage) to $profile:TBD_Export/equipment/attachments/",
// 	category: "TBD"
// )]
class TBD_AttachmentExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][AttachmentExport]";

	[Attribute("$profile:TBD_Export/equipment/", UIWidgets.EditBox, "Base export directory for equipment data")]
	protected string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Include abstract / base prefabs (*_base.et) for ancestor schema linkage")]
	protected bool m_bIncludeAbstract;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Weapon Attachments Export...", LogLevel.NORMAL);

		TBD_EquipmentExportConfig config = new TBD_EquipmentExportConfig();
		if (!m_sDestinationDir.IsEmpty())
			config.m_sDestinationDir = m_sDestinationDir;
		config.m_bIncludeAbstract = m_bIncludeAbstract;

		TBD_AttachmentScanner scanner = new TBD_AttachmentScanner(config);
		int count = scanner.RunScan();
		if (count == 0)
		{
			Print(TAG + " WARNING: Scanner discovered 0 weapon attachments across all categories.", LogLevel.WARNING);
			return;
		}

		int totalElapsed = System.GetTickCount() - startMs;
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 WEAPON ATTACHMENTS EXPORT COMPLETE (%2 ms)", TAG, totalElapsed), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Attachments Exported: %2", TAG, count), LogLevel.NORMAL);
		Print(string.Format("%1 Location: %2attachments/", TAG, TBD_EquipmentExportPaths.NormalizeDirPath(config.m_sDestinationDir)), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
