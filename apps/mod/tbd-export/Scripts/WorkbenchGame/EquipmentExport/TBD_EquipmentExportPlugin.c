/**
 * TBD_EquipmentExportPlugin.c
 *
 * Unified master Workbench plugin for exporting all Reforger equipment in a single operation:
 *   - Phase 1: Weapons (Rifles, MGs, Handguns, Launchers, Flares, Heavy Weapons, Grenades, Explosives, Underbarrel)
 *   - Phase 2: Static Weapons & Emplacements (Mortars, Tripods, Mounts)
 *   - Phase 3: Wearables & Protective Gear (Uniforms, Vests, Helmets, Backpacks, Clothing)
 *   - Phase 4: Inventory Items, Tools & Crates (Medical, Radios, Navigation, Tools, Survival, Ammo Boxes)
 *   - Phase 5: Weapon Attachments (Muzzles, Bipods, Handguards, Illuminators, Bayonets, Stocks, Mounts)
 *   - Phase 6: Optics & Sights (Combat Scopes, Collimators, Magnifiers, Reticles)
 *   - Phase 7: Ammunition & Magazines (Magazines, Round Capacities, Tracers, Projectiles)
 *   - Phase 8: Master Discovery Catalog (equipment_all.json, equipment_meta.json)
 *
 * Menu: Workbench > Plugins > TBD > "Export All Equipment"
 */

[WorkbenchPluginAttribute(
	name: "Export All Equipment",
	description: "Universal one-click export: Weapons (incl. Flares & Heavy), Statics (Mortars & Tripods), Wearables, Items, Attachments, Optics, Ammo, and Master Discovery catalog.",
	category: "TBD"
)]
class TBD_EquipmentExportPlugin : WorkbenchPlugin
{
	protected static const string TAG = "[TBD][MasterEquipmentExport]";
	protected static const int FLUSH_SIZE = 8000;

	[Attribute("$profile:TBD_Export/equipment/", UIWidgets.EditBox, "Base export directory for equipment data")]
	protected string m_sDestinationDir;

	[Attribute("1", UIWidgets.CheckBox, "Include abstract / base prefabs (*_base.et) for ancestor schema linkage")]
	protected bool m_bIncludeAbstract;

	protected ref TBD_EquipmentExportConfig m_Config;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		if (!m_Config)
			m_Config = new TBD_EquipmentExportConfig();

		if (!m_sDestinationDir.IsEmpty())
			m_Config.m_sDestinationDir = m_sDestinationDir;
		m_Config.m_bIncludeAbstract = m_bIncludeAbstract;

		int totalStartMs = System.GetTickCount();
		Print("========================================================================", LogLevel.NORMAL);
		Print(TAG + " STARTING MASTER EQUIPMENT EXPORT PIPELINE", LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);

		// Phase 1: Weapons (Rifles, MGs, Handguns, Launchers, Flares, Heavy Weapons, Grenades, Explosives, Underbarrel)
		int t0 = System.GetTickCount();
		Print(TAG + " [Phase 1/8] Exporting weapons...", LogLevel.NORMAL);
		TBD_WeaponScanner weaponScanner = new TBD_WeaponScanner(m_Config);
		int weaponCount = weaponScanner.RunScan();
		Print(string.Format("%1 [Phase 1/8] Weapons completed: %2 items in %3 ms", TAG, weaponCount, System.GetTickCount() - t0), LogLevel.NORMAL);

		// Phase 2: Static Weapons & Emplacements (Mortars, Tripods, Mounts)
		t0 = System.GetTickCount();
		Print(TAG + " [Phase 2/8] Exporting static & crew-served weapons...", LogLevel.NORMAL);
		TBD_StaticWeaponScanner staticScanner = new TBD_StaticWeaponScanner(m_Config);
		int staticCount = staticScanner.RunScan();
		Print(string.Format("%1 [Phase 2/8] Statics completed: %2 items in %3 ms", TAG, staticCount, System.GetTickCount() - t0), LogLevel.NORMAL);

		// Phase 3: Wearables & Protective Gear (Clothing, Armor, Vests, Helmets, Backpacks)
		t0 = System.GetTickCount();
		Print(TAG + " [Phase 3/8] Exporting wearables & gear...", LogLevel.NORMAL);
		TBD_WearableScanner wearableScanner = new TBD_WearableScanner(m_Config);
		int wearableCount = wearableScanner.RunScan();
		Print(string.Format("%1 [Phase 3/8] Wearables completed: %2 items in %3 ms", TAG, wearableCount, System.GetTickCount() - t0), LogLevel.NORMAL);

