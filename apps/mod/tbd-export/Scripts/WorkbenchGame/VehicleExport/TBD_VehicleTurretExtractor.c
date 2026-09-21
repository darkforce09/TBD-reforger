/**
 * TBD_VehicleTurretExtractor.c
 *
 * Dedicated extractor for vehicle turrets, cupolas, weapon stations, mounted armaments,
 * ballistic muzzles, ammunition capacities, fire modes, and optical sights.
 * Operates purely via Enfusion BaseContainer reflection without hardcoded weapon tables.
 */

class TBD_VehicleTurretExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Introspect all mounted turrets and weapons on the vehicle variant.
	static void Extract(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		ref array<string> inspectedTurretPrefabs = {};

		// 1. Discover turrets attached via SlotManagerComponent
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("SlotManagerComponent")) continue;
			for (int b = 0; b < bucket.Count(); b++)
			{
				BaseContainer mgr = bucket[b];
				if (!mgr) continue;
				BaseContainerList slots = mgr.GetObjectArray("Slots");
				if (!slots) continue;
				for (int s = 0, sn = slots.Count(); s < sn; s++)
				{
					BaseContainer slot = slots.Get(s);
					if (!slot) continue;
					string slotPrefab;
					if (!slot.Get("Prefab", slotPrefab) || slotPrefab.IsEmpty())
						continue;
					string slotName;
					slot.Get("m_sSlotName", slotName);
					if (slotName.IsEmpty()) slotName = string.Format("Slot_%1", s);
					InspectChildSlot(slotPrefab, slotName, varData, inspectedTurretPrefabs);
				}
			}
			break;
		}

		// 2. Discover direct vehicle-level weapon slots
		foreach (string wcls, array<BaseContainer> wbucket : comps)
		{
			if (!wcls.Contains("WeaponSlotComponent")) continue;
			for (int wb = 0; wb < wbucket.Count(); wb++)
			{
				BaseContainer wsc = wbucket[wb];
				if (!wsc) continue;
				string wepPrefab;
				if (!wsc.Get("WeaponTemplate", wepPrefab) || wepPrefab.IsEmpty())
					continue;
				string wSlotName;
				wsc.Get("m_sSlotName", wSlotName);
				if (wSlotName.IsEmpty()) wSlotName = "VehicleWeaponSlot";
				TBD_VehicleMountedWeaponData wd = InspectWeaponPrefab(wepPrefab, wSlotName);
				if (wd) AddDirectWeaponToTurret(varData, wd, wSlotName);
			}
			break;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Inspect a child prefab attached to a slot: determine if it is a turret assembly.
	protected static void InspectChildSlot(string prefabPath, string slotName, TBD_VehicleDeepVariant varData, array<string> seen)
	{
		if (seen.Find(prefabPath) != -1)
			return;

		Resource res = Resource.Load(prefabPath);
		if (!res || !res.IsValid()) return;
		BaseResourceObject resObj = res.GetResource();
		if (!resObj) return;
		BaseContainer root = resObj.ToBaseContainer();
		if (!root) return;

		map<string, ref array<BaseContainer>> childComps = new map<string, ref array<BaseContainer>>();
		CollectComponents(root, childComps);

		bool hasTurretComp = false;
		foreach (string cls, array<BaseContainer> b : childComps)
		{
			if (cls.Contains("TurretComponent"))
			{
				hasTurretComp = true;
				break;
			}
		}

		// If it has a turret component or contains weapon slots, register as a turret assembly
		if (hasTurretComp || HasWeaponSlots(childComps))
		{
			seen.Insert(prefabPath);
			TBD_VehicleTurretData td = new TBD_VehicleTurretData();
			td.m_sSlotName = slotName;
			td.m_sTurretPrefab = prefabPath;
			td.m_sDisplayName = ResolveTurretDisplayName(prefabPath, childComps);

			// Traverse & Elevation limits
			ExtractTurretLimits(childComps, td);

			// Mounted weapons
			ExtractTurretWeapons(childComps, td);

			// Optics & Sights
			ExtractTurretOptics(childComps, td);

			varData.m_aTurrets.Insert(td);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract yaw and pitch limits and slew speeds from TurretComponent.
	protected static void ExtractTurretLimits(map<string, ref array<BaseContainer>> comps, TBD_VehicleTurretData td)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("TurretComponent")) continue;

			foreach (BaseContainer tc : bucket)
			{
				tc.Get("m_fYawMin", td.m_fYawMin);
				tc.Get("m_fYawMax", td.m_fYawMax);
				tc.Get("m_fPitchMin", td.m_fPitchMin);
				tc.Get("m_fPitchMax", td.m_fPitchMax);
				tc.Get("m_fYawSpeed", td.m_fYawSpeed);
				tc.Get("m_fPitchSpeed", td.m_fPitchSpeed);
			}
			break;
		}

		// Defaults for full traverse if unconstrained
		if (td.m_fYawMin == 0 && td.m_fYawMax == 0)
		{
			td.m_fYawMin = -180.0;
			td.m_fYawMax = 180.0;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract all mounted weapons inside a turret assembly.
	protected static void ExtractTurretWeapons(map<string, ref array<BaseContainer>> comps, TBD_VehicleTurretData td)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("WeaponSlotComponent")) continue;

			foreach (BaseContainer wsc : bucket)
			{
				string wepPrefab;
				if (!wsc.Get("WeaponTemplate", wepPrefab) || wepPrefab.IsEmpty())
					continue;

				string wSlotName;
				wsc.Get("m_sSlotName", wSlotName);

				TBD_VehicleMountedWeaponData wd = InspectWeaponPrefab(wepPrefab, wSlotName);
				if (wd)
					td.m_aWeapons.Insert(wd);
			}
			break;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect a mounted weapon prefab: display name, classification, magazine, fire modes, zeroing.
	protected static TBD_VehicleMountedWeaponData InspectWeaponPrefab(string wepPrefab, string slotName)
	{
		Resource res = Resource.Load(wepPrefab);
		if (!res || !res.IsValid()) return null;
		BaseResourceObject resObj = res.GetResource();
		if (!resObj) return null;
		BaseContainer root = resObj.ToBaseContainer();
		if (!root) return null;

		map<string, ref array<BaseContainer>> wepComps = new map<string, ref array<BaseContainer>>();
		CollectComponents(root, wepComps);

		TBD_VehicleMountedWeaponData wd = new TBD_VehicleMountedWeaponData();
		wd.m_sSlotName = slotName;
		wd.m_sWeaponPrefab = wepPrefab;

		// 1. Display name
		ExtractWeaponIdentity(wepComps, wepPrefab, wd);

		// 2. Weapon type classification
		wd.m_sWeaponType = ClassifyWeaponType(wepPrefab, wd.m_sDisplayName);

		// 3. Muzzles, magazines, fire modes, and zeroing
		ExtractWeaponMuzzles(wepComps, wd);
		ExtractWeaponSights(wepComps, wd);

		return wd;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract localized or tokenized display name directly from weapon components.
	protected static void ExtractWeaponIdentity(map<string, ref array<BaseContainer>> comps, string prefabPath, TBD_VehicleMountedWeaponData wd)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("UIInfo") && !cls.Contains("InventoryItemComponent") && !cls.Contains("WeaponComponent"))
				continue;

			foreach (BaseContainer c : bucket)
			{
				BaseContainer ui = c.GetObject("m_UIInfo");
				if (!ui)
				{
					BaseContainer attrs = c.GetObject("Attributes");
					if (attrs) ui = attrs.GetObject("ItemDisplayName");
				}

				if (ui)
				{
					string name;
					if (ui.Get("Name", name) && !name.IsEmpty())
					{
						if (name.StartsWith("#"))
							wd.m_sDisplayName = TBD_VehicleExportNaming.CleanNameFromToken(name.Substring(1, name.Length() - 1));
						else
							wd.m_sDisplayName = name;
						break;
					}
				}
			}
			if (!wd.m_sDisplayName.IsEmpty()) break;
		}

		if (wd.m_sDisplayName.IsEmpty())
		{
			int lastSlash = prefabPath.LastIndexOf("/");
			string stem = prefabPath;
			if (lastSlash != -1) stem = prefabPath.Substring(lastSlash + 1, prefabPath.Length() - lastSlash - 1);
			stem.Replace(".et", "");
			stem.Replace("Weapon_", "");
			stem.Replace("HMG_", "");
			stem.Replace("MG_", "");
			wd.m_sDisplayName = TBD_VehicleExportNaming.HumanizeStem(stem);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Classify weapon role category.
	protected static string ClassifyWeaponType(string prefabPath, string name)
	{
		string lower = (prefabPath + " " + name);
		lower.ToLower();

		if (lower.Contains("periscope") || lower.Contains("dummy")) return "Observation Sight";
		if (lower.Contains("cannon") || lower.Contains("25mm") || lower.Contains("30mm") || lower.Contains("73mm")) return "Autocannon";
		if (lower.Contains("hmg") || lower.Contains("145") || lower.Contains("127") || lower.Contains("50cal") || lower.Contains("m2hb") || lower.Contains("kpvt")) return "Heavy Machine Gun";
		if (lower.Contains("pkt") || lower.Contains("coax")) return "Coaxial Machine Gun";
		if (lower.Contains("rocket") || lower.Contains("ub-32") || lower.Contains("hydra")) return "Rocket Launcher";
		if (lower.Contains("smoke")) return "Smoke Countermeasure";
		return "Machine Gun";
	}

	//------------------------------------------------------------------------------------------------
	//! Extract default magazine, capacity, and fire modes from weapon muzzles.
	protected static void ExtractWeaponMuzzles(map<string, ref array<BaseContainer>> comps, TBD_VehicleMountedWeaponData wd)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("MuzzleComponent")) continue;

			foreach (BaseContainer mc : bucket)
			{
				// Default magazine template
				string magTmpl;
				if (mc.Get("MagazineTemplate", magTmpl) && !magTmpl.IsEmpty())
				{
					wd.m_sDefaultMagazinePrefab = magTmpl;
					InspectMagazinePrefab(magTmpl, wd);
				}

				// Fire modes
				BaseContainerList modes = mc.GetObjectArray("FireModes");
				if (!modes) modes = mc.GetObjectArray("m_aFireModes");

				if (modes)
				{
					for (int m = 0, mn = modes.Count(); m < mn; m++)
					{
						BaseContainer mode = modes.Get(m);
						if (!mode) continue;

						string modeName;
						mode.Get("UIName", modeName);
						if (modeName.IsEmpty()) modeName = "Full Auto";

						int rpm = 0;
						mode.Get("RoundsPerMinute", rpm);
						if (rpm <= 0) mode.Get("m_iRoundsPerMinute", rpm);

						int burst = 0;
						mode.Get("MaxBurst", burst);

						wd.m_aFireModes.Insert(modeName);
						wd.m_aFireModeRpms.Insert(rpm);
						wd.m_aFireModeBursts.Insert(burst);
					}
				}
			}
			break;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect default magazine prefab to resolve display name and round capacity.
	protected static void InspectMagazinePrefab(string magPrefab, TBD_VehicleMountedWeaponData wd)
	{
		Resource res = Resource.Load(magPrefab);
		if (!res || !res.IsValid()) return;
		BaseResourceObject resObj = res.GetResource();
		if (!resObj) return;
		BaseContainer root = resObj.ToBaseContainer();
		if (!root) return;

		map<string, ref array<BaseContainer>> magComps = new map<string, ref array<BaseContainer>>();
		CollectComponents(root, magComps);

		foreach (string cls, array<BaseContainer> bucket : magComps)
		{
			if (!cls.Contains("MagazineComponent")) continue;
			foreach (BaseContainer mc : bucket)
			{
				int cap = 0;
				if (mc.Get("MaxAmmo", cap) && cap > 0) wd.m_iMagazineCapacity = cap;
				else if (mc.Get("m_iCapacity", cap) && cap > 0) wd.m_iMagazineCapacity = cap;
			}
			break;
		}

		// Resolve magazine display name
		foreach (string ucls, array<BaseContainer> ubucket : magComps)
		{
			if (!ucls.Contains("UIInfo") && !ucls.Contains("InventoryItemComponent")) continue;
			foreach (BaseContainer uc : ubucket)
			{
				BaseContainer ui = uc.GetObject("m_UIInfo");
				if (!ui)
				{
					BaseContainer attrs = uc.GetObject("Attributes");
					if (attrs) ui = attrs.GetObject("ItemDisplayName");
				}
				if (ui)
				{
					string nStr;
					if (ui.Get("Name", nStr) && !nStr.IsEmpty())
					{
						if (nStr.StartsWith("#"))
							wd.m_sDefaultMagazineDisplayName = TBD_VehicleExportNaming.CleanNameFromToken(nStr.Substring(1, nStr.Length() - 1));
						else
							wd.m_sDefaultMagazineDisplayName = nStr;
						break;
					}
				}
			}
			if (!wd.m_sDefaultMagazineDisplayName.IsEmpty()) break;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract sights and zeroing ranges from weapon sights components.
	protected static void ExtractWeaponSights(map<string, ref array<BaseContainer>> comps, TBD_VehicleMountedWeaponData wd)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("SightsComponent")) continue;
			foreach (BaseContainer sc : bucket)
			{
				BaseContainerList ranges = sc.GetObjectArray("SightsRanges");
				if (ranges)
				{
					for (int r = 0, rn = ranges.Count(); r < rn; r++)
					{
						BaseContainer rng = ranges.Get(r);
						if (!rng) continue;
						float dist = 0;
						if (rng.Get("Distance", dist) && dist > 0)
							wd.m_aZeroingDistances.Insert(string.Format("%1m", dist));
					}
				}
			}
			break;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract day/night observation sights, periscopes, and magnification from turret components.
	protected static void ExtractTurretOptics(map<string, ref array<BaseContainer>> comps, TBD_VehicleTurretData td)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.Contains("OpticComponent") || cls.Contains("SightsComponent") || cls.Contains("PeriscopeComponent"))
			{
				if (td.m_aOptics.Find(cls) == -1)
					td.m_aOptics.Insert(cls);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Helper to check if any child component is a weapon slot.
	protected static bool HasWeaponSlots(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.Contains("WeaponSlotComponent") && !bucket.IsEmpty())
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Derive turret display name from prefab stem or UI info.
	protected static string ResolveTurretDisplayName(string prefabPath, map<string, ref array<BaseContainer>> comps)
	{
		if (prefabPath.Contains("commander")) return "Commander Cupola";
		if (prefabPath.Contains("pintle") || prefabPath.Contains("Pintle")) return "Pintle Weapon Mount";

		int lastSlash = prefabPath.LastIndexOf("/");
		string stem = prefabPath;
		if (lastSlash != -1) stem = prefabPath.Substring(lastSlash + 1, prefabPath.Length() - lastSlash - 1);
		stem.Replace(".et", "");
		stem.Replace("VehParts_", "");
		return TBD_VehicleExportNaming.HumanizeStem(stem);
	}

	//------------------------------------------------------------------------------------------------
	//! Add direct vehicle weapon to synthetic turret station.
	protected static void AddDirectWeaponToTurret(TBD_VehicleDeepVariant varData, TBD_VehicleMountedWeaponData wd, string slotName)
	{
		TBD_VehicleTurretData td = new TBD_VehicleTurretData();
		td.m_sSlotName = slotName;
		td.m_sDisplayName = wd.m_sDisplayName + " Station";
		td.m_aWeapons.Insert(wd);
		varData.m_aTurrets.Insert(td);
	}

	//------------------------------------------------------------------------------------------------
	//! Recursive component collector across container hierarchy.
	protected static void CollectComponents(BaseContainer root, notnull map<string, ref array<BaseContainer>> outComps)
	{
		BaseContainer cur = root;
		int hops = 0;
		while (cur && hops < 8)
		{
			BaseContainerList comps = cur.GetObjectArray("components");
			if (comps)
			{
				for (int i = 0, n = comps.Count(); i < n; i++)
				{
					BaseContainer comp = comps.Get(i);
					if (!comp) continue;
					string cls = comp.GetClassName();
					array<BaseContainer> bucket = outComps.Get(cls);
					if (!bucket)
					{
						bucket = {};
						outComps.Insert(cls, bucket);
					}
					bucket.Insert(comp);
				}
			}
			cur = cur.GetAncestor();
			hops++;
		}
	}
}
