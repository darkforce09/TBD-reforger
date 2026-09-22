/**
 * TBD_EquipmentScanner.c
 *
 * Unfiltered equipment discovery scanner.
 * Scans all loaded addons, excludes vehicles, characters, and static props,
 * and discovers all equipment items (weapons, ammo, optics, vests, uniforms, gadgets).
 */

class TBD_EquipmentScanner
{
	protected static const string TAG = "[TBD][EquipmentScanner]";
	protected static const int COMPONENT_DEPTH_CAP = 8;
	protected static const int MAX_ITEMS = 200000;

	// Hard denial paths: structures, environment, vegetation, sound, editor systems.
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

	ref array<ref TBD_EquipmentScanItem> m_aDiscoveredItems = {};
	ref map<string, int> m_mItemIndexByResource = new map<string, int>();

	// Counters and stats
	int m_iSeen = 0;
	int m_iSkippedDeny = 0;
	int m_iSkippedNonEquipment = 0;
	int m_iFailedLoad = 0;

	// Signal counters
	int m_iWeaponsCount = 0;
	int m_iMagazinesCount = 0;
	int m_iClothingCount = 0;
	int m_iGadgetsCount = 0;
	int m_iAttachmentsCount = 0;
	int m_iAmmoCount = 0;
	int m_iOtherInventoryCount = 0;

	protected string m_sCurrentAddonId;
	protected ref TBD_EquipmentExportConfig m_Config;

	//------------------------------------------------------------------------------------------------
	void TBD_EquipmentScanner(TBD_EquipmentExportConfig cfg = null)
	{
		if (cfg)
			m_Config = cfg;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	//! Enumerate all loaded addons and search their Prefabs trees.
	bool ScanAllAddons()
	{
		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		if (guids.IsEmpty())
		{
			Print(TAG + " FAIL: GameProject.GetLoadedAddons returned no addons.", LogLevel.ERROR);
			return false;
		}

		array<string> extEt = { "et" };
		foreach (string guid : guids)
		{
			string addonId = GameProject.GetAddonID(guid);
			string addonTitle = GameProject.GetAddonTitle(guid);
			m_sCurrentAddonId = addonId;

			int before = m_aDiscoveredItems.Count();
			string rootPath = "$" + addonId + ":Prefabs";
			Workbench.SearchResources(OnResourceFound, extEt, null, rootPath, true);
			Print(string.Format("%1 Addon %2 (%3): %4 equipment items found (scanned %5 prefabs so far)",
				TAG, addonId, addonTitle, m_aDiscoveredItems.Count() - before, m_iSeen));
		}

		// Fallback rung: if per-addon prefix found nothing, run global search
		if (m_aDiscoveredItems.IsEmpty())
		{
			Print(TAG + " Per-addon search returned 0 items - falling back to global SearchResources pass...", LogLevel.WARNING);
			m_sCurrentAddonId = string.Empty;
			Workbench.SearchResources(OnResourceFound, extEt, null, string.Empty, true);
		}

		return !m_aDiscoveredItems.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	//! Callback invoked by Workbench.SearchResources for each resource.
	void OnResourceFound(ResourceName resName, string filePath = "")
	{
		m_iSeen++;
		if (m_aDiscoveredItems.Count() >= MAX_ITEMS)
			return;

		string path = filePath;
		if (path.IsEmpty())
			path = resName;

		// Only inspect prefab trees
		if (!path.Contains("Prefabs/"))
			return;

		foreach (string deny : DENY_HARD)
		{
			if (path.Contains(deny))
			{
				m_iSkippedDeny++;
				return;
			}
		}

		string addonId = m_sCurrentAddonId;
		if (addonId.IsEmpty() && path.StartsWith("$"))
		{
			int colon = path.IndexOf(":");
			if (colon > 1)
				addonId = path.Substring(1, colon - 1);
		}

		ProcessPrefab(resName, path, addonId);

		if (m_iSeen % 1000 == 0)
		{
			Print(string.Format("%1 Progress: %2 prefabs scanned, %3 equipment items discovered...",
				TAG, m_iSeen, m_aDiscoveredItems.Count()));
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void ProcessPrefab(string resName, string filePath, string addonId)
	{
		Resource res = Resource.Load(resName);
		if (!res || !res.IsValid())
		{
			m_iFailedLoad++;
			return;
		}

		BaseResourceObject obj = res.GetResource();
		if (!obj)
		{
			m_iFailedLoad++;
			return;
		}

		BaseContainer root = obj.ToBaseContainer();
		if (!root)
		{
			m_iFailedLoad++;
			return;
		}

		string rootClass = root.GetClassName();

		// Collect full component chain across ancestry
		map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
		TBD_EquipmentComponentGraph.CollectComponentChain(root, comps, COMPONENT_DEPTH_CAP);

		// 1. Hard exclusions: vehicles & characters
		bool isCharacter = (rootClass == "SCR_ChimeraCharacter" || rootClass == "ChimeraCharacter"
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "CharacterControllerComponent"));
		if (isCharacter)
		{
			m_iSkippedNonEquipment++;
			return;
		}

		bool isVehicle = (rootClass == "Vehicle" || rootClass.EndsWith("Vehicle")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "VehicleWheeledSimulation") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "VehicleHelicopterSimulation")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "VehicleBoatSimulation") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "VehicleTrackedSimulation"));
		if (isVehicle)
		{
			m_iSkippedNonEquipment++;
			return;
		}

