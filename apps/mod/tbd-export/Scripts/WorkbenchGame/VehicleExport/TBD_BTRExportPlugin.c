/**
 * TBD_BTRExportPlugin.c
 *
 * Dedicated Workbench plugin for deep inspection & info dump of the BTR-70 platform.
 * Delegates to the universal dynamic TBD_VehicleDeepExtractor with target filter "BTR70",
 * ensuring 100% data fidelity with zero hardcoded mock tables.
 *
 * Writes to $profile:TBD_Export/vehicles/btr70/btr70_deep_dump.json
 * Menu: Workbench > Plugins > TBD > "Export BTR-70 Deep Analysis & Info Dump"
 */

[WorkbenchPluginAttribute(
	name: "Export BTR-70 Deep Analysis & Info Dump",
	description: "Deep analysis & exhaustive info dump of BTR-70 variants to $profile:TBD_Export/vehicles/btr70/",
	category: "TBD"
)]
class TBD_BTRExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][BTRExport]";

	[Attribute("$profile:TBD_Export/vehicles/btr70/", UIWidgets.EditBox, "Destination directory for BTR-70 export")]
	protected string m_sDestinationDir;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting BTR-70 Deep Platform Analysis & Info Dump...", LogLevel.NORMAL);

		TBD_VehicleDeepExtractor extractor = new TBD_VehicleDeepExtractor();
		if (!extractor.ScanAllAddons("btr70"))
		{
			Print(TAG + " FAIL: Extractor found 0 BTR-70 variants - no files written.", LogLevel.ERROR);
			return;
		}

		TBD_VehicleDeepPlatform btrPlat = null;
		foreach (TBD_VehicleDeepPlatform p : extractor.m_aPlatforms)
		{
			string lower = p.m_sPlatformId;
			lower.ToLower();
			if (lower.Contains("btr70"))
			{
				btrPlat = p;
				break;
			}
		}

		if (!btrPlat && !extractor.m_aPlatforms.IsEmpty())
			btrPlat = extractor.m_aPlatforms[0];

		if (!btrPlat || btrPlat.m_aVariants.IsEmpty())
		{
			Print(TAG + " FAIL: BTR-70 platform record empty.", LogLevel.ERROR);
			return;
		}

		string outDir = m_sDestinationDir;
		if (outDir.IsEmpty())
			outDir = "$profile:TBD_Export/vehicles/btr70/";
		outDir = TBD_VehicleExportPaths.NormalizeDirPath(outDir);
		TBD_VehicleExportPaths.EnsureDestinationDir(outDir);

		string outJson = outDir + "btr70_deep_dump.json";
		string outMeta = outDir + "btr70_deep_dump_meta.json";

		// 1. Write full JSON
		FileHandle f = FileIO.OpenFile(outJson, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " FAIL: Could not open " + outJson + " for writing.", LogLevel.ERROR);
			return;
		}

		string jsonContent = TBD_VehicleDeepSerializer.SerializePlatformToJson(btrPlat);
		bool ok = TBD_VehicleExportJson.Write(f, jsonContent, TAG);
		f.Close();

		if (!ok)
		{
			FileIO.DeleteFile(outJson);
			Print(TAG + " FAIL: Failed writing to " + outJson, LogLevel.ERROR);
			return;
		}

		// 2. Write metadata
		FileHandle mf = FileIO.OpenFile(outMeta, FileMode.WRITE);
		if (mf)
		{
			int elapsedMs = System.GetTickCount() - startMs;
			string meta = "{\n";
			meta += "  \"platform\": \"BTR-70\",\n";
			meta += "  \"generatedAt\": \"" + TBD_VehicleExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"elapsedMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"variantsCount\": " + btrPlat.m_aVariants.Count().ToString() + ",\n";
			meta += "  \"dataFile\": \"btr70_deep_dump.json\"\n";
			meta += "}\n";

			TBD_VehicleExportJson.Write(mf, meta, TAG);
			mf.Close();
		}

		// 3. Print report to console
		PrintSummaryReport(btrPlat, outJson, System.GetTickCount() - startMs);
	}

	//------------------------------------------------------------------------------------------------
	protected void PrintSummaryReport(TBD_VehicleDeepPlatform btrPlat, string outJson, int elapsedMs)
	{
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 BTR-70 VEHICLE DEEP ENGINEERING REPORT (%2 ms)", TAG, elapsedMs), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total BTR-70 Variants: %2", TAG, btrPlat.m_aVariants.Count()), LogLevel.NORMAL);

		foreach (TBD_VehicleDeepVariant v : btrPlat.m_aVariants)
		{
			string pwrStr = "N/A";
			if (v.m_Drivetrain && v.m_Drivetrain.m_fEnginePeakPowerHp > 0)
				pwrStr = string.Format("%1 hp", Math.Round(v.m_Drivetrain.m_fEnginePeakPowerHp));

			Print(string.Format("%1   * %2 [%3] - Mass: %4 kg | Seats: %5 | Turrets: %6 | HitZones: %7 | Power: %8",
				TAG, v.m_sDisplayName, v.m_sRole, v.m_fWeightKg, v.m_aSeats.Count(),
				v.m_aTurrets.Count(), v.m_aHitZones.Count(), pwrStr), LogLevel.NORMAL);
		}

		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Written to: %2", TAG, outJson), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
