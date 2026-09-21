/**
 * TBD_VehicleDeepExportPlugin.c
 *
 * Dedicated Workbench plugin for universal deep engineering data export across all vehicle
 * platforms and variants in all loaded addons.
 *
 * Discovers and exports:
 *   - Master summary matrix: $profile:TBD_Export/vehicles/vehicles_summary.json
 *   - Master catalog index: $profile:TBD_Export/vehicles/vehicles_catalog.json
 *   - Per-platform engineering specs: $profile:TBD_Export/vehicles/<platform_id>/<platform_id>_deep.json
 *   - Preserves backward-compatible alias: $profile:TBD_Export/vehicles/btr70/btr70_deep_dump.json
 *
 * Menu: Workbench > Plugins > TBD > "Export All Vehicles Deep Engineering Data (Universal Discovery)"
 */

[WorkbenchPluginAttribute(
	name: "Export All Vehicles Deep Engineering Data (Universal Discovery)",
	description: "Scan and export deep engineering telemetry for all vehicle platforms and variants to $profile:TBD_Export/vehicles/",
	category: "TBD"
)]
class TBD_VehicleDeepExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][VehicleDeepExport]";

	[Attribute("$profile:TBD_Export/vehicles/", UIWidgets.EditBox, "Base destination directory for vehicle engineering export")]
	protected string m_sDestinationDir;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Universal Vehicle Deep Engineering Export...", LogLevel.NORMAL);

		TBD_VehicleDeepExtractor extractor = new TBD_VehicleDeepExtractor();
		if (!extractor.ScanAllAddons())
		{
			Print(TAG + " FAIL: Extractor found 0 vehicles across loaded addons - no files written.", LogLevel.ERROR);
			return;
		}

		string outDir = m_sDestinationDir;
		if (outDir.IsEmpty())
			outDir = "$profile:TBD_Export/vehicles/";
		outDir = TBD_VehicleExportPaths.NormalizeDirPath(outDir);
		TBD_VehicleExportPaths.EnsureDestinationDir(outDir);

		// 1. Write Master Summary Matrix
		string outSummary = outDir + "vehicles_summary.json";
		FileHandle fSum = FileIO.OpenFile(outSummary, FileMode.WRITE);
		if (fSum)
		{
			string sumJson = TBD_VehicleDeepSerializer.SerializeSummaryToJson(extractor.m_aPlatforms, extractor.m_aAllVariants.Count());
			TBD_VehicleExportJson.Write(fSum, sumJson, TAG);
			fSum.Close();
		}

		// 2. Write Per-Platform Deep Engineering Specifications
		foreach (TBD_VehicleDeepPlatform plat : extractor.m_aPlatforms)
		{
			string platSlug = plat.m_sPlatformId;
			platSlug.ToLower();
			string platDir = outDir + platSlug + "/";
			TBD_VehicleExportPaths.EnsureDestinationDir(platDir);

			string platJsonPath = platDir + platSlug + "_deep.json";
			FileHandle fPlat = FileIO.OpenFile(platJsonPath, FileMode.WRITE);
			if (fPlat)
			{
				string platContent = TBD_VehicleDeepSerializer.SerializePlatformToJson(plat);
				TBD_VehicleExportJson.Write(fPlat, platContent, TAG);
				fPlat.Close();

				// Backward-compatibility alias for BTR-70
				if (platSlug == "btr70")
				{
					string btrAlias = platDir + "btr70_deep_dump.json";
					FileHandle fAlias = FileIO.OpenFile(btrAlias, FileMode.WRITE);
					if (fAlias)
					{
						TBD_VehicleExportJson.Write(fAlias, platContent, TAG);
						fAlias.Close();
					}
				}
			}
		}

		// 3. Write Master Metadata File
		string outMeta = outDir + "vehicles_deep_meta.json";
		FileHandle fMeta = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (fMeta)
		{
			int elapsedMs = System.GetTickCount() - startMs;
			string meta = "{\n";
			meta += "  \"pipeline\": \"Universal Vehicle Deep Engineering Export\",\n";
			meta += "  \"generatedAt\": \"" + TBD_VehicleExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"elapsedMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"totalPlatforms\": " + extractor.m_aPlatforms.Count().ToString() + ",\n";
			meta += "  \"totalVariants\": " + extractor.m_aAllVariants.Count().ToString() + ",\n";
			meta += "  \"summaryFile\": \"vehicles_summary.json\"\n";
			meta += "}\n";

			TBD_VehicleExportJson.Write(fMeta, meta, TAG);
			fMeta.Close();
		}

		// 4. Output Summary Report to Workbench Console
		PrintSummaryReport(extractor, System.GetTickCount() - startMs);
	}

	//------------------------------------------------------------------------------------------------
	protected void PrintSummaryReport(TBD_VehicleDeepExtractor extractor, int elapsedMs)
	{
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 UNIVERSAL VEHICLE DEEP EXPORT COMPLETE (%2 ms)", TAG, elapsedMs), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Platforms: %2 | Total Variants: %3",
			TAG, extractor.m_aPlatforms.Count(), extractor.m_aAllVariants.Count()), LogLevel.NORMAL);
		Print("------------------------------------------------------------------------", LogLevel.NORMAL);

		foreach (TBD_VehicleDeepPlatform plat : extractor.m_aPlatforms)
		{
			Print(string.Format("%1   [Platform] %2 (%3) - %4 variants | Domain: %5 | Faction: %6",
				TAG, plat.m_sDisplayName, plat.m_sPlatformId, plat.m_aVariants.Count(), plat.m_sVehicleDomain, plat.m_sPrimaryFaction), LogLevel.NORMAL);

			int showCount = plat.m_aVariants.Count();
			if (showCount > 3) showCount = 3;

			for (int v = 0; v < showCount; v++)
			{
				TBD_VehicleDeepVariant varData = plat.m_aVariants[v];
				string pwrStr = "N/A";
				if (varData.m_Drivetrain && varData.m_Drivetrain.m_fEnginePeakPowerHp > 0)
					pwrStr = string.Format("%1 hp", Math.Round(varData.m_Drivetrain.m_fEnginePeakPowerHp));

				Print(string.Format("%1     * %2 [%3] (Mass: %4 kg, Seats: %5, Turrets: %6, HitZones: %7, Power: %8)",
					TAG, varData.m_sDisplayName, varData.m_sRole, varData.m_fWeightKg, varData.m_aSeats.Count(),
					varData.m_aTurrets.Count(), varData.m_aHitZones.Count(), pwrStr), LogLevel.NORMAL);
			}
			if (plat.m_aVariants.Count() > 3)
				Print(string.Format("%1     ... and %2 more variants", TAG, plat.m_aVariants.Count() - 3), LogLevel.NORMAL);
		}

		Print("========================================================================", LogLevel.NORMAL);
	}
}
