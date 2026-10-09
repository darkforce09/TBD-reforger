/**
 * TBD_EquipmentScanner.c
 *
 * Equipment discovery index across the established addon prefab search roots.
 * Native root capabilities remain distinct from components installed in child entities.
 * Names and measurements use the same source readers as the specialist catalogs.
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
		TBD_EquipmentExportJson.BeginDomainMetadata();
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
		{
			TBD_EquipmentExportJson.ExtractionError(resName, "discovery", "Equipment discovery limit reached");
			return;
		}

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

		// Collect effective component instances without appending ancestor installations.
		map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
		TBD_EquipmentComponentGraph.CollectComponentChain(root, comps, COMPONENT_DEPTH_CAP);
		map<string, ref array<BaseContainer>> rootComps = RootComponents(comps);

		// 1. Hard exclusions: vehicles & characters
		bool isCharacter = TBD_EquipmentComponentGraph.IsA(rootClass, "ChimeraCharacter")
			|| HasCompInheritedFrom(rootComps, "CharacterControllerComponent");
		if (isCharacter)
		{
			m_iSkippedNonEquipment++;
			return;
		}

		bool isVehicle = TBD_EquipmentComponentGraph.IsA(rootClass, "Vehicle")
			|| HasCompInheritedFrom(rootComps, "VehicleWheeledSimulation") || HasCompInheritedFrom(rootComps, "VehicleHelicopterSimulation")
			|| HasCompInheritedFrom(rootComps, "VehicleBoatSimulation") || HasCompInheritedFrom(rootComps, "VehicleTrackedSimulation");
		if (isVehicle)
		{
			m_iSkippedNonEquipment++;
			return;
		}

		// Non-carryable static vehicle weapon assemblies / mounts
		bool isStaticWeapon = (rootClass == "Turret" || filePath.Contains("/Tripods/") || filePath.Contains("/Mortars/"));
		bool isVehicleAssembly = (filePath.Contains("/VehParts/") || filePath.Contains("Prefabs/Vehicles/"));
		bool hasInvItem = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "InventoryItemComponent");
		if (!hasInvItem && TBD_EquipmentComponentGraph.HasCompSuffix(comps, "CompartmentManagerComponent"))
		{
			if (isVehicleAssembly || !isStaticWeapon)
			{
				m_iSkippedNonEquipment++;
				return;
			}
		}

		// 2. Detect equipment signals
		bool hasWeapon = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "WeaponComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "GrenadeLauncherComponent");
		bool hasMagazine = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MagazineComponent");
		bool hasCloth = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "LoadoutClothComponent");
		bool hasGadget = HasCompInheritedFrom(comps, "SCR_GadgetComponent") || HasCompInheritedFrom(comps, "SCR_BinocularsComponent");
		bool hasAttachmentAttr = HasAttachmentAttributes(comps);
		bool isAmmoPath = filePath.Contains("/Ammo/");
		bool hasStatic = isStaticWeapon || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "TurretComponent");

		// If no equipment signal, drop it (it's world clutter, a building, or a logic object)
		if (!hasInvItem && !hasWeapon && !hasMagazine && !hasCloth && !hasGadget && !hasAttachmentAttr && !isAmmoPath && !hasStatic)
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

		// Build item record
		TBD_EquipmentScanItem item = new TBD_EquipmentScanItem();
		item.m_sResourceName = canonical;
		item.m_sFilePath = filePath;
		item.m_sId = TBD_EquipmentResourceNames.GenerateSlug(filePath);
		item.m_sAddonId = addonId;
		item.m_sCategoryPath = ExtractCategoryPath(filePath);
		item.m_sRootClass = rootClass;
		item.m_sNamesJson = TBD_EquipmentDisplayAttributes.Names(rootComps);

		// Search-path admission preserves technical resources; capability signals require root-rootOwnedInstances native components.
		hasWeapon = HasCompInheritedFrom(rootComps, "WeaponComponent") || HasCompInheritedFrom(rootComps, "GrenadeLauncherComponent");
		hasMagazine = HasCompInheritedFrom(rootComps, "MagazineComponent");
		hasCloth = HasCompInheritedFrom(rootComps, "BaseLoadoutClothComponent");
		hasGadget = HasCompInheritedFrom(rootComps, "SCR_GadgetComponent") || HasCompInheritedFrom(rootComps, "SCR_BinocularsComponent");
		hasAttachmentAttr = HasAttachmentAttributes(rootComps);
		hasInvItem = HasCompInheritedFrom(rootComps, "InventoryItemComponent");
		hasStatic = TBD_EquipmentComponentGraph.IsA(rootClass, "Turret") || HasCompInheritedFrom(rootComps, "TurretComponent");
		bool hasProjectile = TBD_EquipmentComponentGraph.IsA(rootClass, "Projectile") || HasCompInheritedFrom(rootComps, "ProjectileMoveComponent");
		hasProjectile = hasProjectile || HasCompInheritedFrom(rootComps, "ShellMoveComponent") || HasCompInheritedFrom(rootComps, "MissileMoveComponent");
		hasProjectile = hasProjectile || HasCompInheritedFrom(rootComps, "GrenadeMoveComponent");

		// Record source-backed capability presence, without a selectable or abstract claim.
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
		if (hasProjectile)
		{
			item.m_aSignals.Insert("projectile");
			m_iAmmoCount++;
		}
		if (hasStatic)
		{
			item.m_aSignals.Insert("static_weapon");
		}
		if (hasInvItem && !hasWeapon && !hasMagazine && !hasCloth && !hasGadget && !hasAttachmentAttr && !hasProjectile && !hasStatic)
		{
			item.m_aSignals.Insert("inventory_item");
			m_iOtherInventoryCount++;
		}

		// Read physical attributes
		ReadPhysAttrs(rootComps, item);

		// Collect all component class names found
		foreach (string compCls, array<BaseContainer> b : comps)
		{
			if (item.m_aDetectedComponents.Find(compCls) == -1)
				item.m_aDetectedComponents.Insert(compCls);
		}

		foreach (string rootCompCls, array<BaseContainer> rootBucket : rootComps)
			item.m_aRootComponents.Insert(rootCompCls);

		m_mItemIndexByResource.Insert(canonical, m_aDiscoveredItems.Count());
		m_aDiscoveredItems.Insert(item);
	}

	//------------------------------------------------------------------------------------------------
	//! Checks the registered native type hierarchy rather than class-name suffixes.
	protected bool HasCompInheritedFrom(map<string, ref array<BaseContainer>> comps, string baseClass)
	{
		return TBD_EquipmentComponentGraph.HasCompSuffix(comps, baseClass);
	}

	//! Root-owned component signatures do not inherit equipment signals from child entities.
	protected map<string, ref array<BaseContainer>> RootComponents(map<string, ref array<BaseContainer>> comps)
	{
		map<string, ref array<BaseContainer>> rootOwnedInstances = new map<string, ref array<BaseContainer>>();
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			array<BaseContainer> instances = {};
			foreach (BaseContainer component : bucket)
				if (!TBD_EquipmentComponentGraph.StructuralPath(component).Contains("/children/")) instances.Insert(component);
			if (!instances.IsEmpty()) rootOwnedInstances.Insert(cls, instances);
		}
		return rootOwnedInstances;
	}

	//------------------------------------------------------------------------------------------------
	protected bool HasAttachmentAttributes(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "InventoryItemComponent"))
				continue;
			foreach (BaseContainer comp : bucket)
			{
				BaseContainer attrs = comp.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainerList custom = TBD_EquipmentDisplayAttributes.GetCustomAttributes(attrs);
				if (!custom)
					continue;
				for (int i = 0, n = custom.Count(); i < n; i++)
				{
					BaseContainer ca = custom.Get(i);
					if (ca && TBD_EquipmentComponentGraph.IsA(ca.GetClassName(), "WeaponAttachmentAttributes"))
						return true;
				}
			}
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Stores native inventory measurements separately from root physics-body measurements.
	protected void ReadPhysAttrs(map<string, ref array<BaseContainer>> comps, TBD_EquipmentScanItem item)
	{
		item.m_sInventoryJson = TBD_ItemInventoryExtractor.Inventory(comps);
		item.m_sPhysicsJson = TBD_ItemInventoryExtractor.Physics(comps);
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

}
