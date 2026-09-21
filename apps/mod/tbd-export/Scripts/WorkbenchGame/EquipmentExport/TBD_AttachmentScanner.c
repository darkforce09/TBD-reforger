//------------------------------------------------------------------------------------------------
// TBD_AttachmentScanner.c
//
// Universal weapon attachments scanner across all loaded addons for Arma Reforger Workbench.
// Categorizes, introspects, and exports all non-optic weapon attachments (muzzle devices & suppressors,
// bipods & grips, handguards & rail systems with nested slots, tactical lights & lasers, bayonets,
// and stocks & buttstocks) to dedicated JSON catalogs and a unified master catalog at
// $profile:TBD_Export/equipment/attachments/.
//------------------------------------------------------------------------------------------------

class TBD_AttachmentScanner
{
	protected ref TBD_EquipmentExportConfig m_Config;

	protected ref array<ref TBD_AttachmentInfo> m_aMuzzles = {};
	protected ref array<ref TBD_AttachmentInfo> m_aBipods = {};
	protected ref array<ref TBD_AttachmentInfo> m_aHandguards = {};
	protected ref array<ref TBD_AttachmentInfo> m_aIlluminators = {};
	protected ref array<ref TBD_AttachmentInfo> m_aBayonets = {};
	protected ref array<ref TBD_AttachmentInfo> m_aStocks = {};
	protected ref array<ref TBD_AttachmentInfo> m_aMounts = {};
	protected ref array<ref TBD_AttachmentInfo> m_aCamouflage = {};
	protected ref array<ref TBD_AttachmentInfo> m_aAllAttachments = {};

	protected ref set<string> m_SeenResourceNames = new set<string>();

	protected string m_sCurrentAddonId;

	//------------------------------------------------------------------------------------------------
	void TBD_AttachmentScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run universal weapon attachments scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print("[TBD][AttachmentExport] Starting universal weapon attachments scan across all loaded addons...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		// Target paths containing attachments across addons
		array<string> scanPaths = {
			"Prefabs/Weapons/Attachments",
			"Prefabs/Items/Equipment/Flashlights",
			"Prefabs/Items/Equipment/Bipods"
		};

		foreach (string relPath : scanPaths)
		{
			foreach (string guid : guids)
			{
				string addonId = GameProject.GetAddonID(guid);
				m_sCurrentAddonId = addonId;
				string rootPath = "$" + addonId + ":" + relPath;
				Workbench.SearchResources(OnResourceFound, extEt, null, rootPath, true);
			}
		}

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("[TBD][AttachmentExport] Universal scan finished in %1 ms. Total attachments: %2.", elapsedMs, m_aAllAttachments.Count()), LogLevel.NORMAL);
		Print(string.Format("[TBD][AttachmentExport] Breakdown: Muzzles=%1, Bipods/Grips=%2, Handguards=%3, Illuminators=%4, Bayonets=%5, Stocks=%6, Mounts=%7, Camouflage=%8",
			m_aMuzzles.Count(), m_aBipods.Count(), m_aHandguards.Count(), m_aIlluminators.Count(), m_aBayonets.Count(), m_aStocks.Count(), m_aMounts.Count(), m_aCamouflage.Count()), LogLevel.NORMAL);