		// Phase 4: Inventory Items, Tools & Crates (Medical, Radios, Survival, Tools, Weapon Parts, Ammo Boxes)
		t0 = System.GetTickCount();
		Print(TAG + " [Phase 4/8] Exporting inventory items & tools...", LogLevel.NORMAL);
		TBD_ItemScanner itemScanner = new TBD_ItemScanner(m_Config);
		int itemCount = itemScanner.RunScan();
		Print(string.Format("%1 [Phase 4/8] Items completed: %2 items in %3 ms", TAG, itemCount, System.GetTickCount() - t0), LogLevel.NORMAL);

		// Phase 5: Weapon Attachments (Muzzles, Bipods, Handguards, Illuminators, Bayonets, Stocks)
		t0 = System.GetTickCount();
		Print(TAG + " [Phase 5/8] Exporting weapon attachments...", LogLevel.NORMAL);
		TBD_AttachmentScanner attachmentScanner = new TBD_AttachmentScanner(m_Config);
		int attachmentCount = attachmentScanner.RunScan();
		Print(string.Format("%1 [Phase 5/8] Attachments completed: %2 items in %3 ms", TAG, attachmentCount, System.GetTickCount() - t0), LogLevel.NORMAL);

		// Phase 6: Optics & Sights (Combat Scopes, Collimators, Magnifiers, Reticles)
		t0 = System.GetTickCount();
		Print(TAG + " [Phase 6/8] Exporting optics & sights...", LogLevel.NORMAL);
		TBD_OpticScanner opticScanner = new TBD_OpticScanner(m_Config);
		int opticCount = opticScanner.RunScan();
		Print(string.Format("%1 [Phase 6/8] Optics completed: %2 items in %3 ms", TAG, opticCount, System.GetTickCount() - t0), LogLevel.NORMAL);

		// Phase 7: Ammunition & Magazines (Magazines, Round Capacities, Tracers, Projectiles)
		t0 = System.GetTickCount();
		Print(TAG + " [Phase 7/8] Exporting ammunition & magazines...", LogLevel.NORMAL);
		TBD_AmmoScanner ammoScanner = new TBD_AmmoScanner(m_Config);
		int ammoCount = ammoScanner.RunScan();
		Print(string.Format("%1 [Phase 7/8] Ammunition completed: %2 items in %3 ms", TAG, ammoCount, System.GetTickCount() - t0), LogLevel.NORMAL);

		// Phase 8: Master Discovery Catalog (equipment_all.json)
		t0 = System.GetTickCount();
		Print(TAG + " [Phase 8/8] Running master equipment discovery scan...", LogLevel.NORMAL);
		TBD_EquipmentScanner discoveryScanner = new TBD_EquipmentScanner(m_Config);
		int discoveryCount = 0;
		if (discoveryScanner.ScanAllAddons())
		{
			string outDir = TBD_EquipmentExportPaths.NormalizeDirPath(m_Config.m_sDestinationDir);
			TBD_EquipmentExportPaths.EnsureDestinationDir(outDir);

			string outJson = outDir + "equipment_all.json";
			string outMeta = outDir + "equipment_meta.json";

			if (WriteEquipmentJson(discoveryScanner, outJson))
				WriteMetaJson(discoveryScanner, outMeta, t0);

			discoveryCount = discoveryScanner.m_aDiscoveredItems.Count();
		}
		Print(string.Format("%1 [Phase 8/8] Master discovery completed: %2 items in %3 ms", TAG, discoveryCount, System.GetTickCount() - t0), LogLevel.NORMAL);

		int totalElapsedMs = System.GetTickCount() - totalStartMs;
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 MASTER EQUIPMENT EXPORT COMPLETE (%2 ms)", TAG, totalElapsedMs), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
		Print(string.Format("%1 Subsystems Summary:", TAG), LogLevel.NORMAL);
		Print(string.Format("%1   - Weapons (Handheld, Flares, Heavy): %2", TAG, weaponCount), LogLevel.NORMAL);
		Print(string.Format("%1   - Statics (Mortars, Tripods, Mounts): %2", TAG, staticCount), LogLevel.NORMAL);
		Print(string.Format("%1   - Wearables & Gear:                  %2", TAG, wearableCount), LogLevel.NORMAL);
		Print(string.Format("%1   - Inventory Items & Tools:           %2", TAG, itemCount), LogLevel.NORMAL);
		Print(string.Format("%1   - Attachments:                       %2", TAG, attachmentCount), LogLevel.NORMAL);
		Print(string.Format("%1   - Optics & Sights:                   %2", TAG, opticCount), LogLevel.NORMAL);
		Print(string.Format("%1   - Ammunition & Magazines:            %2", TAG, ammoCount), LogLevel.NORMAL);
		Print(string.Format("%1   - Master Discovery Total:            %2", TAG, discoveryCount), LogLevel.NORMAL);
		Print(string.Format("%1 Output Directory: %2", TAG, m_Config.m_sDestinationDir), LogLevel.NORMAL);
		Print("========================================================================", LogLevel.NORMAL);
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
