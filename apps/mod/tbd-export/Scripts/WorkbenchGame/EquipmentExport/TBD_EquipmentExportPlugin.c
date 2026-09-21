/**
 * TBD_EquipmentExportPlugin.c
 *
 * Workbench plugin for unfiltered equipment discovery export.
 * Scans all loaded addons, filters out vehicles/characters/props,
 * and exports discovered equipment to $profile:TBD_Export/equipment/equipment_all.json.
 *
 * Menu: Workbench > Plugins > TBD > "Export All Equipment (Discovery)"
 */

// [WorkbenchPluginAttribute(
// 	name: "Export All Equipment (Discovery)",
// 	description: "Unfiltered scan: discovers all equipment across loaded addons and exports to $profile:TBD_Export/equipment/equipment_all.json",
// 	category: "TBD"
// )]
class TBD_EquipmentExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][EquipmentExport]";
	protected static const int FLUSH_SIZE = 8000;

	protected ref TBD_EquipmentExportConfig m_Config;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_EquipmentExportConfig();

		int startMs = System.GetTickCount();
		Print(TAG + " Starting unfiltered equipment discovery scan...", LogLevel.NORMAL);

		TBD_EquipmentScanner scanner = new TBD_EquipmentScanner(m_Config);
		if (!scanner.ScanAllAddons())
		{
			Print(TAG + " FAIL: Scan returned 0 equipment items - no files written.", LogLevel.ERROR);
			return;
		}

		string outDir = TBD_EquipmentExportPaths.NormalizeDirPath(m_Config.m_sDestinationDir);
		TBD_EquipmentExportPaths.EnsureDestinationDir(outDir);

		string outJson = outDir + "equipment_all.json";
		string outMeta = outDir + "equipment_meta.json";

		// Write equipment_all.json
		if (!WriteEquipmentJson(scanner, outJson))
			return;

		// Write equipment_meta.json
		WriteMetaJson(scanner, outMeta, startMs);

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("%1 DISCOVERY COMPLETE in %2 ms -> %3", TAG, elapsedMs, outJson), LogLevel.NORMAL);
		Print(string.Format("%1 Total Equipment: %2 items (Weapons: %3, Magazines: %4, Clothing/Armor: %5, Gadgets: %6, Attachments: %7, Ammo: %8, Other Inventory: %9)",
			TAG, scanner.m_aDiscoveredItems.Count(),
			scanner.m_iWeaponsCount, scanner.m_iMagazinesCount, scanner.m_iClothingCount,
			scanner.m_iGadgetsCount, scanner.m_iAttachmentsCount, scanner.m_iAmmoCount,
			scanner.m_iOtherInventoryCount), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	override void Configure()
	{
		if (!m_Config)
			m_Config = new TBD_EquipmentExportConfig();

		Workbench.ScriptDialog(
			"Configure Equipment Export",
			"Configure destination directory and equipment scanning parameters:",
			m_Config
		);
	}

	//------------------------------------------------------------------------------------------------
	protected bool WriteEquipmentJson(TBD_EquipmentScanner scanner, string filePath)
	{
		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open " + filePath + " for write", LogLevel.ERROR);
			return false;
		}

		string buf = "{\n";
		buf += "  \"version\": \"1\",\n";
		buf += "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
		buf += "  \"totalCount\": " + scanner.m_aDiscoveredItems.Count().ToString() + ",\n";
		buf += "  \"items\": [\n";

		bool writeOk = true;
		for (int i = 0; i < scanner.m_aDiscoveredItems.Count(); i++)
		{
			TBD_EquipmentScanItem item = scanner.m_aDiscoveredItems[i];
			buf += item.ToJson("    ");
			if (i < scanner.m_aDiscoveredItems.Count() - 1)
				buf += ",";
			buf += "\n";

			if (buf.Length() > FLUSH_SIZE)
			{
				writeOk = TBD_EquipmentExportJson.Write(f, buf, TAG);
				if (!writeOk)
					break;
				buf = "";
			}
		}

		if (writeOk)
		{
			buf += "  ]\n}\n";
			writeOk = TBD_EquipmentExportJson.Write(f, buf, TAG);
		}

		f.Close();

		if (!writeOk)
		{
			FileIO.DeleteFile(filePath);
			Print(TAG + " Write failed - deleted partial " + filePath, LogLevel.ERROR);
			return false;
		}

		Print(string.Format("%1 Wrote %2 equipment items to %3", TAG, scanner.m_aDiscoveredItems.Count(), filePath));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteMetaJson(TBD_EquipmentScanner scanner, string filePath, int startMs)
	{
		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
			return;

		int elapsedMs = System.GetTickCount() - startMs;
		string mj = "{\n";
		mj += "  \"method\": \"unfiltered-equipment-discovery-scan\",\n";
		mj += "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
		mj += "  \"elapsedMs\": " + elapsedMs.ToString() + ",\n";
		mj += "  \"scannedPrefabs\": " + scanner.m_iSeen.ToString() + ",\n";
		mj += "  \"skippedDenyPaths\": " + scanner.m_iSkippedDeny.ToString() + ",\n";
		mj += "  \"skippedNonEquipment\": " + scanner.m_iSkippedNonEquipment.ToString() + ",\n";
		mj += "  \"failedLoadPrefabs\": " + scanner.m_iFailedLoad.ToString() + ",\n";
		mj += "  \"totalEquipment\": " + scanner.m_aDiscoveredItems.Count().ToString() + ",\n";
		mj += "  \"breakdown\": {\n";
		mj += "    \"weapons\": " + scanner.m_iWeaponsCount.ToString() + ",\n";
		mj += "    \"magazines\": " + scanner.m_iMagazinesCount.ToString() + ",\n";
		mj += "    \"clothingAndArmor\": " + scanner.m_iClothingCount.ToString() + ",\n";
		mj += "    \"gadgets\": " + scanner.m_iGadgetsCount.ToString() + ",\n";
		mj += "    \"attachments\": " + scanner.m_iAttachmentsCount.ToString() + ",\n";
		mj += "    \"ammo\": " + scanner.m_iAmmoCount.ToString() + ",\n";
		mj += "    \"otherInventory\": " + scanner.m_iOtherInventoryCount.ToString() + "\n";
		mj += "  }\n";
		mj += "}\n";

		TBD_EquipmentExportJson.Write(f, mj, TAG);
		f.Close();
	}
}
