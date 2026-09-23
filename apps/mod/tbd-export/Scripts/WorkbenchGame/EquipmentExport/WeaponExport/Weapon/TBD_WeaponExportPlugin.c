/**
 * TBD_WeaponExportPlugin.c
 *
 * Dedicated Workbench plugin for universal deep export of all weapon categories across loaded addons:
 *   - Rifles (M16, AK-74, AKS-74U, M14/M21, SVD, VZ-58)
 *   - Machine Guns (M240, M249, M60, PKM, RPK-74, UK-59)
 *   - Handguns (M9, PM Makarov)
 *   - Launchers (M72A3 LAW, RPG-7, RPG-75, RPG-22)
 *   - Grenades (M67, RGD-5, RDG-2, AN-M8 HC)
 *   - Explosives & Mines (TM-62M, M15 AT, M14, PMN-4, Demo Blocks)
 *   - Underbarrel Systems (M203, GP-25)
 *
 * Writes category JSON catalogs and master catalog to $profile:TBD_Export/equipment/weapons/
 *
 * Menu: Workbench > Plugins > TBD > "Export All Weapons (Master Arsenal)"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export All Weapons (Master Arsenal)",
// 	description: "Deep intrinsic export of all weapons (rifles, machine guns, handguns, launchers, grenades, explosives, underbarrel) to $profile:TBD_Export/equipment/weapons/",
// 	category: "TBD"
// )]
class TBD_WeaponExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][WeaponExport]";

	[Attribute("$profile:TBD_Export/equipment/", UIWidgets.EditBox, "Base export directory for equipment data")]
	protected string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Include abstract / base prefabs (*_base.et) for ancestor schema linkage")]
	protected bool m_bIncludeAbstract;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Master Weapon Export...", LogLevel.NORMAL);

		TBD_EquipmentExportConfig config = new TBD_EquipmentExportConfig();
		if (!m_sDestinationDir.IsEmpty())
			config.m_sDestinationDir = m_sDestinationDir;
		config.m_bIncludeAbstract = m_bIncludeAbstract;

		TBD_WeaponScanner scanner = new TBD_WeaponScanner(config);
		int count = scanner.RunScan();
		if (count == 0)
		{
			Print(TAG + " WARNING: Scanner discovered 0 weapons across all categories.", LogLevel.WARNING);
			return;
		}

		int totalElapsed = System.GetTickCount() - startMs;
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 MASTER WEAPON ARSENAL EXPORT COMPLETE (%2 ms)", TAG, totalElapsed), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Weapons Exported: %2", TAG, count), LogLevel.NORMAL);
		Print(string.Format("%1 Location: %2weapons/", TAG, TBD_EquipmentExportPaths.NormalizeDirPath(config.m_sDestinationDir)), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
