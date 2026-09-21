/**
 * TBD_VehicleCatalogExportPlugin.c
 *
 * Dedicated Workbench plugin for universal cataloging of all vehicles and variants:
 *   - Discovers all vehicle platforms (BTR-70, Ural-4320, M923A1, M998 Humvee, UAZ-469, etc.)
 *   - Filters out non-vehicle sub-parts (turrets, mounts, seats, destruction models, lights)
 *   - Categorizes tactical roles, seating capacities, and mounted armaments
 *   - Writes master catalog to $profile:TBD_Export/vehicles/vehicles_all.json
 *   - Writes summary catalog to $profile:TBD_Export/vehicles/vehicles_summary.json
 *   - Writes metadata to $profile:TBD_Export/vehicles/vehicles_meta.json
 *
 * Menu: Workbench > Plugins > TBD > "Export All Vehicles Catalog (Platforms & Variants)"
 */

[WorkbenchPluginAttribute(
	name: "Export All Vehicles Catalog (Platforms & Variants)",
	description: "Scan and export all vehicle platforms and variants across loaded addons to $profile:TBD_Export/vehicles/",
	category: "TBD"
)]
class TBD_VehicleCatalogExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][VehicleCatalogExport]";

	[Attribute("$profile:TBD_Export/vehicles/", UIWidgets.EditBox, "Base export directory for vehicle catalog")]
	protected string m_sDestinationDir;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Vehicle Catalog Export...", LogLevel.NORMAL);

		TBD_VehicleCatalogScanner scanner = new TBD_VehicleCatalogScanner();
		if (!scanner.ScanAllAddons())
		{
			Print(TAG + " FAIL: Scanner found 0 vehicle platforms - no files written.", LogLevel.ERROR);
			return;
		}

		string outDir = m_sDestinationDir;
		if (outDir.IsEmpty())
			outDir = "$profile:TBD_Export/vehicles/";
		outDir = TBD_VehicleExportPaths.NormalizeDirPath(outDir);
		TBD_VehicleExportPaths.EnsureDestinationDir(outDir);

		string outAllJson = outDir + "vehicles_all.json";
		string outSummaryJson = outDir + "vehicles_summary.json";
		string outMetaJson = outDir + "vehicles_meta.json";

		// 1. Write master catalog (platforms & variants)
		FileHandle fAll = FileIO.OpenFile(outAllJson, FileMode.WRITE);
		if (!fAll)
		{
			Print(TAG + " FAIL: Could not open " + outAllJson + " for writing.", LogLevel.ERROR);
			return;
		}
		string allContent = scanner.SerializeAllToJson();
		bool okAll = TBD_VehicleExportJson.Write(fAll, allContent, TAG);
		fAll.Close();

		if (!okAll)
		{
			FileIO.DeleteFile(outAllJson);
			Print(TAG + " FAIL: Failed writing to " + outAllJson, LogLevel.ERROR);
			return;
		}

		// 2. Write summary catalog
		FileHandle fSum = FileIO.OpenFile(outSummaryJson, FileMode.WRITE);
		if (fSum)
		{
			string sumContent = scanner.SerializeSummaryToJson();
			TBD_VehicleExportJson.Write(fSum, sumContent, TAG);
			fSum.Close();
		}

		// 3. Write metadata
		FileHandle fMeta = FileIO.OpenFile(outMetaJson, FileMode.WRITE);
		if (fMeta)
		{
			int elapsedMs = System.GetTickCount() - startMs;
			string meta = "{\n";
			meta += "  \"catalog\": \"Vehicles\",\n";
			meta += "  \"generatedAt\": \"" + TBD_VehicleExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"elapsedMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"totalPlatforms\": " + scanner.m_aPlatforms.Count().ToString() + ",\n";
			meta += "  \"totalVariants\": " + scanner.m_aAllVariants.Count().ToString() + ",\n";
			meta += "  \"masterFile\": \"vehicles_all.json\",\n";
			meta += "  \"summaryFile\": \"vehicles_summary.json\"\n";
			meta += "}\n";

			TBD_VehicleExportJson.Write(fMeta, meta, TAG);
			fMeta.Close();
		}

		// 4. Print comprehensive summary matrix to console
		PrintSummaryReport(scanner, outAllJson, System.GetTickCount() - startMs);
	}

	//------------------------------------------------------------------------------------------------
	protected void PrintSummaryReport(TBD_VehicleCatalogScanner scanner, string outJson, int elapsedMs)
	{
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 UNIVERSAL VEHICLE CATALOG EXPORT COMPLETE (%2 ms)", TAG, elapsedMs), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Platforms: %2 | Total Variants: %3", TAG, scanner.m_aPlatforms.Count(), scanner.m_aAllVariants.Count()), LogLevel.NORMAL);
		Print("------------------------------------------------------------------------", LogLevel.NORMAL);

		foreach (TBD_VehiclePlatformInfo plat : scanner.m_aPlatforms)
		{
			Print(string.Format("%1   [Platform] %2 (%3) - %4 variants | Domain: %5 | Faction: %6",
				TAG, plat.m_sDisplayName, plat.m_sPlatformId, plat.m_aVariants.Count(), plat.m_sVehicleDomain, plat.m_sPrimaryFaction), LogLevel.NORMAL);

			int showCount = plat.m_aVariants.Count();
			if (showCount > 4) showCount = 4;
			for (int v = 0; v < showCount; v++)
			{
				TBD_VehicleVariantInfo varInfo = plat.m_aVariants[v];
				string armedStr = "Unarmed";
				if (varInfo.m_bIsArmed && !varInfo.m_aMountedWeapons.IsEmpty())
					armedStr = "Armed (" + varInfo.m_aMountedWeapons[0] + ")";

				Print(string.Format("%1     * %2 [%3] (Seats: %4, %5)",
					TAG, varInfo.m_sDisplayName, varInfo.m_sRole, varInfo.m_iTotalSeats, armedStr), LogLevel.NORMAL);
			}
			if (plat.m_aVariants.Count() > 4)
				Print(string.Format("%1     ... and %2 more variants", TAG, plat.m_aVariants.Count() - 4), LogLevel.NORMAL);
		}

		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Master Catalog written to: %2", TAG, outJson), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
