/**
 * TBD_RifleExportPlugin.c
 *
 * Dedicated Workbench plugin for generic deep inspection of all Rifles across loaded addons:
 *   - Scans all rifle prefabs (M16, AK-74, AKS-74U, M14, SVD, VZ-58, and modded rifles)
 *   - Extracts pure intrinsic data: muzzles, magazine wells, fire modes, attachment slots,
 *     required attachment types, pre-attached attachments, zeroing distances, mass, volume
 *   - No hardcoding; normalized data model for downstream linking with ammo and attachment catalogs
 *   - Writes to $profile:TBD_Export/equipment/rifles.json
 *
 * Menu: Workbench > Plugins > TBD > "Export All Rifles (Deep Intrinsic)"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export All Rifles (Deep Intrinsic)",
// 	description: "Generic deep extraction of all rifles: muzzles, magazine wells, fire modes, attachment slots, and required attachment types to $profile:TBD_Export/equipment/rifles.json",
// 	category: "TBD"
// )]
class TBD_RifleExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][RifleExport]";

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting Generic Deep Rifles Scan...", LogLevel.NORMAL);

		TBD_RifleScanner scanner = new TBD_RifleScanner();
		int count = scanner.RunScan();
		if (count == 0)
		{
			Print(TAG + " FAIL: Scanner found 0 rifles - no files written.", LogLevel.ERROR);
			return;
		}

		string outDir = "$profile:TBD_Export/equipment/";
		TBD_EquipmentExportPaths.EnsureDestinationDir(outDir);

		string outJson = outDir + "rifles.json";
		string outMeta = outDir + "rifles_meta.json";

		// 1. Write full JSON
		FileHandle f = FileIO.OpenFile(outJson, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " FAIL: Could not open " + outJson + " for writing.", LogLevel.ERROR);
			return;
		}

		string jsonContent = scanner.SerializeToJson();
		bool ok = TBD_EquipmentExportJson.Write(f, jsonContent, TAG);
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
			meta += "  \"category\": \"rifles\",\n";
			meta += "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"elapsedMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"totalCount\": " + count.ToString() + ",\n";
			meta += "  \"dataFile\": \"rifles.json\"\n";
			meta += "}\n";

			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}

		// 3. Print report to console
		int totalElapsed = System.GetTickCount() - startMs;
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 RIFLE DEEP INTRINSIC EXPORT COMPLETE (%2 ms)", TAG, totalElapsed), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total Rifles Exported: %2", TAG, count), LogLevel.NORMAL);
		Print(string.Format("%1 Written to: %2", TAG, outJson), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
