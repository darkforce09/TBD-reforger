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
	string kind;           // registry-items.schema.json v3 kind
	string addonId;

	// v3 (T-068.10.2) metadata. Negative float = absent (EnfScript has no nullable float);
	// absent values are omitted from the JSON — never guessed.
	bool isAbstract;       // *_base.et filename / "* Base" display — UI-hidden template
	string arsenalType;    // SCR_EArsenalItemType flag name when an EntityCatalog entry exists
	float weightKg = -1;       // ItemPhysicalAttributes.Weight (kg)
	float volumeCm3 = -1;      // ItemPhysicalAttributes.ItemVolume (cm3)
	float maxWeightKg = -1;    // container storage m_fMaxWeight (kg)
	float maxVolumeCm3 = -1;   // container storage MaxCumulativeVolume (cm3)
	string ruleId;         // census rule that classified this item (tier counters + verify log)

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
	ref array<string> defaultWeaponRefs = {}; // character WeaponSlotComponent.WeaponTemplate targets (canonical)
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
	protected static const ref array<string> DENY_HARD = {"/Structures/", "/Rocks/", "/Trees/", "/Debris/", "/Foliage/", "Prefabs/Editor/"};

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
	// v3 quality counters (printed every export — tierA + tierB + tierC + other == total).
	int m_iTierA;          // component/ancestor rules (engine-required signals)
	int m_iTierB;          // EntityCatalog-refined rows
	int m_iTierC;          // path-convention fallback rows
	int m_iOtherKind;      // fallthrough rows (kind == other)
	int m_iWeaponUnsplit;  // carryable weapons with no family split (cosmetic, counted)
	int m_iUnknownArea;    // cloth with an unmapped LoadoutAreaType (→ other, counted)
	int m_iCatalogEntries; // SCR_ArsenalItem entries parsed across all EntityCatalogs
	int m_iCatalogHits;    // entries whose prefab matched a scanned item (coverage numerator)
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

		// v3 abstract flag: filename *_base.et / *_Base.et OR "* Base"/"* base" display.
		string fileTail = filePath;
		int tailSlash = fileTail.LastIndexOf("/");
		if (tailSlash >= 0)
			fileTail = fileTail.Substring(tailSlash + 1, fileTail.Length() - tailSlash - 1);
		string tailLower = fileTail;
		tailLower.ToLower();
		item.isAbstract = tailLower.EndsWith("_base.et")
			|| item.displayName.EndsWith(" Base") || item.displayName.EndsWith(" base");

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
	//! Exact LoadoutAreaType class → v3 kind. Mod-defined subclasses resolve through typename
	//! ancestry to the nearest mapped base (AreaKindFor); unmapped areas → "" (counted, → other).
	protected static string AreaKindExact(string areaClass)
	{
		if (areaClass == "LoadoutJacketArea") return "gear_jacket";
		if (areaClass == "LoadoutPantsArea") return "gear_pants";
		if (areaClass == "LoadoutBootsArea") return "gear_boots";
		if (areaClass == "LoadoutVestArea") return "gear_vest";
		if (areaClass == "LoadoutArmoredVestSlotArea") return "gear_armored_vest";
		if (areaClass == "LoadoutHeadCoverArea") return "gear_helmet";
		if (areaClass == "LoadoutCoverArea") return "gear_helmet";
		if (areaClass == "LoadoutBackpackArea") return "gear_backpack";
		if (areaClass == "LoadoutGooglesArea") return "gear_glasses";
		if (areaClass == "LoadoutHandwearSlotArea") return "gear_gloves";
		if (areaClass == "LoadoutBinocularsArea") return "gear_binoculars";
		if (areaClass == "LoadoutIdentityItemArea") return "gear_item";
		if (areaClass == "LoadoutWatchArea") return "gear_item";
		if (areaClass == "LoadoutHandSlotArea") return "gear_item";
		return string.Empty;
	}

	//! The mapped LoadoutAreaType base classes (inheritance targets for mod subclasses).
	protected static const ref array<string> AREA_BASES = {
		"LoadoutJacketArea", "LoadoutPantsArea", "LoadoutBootsArea", "LoadoutVestArea",
		"LoadoutArmoredVestSlotArea", "LoadoutHeadCoverArea", "LoadoutCoverArea",
		"LoadoutBackpackArea", "LoadoutGooglesArea", "LoadoutHandwearSlotArea",
		"LoadoutBinocularsArea", "LoadoutIdentityItemArea", "LoadoutWatchArea",
		"LoadoutHandSlotArea"
	};

	//------------------------------------------------------------------------------------------------
	//! Area class → kind with inheritance fallback for mod subclasses (RHS_JacketArea extends
	//! LoadoutJacketArea classifies as gear_jacket). typename has no parent-walk API, so the
	//! unknown class is tested with IsInherited against every mapped base instead.
	protected string AreaKindFor(string areaClass)
	{
		string k = AreaKindExact(areaClass);
		if (!k.IsEmpty())
			return k;
		typename t = areaClass.ToType();
		if (!t)
			return string.Empty;
		foreach (string baseName : AREA_BASES)
		{
			typename bt = baseName.ToType();
			if (bt && t.IsInherited(bt))
				return AreaKindExact(baseName);
		}
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! True when any collected component class derives from baseClass (typename ancestry;
	//! suffix match as fallback when the typename is unresolvable in this VM).
	protected bool HasCompInheritedFrom(map<string, ref array<BaseContainer>> comps, string baseClass)
	{
		typename baseType = baseClass.ToType();
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls == baseClass || cls.EndsWith(baseClass))
				return true;
			if (!baseType)
				continue;
			typename t = cls.ToType();
			if (t && t.IsInherited(baseType))
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! ItemPhysicalAttributes (weight/volume, leaf override wins — buckets are most-derived-first)
	//! + container capacity vars off storage components. Absent stays -1 (omitted from JSON).
	protected void ReadPhysAttrs(map<string, ref array<BaseContainer>> comps, TBD_RegistryScanItem item)
	{
		// Single inner loop per outer entry — a second foreach over the same (write-protected)
		// map-value variable is rejected by the Enforce compiler.
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			bool isInvItem = cls.EndsWith("InventoryItemComponent");
			bool isStorage = cls.EndsWith("StorageComponent");
			if (!isInvItem && !isStorage)
				continue;
			foreach (BaseContainer comp : bucket)
			{
				if (isInvItem)
				{
					BaseContainer attrs = comp.GetObject("Attributes");
					if (attrs)
					{
						BaseContainer phys = attrs.GetObject("ItemPhysAttributes");
						if (phys)
						{
							float w;
							if (item.weightKg < 0 && phys.Get("Weight", w) && w >= 0)
								item.weightKg = w;
							float v;
							if (item.volumeCm3 < 0 && phys.Get("ItemVolume", v) && v >= 0)
								item.volumeCm3 = v;
						}
					}
				}
				if (isStorage)
				{
					float mw;
					if (item.maxWeightKg < 0 && comp.Get("m_fMaxWeight", mw) && mw >= 0)
						item.maxWeightKg = mw;
					float mv;
					if (item.maxVolumeCm3 < 0 && comp.Get("MaxCumulativeVolume", mv) && mv >= 0)
						item.maxVolumeCm3 = mv;
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Ancestor prefab file tails (self first), for the R7a vanilla weapon-family rule.
	protected void CollectAncestorTails(BaseContainer prefabRoot, notnull array<string> outTails)
	{
		BaseContainer cur = prefabRoot;
		int hops = 0;
		while (cur && hops < ANCESTOR_CAP)
		{
			string rn = cur.GetResourceName();
			int slash = rn.LastIndexOf("/");
			if (slash >= 0)
				rn = rn.Substring(slash + 1, rn.Length() - slash - 1);
			if (!rn.IsEmpty())
				outTails.Insert(rn);
			cur = cur.GetAncestor();
			hops++;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! R7a: vanilla weapon-family ancestry → kind. Weapon_Base deliberately unmapped (every
	//! weapon incl. cannons/mortars descends it — mapping it would bypass the R6 statics rule).
	protected static string WeaponAncestorKind(string tail)
	{
		string t = tail;
		t.ToLower();
		if (t == "rifle_base.et" || t == "machinegun_base.et" || t == "longrangerifle_base.et") return "gear_primary";
		if (t == "handgun_base.et") return "gear_handgun";
		if (t == "launcher_base.et") return "gear_launcher";
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Classify the prefab into a registry-items v3 kind and collect its compat facts.
	//! Rule order mirrors .ai/artifacts/t068_10_2_census.md §Rules (R0/R2..R9); the census
	//! H_pred is the acceptance contract (gate G1). Returns false when the prefab carries no
	//! item signal at all (world dressing).
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

		ReadPhysAttrs(comps, item);

		// R0 — vehicles / characters / magazines keep their T-150 rules.
		if (isVehicle)
		{
			item.kind = "vehicle";
			item.ruleId = "R0";
			CollectVehicleWeapons(root, comps, item.vehicleWeaponRefs, 0);
			return true;
		}

		if (isCharacter)
		{
			item.kind = "character";
			item.ruleId = "R0";
			CollectLoadoutPrefabs(comps, item.loadoutPrefabs);
			// v3: default weapons per slot. Grenade/throwable slots are CharacterGrenadeSlot-
			// Component — a DIFFERENT suffix (a default RGD5 is a default weapon too).
			CollectResourceVarValues(comps, "WeaponSlotComponent", "WeaponTemplate", item.defaultWeaponRefs);
			CollectResourceVarValues(comps, "GrenadeSlotComponent", "WeaponTemplate", item.defaultWeaponRefs);
			return true;
		}

		if (hasMagazine && !hasWeapon)
		{
			item.kind = "magazine";
			item.ruleId = "R0";
			array<string> wells = {};
			CollectObjectVarClasses(comps, "MagazineComponent", "MagazineWell", wells);
			if (!wells.IsEmpty())
				item.magWell = wells[0];
			return true;
		}

		// R2 — wear areas (exact class map + typename ancestry for mod subclasses).
		if (hasCloth)
		{
			array<string> areas = {};
			CollectObjectVarClasses(comps, "LoadoutClothComponent", "AreaType", areas);
			string area = "";
			if (!areas.IsEmpty())
				area = areas[0]; // most-derived-first: the leaf's area wins

			string areaKind = "";
			if (!area.IsEmpty())
				areaKind = AreaKindFor(area);

			if (!areaKind.IsEmpty())
			{
				item.kind = areaKind;
				item.ruleId = "R2";
				return true;
			}

			// Empty AreaType = decorative cloth node (73 such in vanilla) — NOT an unknown
			// area; let the remaining rules (gadgets etc.) classify it instead of forcing
			// other here. Only a NAMED-but-unmapped area is worth the counter + warning.
			if (!area.IsEmpty())
			{
				m_iUnknownArea++;
				item.kind = "other";
				item.ruleId = "R2_UNKNOWN_AREA";
				Print(string.Format("%1 unknown LoadoutAreaType '%2' on %3 -> other", m_sLogTag, area, filePath), LogLevel.WARNING);
				return true;
			}
		}

		// R3 — gadgets (SCR_GadgetComponent family covers compass/map/radio/flashlight/
		// consumable/detonator/... in one inheritance check).
		if (HasCompInheritedFrom(comps, "SCR_BinocularsComponent"))
		{
			item.kind = "gear_binoculars";
			item.ruleId = "R3";
			return true;
		}
		if (HasCompInheritedFrom(comps, "SCR_GadgetComponent"))
		{
			item.kind = "gear_item";
			item.ruleId = "R3";
			return true;
		}

		// R4 — throwables (grenades, smokes; they carry WeaponComponent, so before weapons).
		if (HasCompInheritedFrom(comps, "GrenadeMoveComponent"))
		{
			item.kind = "gear_throwable";
			item.ruleId = "R4";
			CollectObjectVarClasses(comps, "AttachmentSlotComponent", "AttachmentType", item.slotAttachTypes);
			return true;
		}

		// R5 — explosive charges (DemoBlocks carry WeaponComponent, so before weapons).
		if (HasCompInheritedFrom(comps, "SCR_ExplosiveChargeComponent")
			|| HasCompInheritedFrom(comps, "SCR_ExplosiveTriggerComponent")
			|| HasCompInheritedFrom(comps, "SCR_ExplosiveChargeInventoryItemComponent"))
		{
			item.kind = "gear_explosive";
			item.ruleId = "R5";
			return true;
		}

		if (hasWeapon)
		{
			CollectObjectVarClasses(comps, "MuzzleComponent", "MagazineWell", item.muzzleWells);
			CollectResourceVarValues(comps, "MuzzleComponent", "MagazineTemplate", item.magTemplates);
			CollectObjectVarClasses(comps, "AttachmentSlotComponent", "AttachmentType", item.slotAttachTypes);

			// R7a — vanilla weapon-family ancestry (before R6 so abstract Core templates,
			// which have no phys attrs by design, keep their family kind).
			array<string> tails = {};
			CollectAncestorTails(root, tails);
			foreach (string tail : tails)
			{
				string ancKind = WeaponAncestorKind(tail);
				if (!ancKind.IsEmpty())
				{
					item.kind = ancKind;
					item.ruleId = "R7a";
					return true;
				}
			}

			// R6 — statics: crewed emplacements, rocket pods, and weapons that cannot be
			// carried (no phys attrs and no inventory item component anywhere in the chain).
			bool carryable = hasInventoryItem || item.weightKg >= 0 || item.volumeCm3 >= 0;
			if (HasCompSuffix(comps, "CompartmentManagerComponent")
				|| HasCompInheritedFrom(comps, "SCR_RocketEjectorMuzzleComponent"))
			{
				item.kind = "vehicle_weapon";
				item.ruleId = "R6";
				return true;
			}
			if (!carryable)
			{
				item.kind = "vehicle_weapon";
				item.ruleId = "R6";
				return true;
			}

			// R7b/c/d — carryable weapons without vanilla ancestry.
			if (HasCompInheritedFrom(comps, "MuzzleInMagComponent"))
			{
				item.kind = "gear_launcher";
				item.ruleId = "R7b";
				return true;
			}
			if (filePath.Contains("/Handguns/"))
			{
				item.kind = "gear_handgun";
				item.ruleId = "R7c";
				return true;
			}
			if (filePath.Contains("/Launchers/"))
			{
				item.kind = "gear_launcher";
				item.ruleId = "R7c";
				return true;
			}
			m_iWeaponUnsplit++;
			item.kind = "gear_primary";
			item.ruleId = "R7d";
			return true;
		}

		// Attachment items declare their type in InventoryItemComponent.Attributes.CustomAttributes.
		string attachType = ItemAttachmentType(comps);
		if (!attachType.IsEmpty())
		{
			item.itemAttachType = attachType;
			item.ruleId = "R0";
			if (attachType.Contains("Optics") || HasCompSuffix(comps, "SightsComponent"))
				item.kind = "optic";
			else
				item.kind = "attachment";
			return true;
		}

		if (filePath.Contains("/Ammo/"))
		{
			item.kind = "ammo";
			item.ruleId = "R0";
			return true;
		}

		if (hasStorage && !hasInventoryItem && hasTurretWeaponSlot)
		{
			// Gun mounts / VehParts hosting a weapon: classify as vehicle_weapon host part.
			item.kind = "vehicle_weapon";
			item.ruleId = "R0";
			CollectResourceVarValues(comps, "WeaponSlotComponent", "WeaponTemplate", item.vehicleWeaponRefs);
			return true;
		}

		if (hasStorage && !hasInventoryItem)
		{
			item.kind = "crate";
			item.ruleId = "R0";
			return true;
		}

		// R9 — inventory item with no stronger signal: quarantined, counted, never gear_primary.
		if (hasInventoryItem)
		{
			item.kind = "other";
			item.ruleId = "R9";
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
	// ---- R1 / Tier B: faction EntityCatalog SCR_ArsenalItem pass ------------------------------
	// BI (and well-behaved mods) hand-classify every arsenal-facing item in
	// Configs/EntityCatalog/*: entries pair a prefab with an SCR_EArsenalItemType flag.
	// The pass records arsenal_type metadata for every matched item and refines the kind ONLY
	// where component rules could not split (weapon_unsplit / other) plus the census
	// pre-authorized flare delta — it never contradicts a component-derived kind, so the
	// census H_pred stays the offline-computable contract.

	protected ref map<string, string> m_CatalogTypeByRn = new map<string, string>();
	protected ref array<string> m_CatalogConfPaths = {};

	//! SCR_EArsenalItemType bit → flag name (verbatim from SCR_EArsenalItemType.c, bits 1..22).
	protected static string ArsenalFlagName(int value)
	{
		if ((value & (1 << 1)) != 0) return "RIFLE";
		if ((value & (1 << 2)) != 0) return "PISTOL";
		if ((value & (1 << 3)) != 0) return "LETHAL_THROWABLE";
		if ((value & (1 << 4)) != 0) return "ROCKET_LAUNCHER";
		if ((value & (1 << 5)) != 0) return "MACHINE_GUN";
		if ((value & (1 << 6)) != 0) return "HEAL";
		if ((value & (1 << 7)) != 0) return "BACKPACK";
		if ((value & (1 << 8)) != 0) return "SNIPER_RIFLE";
		if ((value & (1 << 9)) != 0) return "NON_LETHAL_THROWABLE";
		if ((value & (1 << 10)) != 0) return "HEADWEAR";
		if ((value & (1 << 11)) != 0) return "TORSO";
		if ((value & (1 << 12)) != 0) return "VEST_AND_WAIST";
		if ((value & (1 << 13)) != 0) return "LEGS";
		if ((value & (1 << 14)) != 0) return "FOOTWEAR";
		if ((value & (1 << 15)) != 0) return "RADIO_BACKPACK";
		if ((value & (1 << 16)) != 0) return "EQUIPMENT";
		if ((value & (1 << 17)) != 0) return "WEAPON_ATTACHMENT";
		if ((value & (1 << 18)) != 0) return "EXPLOSIVES";
		if ((value & (1 << 19)) != 0) return "HANDWEAR";
		if ((value & (1 << 20)) != 0) return "MORTARS";
		if ((value & (1 << 21)) != 0) return "HELICOPTER";
		if ((value & (1 << 22)) != 0) return "VEHICLE";
		return string.Empty;
	}

	//! Arsenal flag → v3 kind (for refinement of unsplit/other rows only).
	protected static string CatalogKindFor(string flagName)
	{
		if (flagName == "RIFLE" || flagName == "MACHINE_GUN" || flagName == "SNIPER_RIFLE") return "gear_primary";
		if (flagName == "PISTOL") return "gear_handgun";
		if (flagName == "ROCKET_LAUNCHER") return "gear_launcher";
		if (flagName == "LETHAL_THROWABLE" || flagName == "NON_LETHAL_THROWABLE") return "gear_throwable";
		if (flagName == "EXPLOSIVES") return "gear_explosive";
		if (flagName == "HEAL" || flagName == "EQUIPMENT") return "gear_item";
		if (flagName == "HEADWEAR") return "gear_helmet";
		if (flagName == "TORSO") return "gear_jacket";
		if (flagName == "LEGS") return "gear_pants";
		if (flagName == "FOOTWEAR") return "gear_boots";
		if (flagName == "VEST_AND_WAIST") return "gear_vest";
		if (flagName == "BACKPACK" || flagName == "RADIO_BACKPACK") return "gear_backpack";
		if (flagName == "HANDWEAR") return "gear_gloves";
		if (flagName == "WEAPON_ATTACHMENT") return "attachment";
		if (flagName == "MORTARS") return "vehicle_weapon";
		if (flagName == "VEHICLE" || flagName == "HELICOPTER") return "vehicle";
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	void OnCatalogConfFound(ResourceName resName, string filePath = "")
	{
		if (m_CatalogConfPaths.Find(resName) == -1)
			m_CatalogConfPaths.Insert(resName);
	}

	//------------------------------------------------------------------------------------------------
	//! Enumerate Configs/EntityCatalog across all loaded addons and build rn → arsenal flag map.
	//! Var names probed with logged fallbacks (SCR naming conventions) — the first successful
	//! probe set is printed for the verify log.
	void ScanEntityCatalogs()
	{
		m_CatalogConfPaths.Clear();
		foreach (TBD_RegistryAddonInfo addon : m_Addons)
		{
			string root = "$" + addon.id + ":Configs/EntityCatalog";
			Workbench.SearchResources(OnCatalogConfFound, {"conf"}, null, root, true);
		}
		Print(string.Format("%1 EntityCatalog scan: %2 conf files", m_sLogTag, m_CatalogConfPaths.Count()));

		// Structure dump of the first catalog (one-time, feeds the verify log): the parser
		// below self-discovers names, and this print shows what it had to work with.
		if (!m_CatalogConfPaths.IsEmpty())
		{
			Resource dbgRes = Resource.Load(m_CatalogConfPaths[0]);
			if (dbgRes && dbgRes.IsValid())
			{
				BaseResourceObject dbgObj = dbgRes.GetResource();
				if (dbgObj)
				{
					BaseContainer dbgCat = dbgObj.ToBaseContainer();
					if (dbgCat)
					{
						int dnv = dbgCat.GetNumVars();
						string names = "";
						for (int dv = 0; dv < dnv; dv++)
							names = names + dbgCat.GetVarName(dv) + ",";
						Print(string.Format("%1 catalog[0] %2 class=%3 vars(%4): %5",
							m_sLogTag, m_CatalogConfPaths[0], dbgCat.GetClassName(), dnv, names));
					}
				}
			}
		}

		bool loggedShape = false;
		foreach (string confRn : m_CatalogConfPaths)
		{
			Resource res = Resource.Load(confRn);
			if (!res || !res.IsValid())
				continue;
			BaseResourceObject obj = res.GetResource();
			if (!obj)
				continue;
			BaseContainer cat = obj.ToBaseContainer();
			if (!cat)
				continue;

			// Var names are self-discovered via BaseContainer var enumeration — no guessed
			// names anywhere: the entry list is the object-array whose elements are
			// *CatalogEntry*-classed; the prefab is the entry's string var ending '.et';
			// the arsenal data is the *ArsenalItem*-classed element; the type is the int
			// var whose value maps to a SCR_EArsenalItemType flag.
			BaseContainerList entries = FindObjectArrayByElemClass(cat, "CatalogEntry");
			if (!entries)
				continue;

			for (int i = 0, n = entries.Count(); i < n; i++)
			{
				BaseContainer entry = entries.Get(i);
				if (!entry)
					continue;

				string prefab = FindResourceVar(entry);
				if (prefab.IsEmpty())
					continue;

				BaseContainerList dataList = FindObjectArrayByElemClass(entry, "ArsenalItem");
				if (!dataList)
					continue;

				for (int d = 0, dn = dataList.Count(); d < dn; d++)
				{
					BaseContainer data = dataList.Get(d);
					if (!data || !data.GetClassName().Contains("ArsenalItem"))
						continue;

					string flag = FindArsenalFlag(data);
					if (flag.IsEmpty())
						continue;

					m_iCatalogEntries++;
					string canonical = ResolveCanonical(prefab);
					if (canonical.IsEmpty())
						continue;
					if (!m_CatalogTypeByRn.Contains(canonical))
						m_CatalogTypeByRn.Insert(canonical, flag);

					if (!loggedShape)
					{
						loggedShape = true;
						Print(string.Format("%1 catalog shape discovered: %2 data=%3 flag=%4 prefab=%5",
							m_sLogTag, confRn, data.GetClassName(), flag, prefab));
					}
					break;
				}
			}
		}
		Print(string.Format("%1 EntityCatalog parsed: %2 arsenal entries, %3 distinct prefabs",
			m_sLogTag, m_iCatalogEntries, m_CatalogTypeByRn.Count()));
	}

	//------------------------------------------------------------------------------------------------
	//! First object-array var on holder whose first element's class name contains elemClassFrag.
	protected BaseContainerList FindObjectArrayByElemClass(BaseContainer holder, string elemClassFrag)
	{
		int nv = holder.GetNumVars();
		for (int v = 0; v < nv; v++)
		{
			string varName = holder.GetVarName(v);
			if (varName.IsEmpty())
				continue;
			BaseContainerList list = holder.GetObjectArray(varName);
			if (!list || list.Count() == 0)
				continue;
			BaseContainer first = list.Get(0);
			if (first && first.GetClassName().Contains(elemClassFrag))
				return list;
		}
		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! First string var on holder whose value looks like a prefab ResourceName (ends '.et').
	protected string FindResourceVar(BaseContainer holder)
	{
		int nv = holder.GetNumVars();
		for (int v = 0; v < nv; v++)
		{
			string varName = holder.GetVarName(v);
			if (varName.IsEmpty())
				continue;
			string s;
			if (holder.Get(varName, s) && !s.IsEmpty() && s.EndsWith(".et"))
				return s;
		}
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! First int var on holder whose value maps to a SCR_EArsenalItemType flag name.
	protected string FindArsenalFlag(BaseContainer holder)
	{
		int nv = holder.GetNumVars();
		for (int v = 0; v < nv; v++)
		{
			string varName = holder.GetVarName(v);
			if (varName.IsEmpty())
				continue;
			int tv;
			if (!holder.Get(varName, tv) || tv == 0)
				continue;
			string flag = ArsenalFlagName(tv);
			if (!flag.IsEmpty())
				return flag;
		}
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Apply Tier-B metadata + bounded refinements (see header note). Call after the prefab
	//! scan and BEFORE DeriveEdges (edge derivation reads kinds).
	void ApplyCatalogRefinements()
	{
		foreach (TBD_RegistryScanItem it : m_Items)
		{
			string flag;
			if (!m_CatalogTypeByRn.Find(it.resourceName, flag))
				continue;

			m_iCatalogHits++;
			it.arsenalType = flag;

			string catKind = CatalogKindFor(flag);
			if (catKind.IsEmpty())
				continue;

			// Refinement 1: quarantined rows get a real kind from the catalog.
			if (it.kind == "other")
			{
				it.kind = catKind;
				it.ruleId = "R1";
				continue;
			}
			// Refinement 2: unsplit carryable weapons get their family from the catalog.
			if (it.ruleId == "R7d" && (catKind == "gear_primary" || catKind == "gear_handgun" || catKind == "gear_launcher"))
			{
				it.kind = catKind;
				it.ruleId = "R1";
				continue;
			}
			// Refinement 3 (census pre-authorized flare delta): launcher-classified throwables.
			if (it.kind == "gear_launcher" && catKind == "gear_throwable")
			{
				it.kind = "gear_throwable";
				it.ruleId = "R1";
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Tier counters for the export log: tierA + tierB + tierC + other == total.
	void ComputeTierCounters()
	{
		m_iTierA = 0;
		m_iTierB = 0;
		m_iTierC = 0;
		m_iOtherKind = 0;
		foreach (TBD_RegistryScanItem it : m_Items)
		{
			if (it.kind == "other")
				m_iOtherKind++;
			else if (it.ruleId == "R1")
				m_iTierB++;
			else if (it.ruleId == "R7c")
				m_iTierC++;
			else
				m_iTierA++;
		}
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
				// v3: weapon defaults were never exported (cloth-only LoadoutSlotInfo) — the
				// Primary picker degrade documented in the hub. WeaponTemplate per weapon slot.
				foreach (string weaponRn : host.defaultWeaponRefs)
				{
					int wIdx;
					if (!m_ItemIndexByRn.Find(weaponRn, wIdx))
					{
						m_iDroppedEndpoints++;
						continue;
					}
					EmitEdge(weaponRn, host.resourceName, "character_default_weapon", "CharacterWeaponSlotComponent.WeaponTemplate");
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