		// Non-carryable static vehicle weapon assemblies / mounts
		bool hasInvItem = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "InventoryItemComponent");
		if (!hasInvItem && TBD_EquipmentComponentGraph.HasCompSuffix(comps, "CompartmentManagerComponent"))
		{
			m_iSkippedNonEquipment++;
			return;
		}

		// 2. Detect equipment signals
		bool hasWeapon = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "WeaponComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "GrenadeLauncherComponent");
		bool hasMagazine = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MagazineComponent");
		bool hasCloth = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "LoadoutClothComponent");
		bool hasGadget = HasCompInheritedFrom(comps, "SCR_GadgetComponent") || HasCompInheritedFrom(comps, "SCR_BinocularsComponent");
		bool hasAttachmentAttr = HasAttachmentAttributes(comps);
		bool isAmmoPath = filePath.Contains("/Ammo/");

		// If no equipment signal, drop it (it's world clutter, a building, or a logic object)
		if (!hasInvItem && !hasWeapon && !hasMagazine && !hasCloth && !hasGadget && !hasAttachmentAttr && !isAmmoPath)
		{
			m_iSkippedNonEquipment++;
			return;
		}

		// Canonical resource locator
		string canonical = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		// Deduplicate
		if (m_mItemIndexByResource.Contains(canonical))
			return;

		// Abstract / template check
		bool isAbstract = CheckIsAbstract(filePath, root);
		if (!m_Config.m_bIncludeAbstract && isAbstract)
			return;

		// Build item record
		TBD_EquipmentScanItem item = new TBD_EquipmentScanItem();
		item.m_sResourceName = canonical;
		item.m_sFilePath = filePath;
		item.m_sId = TBD_EquipmentResourceNames.GenerateSlug(filePath);
		item.m_sAddonId = addonId;
		item.m_sCategoryPath = ExtractCategoryPath(filePath);
		item.m_sRootClass = rootClass;
		item.m_bIsAbstract = isAbstract;
		item.m_sDisplayName = DisplayNameFor(comps, filePath);

		// Record signals
		if (hasWeapon)
		{
			item.m_aSignals.Insert("weapon");
			m_iWeaponsCount++;
		}
		if (hasMagazine)
		{
			item.m_aSignals.Insert("magazine");
			m_iMagazinesCount++;
		}
		if (hasCloth)
		{
			item.m_aSignals.Insert("cloth");
			m_iClothingCount++;
		}
		if (hasGadget)
		{
			item.m_aSignals.Insert("gadget");
			m_iGadgetsCount++;
		}
		if (hasAttachmentAttr)
		{
			item.m_aSignals.Insert("attachment");
			m_iAttachmentsCount++;
		}
		if (isAmmoPath && !hasMagazine)
		{
			item.m_aSignals.Insert("ammo");
			m_iAmmoCount++;
		}
		if (hasInvItem && !hasWeapon && !hasMagazine && !hasCloth && !hasGadget && !hasAttachmentAttr && !isAmmoPath)
		{
			item.m_aSignals.Insert("inventory_item");
			m_iOtherInventoryCount++;
		}

		// Read physical attributes
		ReadPhysAttrs(comps, item);

		// Collect all component class names found
		foreach (string compCls, array<BaseContainer> b : comps)
		{
			if (item.m_aDetectedComponents.Find(compCls) == -1)
				item.m_aDetectedComponents.Insert(compCls);
		}

		m_mItemIndexByResource.Insert(canonical, m_aDiscoveredItems.Count());
		m_aDiscoveredItems.Insert(item);
	}

	//------------------------------------------------------------------------------------------------
	protected bool HasCompInheritedFrom(map<string, ref array<BaseContainer>> comps, string baseClass)
	{
		typename baseType = baseClass.ToType();
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls == baseClass || cls.EndsWith(baseClass))
				return true;
			if (baseType)
			{
				typename t = cls.ToType();
				if (t && t.IsInherited(baseType))
					return true;
			}
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected bool HasAttachmentAttributes(map<string, ref array<BaseContainer>> comps)
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
	protected void ReadPhysAttrs(map<string, ref array<BaseContainer>> comps, TBD_EquipmentScanItem item)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("StorageComponent"))
				continue;

			foreach (BaseContainer comp : bucket)
			{
				BaseContainer attrs = comp.GetObject("Attributes");
				if (attrs)
				{
					BaseContainer phys = attrs.GetObject("ItemPhysAttributes");
					if (phys)
					{
						float w;
						if (item.m_fWeightKg < 0 && phys.Get("Weight", w) && w >= 0)
							item.m_fWeightKg = w;
						float v;
						if (item.m_fVolumeCm3 < 0 && phys.Get("ItemVolume", v) && v >= 0)
							item.m_fVolumeCm3 = v;
					}
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected string DisplayNameFor(map<string, ref array<BaseContainer>> comps, string filePath)
	{
		// 1. Try InventoryItemComponent.Attributes.ItemDisplayName.Name
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;
			foreach (BaseContainer inv : bucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer disp = attrs.GetObject("ItemDisplayName");
				if (!disp)
					continue;
				string n;
				if (disp.Get("Name", n) && !n.IsEmpty() && !n.StartsWith("#"))
					return n;
			}
		}

		// 2. Try WeaponComponent / MagazineComponent UIInfo Name
		array<string> uiHolders = {"WeaponComponent", "MagazineComponent"};
		foreach (string holder : uiHolders)
		{
			array<BaseContainer> bucket2 = comps.Get(holder);
			if (!bucket2)
				continue;
			foreach (BaseContainer c : bucket2)
			{
				BaseContainer ui = c.GetObject("UIInfo");
				if (!ui)
					continue;
				string n2;
				if (ui.Get("Name", n2) && !n2.IsEmpty() && !n2.StartsWith("#"))
					return n2;
			}
		}

		// Fallback: Humanize file stem
		return HumanizeStem(filePath);
	}

	//------------------------------------------------------------------------------------------------
	protected string HumanizeStem(string filePath)
	{
		string stem = filePath;
		int slash = stem.LastIndexOf("/");
		if (slash >= 0)
			stem = stem.Substring(slash + 1, stem.Length() - slash - 1);
		if (stem.EndsWith(".et"))
			stem = stem.Substring(0, stem.Length() - 3);

		if (stem.StartsWith("Rifle_"))
			stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Pistol_"))
			stem = stem.Substring(7, stem.Length() - 7);
		else if (stem.StartsWith("MG_"))
			stem = stem.Substring(3, stem.Length() - 3);
		else if (stem.StartsWith("Magazine_"))
			stem = stem.Substring(9, stem.Length() - 9);
		else if (stem.StartsWith("Vest_"))
			stem = stem.Substring(5, stem.Length() - 5);
		else if (stem.StartsWith("Uniform_"))
			stem = stem.Substring(8, stem.Length() - 8);
		else if (stem.StartsWith("Jacket_"))
			stem = stem.Substring(7, stem.Length() - 7);
		else if (stem.StartsWith("Pants_"))
			stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Helmet_"))
			stem = stem.Substring(7, stem.Length() - 7);

		stem.Replace("_", " ");
		stem.Trim();
		return stem;
	}

	//------------------------------------------------------------------------------------------------
	protected string ExtractCategoryPath(string filePath)
	{
		int idx = filePath.IndexOf("Prefabs/");
		if (idx < 0)
			return "";
		string sub = filePath.Substring(idx + 8, filePath.Length() - idx - 8);
		int lastSlash = sub.LastIndexOf("/");
		if (lastSlash < 0)
			return "";
		return sub.Substring(0, lastSlash);
	}

	//------------------------------------------------------------------------------------------------
	protected bool CheckIsAbstract(string filePath, BaseContainer root)
	{
		string lower = filePath;
		lower.ToLower();
		if (lower.EndsWith("_base.et") || lower.Contains("/base/"))
			return true;
		return false;
	}
}
