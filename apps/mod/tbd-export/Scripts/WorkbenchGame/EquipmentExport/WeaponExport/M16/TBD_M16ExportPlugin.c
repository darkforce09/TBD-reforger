/**
 * TBD_M16ExportPlugin.c
 *
 * Dedicated Workbench plugin for deep inspection of the M16 platform:
 *   - Extracts all M16 rifle variants
 *   - Inspects muzzles, magazine wells, fire modes, and zeroing
 *   - Resolves all compatible STANAG magazines with capacities
 *   - Resolves all compatible optics (e.g. Colt 4x20, AP2000) and other slot attachments
 *   - Writes to $profile:TBD_Export/equipment/m16_deep_export.json
 *
 * Menu: Workbench > Plugins > TBD > "Export M16 Deep Analysis & Compatibility"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export M16 Deep Analysis & Compatibility",
// 	description: "Deep analysis of M16 variants: extracts muzzles, fire modes, zeroing, compatible STANAG magazines, and compatible optics/attachments to $profile:TBD_Export/equipment/m16_deep_export.json",
// 	category: "TBD"
// )]
class TBD_M16ExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][M16Export]";

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting M16 Deep Platform Analysis...", LogLevel.NORMAL);

		TBD_M16DeepScanner scanner = new TBD_M16DeepScanner();
		if (!scanner.RunDeepScan())
		{
			Print(TAG + " FAIL: Scanner found 0 M16 variants - no files written.", LogLevel.ERROR);
			return;
		}

		string outDir = "$profile:TBD_Export/equipment/";
		TBD_EquipmentExportPaths.EnsureDestinationDir(outDir);

		string outJson = outDir + "m16_deep_export.json";
		string outMeta = outDir + "m16_deep_export_meta.json";

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
			meta += "  \"platform\": \"M16\",\n";
			meta += "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"elapsedMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"variantsCount\": " + scanner.m_aM16Variants.Count().ToString() + ",\n";
			meta += "  \"magazinesInPool\": " + scanner.m_aAllMagazines.Count().ToString() + ",\n";
			meta += "  \"attachmentsInPool\": " + scanner.m_aAllAttachments.Count().ToString() + ",\n";
			meta += "  \"dataFile\": \"m16_deep_export.json\"\n";
			meta += "}\n";

			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}

		// 3. Print comprehensive report to Workbench console
		PrintSummaryReport(scanner, outJson, System.GetTickCount() - startMs);
	}

	//------------------------------------------------------------------------------------------------
	protected void PrintSummaryReport(TBD_M16DeepScanner scanner, string outJson, int elapsedMs)
	{
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 M16 PLATFORM DEEP COMPATIBILITY REPORT (%2 ms)", TAG, elapsedMs), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Total M16 Variants Analyzed: %2", TAG, scanner.m_aM16Variants.Count()), LogLevel.NORMAL);

		// Print representative base rifle findings
		TBD_M16WeaponVariantInfo baseRifle = null;
		foreach (TBD_M16WeaponVariantInfo v : scanner.m_aM16Variants)
		{
			if (v.m_sFilePath.EndsWith("Rifle_M16A2.et"))
			{
				baseRifle = v;
				break;
			}
		}

		if (!baseRifle && !scanner.m_aM16Variants.IsEmpty())
			baseRifle = scanner.m_aM16Variants[0];

		if (baseRifle)
		{
			Print(string.Format("%1 [Target Platform]: %2 (%3)", TAG, baseRifle.m_sDisplayName, baseRifle.m_sId), LogLevel.NORMAL);
			Print(string.Format("%1   Weight: %2 kg | Volume: %3 cm3", TAG, baseRifle.m_fWeightKg, baseRifle.m_fVolumeCm3), LogLevel.NORMAL);

			// Muzzles & Magazines
			foreach (TBD_M16MuzzleInfo muz : baseRifle.m_aMuzzles)
			{
				string wells = "";
				for (int w = 0; w < muz.m_aMagazineWells.Count(); w++)
				{
					wells += muz.m_aMagazineWells[w];
					if (w < muz.m_aMagazineWells.Count() - 1) wells += ", ";
				}
				Print(string.Format("%1   Muzzle #%2 Magazine Wells: [%3]", TAG, muz.m_iIndex, wells), LogLevel.NORMAL);
				Print(string.Format("%1   Compatible Magazines Found: %2", TAG, muz.m_aCompatibleMagResourceNames.Count()), LogLevel.NORMAL);

				int showCount = muz.m_aCompatibleMagDisplayNames.Count();
				if (showCount > 8) showCount = 8;
				for (int cm = 0; cm < showCount; cm++)
				{
					Print(string.Format("%1     * %2 (%3 rounds)", TAG, muz.m_aCompatibleMagDisplayNames[cm], muz.m_aCompatibleMagCapacities[cm]), LogLevel.NORMAL);
				}
				if (muz.m_aCompatibleMagDisplayNames.Count() > 8)
					Print(string.Format("%1     ... and %2 more magazines", TAG, muz.m_aCompatibleMagDisplayNames.Count() - 8), LogLevel.NORMAL);
			}

			// Attachment Slots & Compatible Optics
			Print(string.Format("%1   Attachment Slots Count: %2", TAG, baseRifle.m_aAttachmentSlots.Count()), LogLevel.NORMAL);
			foreach (TBD_M16AttachmentSlotInfo slot : baseRifle.m_aAttachmentSlots)
			{
				Print(string.Format("%1   -> Slot \"%2\" (Requires: %3)", TAG, slot.m_sSlotName, slot.m_sRequiredAttachType), LogLevel.NORMAL);
				Print(string.Format("%1      Compatible Items Found: %2", TAG, slot.m_aCompatibleItemResourceNames.Count()), LogLevel.NORMAL);

				int showAtt = slot.m_aCompatibleItemDisplayNames.Count();
				if (showAtt > 6) showAtt = 6;
				for (int ca = 0; ca < showAtt; ca++)
				{
					Print(string.Format("%1        + %2", TAG, slot.m_aCompatibleItemDisplayNames[ca]), LogLevel.NORMAL);
				}
				if (slot.m_aCompatibleItemDisplayNames.Count() > 6)
					Print(string.Format("%1        ... and %2 more items", TAG, slot.m_aCompatibleItemDisplayNames.Count() - 6), LogLevel.NORMAL);
			}
		}

		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Wrote complete dataset to: %2", TAG, outJson), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
	}
}
