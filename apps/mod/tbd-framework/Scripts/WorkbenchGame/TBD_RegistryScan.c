/**
 * TBD_RegistryScan.c - T-150 universal registry scan helpers (non-plugin classes).
 *
 * Mod-agnostic prefab scanner used by TBD_RegistryItemsExportPlugin: enumerates every
 * .et under $<addon>:Prefabs for all loaded addons (Workbench.SearchResources +
 * GameProject.GetLoadedAddons), classifies items by component introspection over the
 * raw prefab container tree (BaseContainer, ancestry walk included — variants keep
 * their facts on ancestor copies), and derives compat edges from engine data only:
 * magazine wells, attachment slot types, vehicle weapon slot chains, character
 * loadout slots. No curated GUID/path lists anywhere.
 *
 * @contract registry-items.schema.json#/
 * @contract registry-compat.schema.json#/
 */

//! One loaded addon (scan-set metadata for the export envelopes).
class TBD_RegistryAddonInfo
{
	string guid;
	string id;
	string title;
	bool isVanilla; // 'vanilla' is a reserved EnfScript keyword (modded-class super call)
}

//! Per-prefab scan result: catalog row fields + the raw compat facts read from containers.
class TBD_RegistryScanItem
{
	string resourceName;   // canonical {GUID}Prefabs/....et
	string displayName;
	string category;       // <addonId>/<path between Prefabs/ and the file>
	string kind;           // registry-items.schema.json v2 kind
	string addonId;

	// Weapon-side facts (hand weapons and vehicle weapons alike).
	ref array<string> muzzleWells = {};      // MuzzleComponent.MagazineWell class names
	ref array<string> magTemplates = {};     // MuzzleComponent.MagazineTemplate targets (canonical)
	ref array<string> slotAttachTypes = {};  // AttachmentSlotComponent.AttachmentType class names

	// Magazine-side / attachment-side facts.
	string magWell;                          // MagazineComponent.MagazineWell class name
	string itemAttachType;                   // WeaponAttachmentAttributes.AttachmentType class name

	// Vehicle / character facts.
	ref array<string> vehicleWeaponRefs = {}; // WeaponSlotComponent.WeaponTemplate targets (canonical)
	ref array<string> loadoutPrefabs = {};    // BaseLoadoutManagerComponent LoadoutSlotInfo.Prefab targets
}

//! One derived compat edge (registry-compat.schema.json $defs/edge).
class TBD_RegistryEdge
{
	string fromNode;
	string toNode;
	string edgeType;
	string evidence;
}

//! The scanner. Instantiate once per export run.
class TBD_RegistryScanner
{
	// Skip-without-load path fragments: pure world dressing that cannot be an item. /Props/
	// is deliberately NOT hard-skipped (supply crates and arsenal boxes live there) — those
	// paths flow through normal classification, which drops component-less prefabs anyway.
	protected static const ref array<string> DENY_HARD = {"/Structures/", "/Rocks/", "/Trees/", "/Debris/", "/Foliage/"};

	//! Component classes whose UIInfo.Name is a usable display name (probe order).
	protected static const ref array<string> UI_HOLDERS = {"WeaponComponent", "MagazineComponent"};

	protected static const int MAX_ITEMS = 200000;
	protected static const int ANCESTOR_CAP = 16;
	protected static const int COMPONENT_DEPTH_CAP = 8;
	protected static const int VEHICLE_SLOT_DEPTH_CAP = 4;

	ref array<ref TBD_RegistryScanItem> m_Items = {};
	ref map<string, int> m_ItemIndexByRn = new map<string, int>();
	ref array<ref TBD_RegistryAddonInfo> m_Addons = {};

	// DeriveEdges output (members — strong-ref containers cannot be method arguments).
	ref array<ref TBD_RegistryEdge> m_Edges = {};
	ref map<string, int> m_EdgeHistogram = new map<string, int>();
	protected ref map<string, bool> m_EmittedKeys = new map<string, bool>();
	int m_iDroppedEndpoints;