		// Write all JSON catalogs to disk
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllAttachments.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aMuzzles.Clear();
		m_aBipods.Clear();
		m_aHandguards.Clear();
		m_aIlluminators.Clear();
		m_aBayonets.Clear();
		m_aStocks.Clear();
		m_aMounts.Clear();
		m_aCamouflage.Clear();
		m_aAllAttachments.Clear();
		m_SeenResourceNames.Clear();
	}

	//------------------------------------------------------------------------------------------------
	protected bool IsOpticPrefab(string lowerPath, map<string, ref array<BaseContainer>> comps)
	{
		if (lowerPath.Contains("/optics/") || lowerPath.Contains("optic_"))
			return true;
		if (lowerPath.Contains("collim_") || lowerPath.Contains("scope_"))
			return true;
		if (TBD_AttachmentExtractor.HasCompSuffix(comps, "SCR_OpticComponent"))
			return true;
		if (TBD_AttachmentExtractor.HasCompSuffix(comps, "SCR_2DOpticComponent"))
			return true;
		if (TBD_AttachmentExtractor.HasCompSuffix(comps, "OpticComponent"))
			return true;
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected bool IsUnderbarrelLauncher(string lowerPath, map<string, ref array<BaseContainer>> comps)
	{
		if (lowerPath.Contains("/underbarrel/"))
			return true;
		if (TBD_AttachmentExtractor.HasCompSuffix(comps, "WeaponComponent") && TBD_AttachmentExtractor.HasCompSuffix(comps, "MuzzleInMagComponent"))
			return true;
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected bool IsAttachmentPath(string lowerPath)
	{
		if (lowerPath.Contains("/attachments/"))
			return true;
		if (lowerPath.Contains("/muzzle/") || lowerPath.Contains("/muzzles/"))
			return true;
		if (lowerPath.Contains("/bayonet/") || lowerPath.Contains("/bayonets/"))
			return true;
		if (lowerPath.Contains("/stock/") || lowerPath.Contains("/stocks/"))
			return true;
		if (lowerPath.Contains("/handguard/") || lowerPath.Contains("/handguards/"))
			return true;
		if (lowerPath.Contains("/bipod/") || lowerPath.Contains("/bipods/"))
			return true;
		if (lowerPath.Contains("/flashlight/") || lowerPath.Contains("/flashlights/"))
			return true;
		if (lowerPath.Contains("/laser/") || lowerPath.Contains("/lasers/"))
			return true;
		if (lowerPath.Contains("/mounts/") || lowerPath.Contains("/mount/"))
			return true;
		if (lowerPath.Contains("/camouflage/") || lowerPath.Contains("wrap"))
			return true;
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected bool HasAttachmentComponent(map<string, ref array<BaseContainer>> comps)
	{
		if (TBD_AttachmentExtractor.HasCompSuffix(comps, "SuppressorComponent"))
			return true;
		if (TBD_AttachmentExtractor.HasCompSuffix(comps, "BipodComponent"))
			return true;
		if (TBD_AttachmentExtractor.HasCompSuffix(comps, "SCR_BayonetComponent"))
			return true;
		if (TBD_AttachmentExtractor.HasCompSuffix(comps, "SCR_BayonetEffectComponent"))
			return true;
		if (TBD_AttachmentExtractor.HasCompSuffix(comps, "SCR_FlashlightComponent"))
			return true;
		if (TBD_AttachmentExtractor.HasCompSuffix(comps, "AttachmentSlotComponent"))
			return true;
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected void OnResourceFound(ResourceName resName, string filePath = "")
	{
		string path = filePath;
		if (path.IsEmpty())
			path = resName;

		bool isAbstract = (path.EndsWith("_base.et") || path.Contains("/base/"));
		if (isAbstract && !m_Config.m_bIncludeAbstract)
			return;

		Resource res = Resource.Load(resName);
		if (!res || !res.IsValid())
			return;

		BaseResourceObject obj = res.GetResource();
		if (!obj)
			return;

		BaseContainer root = obj.ToBaseContainer();
		if (!root)
			return;

		string rootClass = root.GetClassName();
		if (rootClass == "SCR_ChimeraCharacter" || rootClass == "ChimeraCharacter" || rootClass == "Vehicle" || rootClass.EndsWith("Vehicle"))
			return;

		map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
		TBD_AttachmentExtractor.CollectComponentChain(root, comps);

		string lowerPath = path;
		lowerPath.ToLower();

		// Filter out optical sights (already exported by Optics Exporter)
		if (IsOpticPrefab(lowerPath, comps))
			return;

		// Filter out underbarrel weapons (Option A: already exported in equipment/weapons/underbarrel.json)
		if (IsUnderbarrelLauncher(lowerPath, comps))
			return;

		// Filter out weapons (weapons with WeaponComponent that are not attachments)
		if (TBD_AttachmentExtractor.HasCompSuffix(comps, "WeaponComponent"))
			return;

		// Ensure it has attachment signals or is located in an attachment folder
		if (!IsAttachmentPath(lowerPath) && !HasAttachmentComponent(comps))
			return;

		string canonical = TBD_AttachmentExtractor.ResolveCanonical(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		// Deduplicate across search paths
		if (m_SeenResourceNames.Contains(canonical))
			return;
		m_SeenResourceNames.Insert(canonical);

		// Build attachment record
		TBD_AttachmentInfo info = new TBD_AttachmentInfo();
		info.m_sResourceName = canonical;
		info.m_sFilePath = path;
		info.m_bIsAbstract = isAbstract;
		info.m_sAddonId = m_sCurrentAddonId;

		// ID stem
		string idStem = path;
		int slashIdx = idStem.LastIndexOf("/");
		if (slashIdx >= 0)
			idStem = idStem.Substring(slashIdx + 1, idStem.Length() - slashIdx - 1);
		if (idStem.EndsWith(".et"))
			idStem = idStem.Substring(0, idStem.Length() - 3);
		info.m_sId = idStem;

		// Identity
		info.m_sDisplayName = TBD_AttachmentExtractor.DisplayNameFor(comps, path);
		info.m_sDescription = TBD_AttachmentExtractor.DescriptionFor(comps);
		info.m_sIcon = TBD_AttachmentExtractor.IconFor(comps);
		info.m_sFamily = TBD_AttachmentExtractor.DeriveFamily(path, canonical);

		// Variant check
		BaseContainer anc = root.GetAncestor();
		if (anc)
		{
			string ancRes = TBD_AttachmentExtractor.ResolveCanonical(anc.GetResourceName());
			if (!ancRes.IsEmpty() && ancRes != canonical)
				info.m_sVariantOf = ancRes;
		}

		// Categorize attachment (preliminary)
		info.m_sCategory = TBD_AttachmentExtractor.CategorizeAttachment(comps, path, "");

		// Mounting (pure relational intrinsic keys)
		TBD_AttachmentExtractor.ExtractMounting(comps, info.m_Mounting, path, info.m_sCategory);

		// If category needs refinement with mounting type
		if (!info.m_Mounting.m_sAttachmentType.IsEmpty())
		{
			string refinedCat = TBD_AttachmentExtractor.CategorizeAttachment(comps, path, info.m_Mounting.m_sAttachmentType);
			if (!refinedCat.IsEmpty())
				info.m_sCategory = refinedCat;
		}

		// If still unrecognized or rejected, skip
		if (info.m_sCategory.IsEmpty())
			return;

		// Physical attributes
		TBD_AttachmentExtractor.ExtractPhysical(comps, info.m_Physical, info.m_sCategory, path);

		// Visuals (3D Mesh)
		TBD_AttachmentExtractor.ExtractVisuals(comps, info.m_Visuals);

		// Category technical payloads
		if (info.m_sCategory == "muzzles")
		{
			TBD_AttachmentExtractor.ExtractMuzzleData(comps, info.m_Muzzle, path, info.m_Mounting.m_sAttachmentType);
			m_aMuzzles.Insert(info);
		}
		else if (info.m_sCategory == "bipods")
		{
			TBD_AttachmentExtractor.ExtractBipodData(comps, info.m_Bipod, path, info.m_Mounting.m_sAttachmentType);
			m_aBipods.Insert(info);
		}
		else if (info.m_sCategory == "handguards")
		{
			TBD_AttachmentExtractor.ExtractHandguardData(comps, info.m_Handguard, path, info.m_Mounting.m_sAttachmentType);
			m_aHandguards.Insert(info);
		}
		else if (info.m_sCategory == "illuminators")
		{
			TBD_AttachmentExtractor.ExtractIlluminatorData(comps, info.m_Illuminator, path, info.m_Mounting.m_sAttachmentType);
			m_aIlluminators.Insert(info);
		}
		else if (info.m_sCategory == "bayonets")
		{
			TBD_AttachmentExtractor.ExtractBayonetData(comps, info.m_Bayonet, path, info.m_Mounting.m_sAttachmentType);
			m_aBayonets.Insert(info);
		}
		else if (info.m_sCategory == "stocks")
		{
			TBD_AttachmentExtractor.ExtractStockData(comps, info.m_Stock, path, info.m_Mounting.m_sAttachmentType);
			m_aStocks.Insert(info);
		}
		else if (info.m_sCategory == "mounts")
		{
			TBD_AttachmentExtractor.ExtractMountData(comps, info.m_Mount, path, info.m_Mounting.m_sAttachmentType);
			m_aMounts.Insert(info);
		}
		else if (info.m_sCategory == "camouflage")
		{
			TBD_AttachmentExtractor.ExtractCamouflageData(comps, info.m_Camouflage, path, info.m_Mounting.m_sAttachmentType);
			m_aCamouflage.Insert(info);
		}

		m_aAllAttachments.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	//! Write all JSON category catalogs, master catalog, and metadata sidecars to disk.
	protected void WriteAllFiles(string destDir, int elapsedMs)
	{
		WriteCategoryCatalog(destDir, "muzzles", "muzzles.json", "muzzles_meta.json", m_aMuzzles);
		WriteCategoryCatalog(destDir, "bipods", "bipods.json", "bipods_meta.json", m_aBipods);
		WriteCategoryCatalog(destDir, "handguards", "handguards.json", "handguards_meta.json", m_aHandguards);
		WriteCategoryCatalog(destDir, "illuminators", "illuminators.json", "illuminators_meta.json", m_aIlluminators);
		WriteCategoryCatalog(destDir, "bayonets", "bayonets.json", "bayonets_meta.json", m_aBayonets);
		WriteCategoryCatalog(destDir, "stocks", "stocks.json", "stocks_meta.json", m_aStocks);
		WriteCategoryCatalog(destDir, "mounts", "mounts.json", "mounts_meta.json", m_aMounts);
		WriteCategoryCatalog(destDir, "camouflage", "camouflage.json", "camouflage_meta.json", m_aCamouflage);

		WriteMasterCatalog(destDir, elapsedMs);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteCategoryCatalog(string destDir, string category, string filename, string metaFilename, notnull array<ref TBD_AttachmentInfo> list)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "attachments", filename);
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "attachments", metaFilename);

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("[TBD][AttachmentExport] ERROR: Failed to open %1 for writing!", filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"category\": \"" + category + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + list.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"attachments\": [\n", "[TBD]");

		for (int i = 0; i < list.Count(); i++)
		{
			TBD_AttachmentInfo item = list[i];
			string itemJson = item.SerializeJson("    ");
			if (i < list.Count() - 1)
				itemJson += ",\n";
			else
				itemJson += "\n";

			TBD_EquipmentExportJson.Write(f, itemJson, "[TBD]");
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", "[TBD]");
		f.Close();

		// Metadata sidecar
		FileHandle mf = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (mf)
		{
			string meta = "{\n";
			meta += "  \"category\": \"" + category + "\",\n";
			meta += "  \"totalCount\": " + list.Count().ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, "[TBD]");
			mf.Close();
		}

		Print(string.Format("[TBD][AttachmentExport] Wrote %1 attachments to %2", list.Count(), filePath), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteMasterCatalog(string destDir, int elapsedMs)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "attachments", "attachments_all.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "attachments", "attachments_all_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("[TBD][AttachmentExport] ERROR: Failed to open %1 for writing!", filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllAttachments.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"categories\": {\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "    \"muzzles\": " + m_aMuzzles.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "    \"bipods\": " + m_aBipods.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "    \"handguards\": " + m_aHandguards.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "    \"illuminators\": " + m_aIlluminators.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "    \"bayonets\": " + m_aBayonets.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "    \"stocks\": " + m_aStocks.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "    \"mounts\": " + m_aMounts.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "    \"camouflage\": " + m_aCamouflage.Count().ToString() + "\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  },\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"attachments\": [\n", "[TBD]");

		for (int i = 0; i < m_aAllAttachments.Count(); i++)
		{
			TBD_AttachmentInfo item = m_aAllAttachments[i];
			string itemJson = item.SerializeJson("    ");
			if (i < m_aAllAttachments.Count() - 1)
				itemJson += ",\n";
			else
				itemJson += "\n";

			TBD_EquipmentExportJson.Write(f, itemJson, "[TBD]");
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", "[TBD]");
		f.Close();

		// Master metadata sidecar
		FileHandle mf = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (mf)
		{
			string meta = "{\n";
			meta += "  \"totalCount\": " + m_aAllAttachments.Count().ToString() + ",\n";
			meta += "  \"muzzles\": " + m_aMuzzles.Count().ToString() + ",\n";
			meta += "  \"bipods\": " + m_aBipods.Count().ToString() + ",\n";
			meta += "  \"handguards\": " + m_aHandguards.Count().ToString() + ",\n";
			meta += "  \"illuminators\": " + m_aIlluminators.Count().ToString() + ",\n";
			meta += "  \"bayonets\": " + m_aBayonets.Count().ToString() + ",\n";
			meta += "  \"stocks\": " + m_aStocks.Count().ToString() + ",\n";
			meta += "  \"mounts\": " + m_aMounts.Count().ToString() + ",\n";
			meta += "  \"camouflage\": " + m_aCamouflage.Count().ToString() + ",\n";
			meta += "  \"exportDurationMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, "[TBD]");
			mf.Close();
		}

		Print(string.Format("[TBD][AttachmentExport] Wrote master catalog (%1 attachments) to %2", m_aAllAttachments.Count(), filePath), LogLevel.NORMAL);
	}
}
