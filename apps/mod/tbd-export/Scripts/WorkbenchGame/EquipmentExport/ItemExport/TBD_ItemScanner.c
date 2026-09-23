/**
 * TBD_ItemScanner.c
 *
 * Universal inventory item scanner across all loaded addons.
 * Discovers, introspects, and exports all inventory items (medical supplies, radios,
 * navigation instruments, optics, tools, explosives, throwables, weapon parts, survival gear,
 * and misc items) to dedicated JSON catalogs and a master catalog at $profile:TBD_Export/equipment/items/.
 */

class TBD_ItemScanner
{
	protected static const string TAG = "[TBD][ItemExport]";
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
		"Prefabs/UI/",
		"Prefabs/Items/Equipment/Test/"
	};

	protected ref TBD_EquipmentExportConfig m_Config;

	protected ref array<ref TBD_ItemInfo> m_aMedical = {};
	protected ref array<ref TBD_ItemInfo> m_aRadios = {};
	protected ref array<ref TBD_ItemInfo> m_aNavigation = {};
	protected ref array<ref TBD_ItemInfo> m_aBinoculars = {};
	protected ref array<ref TBD_ItemInfo> m_aFlashlights = {};
	protected ref array<ref TBD_ItemInfo> m_aTools = {};
	protected ref array<ref TBD_ItemInfo> m_aExplosives = {};
	protected ref array<ref TBD_ItemInfo> m_aThrowables = {};
	protected ref array<ref TBD_ItemInfo> m_aWeaponParts = {};
	protected ref array<ref TBD_ItemInfo> m_aSurvival = {};
	protected ref array<ref TBD_ItemInfo> m_aIntelAndMisc = {};
	protected ref array<ref TBD_ItemInfo> m_aAllItems = {};

	protected ref set<string> m_SeenResourceNames = new set<string>();

	//------------------------------------------------------------------------------------------------
	void TBD_ItemScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Run universal multi-category inventory item scan across all loaded addons.
	int RunScan()
	{
		ClearAll();

		Print(TAG + " Starting universal inventory item scan...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		array<string> scanPaths = {
			"Prefabs/Items",
			"Prefabs/Weapons",
			"Prefabs/Props"
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
		Print(string.Format("%1 Universal item scan finished in %2 ms. Total items: %3.", TAG, elapsedMs, m_aAllItems.Count()), LogLevel.NORMAL);

		// Write all JSON files to disk
		WriteAllFiles(m_Config.m_sDestinationDir, elapsedMs);

		return m_aAllItems.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void ClearAll()
	{
		m_aMedical.Clear();
		m_aRadios.Clear();
		m_aNavigation.Clear();
		m_aBinoculars.Clear();
		m_aFlashlights.Clear();
		m_aTools.Clear();
		m_aExplosives.Clear();
		m_aThrowables.Clear();
		m_aWeaponParts.Clear();
		m_aSurvival.Clear();
		m_aIntelAndMisc.Clear();
		m_aAllItems.Clear();
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

		// 1. Hard filters: Character controllers, vehicles, non-item props, and virtual templates
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "CharacterControllerComponent"))
			return;
		if (path.Contains("_Vehicle") || path.Contains("FreeRoamBuilding") || path.Contains("FuelNozzle")
			|| path.Contains("DeployablePreview") || path.Contains("Inventory_Virtual_Entity"))
			return;

		// 2. Filter out already-exported items:
		// Wearables and gear accessories (handled by WearableExport)
		bool hasCloth = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "LoadoutClothComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "ClothComponent");
		if (hasCloth || path.Contains("/Accessories/") || path.Contains("Pouch_") || path.Contains("Holster_") || path.Contains("Scabbard_") || path.Contains("Equip_Accessory") || path.Contains("Backpack_Base"))
		{
			if (path.Contains("Prefabs/Characters") || path.Contains("/Headgear/") || path.Contains("/Uniforms/")
				|| path.Contains("/Vests/") || path.Contains("/Footwear/") || path.Contains("/Handwear/")
				|| path.Contains("/Backpacks/") || path.Contains("/Accessories/") || path.Contains("Pouch")
				|| path.Contains("Holster") || path.Contains("Scabbard") || path.Contains("Equip_Accessory") || path.Contains("Backpack_Base"))
				return;
		}

		// Firearm weapons (handled by WeaponExport/Weapon)
		bool hasWeapon = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "WeaponComponent");
		bool isThrowable = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "GrenadeMoveComponent");
		bool isMine = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MineWeaponComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MineComponent");
		bool isExplosive = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "ExplosiveChargeComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "ExplosiveTriggerComponent");

		if (hasWeapon && !isThrowable && !isMine && !isExplosive)
			return;

		// Ammunition, magazines, submunitions, penetrators, and spalls (handled by WeaponExport/Ammo)
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MagazineComponent") && !hasWeapon)
			return;
		bool isCarryableBox = (path.Contains("AmmoBox_") || path.Contains("AmmoBoxes"));
		if (!isCarryableBox && (path.Contains("/Ammo/") || path.Contains("/Ammo_") || path.Contains("Ammo_")))
			return;

		// Weapon attachments (handled by WeaponExport/Attachment, Optic, Muzzle, etc.)
		if (HasWeaponAttachmentAttributes(comps))
			return;

		// 3. Must be an inventory item or carryable gadget
		bool hasInvItem = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "InventoryItemComponent");
		bool hasGadget = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "GadgetComponent");
		if (!hasInvItem && !hasGadget && !isThrowable && !isMine && !isExplosive && !path.Contains("Prefabs/Items"))
			return;

		string canonical = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		if (m_SeenResourceNames.Contains(canonical))
			return;

		string category = CategorizeItem(path, comps);
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

		TBD_ItemInfo info = new TBD_ItemInfo();
		info.m_sResourceName = canonical;
		info.m_sFilePath = path;
		info.m_sId = TBD_EquipmentResourceNames.GenerateSlug(path);
		info.m_sDisplayName = TBD_ItemNaming.DisplayNameFor(comps, path);
		info.m_sDescription = TBD_ItemNaming.DescriptionFor(comps);
		info.m_sIcon = TBD_ItemNaming.IconFor(comps);
		info.m_sCategory = category;
		info.m_sFamily = TBD_ItemNaming.ExtractFamily(path, category);
		info.m_sAddonId = addonId;
		info.m_bIsAbstract = isAbstract;

		// Ancestor / Variant Of
		BaseContainer anc = root.GetAncestor();
		if (anc)
		{
			string ancRn = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(anc.GetResourceName());
			if (!ancRn.IsEmpty() && ancRn != canonical)
				info.m_sVariantOf = ancRn;
		}

		// Extract detailed domain properties
		TBD_ItemExtractor.ExtractPhysical(comps, info.m_Physical);
		TBD_ItemExtractor.ExtractMedical(comps, info.m_Medical, path);
		TBD_ItemExtractor.ExtractRadio(comps, info.m_Radio, path);
		TBD_ItemExtractor.ExtractGadget(comps, info.m_Gadget, path);
		TBD_ItemExtractor.ExtractExplosive(comps, info.m_Explosive, path);
		TBD_ItemExtractor.ExtractTool(comps, info.m_Tool, path);
		TBD_ItemExtractor.ExtractSurvival(comps, info.m_Survival, path);

		InsertCategoryItem(category, info);
		m_aAllItems.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	protected bool HasWeaponAttachmentAttributes(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;
			foreach (BaseContainer comp : bucket)
			{
				BaseContainer attrs = comp.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainerList custom = attrs.GetObjectArray("CustomAttributes");
				if (!custom)
					continue;
				for (int i = 0, n = custom.Count(); i < n; i++)
				{
					BaseContainer ca = custom.Get(i);
					if (ca && ca.GetClassName() == "WeaponAttachmentAttributes")
						return true;
				}
			}
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected string CategorizeItem(string path, map<string, ref array<BaseContainer>> comps)
	{
		// 1. Component-based introspection & path heuristics
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "ConsumableItemComponent") || path.Contains("/Medicine/") || path.Contains("MedicalKit"))
			return "medical";

		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "RadioComponent") || path.Contains("/Radios/") || path.Contains("Radio_"))
			return "radios";

		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "BinocularsComponent") || path.Contains("/Binoculars/"))
			return "binoculars";

		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "FlashlightComponent") || path.Contains("/Flashlights/"))
			return "flashlights";

		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "CompassComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MapGadgetComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "WatchGadgetComponent") || path.Contains("Compass") || path.Contains("Map_") || path.Contains("/Maps/") || path.Contains("Watch") || path.Contains("GPS"))
			return "navigation";

		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "GrenadeMoveComponent") || path.Contains("/Grenades/"))
			return "throwables";

		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MineComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MineWeaponComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "ExplosiveChargeComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "ExplosiveTriggerComponent")
			|| path.Contains("/Explosives/") || path.Contains("/Detonators/") || path.Contains("BlastingMachine"))
			return "explosives";

		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MineFlagComponent") || path.Contains("ETool") || path.Contains("Shovel") || path.Contains("/Demining/") || path.Contains("RepairKit") || path.Contains("RearmingKit") || path.Contains("BarbedTape") || path.Contains("Barbed_Tape"))
			return "tools";

		if (path.Contains("Part_") || path.Contains("/Tripods/") || path.Contains("/Mortars/") || path.Contains("BallisticTable"))
			return "weapon_parts";

		if (path.Contains("/Food/") || path.Contains("Canteen") || path.Contains("/Tents/") || path.Contains("Tent") || path.Contains("/Fuel/") || path.Contains("Canister") || path.Contains("SupplyPortableContainers") || path.Contains("SupplyCrate") || path.Contains("SupplyStack") || path.Contains("AmmoBox"))
			return "survival";

		return "intel_and_misc";
	}

	//------------------------------------------------------------------------------------------------
	protected void InsertCategoryItem(string category, TBD_ItemInfo info)
	{
		if (category == "medical") m_aMedical.Insert(info);
		else if (category == "radios") m_aRadios.Insert(info);
		else if (category == "navigation") m_aNavigation.Insert(info);
		else if (category == "binoculars") m_aBinoculars.Insert(info);
		else if (category == "flashlights") m_aFlashlights.Insert(info);
		else if (category == "tools") m_aTools.Insert(info);
		else if (category == "explosives") m_aExplosives.Insert(info);
		else if (category == "throwables") m_aThrowables.Insert(info);
		else if (category == "weapon_parts") m_aWeaponParts.Insert(info);
		else if (category == "survival") m_aSurvival.Insert(info);
		else m_aIntelAndMisc.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	//! Write all category files and the combined master catalog to disk.
	void WriteAllFiles(string destDir, int elapsedMs)
	{
		WriteCategoryCatalog("medical", m_aMedical, destDir);
		WriteCategoryCatalog("radios", m_aRadios, destDir);
		WriteCategoryCatalog("navigation", m_aNavigation, destDir);
		WriteCategoryCatalog("binoculars", m_aBinoculars, destDir);
		WriteCategoryCatalog("flashlights", m_aFlashlights, destDir);
		WriteCategoryCatalog("tools", m_aTools, destDir);
		WriteCategoryCatalog("explosives", m_aExplosives, destDir);
		WriteCategoryCatalog("throwables", m_aThrowables, destDir);
		WriteCategoryCatalog("weapon_parts", m_aWeaponParts, destDir);
		WriteCategoryCatalog("survival", m_aSurvival, destDir);
		WriteCategoryCatalog("intel_and_misc", m_aIntelAndMisc, destDir);
		WriteMasterCatalog(destDir, elapsedMs);
	}

	//------------------------------------------------------------------------------------------------
	protected void WriteCategoryCatalog(string category, array<ref TBD_ItemInfo> list, string destDir)
	{
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "items", category + ".json");
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
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "items", category + "_meta.json");
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
		string filePath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "items", "items_master.json");
		FileHandle f = FileIO.OpenFile(filePath, FileMode.WRITE);
		if (!f)
		{
			Print(TAG + " Failed to open " + filePath + " for write", LogLevel.ERROR);
			return;
		}

		TBD_EquipmentExportJson.Write(f, "{\n  \"version\": \"1\",\n  \"domain\": \"items\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"totalCount\": " + m_aAllItems.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  \"counts\": {\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"medical\": " + m_aMedical.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"radios\": " + m_aRadios.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"navigation\": " + m_aNavigation.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"binoculars\": " + m_aBinoculars.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"flashlights\": " + m_aFlashlights.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"tools\": " + m_aTools.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"explosives\": " + m_aExplosives.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"throwables\": " + m_aThrowables.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"weapon_parts\": " + m_aWeaponParts.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"survival\": " + m_aSurvival.Count().ToString() + ",\n", TAG);
		TBD_EquipmentExportJson.Write(f, "    \"intel_and_misc\": " + m_aIntelAndMisc.Count().ToString() + "\n", TAG);
		TBD_EquipmentExportJson.Write(f, "  },\n  \"items\": [\n", TAG);

		for (int i = 0; i < m_aAllItems.Count(); i++)
		{
			string itemJson = m_aAllItems[i].SerializeJson("    ");
			if (i < m_aAllItems.Count() - 1)
				itemJson += ",";
			itemJson += "\n";
			TBD_EquipmentExportJson.Write(f, itemJson, TAG);
		}

		TBD_EquipmentExportJson.Write(f, "  ]\n}\n", TAG);
		f.Close();

		// Sidecar meta
		string metaPath = TBD_EquipmentExportPaths.BuildCategoryPath(destDir, "items", "items_master_meta.json");
		FileHandle mf = FileIO.OpenFile(metaPath, FileMode.WRITE);
		if (mf)
		{
			string meta = "{\n  \"totalCount\": " + m_aAllItems.Count().ToString() + ",\n  \"elapsedMs\": " + elapsedMs.ToString() + ",\n  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\"\n}\n";
			TBD_EquipmentExportJson.Write(mf, meta, TAG);
			mf.Close();
		}

		Print(string.Format("%1 MASTER CATALOG EXPORTED (%2 items) -> %3", TAG, m_aAllItems.Count(), filePath), LogLevel.NORMAL);
	}
}
