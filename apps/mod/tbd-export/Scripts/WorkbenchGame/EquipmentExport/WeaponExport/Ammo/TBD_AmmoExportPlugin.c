/**
 * TBD_AmmoExportPlugin.c
 *
 * Dedicated Workbench plugin for deep intrinsic export of all ammunition & magazines across loaded addons:
 *   - Magazines: rifle mags, MG boxes/belts, handgun mags, autocannon belts, 40mm grenades, rocket munitions
 *   - Capacity & Styles: box, belt, drum, clip, single round
 *   - Caliber & Ammo Types: Ball, AP, Tracer, APTracer, HEDP, HEAT, Smoke, Illum
 *   - Pure Relational Keys: magazine_wells (links to weapon muzzles), ammo_resources (links to projectiles)
 *   - Tracer Analysis: exact round counts, ratio strings (4:1), intervals, terminal belt clusters
 *   - Projectile Ballistics: muzzle velocity (InitSpeed), variation, air drag, mass, damage values, warhead links
 *
 * Writes category JSON catalogs, master magazines/projectiles catalogs, and unified master catalog to:
 *   $profile:TBD_Export/equipment/ammunition/
 *
 * Menu: Workbench > Plugins > TBD > "Export All Ammunition & Magazines"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export All Ammunition & Magazines",
// 	description: "Deep intrinsic export of all magazines, round capacities, tracer ratios, and projectile ballistics to $profile:TBD_Export/equipment/ammunition/",
// 	category: "TBD"
// )]
class TBD_AmmoExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][AmmoExport]";

	[Attribute("$profile:TBD_Export/equipment/", UIWidgets.EditBox, "Base export directory for equipment data")]
	protected string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Include abstract / base prefabs (*_base.et) for ancestor schema linkage")]
	protected bool m_bIncludeAbstract;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Ammunition & Magazines Export...", LogLevel.NORMAL);

		TBD_EquipmentExportConfig config = new TBD_EquipmentExportConfig();
		if (!m_sDestinationDir.IsEmpty())
			config.m_sDestinationDir = m_sDestinationDir;
		config.m_bIncludeAbstract = m_bIncludeAbstract;

		TBD_AmmoScanner scanner = new TBD_AmmoScanner(config);
		int count = scanner.RunScan();
		if (count == 0)
		{
			Print(TAG + " WARNING: Scanner discovered 0 ammunition or magazine items across all categories.", LogLevel.WARNING);
			return;
		}

		int totalElapsed = System.GetTickCount() - startMs;
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 AMMUNITION & MAGAZINES EXPORT COMPLETE (%2 ms)", TAG, totalElapsed), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Items Exported (Magazines + Projectiles): %2", TAG, count), LogLevel.NORMAL);
		Print(string.Format("%1 Location: %2ammunition/", TAG, TBD_EquipmentExportPaths.NormalizeDirPath(config.m_sDestinationDir)), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
