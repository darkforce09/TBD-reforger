/**
 * TBD_WearableScanner.c
 *
 * Universal wearable, clothing, armor, and gear scanner across all loaded addons.
 * Discovers, introspects, and exports all wearables (headgear, face covers, eyewear,
 * jackets, pants, boots, gloves, armored vests, carry rigs, backpacks, and accessories)
 * to dedicated category JSON catalogs and a master catalog at $profile:TBD_Export/equipment/wearables/.
 */

class TBD_WearableScanner
{
	protected static const string TAG = "[TBD][WearableExport]";
	protected static const int COMPONENT_DEPTH_CAP = 4;

	protected static const ref array<string> DENY_HARD = {
		"/Structures/",
		"/Rocks/",
		"/Trees/",
		"/Debris/",
		"/Foliage/",
		"Prefabs/Editor/",
		"Prefabs/Systems/",
		"Prefabs/Waypoints/",
		"Prefabs/Triggers/",
		"Prefabs/Compositions/",
		"Prefabs/Sounds/",
		"Prefabs/UI/"
	};

	protected static const ref array<string> AREA_BASES = {
		"LoadoutHeadCoverArea", "LoadoutCoverArea", "LoadoutGooglesArea",
		"LoadoutJacketArea", "LoadoutPantsArea", "LoadoutBootsArea",
		"LoadoutHandwearSlotArea", "LoadoutArmoredVestSlotArea", "LoadoutVestArea",
		"LoadoutBackpackArea", "LoadoutWatchArea", "LoadoutBinocularsArea",
		"LoadoutSalineBagArea", "LoadoutIdentityItemArea", "SCR_LoadoutHandSlotArea"
	};

	protected ref TBD_EquipmentExportConfig m_Config;

	protected ref array<ref TBD_WearableInfo> m_aHeadgear = {};
	protected ref array<ref TBD_WearableInfo> m_aFaceCover = {};
	protected ref array<ref TBD_WearableInfo> m_aEyewear = {};
	protected ref array<ref TBD_WearableInfo> m_aJackets = {};
	protected ref array<ref TBD_WearableInfo> m_aPants = {};
	protected ref array<ref TBD_WearableInfo> m_aBoots = {};
	protected ref array<ref TBD_WearableInfo> m_aGloves = {};
	protected ref array<ref TBD_WearableInfo> m_aArmoredVests = {};
	protected ref array<ref TBD_WearableInfo> m_aVestsAndRigs = {};
	protected ref array<ref TBD_WearableInfo> m_aBackpacks = {};
	protected ref array<ref TBD_WearableInfo> m_aAccessories = {};
	protected ref array<ref TBD_WearableInfo> m_aAllWearables = {};

	protected ref set<string> m_SeenResourceNames = new set<string>();

	//------------------------------------------------------------------------------------------------
	void TBD_WearableScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run universal multi-category wearable scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print(TAG + " Starting universal wearables & gear scan...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		array<string> scanPaths = {
			"Prefabs/Characters",
			"Prefabs/Items/Equipment",
			"Prefabs/Items"
		};

