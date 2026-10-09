/**
 * TBD_VehicleCatalogClassifier.c
 *
 * Generic introspection helper for classifying vehicle platforms, tactical roles,
 * seating capacity, mounted armaments, and physical specifications without hardcoded
 * vehicle names, weapon tables, or mock data.
 */

class TBD_VehicleCatalogClassifier
{
	//------------------------------------------------------------------------------------------------
	//! Extract display name, description, and faction directly from component metadata.
	static void ExtractIdentity(map<string, ref array<BaseContainer>> comps, TBD_VehicleVariantInfo varInfo)
	{
		// 1. Display name and description
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("EditableEntityComponent") && !cls.Contains("UIComponent") && !cls.Contains("InventoryItemComponent"))
				continue;

			foreach (BaseContainer c : bucket)
			{
				BaseContainer ui = c.GetObject("m_UIInfo");
				if (!ui)
				{
					BaseContainer attrs = c.GetObject("Attributes");
					if (attrs) ui = attrs.GetObject("ItemDisplayName");
				}
				if (!ui) continue;

				string name;
				if (ui.Get("Name", name) && !name.IsEmpty())
				{
					if (name.StartsWith("#"))
					{
						string token = name.Substring(1, name.Length() - 1);
						if (varInfo.m_sDisplayName.IsEmpty())
							varInfo.m_sDisplayName = TBD_VehicleExportNaming.CleanNameFromToken(token);
					}
					else if (varInfo.m_sDisplayName.IsEmpty())
						varInfo.m_sDisplayName = name;
				}

				string desc;
				if (ui.Get("Description", desc) && !desc.IsEmpty())
				{
					if (desc.StartsWith("#"))
						varInfo.m_sDescription = desc.Substring(1, desc.Length() - 1);
					else
						varInfo.m_sDescription = desc;
				}
			}
		}

		if (varInfo.m_sDisplayName.IsEmpty())
			varInfo.m_sDisplayName = TBD_VehicleExportNaming.HumanizeStem(varInfo.m_sId);

		// 2. Faction extraction from FactionAffiliationComponent / SCR_VehicleFactionAffiliationComponent
		foreach (string fcls, array<BaseContainer> fbucket : comps)
		{
			if (!fcls.Contains("FactionAffiliationComponent")) continue;

			foreach (BaseContainer fc : fbucket)
			{
				string faction;
				if (fc.Get("faction affiliation", faction) && !faction.IsEmpty())
					varInfo.m_sFaction = faction;
				else if (fc.Get("m_sAffiliatedFaction", faction) && !faction.IsEmpty())
					varInfo.m_sFaction = faction;
				else if (fc.Get("defaultAffiliation", faction) && !faction.IsEmpty())
					varInfo.m_sFaction = faction;
				else if (fc.Get("factionKey", faction) && !faction.IsEmpty())
					varInfo.m_sFaction = faction;
				else
				{
					for (int v = 0, vn = fc.GetNumVars(); v < vn; v++)
					{
						string vname = fc.GetVarName(v);
						string vLower = vname;
						vLower.ToLower();
						if (vLower.Contains("faction") || vLower.Contains("affiliation"))
						{
							if (fc.Get(vname, faction) && !faction.IsEmpty())
							{
								varInfo.m_sFaction = faction;
								break;
							}
						}
					}
				}
				if (!varInfo.m_sFaction.IsEmpty()) break;
			}
			if (!varInfo.m_sFaction.IsEmpty()) break;
		}

		// Also check editable entity faction attribute if not yet resolved
		if (varInfo.m_sFaction.IsEmpty())
		{
			foreach (string ecls, array<BaseContainer> ebucket : comps)
			{
				if (!ecls.Contains("EditableEntityComponent") && !ecls.Contains("EditableVehicleComponent"))
					continue;

				foreach (BaseContainer ec : ebucket)
				{
					string f;
					if (ec.Get("m_sFaction", f) && !f.IsEmpty())
					{
						varInfo.m_sFaction = f;
						break;
					}
				}
				if (!varInfo.m_sFaction.IsEmpty()) break;
			}
		}

		if (varInfo.m_sFaction.IsEmpty())
			varInfo.m_sFaction = "Unaffiliated";
	}

	//------------------------------------------------------------------------------------------------
	//! Classify vehicle domain (Wheeled/Helicopter/Tracked/Boat/Plane) and tactical role from components.
	static void ExtractDomainAndRole(map<string, ref array<BaseContainer>> comps, TBD_VehicleVariantInfo varInfo)
	{
		if (HasCompSuffix(comps, "VehicleHelicopterSimulation") || HasCompSuffix(comps, "HelicopterControllerComponent"))
			varInfo.m_sVehicleDomain = "Helicopter";
		else if (HasCompSuffix(comps, "VehicleTrackedSimulation"))
			varInfo.m_sVehicleDomain = "Tracked";
		else if (HasCompSuffix(comps, "VehicleBoatSimulation"))
			varInfo.m_sVehicleDomain = "Boat";
		else if (HasCompSuffix(comps, "VehiclePlaneSimulation") || HasCompSuffix(comps, "VehicleFixedWingSimulation"))
			varInfo.m_sVehicleDomain = "Plane";
		else
			varInfo.m_sVehicleDomain = "Wheeled";

		if (HasCompSuffix(comps, "SCR_VehicleMedicComponent") || HasCompSuffix(comps, "SCR_CasualtyStationComponent") || HasCompSuffix(comps, "SCR_MedicalCargoComponent"))
			varInfo.m_sRole = "Ambulance";
		else if (HasCompSuffix(comps, "SCR_FuelNodeComponent") || HasCompSuffix(comps, "SCR_FuelSupportStationComponent"))
			varInfo.m_sRole = "Fuel Tanker";
		else if (HasCompSuffix(comps, "SCR_RepairSupportStationComponent") || HasCompSuffix(comps, "SCR_ToolboxSupportStationComponent") || HasCompSuffix(comps, "SCR_VehicleSupportStationComponent"))
			varInfo.m_sRole = "Repair & Maintenance";
		else if (HasCompSuffix(comps, "SCR_AmmoSupportStationComponent"))
			varInfo.m_sRole = "Ammunition Logistics";
		else if (HasCompSuffix(comps, "SCR_ResourceComponent") && varInfo.m_fSupplyCapacity > 0)
			varInfo.m_sRole = "Supply Logistics";
		else if (HasCompSuffix(comps, "SCR_SpawnPointComponent") || HasCompSuffix(comps, "SCR_CampaignMobileHQComponent") || HasCompSuffix(comps, "SCR_RadioSupportStationComponent"))
			varInfo.m_sRole = "Command & Control";
		else
		{
			bool hasArmor = HasCompSuffix(comps, "SCR_ArmorDamageManagerComponent") || HasCompSuffix(comps, "ArmorDamageManagerComponent");
			if (hasArmor && varInfo.m_bIsArmed)
			{
				if (varInfo.m_iPassengerSeats >= 4)
					varInfo.m_sRole = "Armored Personnel Carrier";
				else
					varInfo.m_sRole = "Armored Fighting Vehicle";
			}
			else if (varInfo.m_bIsArmed)
				varInfo.m_sRole = "Armed Combat";
			else if (varInfo.m_iPassengerSeats >= 4)
				varInfo.m_sRole = "Personnel Transport";
			else if (varInfo.m_sFaction == "CIV" || varInfo.m_sFaction.Contains("Civilian"))
				varInfo.m_sRole = "Civilian";
			else
				varInfo.m_sRole = "General Utility";
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Count seating capacity across pilot, crew, and passenger slots via primary compartment manager.
	static void ExtractSeating(map<string, ref array<BaseContainer>> comps, TBD_VehicleVariantInfo varInfo)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("CompartmentManagerComponent") || bucket.IsEmpty())
				continue;

			// Primary compartment manager contains all effective merged slots
			BaseContainer cm = bucket.Get(0);
			if (!cm) continue;

			BaseContainerList compList = cm.GetObjectArray("Compartments");
			if (!compList) compList = cm.GetObjectArray("m_aCompartments");
			if (!compList) compList = cm.GetObjectArray("CompartmentSlots");
			if (!compList) continue;

			for (int i = 0, n = compList.Count(); i < n; i++)
			{
				BaseContainer slot = compList.Get(i);
				if (!slot) continue;

				string slotCls = slot.GetClassName();
				if (slotCls.Contains("Pilot"))
					varInfo.m_iDriverSeats++;
				else if (slotCls.Contains("Turret"))
					varInfo.m_iCrewSeats++;
				else
					varInfo.m_iPassengerSeats++;
			}
			break;
		}

		if (varInfo.m_iDriverSeats == 0)
			varInfo.m_iDriverSeats = 1;

		varInfo.m_iTotalSeats = varInfo.m_iDriverSeats + varInfo.m_iCrewSeats + varInfo.m_iPassengerSeats;
	}

	//------------------------------------------------------------------------------------------------
	//! Detect mounted weapons from WeaponSlotComponent and child turrets via dynamic prefab inspection.
	static void ExtractArmament(map<string, ref array<BaseContainer>> comps, TBD_VehicleVariantInfo varInfo)
	{
		foreach (string wcls, array<BaseContainer> wbucket : comps)
		{
			if (!wcls.Contains("WeaponSlotComponent")) continue;

			foreach (BaseContainer wsc : wbucket)
			{
				string wepPrefab;
				if (wsc.Get("WeaponTemplate", wepPrefab) && !wepPrefab.IsEmpty())
					IntrospectWeaponPrefab(wepPrefab, varInfo.m_aMountedWeapons);
			}
		}

		foreach (string scls, array<BaseContainer> sbucket : comps)
		{
			if (!scls.EndsWith("SlotManagerComponent")) continue;

			foreach (BaseContainer mgr : sbucket)
			{
				BaseContainerList slots = mgr.GetObjectArray("Slots");
				if (!slots) continue;

				for (int s = 0, sn = slots.Count(); s < sn; s++)
				{
					BaseContainer slot = slots.Get(s);
					if (!slot) continue;

					string slotPrefab;
					if (slot.Get("Prefab", slotPrefab) && !slotPrefab.IsEmpty())
						ScanChildSlotWeapons(slotPrefab, varInfo.m_aMountedWeapons);
				}
			}
		}

		varInfo.m_bIsArmed = !varInfo.m_aMountedWeapons.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	protected static void ScanChildSlotWeapons(string childPrefab, notnull array<string> ioWeapons)
	{
		Resource res = Resource.Load(childPrefab);
		if (!res || !res.IsValid()) return;
		BaseResourceObject resObj = res.GetResource();
		if (!resObj) return;
		BaseContainer root = resObj.ToBaseContainer();
		if (!root) return;

		BaseContainerList comps = root.GetObjectArray("components");
		if (!comps) return;

		for (int i = 0, n = comps.Count(); i < n; i++)
		{
			BaseContainer comp = comps.Get(i);
			if (!comp) continue;

			if (comp.GetClassName().Contains("WeaponSlotComponent"))
			{
				string wepPrefab;
				if (comp.Get("WeaponTemplate", wepPrefab) && !wepPrefab.IsEmpty())
					IntrospectWeaponPrefab(wepPrefab, ioWeapons);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect weapon prefab: verify it has a weapon and firing muzzle component.
	protected static void IntrospectWeaponPrefab(string wepPrefab, notnull array<string> ioWeapons)
	{
		Resource res = Resource.Load(wepPrefab);
		if (!res || !res.IsValid()) return;
		BaseResourceObject resObj = res.GetResource();
		if (!resObj) return;
		BaseContainer root = resObj.ToBaseContainer();
		if (!root) return;

		BaseContainerList comps = root.GetObjectArray("components");
		if (!comps) return;

		bool hasWeaponComp = false;
		string weaponDisplayName;

		for (int i = 0, n = comps.Count(); i < n; i++)
		{
			BaseContainer comp = comps.Get(i);
			if (!comp) continue;

			string cls = comp.GetClassName();
			if (cls.Contains("WeaponComponent") || cls.Contains("BaseWeaponComponent"))
				hasWeaponComp = true;

			if (weaponDisplayName.IsEmpty() && (cls.Contains("UIInfo") || cls.Contains("InventoryItemComponent") || cls.Contains("WeaponComponent")))
			{
				BaseContainer ui = comp.GetObject("m_UIInfo");
				if (!ui)
				{
					BaseContainer attrs = comp.GetObject("Attributes");
					if (attrs) ui = attrs.GetObject("ItemDisplayName");
				}
				string nStr;
				if (ui && ui.Get("Name", nStr) && !nStr.IsEmpty())
				{
					if (nStr.StartsWith("#"))
						weaponDisplayName = TBD_VehicleExportNaming.CleanNameFromToken(nStr.Substring(1, nStr.Length() - 1));
					else
						weaponDisplayName = nStr;
				}
			}
		}

		if (!hasWeaponComp) return;

		if (weaponDisplayName.IsEmpty())
		{
			int lastSlash = wepPrefab.LastIndexOf("/");
			string stem = wepPrefab;
			if (lastSlash != -1) stem = wepPrefab.Substring(lastSlash + 1, wepPrefab.Length() - lastSlash - 1);
			stem.Replace(".et", "");
			stem.Replace("Weapon_", "");
			stem.Replace("_Base", "");
			weaponDisplayName = TBD_VehicleExportNaming.HumanizeStem(stem);
		}

		if (ioWeapons.Find(weaponDisplayName) == -1)
			ioWeapons.Insert(weaponDisplayName);
	}

	//------------------------------------------------------------------------------------------------
	static void ExtractSpecs(map<string, ref array<BaseContainer>> comps, TBD_VehicleVariantInfo varInfo)
	{
		foreach (string bcls, array<BaseContainer> bbucket : comps)
		{
			if (!bcls.Contains("BuoyancyComponent")) continue;

			foreach (BaseContainer bc : bbucket)
			{
				float thrustFwd = 0;
				string thrustPts;
				if (bc.Get("ThurstForward", thrustFwd) && thrustFwd > 0)
					varInfo.m_bIsAmphibious = true;
				else if (bc.Get("ThrustForward", thrustFwd) && thrustFwd > 0)
					varInfo.m_bIsAmphibious = true;
				else if (bc.Get("ThrustPoints", thrustPts) && !thrustPts.IsEmpty() && thrustPts != "0 0 0")
					varInfo.m_bIsAmphibious = true;
				else if (bc.Get("m_fWaterThrust", thrustFwd) && thrustFwd > 0)
					varInfo.m_bIsAmphibious = true;
			}
		}

		foreach (string rcls, array<BaseContainer> rbucket : comps)
		{
			if (!rcls.Contains("RigidBody")) continue;
			foreach (BaseContainer rb : rbucket)
			{
				float mass;
				if (rb.Get("Mass", mass) && mass > 0) varInfo.m_fMassKg = mass;
			}
		}

		foreach (string scls, array<BaseContainer> sbucket : comps)
		{
			if (!scls.Contains("SCR_ResourceComponent")) continue;
			foreach (BaseContainer sc : sbucket)
			{
				float supp;
				if (sc.Get("m_fMaxResourceValue", supp) && supp > 0) varInfo.m_fSupplyCapacity = supp;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Dynamically resolve vehicle platform family from directory taxonomy and prefab inheritance.
	static string ResolvePlatformId(string filePath, string resName, BaseContainer root)
	{
		string path = filePath;
		if (path.IsEmpty()) path = resName;
		path.Replace("\\", "/");

		// 1. If inside a non-family folder (Conflict_Variants, Scenarios, MP), trace to parent prefab's family
		if (path.Contains("Conflict_Variants") || path.Contains("Scenarios") || path.Contains("/MP/"))
		{
			if (root)
			{
				BaseContainer parent = root.GetAncestor();
				if (parent)
				{
					string parentRes = parent.GetResourceName();
					string parentPlat = ResolvePlatformId(parentRes, parentRes, parent);
					if (parentPlat != "Vehicle" && parentPlat != "Conflict_Variants" && parentPlat != "Core")
						return parentPlat;
				}
			}
		}

		// 2. Directory taxonomy under Prefabs/Vehicles/
		int vehIdx = path.IndexOf("Prefabs/Vehicles/");
		if (vehIdx != -1)
		{
			string sub = path.Substring(vehIdx + 17, path.Length() - vehIdx - 17);
			array<string> parts = {};
			sub.Split("/", parts, true);
			if (parts.Count() >= 3 && !parts[1].EndsWith(".et") && parts[1] != "Conflict_Variants")
				return parts[1];
			else if (parts.Count() == 2 && !parts[0].EndsWith(".et") && parts[0] != "Conflict_Variants")
				return parts[0];
		}

		// 3. Ancestor traversal: find root base prefab before generic vehicle base classes
		if (root)
		{
			BaseContainer cur = root.GetAncestor();
			BaseContainer lastValidPlatform = null;
			int hops = 0;
			while (cur && hops < 16)
			{
				string ancRes = cur.GetResourceName();
				int lastSlash = ancRes.LastIndexOf("/");
				string stem = ancRes;
				if (lastSlash != -1) stem = ancRes.Substring(lastSlash + 1, ancRes.Length() - lastSlash - 1);
				stem.Replace(".et", "");
				if (TBD_VehicleExportNaming.IsGenericVehicleBaseStem(stem)) break;
				lastValidPlatform = cur;
				cur = cur.GetAncestor();
				hops++;
			}

			if (lastValidPlatform)
			{
				string platRes = lastValidPlatform.GetResourceName();
				int pSlash = platRes.LastIndexOf("/");
				string pStem = platRes;
				if (pSlash != -1) pStem = platRes.Substring(pSlash + 1, platRes.Length() - pSlash - 1);
				pStem.Replace(".et", "");
				pStem.Replace("_Base", "");
				pStem.Replace("_base", "");
				if (!pStem.IsEmpty() && pStem != "Conflict_Variants" && pStem != "Core")
					return pStem;
			}
		}

		// 4. Fallback: immediate parent folder of the prefab
		int lastSlash2 = path.LastIndexOf("/");
		if (lastSlash2 > 0)
		{
			string parentPath = path.Substring(0, lastSlash2);
			int prevSlash = parentPath.LastIndexOf("/");
			if (prevSlash != -1)
			{
				string folderName = parentPath.Substring(prevSlash + 1, parentPath.Length() - prevSlash - 1);
				if (!folderName.IsEmpty() && folderName != "Vehicles" && folderName != "Wheeled" && folderName != "Helicopters" && folderName != "Tracked" && folderName != "Conflict_Variants")
					return folderName;
			}
		}

		return "Vehicle";
	}

	//------------------------------------------------------------------------------------------------
	protected static bool HasCompSuffix(map<string, ref array<BaseContainer>> comps, string suffix)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.EndsWith(suffix))
				return true;
		}
		return false;
	}
}
