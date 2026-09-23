//------------------------------------------------------------------------------------------------
// TBD_WeaponScanner.c
//
// Universal weapon scanner across all loaded addons for Arma Reforger Workbench.
// Categorizes, introspects, and exports all weapons (rifles, machine guns, handguns, launchers,
// grenades, explosives, and underbarrel systems) to dedicated JSON catalogs and a master catalog.
//------------------------------------------------------------------------------------------------

class TBD_WeaponScanner
{
	//! How deep into nested component arrays this scan reads. Passed to the shared graph walk.
	protected static const int COMPONENT_DEPTH_CAP = 4;

	protected ref TBD_EquipmentExportConfig m_Config;

	protected ref array<ref TBD_WeaponInfo> m_aRifles = {};
	protected ref array<ref TBD_WeaponInfo> m_aMachineGuns = {};
	protected ref array<ref TBD_WeaponInfo> m_aHandguns = {};
	protected ref array<ref TBD_WeaponInfo> m_aLaunchers = {};
	protected ref array<ref TBD_WeaponInfo> m_aGrenades = {};
	protected ref array<ref TBD_WeaponInfo> m_aExplosives = {};
	protected ref array<ref TBD_WeaponInfo> m_aUnderbarrel = {};
	protected ref array<ref TBD_WeaponInfo> m_aFlares = {};
	protected ref array<ref TBD_WeaponInfo> m_aHeavyWeapons = {};
	protected ref array<ref TBD_WeaponInfo> m_aAllWeapons = {};

	protected ref set<string> m_SeenResourceNames = new set<string>();

	protected string m_sCurrentAddonId;
	protected string m_sCurrentCategory;

	//------------------------------------------------------------------------------------------------
	void TBD_WeaponScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run universal multi-category weapon scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print("[TBD][WeaponExport] Starting universal weapon scan across all categories...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		// Scan order: Rifles, MachineGuns, Handguns, Launchers, Flares, HeavyWeapons, Grenades, Explosives, Underbarrel
		array<string> categories = {
			"rifles",
			"machine_guns",
			"handguns",
			"launchers",
			"flares",
			"heavy_weapons",
			"grenades",
			"explosives",
			"underbarrel"
		};

		foreach (string cat : categories)
		{
			m_sCurrentCategory = cat;
			string relPath = RelativePathForCategory(cat);

			foreach (string guid : guids)
			{
				string addonId = GameProject.GetAddonID(guid);
				m_sCurrentAddonId = addonId;
				string rootPath = "$" + addonId + ":" + relPath;
				Workbench.SearchResources(OnResourceFound, extEt, null, rootPath, true);
			}

			Print(string.Format("[TBD][WeaponExport] Category '%1' scan complete: %2 weapons found.", cat, CountForCategory(cat)), LogLevel.NORMAL);
		}

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("[TBD][WeaponExport] Universal scan finished in %1 ms. Total weapons: %2.", elapsedMs, m_aAllWeapons.Count()), LogLevel.NORMAL);

		// Write all JSON files to disk
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllWeapons.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected string RelativePathForCategory(string category)
	{
		if (category == "rifles") return "Prefabs/Weapons/Rifles";
		if (category == "machine_guns") return "Prefabs/Weapons/MachineGuns";
		if (category == "handguns") return "Prefabs/Weapons/Handguns";
		if (category == "launchers") return "Prefabs/Weapons/Launchers";
		if (category == "flares") return "Prefabs/Weapons/Flares";
		if (category == "heavy_weapons") return "Prefabs/Weapons/HeavyWeapons";
		if (category == "grenades") return "Prefabs/Weapons/Grenades";
		if (category == "explosives") return "Prefabs/Weapons/Explosives";
		if (category == "underbarrel") return "Prefabs/Weapons/Attachments/Underbarrel";
		return "Prefabs/Weapons";
	}