		foreach (string relPath : scanPaths)
		{
			foreach (string guid : guids)
			{
				string addonId = GameProject.GetAddonID(guid);
				string rootPath = "$" + addonId + ":" + relPath;
				Workbench.SearchResources(OnResourceFound, extEt, null, rootPath, true);
			}
		}

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("%1 Universal scan finished in %2 ms. Total wearables: %3.", TAG, elapsedMs, m_aAllWearables.Count()), LogLevel.NORMAL);

		// Write all JSON files to disk
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllWearables.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aHeadgear.Clear();
		m_aFaceCover.Clear();
		m_aEyewear.Clear();
		m_aJackets.Clear();
		m_aPants.Clear();
		m_aBoots.Clear();
		m_aGloves.Clear();
		m_aArmoredVests.Clear();
		m_aVestsAndRigs.Clear();
		m_aBackpacks.Clear();
		m_aAccessories.Clear();
		m_aAllWearables.Clear();
		m_SeenResourceNames.Clear();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnResourceFound(ResourceName resName, string filePath = "")
	{
		string path = filePath;
		if (path.IsEmpty())
			path = resName;

		foreach (string deny : DENY_HARD)
		{
			if (path.Contains(deny))
				return;
		}

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

		// Discard character controllers, weapons, magazines
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "CharacterControllerComponent"))
			return;
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "WeaponComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "GrenadeLauncherComponent"))
			return;
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MagazineComponent"))
			return;

		// Check for wearable signals
		bool hasCloth = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "LoadoutClothComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "ClothComponent");
		bool hasStorage = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "UniversalInventoryStorageComponent");
		bool hasInvItem = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "InventoryItemComponent");

		if (!hasCloth && !hasStorage && !hasInvItem)
			return;

		string canonical = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		if (m_SeenResourceNames.Contains(canonical))
			return;

		// Extract wear area and blocked slots
		string areaType;
		array<string> blockedSlots = {};
		TBD_WearableExtractor.ExtractWearableArea(comps, areaType, blockedSlots);

		// If no cloth area and no inventory item, skip
		if (areaType.IsEmpty() && !hasCloth && !path.Contains("Prefabs/Characters") && !path.Contains("Backpack") && !path.Contains("Pouch") && !path.Contains("Holster"))
			return;

		string category = CategorizeItem(areaType, path, comps);
		if (category.IsEmpty())
			return;

		m_SeenResourceNames.Insert(canonical);

		string addonId = string.Empty;
		if (path.StartsWith("$"))
		{
			int colon = path.IndexOf(":");
			if (colon > 1)
				addonId = path.Substring(1, colon - 1);
		}

		TBD_WearableInfo info = new TBD_WearableInfo();
		info.m_sResourceName = canonical;
		info.m_sFilePath = path;
		info.m_sId = TBD_EquipmentResourceNames.GenerateSlug(path);
		info.m_sDisplayName = TBD_WearableNaming.DisplayNameFor(comps, path);
		info.m_sDescription = TBD_WearableNaming.DescriptionFor(comps);
		info.m_sIcon = TBD_WearableNaming.IconFor(comps);
		info.m_sCategory = category;
		info.m_sAreaType = areaType;
		info.m_sAddonId = addonId;
		info.m_bIsAbstract = isAbstract;
		info.m_aBlockedSlots = blockedSlots;

		// Ancestor / Variant Of
		BaseContainer anc = root.GetAncestor();
		if (anc)
		{
			string ancRn = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(anc.GetResourceName());
			if (!ancRn.IsEmpty() && ancRn != canonical)
				info.m_sVariantOf = ancRn;
		}

		// Extract physical, storage, armor, slots, visual
		TBD_WearableExtractor.ExtractPhysical(comps, info.m_Physical);
		TBD_WearableExtractor.ExtractStorage(comps, info.m_Storage);
		TBD_WearableExtractor.ExtractArmor(comps, info.m_Armor);
		TBD_WearableExtractor.ExtractSlots(comps, info.m_aSlots);
		TBD_WearableExtractor.ExtractVisual(comps, info.m_Visual);

		InsertCategoryWearable(category, info);
		m_aAllWearables.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	protected string CategorizeItem(string areaType, string path, map<string, ref array<BaseContainer>> comps)
	{
		string cat = CategoryForAreaClass(areaType);
		if (!cat.IsEmpty())
			return cat;

		// Typename inheritance fallback for mod subclasses
		if (!areaType.IsEmpty())
		{
			typename t = areaType.ToType();
			if (t)
			{
				foreach (string baseArea : AREA_BASES)
				{
					typename bt = baseArea.ToType();
					if (bt && t.IsInherited(bt))
						return CategoryForAreaClass(baseArea);
				}
			}
		}

		// Path and component fallback
		string lower = path;
		lower.ToLower();

		if (lower.Contains("/headgear/")) return "headgear";
		if (lower.Contains("/eyewear/")) return "eyewear";
		if (lower.Contains("/footwear/")) return "boots";
		if (lower.Contains("/handwear/")) return "gloves";
		if (lower.Contains("/backpacks/")) return "backpacks";
		if (lower.Contains("/vests/"))
		{
			if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "ArmorDamageManagerComponent"))
				return "armored_vests";
			return "vests_and_rigs";
		}
		if (lower.Contains("/uniforms/"))
		{
			if (lower.Contains("jacket") || lower.Contains("shirt") || lower.Contains("coat")) return "jackets";
			if (lower.Contains("pants") || lower.Contains("trouser")) return "pants";
		}
		if (lower.Contains("pouch") || lower.Contains("holster") || lower.Contains("/accessories/"))
			return "accessories";

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	protected string CategoryForAreaClass(string areaClass)
	{
		if (areaClass == "LoadoutHeadCoverArea") return "headgear";
		if (areaClass == "LoadoutCoverArea") return "face_cover";
		if (areaClass == "LoadoutGooglesArea") return "eyewear";
		if (areaClass == "LoadoutJacketArea") return "jackets";
		if (areaClass == "LoadoutPantsArea") return "pants";
		if (areaClass == "LoadoutBootsArea") return "boots";
		if (areaClass == "LoadoutHandwearSlotArea") return "gloves";
		if (areaClass == "LoadoutArmoredVestSlotArea") return "armored_vests";
		if (areaClass == "LoadoutVestArea") return "vests_and_rigs";
		if (areaClass == "LoadoutBackpackArea") return "backpacks";
		if (areaClass == "LoadoutWatchArea" || areaClass == "LoadoutBinocularsArea"
			|| areaClass == "LoadoutSalineBagArea" || areaClass == "LoadoutIdentityItemArea"
			|| areaClass == "SCR_IdentityItemLoadoutArea" || areaClass == "SCR_LoadoutHandSlotArea")
			return "accessories";

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	protected void InsertCategoryWearable(string category, TBD_WearableInfo info)
	{
		if (category == "headgear") m_aHeadgear.Insert(info);
		else if (category == "face_cover") m_aFaceCover.Insert(info);
		else if (category == "eyewear") m_aEyewear.Insert(info);
		else if (category == "jackets") m_aJackets.Insert(info);
		else if (category == "pants") m_aPants.Insert(info);
		else if (category == "boots") m_aBoots.Insert(info);
		else if (category == "gloves") m_aGloves.Insert(info);
		else if (category == "armored_vests") m_aArmoredVests.Insert(info);
		else if (category == "vests_and_rigs") m_aVestsAndRigs.Insert(info);
		else if (category == "backpacks") m_aBackpacks.Insert(info);
		else if (category == "accessories") m_aAccessories.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	//! Write all category files and the combined master catalog to disk.
	void WriteAllFiles(string destDir, int elapsedMs)
	{
		WriteCategoryCatalog("headgear", m_aHeadgear, destDir);
		WriteCategoryCatalog("face_cover", m_aFaceCover, destDir);
		WriteCategoryCatalog("eyewear", m_aEyewear, destDir);
		WriteCategoryCatalog("jackets", m_aJackets, destDir);
		WriteCategoryCatalog("pants", m_aPants, destDir);
		WriteCategoryCatalog("boots", m_aBoots, destDir);
		WriteCategoryCatalog("gloves", m_aGloves, destDir);
		WriteCategoryCatalog("armored_vests", m_aArmoredVests, destDir);
		WriteCategoryCatalog("vests_and_rigs", m_aVestsAndRigs, destDir);
		WriteCategoryCatalog("backpacks", m_aBackpacks, destDir);
		WriteCategoryCatalog("accessories", m_aAccessories, destDir);
		WriteMasterCatalog(destDir, elapsedMs);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteCategoryCatalog(string category, array<ref TBD_WearableInfo> list, string destDir)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "wearables", category + ".json");
		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open " + filePath + " for write", LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n  \"version\": \"1\",\n  \"category\": \"" + category + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"count\": " + list.Count().ToString() + ",\n  \"items\": [\n", TAG);

		for (int i = 0; i < list.Count(); i++)
		{
			string itemJson = list[i].SerializeJson("    ");
			if (i < list.Count() - 1)
				itemJson += ",";
			itemJson += "\n";
			TBD_EquipmentExportJson.Write(f, itemJson, TAG);
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", TAG);
		f.Close();

		// Sidecar meta
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "wearables", category + "_meta.json");
		FileHandle mf = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (mf)
		{
			string meta = "{\n  \"category\": \"" + category + "\",\n  \"count\": " + list.Count().ToString() + ",\n  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n}\n";
			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}

		Print(string.Format("%1 Category '%2' exported (%3 items) -> %4", TAG, category, list.Count(), filePath), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteMasterCatalog(string destDir, int elapsedMs)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "wearables", "wearables_master.json");
		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open " + filePath + " for write", LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n  \"version\": \"1\",\n  \"domain\": \"wearables\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllWearables.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"counts\": {\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"headgear\": " + m_aHeadgear.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"face_cover\": " + m_aFaceCover.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"eyewear\": " + m_aEyewear.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"jackets\": " + m_aJackets.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"pants\": " + m_aPants.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"boots\": " + m_aBoots.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"gloves\": " + m_aGloves.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"armored_vests\": " + m_aArmoredVests.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"vests_and_rigs\": " + m_aVestsAndRigs.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"backpacks\": " + m_aBackpacks.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"accessories\": " + m_aAccessories.Count().ToString() + "\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  },\n  \"items\": [\n", TAG);

		for (int i = 0; i < m_aAllWearables.Count(); i++)
		{
			string itemJson = m_aAllWearables[i].SerializeJson("    ");
			if (i < m_aAllWearables.Count() - 1)
				itemJson += ",";
			itemJson += "\n";
			TBD_EquipmentExportJson.Write(f, itemJson, TAG);
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", TAG);
		f.Close();

		// Sidecar meta
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "wearables", "wearables_master_meta.json");
		FileHandle mf = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (mf)
		{
			string meta = "{\n  \"totalCount\": " + m_aAllWearables.Count().ToString() + ",\n  \"elapsedMs\": " + elapsedMs.ToString() + ",\n  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n}\n";
			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}

		Print(string.Format("%1 MASTER CATALOG EXPORTED (%2 items) -> %3", TAG, m_aAllWearables.Count(), filePath), LogLevel.NORMAL);
	}
}