	int m_iSkippedDeny;
	int m_iSkippedNoSignal;
	int m_iFailedLoad;
	int m_iSeen;
	protected string m_sCurrentAddonId;
	protected string m_sLogTag;

	//------------------------------------------------------------------------------------------------
	void TBD_RegistryScanner(string logTag)
	{
		m_sLogTag = logTag;
	}

	//------------------------------------------------------------------------------------------------
	//! Enumerate all loaded addons and scan each addon's Prefabs tree. Returns false when the
	//! enumeration API yielded nothing at all (caller must fail loudly, not write empty files).
	bool ScanLoadedAddons()
	{
		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		if (guids.IsEmpty())
		{
			Print(m_sLogTag + " GameProject.GetLoadedAddons returned no addons", LogLevel.ERROR);
			return false;
		}

		foreach (string guid : guids)
		{
			TBD_RegistryAddonInfo info = new TBD_RegistryAddonInfo();
			info.guid = guid;
			info.id = GameProject.GetAddonID(guid);
			info.title = GameProject.GetAddonTitle(guid);
			info.isVanilla = GameProject.IsVanillaAddon(guid);
			m_Addons.Insert(info);
		}

		foreach (TBD_RegistryAddonInfo addon : m_Addons)
		{
			int before = m_Items.Count();
			m_sCurrentAddonId = addon.id;
			string root = "$" + addon.id + ":Prefabs";
			Workbench.SearchResources(OnResourceFound, {"et"}, null, root, true);
			Print(string.Format("%1 addon %2 (%3): %4 items (seen %5 so far)", m_sLogTag, addon.id, addon.guid, m_Items.Count() - before, m_iSeen));
		}

		// Fallback rung: some Workbench builds only search the whole database with an empty
		// rootPath. Re-run globally and bucket per addon via the "$Addon:" filePath prefix.
		if (m_Items.IsEmpty())
		{
			Print(m_sLogTag + " per-addon rootPath search found nothing — retrying one global SearchResources pass", LogLevel.WARNING);
			m_sCurrentAddonId = string.Empty;
			Workbench.SearchResources(OnResourceFound, {"et"}, null, string.Empty, true);
		}

		return !m_Items.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	//! Workbench.SearchResources callback (WorkbenchSearchResourcesCallback shape).
	void OnResourceFound(ResourceName resName, string filePath = "")
	{
		m_iSeen++;
		if (m_Items.Count() >= MAX_ITEMS)
			return;

		string path = filePath;
		if (path.IsEmpty())
			path = resName;

		// Only prefab trees are items; the global fallback rung sees every addon file.
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

		if (m_iSeen % 500 == 0)
			Print(string.Format("%1 progress: seen %2, items %3", m_sLogTag, m_iSeen, m_Items.Count()));
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

		string canonical = root.GetResourceName();
		if (canonical.IsEmpty())
			canonical = resName;
		if (m_ItemIndexByRn.Contains(canonical))
			return;

		map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
		CollectComponentChain(root, comps);

		TBD_RegistryScanItem item = new TBD_RegistryScanItem();
		item.resourceName = canonical;
		item.addonId = addonId;
		item.category = CategoryFor(addonId, filePath);

		if (!ClassifyAndCollect(root, comps, filePath, item))
		{
			m_iSkippedNoSignal++;
			return;
		}

		item.displayName = DisplayNameFor(comps, filePath);

		m_ItemIndexByRn.Insert(canonical, m_Items.Count());
		m_Items.Insert(item);
	}

	//------------------------------------------------------------------------------------------------
	//! Collect every component container across the prefab's ancestry, keyed by component class
	//! name. Values are ordered most-derived-first; nested component blocks (e.g. MuzzleComponent
	//! inside WeaponComponent, AttachmentSlotComponent inside MuzzleComponent) are included.
	protected void CollectComponentChain(BaseContainer prefabRoot, notnull map<string, ref array<BaseContainer>> outComps)
	{
		BaseContainer cur = prefabRoot;
		int hops = 0;
		while (cur && hops < ANCESTOR_CAP)
		{
			CollectComponentsRec(cur, outComps, 0);
			cur = cur.GetAncestor();
			hops++;
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void CollectComponentsRec(BaseContainer holder, notnull map<string, ref array<BaseContainer>> outComps, int depth)
	{
		if (depth > COMPONENT_DEPTH_CAP)
			return;

		BaseContainerList comps = holder.GetObjectArray("components");
		if (!comps)
			return;

		for (int i = 0, n = comps.Count(); i < n; i++)
		{
			BaseContainer comp = comps.Get(i);
			if (!comp)
				continue;

			string cls = comp.GetClassName();
			array<BaseContainer> bucket = outComps.Get(cls);
			if (!bucket)
			{
				bucket = {};
				outComps.Insert(cls, bucket);
			}
			bucket.Insert(comp);

			CollectComponentsRec(comp, outComps, depth + 1);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected bool HasComp(map<string, ref array<BaseContainer>> comps, string exactClass)
	{
		return comps.Contains(exactClass);
	}

	//------------------------------------------------------------------------------------------------
	//! True when any collected component class name ends with the given suffix (e.g. every
	//! *MuzzleComponent specialisation).
	protected bool HasCompSuffix(map<string, ref array<BaseContainer>> comps, string suffix)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.EndsWith(suffix))
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! All non-null object-var class names for varName across every container of every class whose
	//! name ends with classSuffix, deduped, most-derived first.
	protected void CollectObjectVarClasses(map<string, ref array<BaseContainer>> comps, string classSuffix, string varName, notnull array<string> outClasses)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith(classSuffix))
				continue;
			foreach (BaseContainer c : bucket)
			{
				BaseContainer o = c.GetObject(varName);
				if (!o)
					continue;
				string ocls = o.GetClassName();
				if (!ocls.IsEmpty() && outClasses.Find(ocls) == -1)
					outClasses.Insert(ocls);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! All non-empty string/ResourceName values for varName across containers whose class name ends
	//! with classSuffix, resolved to canonical ResourceNames, deduped.
	protected void CollectResourceVarValues(map<string, ref array<BaseContainer>> comps, string classSuffix, string varName, notnull array<string> outValues)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith(classSuffix))
				continue;
			foreach (BaseContainer c : bucket)
			{
				string v;
				if (!c.Get(varName, v) || v.IsEmpty())
					continue;
				string canonical = ResolveCanonical(v);
				if (!canonical.IsEmpty() && outValues.Find(canonical) == -1)
					outValues.Insert(canonical);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Classify the prefab into a registry-items v2 kind and collect its compat facts.
	//! Returns false when the prefab carries no item signal at all (world dressing).
	protected bool ClassifyAndCollect(BaseContainer root, map<string, ref array<BaseContainer>> comps, string filePath, TBD_RegistryScanItem item)
	{
		string rootClass = root.GetClassName();

		bool isCharacter = rootClass == "SCR_ChimeraCharacter" || rootClass == "ChimeraCharacter"
			|| HasCompSuffix(comps, "CharacterControllerComponent");
		bool isVehicle = !isCharacter && (rootClass == "Vehicle" || rootClass.EndsWith("Vehicle")
			|| HasCompSuffix(comps, "VehicleWheeledSimulation") || HasCompSuffix(comps, "VehicleHelicopterSimulation")
			|| HasCompSuffix(comps, "VehicleBoatSimulation") || HasCompSuffix(comps, "VehicleTrackedSimulation"));
		bool hasWeapon = HasComp(comps, "WeaponComponent") || HasCompSuffix(comps, "GrenadeLauncherComponent");
		bool hasMagazine = HasComp(comps, "MagazineComponent");
		bool hasInventoryItem = HasCompSuffix(comps, "InventoryItemComponent");
		bool hasCloth = HasCompSuffix(comps, "LoadoutClothComponent");
		bool hasStorage = HasCompSuffix(comps, "UniversalInventoryStorageComponent") || HasCompSuffix(comps, "ArsenalComponent");
		bool hasTurretWeaponSlot = HasComp(comps, "WeaponSlotComponent");

		if (isVehicle)
		{
			item.kind = "vehicle";
			CollectVehicleWeapons(root, comps, item.vehicleWeaponRefs, 0);
			return true;
		}

		if (isCharacter)
		{
			item.kind = "character";
			CollectLoadoutPrefabs(comps, item.loadoutPrefabs);
			return true;
		}

		if (hasMagazine && !hasWeapon)
		{
			item.kind = "magazine";
			array<string> wells = {};
			CollectObjectVarClasses(comps, "MagazineComponent", "MagazineWell", wells);
			if (!wells.IsEmpty())
				item.magWell = wells[0];
			return true;
		}

		if (hasWeapon)
		{
			CollectObjectVarClasses(comps, "MuzzleComponent", "MagazineWell", item.muzzleWells);
			CollectResourceVarValues(comps, "MuzzleComponent", "MagazineTemplate", item.magTemplates);
			CollectObjectVarClasses(comps, "AttachmentSlotComponent", "AttachmentType", item.slotAttachTypes);

			// Standalone turret/pintle weapons carry WeaponSlotComponent hosts on vehicles; the
			// vehicle pass reclassifies referenced targets to vehicle_weapon after the scan.
			if (filePath.Contains("/Handguns/"))
				item.kind = "gear_handgun";
			else if (filePath.Contains("/Launchers/"))
				item.kind = "gear_launcher";
			else
				item.kind = "gear_primary";
			return true;
		}

		if (hasCloth)
		{
			array<string> areas = {};
			CollectObjectVarClasses(comps, "LoadoutClothComponent", "AreaType", areas);
			string area = "";
			if (!areas.IsEmpty())
				area = areas[0];

			if (area.Contains("Jacket") || area.Contains("Pants") || area.Contains("Boots"))
				item.kind = "gear_uniform";
			else if (area.Contains("Vest"))
				item.kind = "gear_vest";
			else if (area.Contains("HeadCover") || area.Contains("Cover"))
				item.kind = "gear_helmet";
			else if (area.Contains("Backpack") || area.Contains("Back"))
				item.kind = "gear_backpack";
			else
				item.kind = "other";
			return true;
		}

		// Attachment items declare their type in InventoryItemComponent.Attributes.CustomAttributes.
		string attachType = ItemAttachmentType(comps);
		if (!attachType.IsEmpty())
		{
			item.itemAttachType = attachType;
			if (attachType.Contains("Optics") || HasCompSuffix(comps, "SightsComponent"))
				item.kind = "optic";
			else
				item.kind = "attachment";
			return true;
		}

		if (filePath.Contains("/Ammo/"))
		{
			item.kind = "ammo";
			return true;
		}

		if (hasStorage && !hasInventoryItem && hasTurretWeaponSlot)
		{
			// Gun mounts / VehParts hosting a weapon: classify as vehicle_weapon host part.
			item.kind = "vehicle_weapon";
			CollectResourceVarValues(comps, "WeaponSlotComponent", "WeaponTemplate", item.vehicleWeaponRefs);
			return true;
		}

		if (hasStorage && !hasInventoryItem)
		{
			item.kind = "crate";
			return true;
		}

		if (hasInventoryItem)
		{
			item.kind = "other";
			return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! InventoryItemComponent.Attributes(SCR_ItemAttributeCollection).CustomAttributes[] →
	//! WeaponAttachmentAttributes.AttachmentType class name (empty when absent).
	protected string ItemAttachmentType(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;
			foreach (BaseContainer inv : bucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainerList custom = attrs.GetObjectArray("CustomAttributes");
				if (!custom)
					continue;
				for (int i = 0, n = custom.Count(); i < n; i++)
				{
					BaseContainer ca = custom.Get(i);
					if (!ca || ca.GetClassName() != "WeaponAttachmentAttributes")
						continue;
					BaseContainer t = ca.GetObject("AttachmentType");
					if (t)
						return t.GetClassName();
				}
			}
		}
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Vehicle pass: walk SlotManagerComponent slots (Roof → Turret → gun mount …) through VehPart
	//! prefab refs, collecting every WeaponSlotComponent.WeaponTemplate target transitively.
	protected void CollectVehicleWeapons(BaseContainer root, map<string, ref array<BaseContainer>> comps, notnull array<string> outWeapons, int depth)
	{
		CollectResourceVarValues(comps, "WeaponSlotComponent", "WeaponTemplate", outWeapons);

		if (depth >= VEHICLE_SLOT_DEPTH_CAP)
			return;

		array<string> slotPrefabs = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("SlotManagerComponent"))
				continue;
			foreach (BaseContainer mgr : bucket)
			{
				BaseContainerList slots = mgr.GetObjectArray("Slots");
				if (!slots)
					continue;
				for (int i = 0, n = slots.Count(); i < n; i++)
				{
					BaseContainer slot = slots.Get(i);
					if (!slot)
						continue;
					string prefab;
					if (!slot.Get("Prefab", prefab) || prefab.IsEmpty())
						continue;
					string canonical = ResolveCanonical(prefab);
					if (!canonical.IsEmpty() && slotPrefabs.Find(canonical) == -1)
						slotPrefabs.Insert(canonical);
				}
			}
		}

		foreach (string partRn : slotPrefabs)
		{
			Resource res = Resource.Load(partRn);
			if (!res || !res.IsValid())
				continue;
			BaseResourceObject obj = res.GetResource();
			if (!obj)
				continue;
			BaseContainer part = obj.ToBaseContainer();
			if (!part)
				continue;

			map<string, ref array<BaseContainer>> partComps = new map<string, ref array<BaseContainer>>();
			CollectComponentChain(part, partComps);
			CollectVehicleWeapons(part, partComps, outWeapons, depth + 1);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Character pass: BaseLoadoutManagerComponent.Slots[].Prefab → default cloth/gear prefabs.
	protected void CollectLoadoutPrefabs(map<string, ref array<BaseContainer>> comps, notnull array<string> outPrefabs)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("LoadoutManagerComponent"))
				continue;
			foreach (BaseContainer mgr : bucket)
			{
				BaseContainerList slots = mgr.GetObjectArray("Slots");
				if (!slots)
					continue;
				for (int i = 0, n = slots.Count(); i < n; i++)
				{
					BaseContainer slot = slots.Get(i);
					if (!slot)
						continue;
					string prefab;
					if (!slot.Get("Prefab", prefab) || prefab.IsEmpty())
						continue;
					string canonical = ResolveCanonical(prefab);
					if (!canonical.IsEmpty() && outPrefabs.Find(canonical) == -1)
						outPrefabs.Insert(canonical);
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Display name: first UIInfo-style Name across item/weapon/magazine/editable-vehicle
	//! attributes; localisation keys ("#AR-...") and misses fall back to a humanised file stem.
	protected string DisplayNameFor(map<string, ref array<BaseContainer>> comps, string filePath)
	{
		string name = FirstUiName(comps);
		if (!name.IsEmpty() && !name.StartsWith("#"))
			return name;
		return HumanizeStem(filePath);
	}

	//------------------------------------------------------------------------------------------------
	protected string FirstUiName(map<string, ref array<BaseContainer>> comps)
	{
		// InventoryItemComponent.Attributes.ItemDisplayName.Name
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
				if (disp.Get("Name", n) && !n.IsEmpty())
					return n;
			}
		}

		// WeaponComponent / MagazineComponent / SCR_EditableVehicleComponent UIInfo Name
		foreach (string holder : UI_HOLDERS)
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
				if (ui.Get("Name", n2) && !n2.IsEmpty())
					return n2;
			}
		}

		array<BaseContainer> editable = comps.Get("SCR_EditableVehicleComponent");
		if (editable)
		{
			foreach (BaseContainer c : editable)
			{
				BaseContainer ui = c.GetObject("m_UIInfo");
				if (!ui)
					continue;
				string n3;
				if (ui.Get("Name", n3) && !n3.IsEmpty())
					return n3;
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	protected string HumanizeStem(string filePath)
	{
		string stem = filePath;
		int slash = stem.LastIndexOf("/");
		if (slash >= 0)
			stem = stem.Substring(slash + 1, stem.Length() - slash - 1);
		stem.Replace(".et", "");
		stem.Replace("_", " ");
		if (stem.IsEmpty())
			return "Unknown";
		return stem;
	}

	//------------------------------------------------------------------------------------------------
	protected string CategoryFor(string addonId, string filePath)
	{
		string category = addonId;
		if (category.IsEmpty())
			category = "Unknown";

		int idx = filePath.IndexOf("Prefabs/");
		if (idx >= 0)
		{
			string sub = filePath.Substring(idx + 8, filePath.Length() - idx - 8);
			int slash = sub.LastIndexOf("/");
			if (slash > 0)
				category = category + "/" + sub.Substring(0, slash);
		}
		return category;
	}

	//------------------------------------------------------------------------------------------------
	//! Resolve a prefab path or ResourceName to the canonical {GUID}path form via the engine.
	string ResolveCanonical(string pathOrName)
	{
		if (pathOrName.StartsWith("{"))
			return pathOrName;

		Resource res = Resource.Load(pathOrName);
		if (!res || !res.IsValid())
			return string.Empty;

		BaseResourceObject obj = res.GetResource();
		if (!obj)
			return string.Empty;

		BaseContainer ctr = obj.ToBaseContainer();
		if (!ctr)
			return string.Empty;

		return ctr.GetResourceName();
	}

	//------------------------------------------------------------------------------------------------
	//! item type A fits slot type T when A == T or A derives from T (slot accepts a base type,
	//! the item declares a specialisation). Unresolvable typenames fall back to exact match only.
	protected bool AttachTypeFits(string itemType, string slotType)
	{
		if (itemType == slotType)
			return true;
		typename ti = itemType.ToType();
		typename ts = slotType.ToType();
		if (!ti || !ts)
			return false;
		return ti.IsInherited(ts);
	}

	//------------------------------------------------------------------------------------------------
	//! Derive the compat edge graph from the scanned item facts into m_Edges/m_EdgeHistogram.
	//! Every endpoint is an item from this scan by construction (misses count as dropped).
	void DeriveEdges()
	{
		m_Edges.Clear();
		m_EdgeHistogram.Clear();
		m_EmittedKeys.Clear();
		m_iDroppedEndpoints = 0;

		// Reclassify weapons referenced by vehicle weapon slots as vehicle_weapon.
		foreach (TBD_RegistryScanItem vehicle : m_Items)
		{
			if (vehicle.kind != "vehicle" && vehicle.kind != "vehicle_weapon")
				continue;
			foreach (string weaponRn : vehicle.vehicleWeaponRefs)
			{
				int idx;
				if (m_ItemIndexByRn.Find(weaponRn, idx))
				{
					TBD_RegistryScanItem target = m_Items[idx];
					if (target.kind == "gear_primary" || target.kind == "gear_handgun" || target.kind == "gear_launcher")
						target.kind = "vehicle_weapon";
				}
			}
		}

		// Index magazines by well class.
		map<string, ref array<int>> magsByWell = new map<string, ref array<int>>();
		for (int i = 0, n = m_Items.Count(); i < n; i++)
		{
			TBD_RegistryScanItem it = m_Items[i];
			if (it.kind != "magazine" || it.magWell.IsEmpty())
				continue;
			array<int> bucket = magsByWell.Get(it.magWell);
			if (!bucket)
			{
				bucket = {};
				magsByWell.Insert(it.magWell, bucket);
			}
			bucket.Insert(i);
		}

		// Index attachment items by declared type.
		array<int> attachItems = {};
		for (int i2 = 0, n2 = m_Items.Count(); i2 < n2; i2++)
		{
			TBD_RegistryScanItem it2 = m_Items[i2];
			if ((it2.kind == "optic" || it2.kind == "attachment") && !it2.itemAttachType.IsEmpty())
				attachItems.Insert(i2);
		}

		foreach (TBD_RegistryScanItem host : m_Items)
		{
			bool hostIsHandWeapon = host.kind == "gear_primary" || host.kind == "gear_handgun" || host.kind == "gear_launcher";
			bool hostIsVehicleWeapon = host.kind == "vehicle_weapon";
			if (!hostIsHandWeapon && !hostIsVehicleWeapon && host.kind != "vehicle" && host.kind != "character")
				continue;

			string magEdgeType = "mag_in_weapon";
			if (hostIsVehicleWeapon)
				magEdgeType = "mag_in_vehicle_weapon";

			if (hostIsHandWeapon || hostIsVehicleWeapon)
			{
				// Well-class matches.
				foreach (string well : host.muzzleWells)
				{
					array<int> mags = magsByWell.Get(well);
					if (!mags)
						continue;
					foreach (int magIdx : mags)
						EmitEdge(m_Items[magIdx].resourceName, host.resourceName, magEdgeType, well);
				}

				// Direct MagazineTemplate refs (also covers ammo-prefab templates on vehicle weapons).
				foreach (string tmpl : host.magTemplates)
				{
					int tIdx;
					if (!m_ItemIndexByRn.Find(tmpl, tIdx))
					{
						m_iDroppedEndpoints++;
						continue;
					}
					TBD_RegistryScanItem target = m_Items[tIdx];
					if (target.kind == "magazine")
						EmitEdge(target.resourceName, host.resourceName, magEdgeType, "MagazineTemplate");
					else if (target.kind == "ammo" && hostIsVehicleWeapon)
						EmitEdge(target.resourceName, host.resourceName, "ammo_in_vehicle_weapon", "MagazineTemplate");
				}

				// Attachment slots × attachment items.
				foreach (string slotType : host.slotAttachTypes)
				{
					foreach (int aIdx : attachItems)
					{
						TBD_RegistryScanItem att = m_Items[aIdx];
						if (!AttachTypeFits(att.itemAttachType, slotType))
							continue;
						string edgeType = "attachment_on_weapon";
						if (att.kind == "optic")
							edgeType = "optic_on_weapon";
						EmitEdge(att.resourceName, host.resourceName, edgeType, slotType);
					}
				}
			}

			if (host.kind == "character")
			{
				foreach (string gearRn : host.loadoutPrefabs)
				{
					int gIdx;
					if (!m_ItemIndexByRn.Find(gearRn, gIdx))
					{
						m_iDroppedEndpoints++;
						continue;
					}
					EmitEdge(gearRn, host.resourceName, "character_default_loadout", "LoadoutSlotInfo");
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void EmitEdge(string fromNode, string toNode, string edgeType, string evidence)
	{
		string key = edgeType + "|" + fromNode + "|" + toNode;
		if (m_EmittedKeys.Contains(key))
			return;
		m_EmittedKeys.Insert(key, true);

		TBD_RegistryEdge edge = new TBD_RegistryEdge();
		edge.fromNode = fromNode;
		edge.toNode = toNode;
		edge.edgeType = edgeType;
		edge.evidence = evidence;
		m_Edges.Insert(edge);

		int count;
		if (!m_EdgeHistogram.Find(edgeType, count))
			count = 0;
		m_EdgeHistogram.Set(edgeType, count + 1);
	}
}