	//------------------------------------------------------------------------------------------------
	protected int CountForCategory(string category)
	{
		if (category == "rifles") return m_aRifles.Count();
		if (category == "machine_guns") return m_aMachineGuns.Count();
		if (category == "handguns") return m_aHandguns.Count();
		if (category == "launchers") return m_aLaunchers.Count();
		if (category == "flares") return m_aFlares.Count();
		if (category == "heavy_weapons") return m_aHeavyWeapons.Count();
		if (category == "grenades") return m_aGrenades.Count();
		if (category == "explosives") return m_aExplosives.Count();
		if (category == "underbarrel") return m_aUnderbarrel.Count();
		return 0;
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aRifles.Clear();
		m_aMachineGuns.Clear();
		m_aHandguns.Clear();
		m_aLaunchers.Clear();
		m_aFlares.Clear();
		m_aHeavyWeapons.Clear();
		m_aGrenades.Clear();
		m_aExplosives.Clear();
		m_aUnderbarrel.Clear();
		m_aAllWeapons.Clear();
		m_SeenResourceNames.Clear();
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
		TBD_EquipmentComponentGraph.CollectComponentChain(root, comps, COMPONENT_DEPTH_CAP);

		// Must be a weapon entity or explosive device
		bool isWeapon = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "WeaponComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MineWeaponComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MuzzleInMagComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MuzzleComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "GrenadeMoveComponent");

		if (!isWeapon)
			return;

		string canonical = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		// Deduplicate across search paths
		if (m_SeenResourceNames.Contains(canonical))
			return;
		m_SeenResourceNames.Insert(canonical);

		TBD_WeaponInfo info = new TBD_WeaponInfo();
		info.m_sResourceName = canonical;
		info.m_sFilePath = path;
		info.m_sId = TBD_EquipmentResourceNames.GenerateSlug(path);
		info.m_sDisplayName = TBD_WeaponNaming.DisplayNameFor(comps, path);
		info.m_sDescription = TBD_WeaponNaming.DescriptionFor(comps);
		info.m_sIcon = TBD_WeaponNaming.IconFor(comps);
		info.m_sCategory = m_sCurrentCategory;
		info.m_sFamily = TBD_WeaponNaming.ExtractFamily(path, m_sCurrentCategory);
		info.m_sAddonId = m_sCurrentAddonId;
		info.m_bIsAbstract = isAbstract;

		// Ancestor / Variant Of
		BaseContainer anc = root.GetAncestor();
		if (anc)
		{
			string ancRn = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(anc.GetResourceName());
			if (!ancRn.IsEmpty() && ancRn != canonical)
				info.m_sVariantOf = ancRn;
		}

		// Classification
		TBD_WeaponClassificationExtractor.ExtractClassification(comps, info.m_Classification, m_sCurrentCategory);

		// Physical Attributes
		TBD_WeaponExtractor.ExtractPhysical(comps, info.m_Physical);

		// Sights & Zeroing
		TBD_WeaponExtractor.ExtractSights(comps, info.m_Sights);

		// Ballistics
		TBD_WeaponExtractor.ExtractBallistics(comps, info.m_Ballistics);

		// Muzzles & Fire Modes
		TBD_WeaponMuzzleExtractor.ExtractMuzzles(comps, info.m_aMuzzles);

		// Attachment Slots & Obstructions
		TBD_WeaponMountingExtractor.ExtractAttachmentSlots(comps, info.m_aAttachmentSlots);

		// Insert into category bucket
		InsertCategoryWeapon(m_sCurrentCategory, info);
		m_aAllWeapons.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	protected void InsertCategoryWeapon(string category, TBD_WeaponInfo info)
	{
		if (category == "rifles") m_aRifles.Insert(info);
		else if (category == "machine_guns") m_aMachineGuns.Insert(info);
		else if (category == "handguns") m_aHandguns.Insert(info);
		else if (category == "launchers") m_aLaunchers.Insert(info);
		else if (category == "flares") m_aFlares.Insert(info);
		else if (category == "heavy_weapons") m_aHeavyWeapons.Insert(info);
		else if (category == "grenades") m_aGrenades.Insert(info);
		else if (category == "explosives") m_aExplosives.Insert(info);
		else if (category == "underbarrel") m_aUnderbarrel.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	//! Write all category files and the combined master catalog to disk.
	void WriteAllFiles(string destDir, int elapsedMs)
	{
		WriteCategoryCatalog("rifles", m_aRifles, destDir);
		WriteCategoryCatalog("machine_guns", m_aMachineGuns, destDir);
		WriteCategoryCatalog("handguns", m_aHandguns, destDir);
		WriteCategoryCatalog("launchers", m_aLaunchers, destDir);
		WriteCategoryCatalog("flares", m_aFlares, destDir);
		WriteCategoryCatalog("heavy_weapons", m_aHeavyWeapons, destDir);
		WriteCategoryCatalog("grenades", m_aGrenades, destDir);
		WriteCategoryCatalog("explosives", m_aExplosives, destDir);
		WriteCategoryCatalog("underbarrel", m_aUnderbarrel, destDir);
		WriteMasterCatalog(destDir, elapsedMs);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteCategoryCatalog(string category, array<ref TBD_WeaponInfo> list, string destDir)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "weapons", category + ".json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "weapons", category + "_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("[TBD][WeaponExport] ERROR: Failed to open %1 for writing!", filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"category\": \"" + category + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + list.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"weapons\": [\n", "[TBD]");

		for (int i = 0; i < list.Count(); i++)
		{
			TBD_WeaponInfo item = list[i];
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

		Print(string.Format("[TBD][WeaponExport] Wrote %1 weapons to %2", list.Count(), filePath), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteMasterCatalog(string destDir, int elapsedMs)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "weapons", "weapons_all.json");
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "weapons", "weapons_all_meta.json");

		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(string.Format("[TBD][WeaponExport] ERROR: Failed to open %1 for writing!", filePath), LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"version\": \"1\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"category\": \"weapons_all\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllWeapons.Count().ToString() + ",\n", "[TBD]");
		TBD_EquipmentExportJson.Write(f, "  \"weapons\": [\n", "[TBD]");

		for (int i = 0; i < m_aAllWeapons.Count(); i++)
		{
			TBD_WeaponInfo item = m_aAllWeapons[i];
			string itemJson = item.SerializeJson("    ");
			if (i < m_aAllWeapons.Count() - 1)
				itemJson += ",\n";
			else
				itemJson += "\n";

			TBD_EquipmentExportJson.Write(f, itemJson, "[TBD]");
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", "[TBD]");
		f.Close();

		// Metadata sidecar with category breakdown
		FileHandle mf = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (mf)
		{
			string meta = "{\n";
			meta += "  \"category\": \"weapons_all\",\n";
			meta += "  \"totalCount\": " + m_aAllWeapons.Count().ToString() + ",\n";
			meta += "  \"elapsedMs\": " + elapsedMs.ToString() + ",\n";
			meta += "  \"exportedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
			meta += "  \"categories\": {\n";
			meta += "    \"rifles\": " + m_aRifles.Count().ToString() + ",\n";
			meta += "    \"machine_guns\": " + m_aMachineGuns.Count().ToString() + ",\n";
			meta += "    \"handguns\": " + m_aHandguns.Count().ToString() + ",\n";
			meta += "    \"launchers\": " + m_aLaunchers.Count().ToString() + ",\n";
			meta += "    \"flares\": " + m_aFlares.Count().ToString() + ",\n";
			meta += "    \"heavy_weapons\": " + m_aHeavyWeapons.Count().ToString() + ",\n";
			meta += "    \"grenades\": " + m_aGrenades.Count().ToString() + ",\n";
			meta += "    \"explosives\": " + m_aExplosives.Count().ToString() + ",\n";
			meta += "    \"underbarrel\": " + m_aUnderbarrel.Count().ToString() + "\n";
			meta += "  }\n";
			meta += "}\n";
			TBD_EquipmentExportJson.Write(mf, meta, "[TBD]");
			mf.Close();
		}

		Print(string.Format("[TBD][WeaponExport] Master weapons catalog written: %1 total items across 9 categories.", m_aAllWeapons.Count()), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	int GetTotalCount()
	{
		return m_aAllWeapons.Count();
	}
}
