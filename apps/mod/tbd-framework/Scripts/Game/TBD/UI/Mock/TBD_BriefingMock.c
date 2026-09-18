//! Briefing pass (2026-09-14) — the mock catalog behind the Briefing screen.
//!
//! Dataset = the Stitch pre-game mockups verbatim (frequencies, objectives, rules, lore,
//! parameters, markers, friendly/enemy assets, friendly/enemy uniforms). Where a mockup names a
//! thing vanilla does not ship (BMP-2, T-72B, GAZ-66; the CDF / VDV uniforms) the row keeps the
//! mockup's words and the 3D preview uses a vanilla prefab that exists on record
//! (`contracts_v2/catalogs/registry-items.workbench.json`, `Data/registry.json`), so the
//! previews are real renders, not placeholders. Consumed only through `TBD_BriefingCatalog.Get()`.
class TBD_BriefingMock
{
	static const ResourceName VEH_BTR70    = "{1C5FE7B7FF49BB8D}Prefabs/Vehicles/Wheeled/BTR70/BTR70_Base.et";
	static const ResourceName VEH_BRDM2    = "{254289B9C09904AB}Prefabs/Vehicles/Wheeled/BRDM2/BRDM2.et";
	static const ResourceName VEH_URAL     = "{16C1F16C9B053801}Prefabs/Vehicles/Wheeled/Ural4320/Ural4320_transport.et";
	static const ResourceName VEH_UAZ469   = "{259EE7B78C51B624}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469.et";
	static const ResourceName VEH_M1025    = "{3EA6F47D95867114}Prefabs/Vehicles/Wheeled/M998/M1025_armed_M2HB.et";
	static const ResourceName VEH_M923     = "{3F2AA823B6C65E1E}Prefabs/Vehicles/Wheeled/M923A1/M923A1_transport_MERDC.et";
	static const ResourceName VEH_M151     = "{F6B23D17D5067C11}Prefabs/Vehicles/Wheeled/M151A2/M151A2_M2HB.et";

	//------------------------------------------------------------------------------------------------
	static TBD_BriefingCatalog Build()
	{
		TBD_BriefingCatalog c = new TBD_BriefingCatalog();
		c.m_sMissionId = "wog_187_chollima_on_the_wing_10";
		c.m_Own = new TBD_BriefingFaction("BLUFOR", "BLUFOR", "DEFENDERS", TBD_EUITint.BLUFOR);
		c.m_Enemy = new TBD_BriefingFaction("OPFOR", "OPFOR", "ATTACKERS", TBD_EUITint.OPFOR);
		c.m_sTimeLimit = "60 Minutes";
		c.m_sCaptureSummary = "Capture 2 of 2";

		BuildNets(c);
		BuildObjectives(c);
		BuildRules(c);
		BuildLore(c);
		BuildParams(c);
		BuildAssets(c);
		BuildUniforms(c);
		BuildPlans(c);
		return c;
	}

	//------------------------------------------------------------------------------------------------
	protected static void BuildNets(TBD_BriefingCatalog c)
	{
		Net(c, "", "LR Command Net", 76.2, true, false, 50.6, 35.4, 71.3);
		Net(c, "A1-1", "Company HQ (Mission Maker)", 390.7, false, true, 133.3, 276.9, 384.0);
		Net(c, "A1-2", "1st Platoon, 2nd Squad", 163.7, false, false, 382.1, 346.4, 250.9);
		Net(c, "A1-3", "1st Platoon, 3rd Squad", 263.5, false, false, 260.7, 318.7, 127.2);
		Net(c, "A1-4", "1st Platoon, 4th Squad", 273.8, false, false, 319.1, 334.3, 441.3);
		Net(c, "A2-1", "2nd Platoon, 1st Squad", 499.5, false, false, 227.4, 279.1, 391.1);
		Net(c, "A2-2", "2nd Platoon, 2nd Squad", 388.8, false, false, 457.1, 461.7, 366.1);
	}

	//------------------------------------------------------------------------------------------------
	protected static void Net(TBD_BriefingCatalog c, string callsign, string label, float freq, bool longRange, bool own, float a, float b, float d)
	{
		TBD_NetInfo net = new TBD_NetInfo(callsign, label, freq, longRange, own);
		net.Aux(a, b, d);
		c.m_aNets.Insert(net);
	}

	//------------------------------------------------------------------------------------------------
	protected static void BuildObjectives(TBD_BriefingCatalog c)
	{
		// Coordinates are on Everon so Locate has somewhere to pan.
		c.m_aObjectives.Insert(new TBD_ObjectiveInfo(1, "Southern Zone", "Defend", "Sector", "60s", "Permanent (Locked)", 5180, 4260));
		c.m_aObjectives.Insert(new TBD_ObjectiveInfo(2, "Northern Zone", "Defend", "Sector", "90s", "Permanent (Locked)", 5420, 6140));
	}

	//------------------------------------------------------------------------------------------------
	protected static void BuildRules(TBD_BriefingCatalog c)
	{
		TBD_RuleGroup mission = new TBD_RuleGroup("Mission Rules", true);
		mission.Add("Dual Zone Capture Requirement", "Attackers must capture and secure both the Southern and Northern sectors to achieve victory. Defenders win by holding at least one sector at mission timeout.");
		mission.Add("Crewman & Armor Operation Restrictions", "Armored combat vehicles (BMP-2, M60A1, BTR-80) may only be crewed and operated by designated Engineer / Crewman slot qualifications. Regular infantry may not operate primary armament or drive.");
		mission.Add("Sector Fortification Perimeter", "Field fortifications, sandbag barriers, and trenching tools cannot be constructed within 50 meters of the central objective flag markers.");
		mission.Add("Capture Activation Delay", "Sector capture flags become active exactly 5 minutes after Safe Start lifts to allow defender deployment and initial fortification.");
		c.m_aRuleGroups.Insert(mission);

		TBD_RuleGroup general = new TBD_RuleGroup("General Rules", true);
		general.Add("Strict One-Life Elimination", "Death is permanent with no respawns. Eliminated players are transitioned immediately into spectator mode. Leaking intel or ghosting over external comms results in an administrative ban.");
		general.Add("Safe Start & Staging Adherence", "Discharging weapons, throwing ordnance, or crossing designated faction staging boundaries prior to \"WEAPONS FREE\" is strictly prohibited.");
		general.Add("Radio Discipline & Net Scoping", "Players must operate exclusively on their designated Long Range (LR) and Short Range (SR) assigned frequencies. Do not scan or transmit on enemy frequencies unless authorized by scenario rules.");
		general.Add("Area of Operations (AO) Boundaries", "Players must remain inside the marked AO at all times. Crossing the boundary triggers a warning; remaining outside it for 60 seconds counts as elimination.");
		c.m_aRuleGroups.Insert(general);
	}

	//------------------------------------------------------------------------------------------------
	protected static void BuildLore(TBD_BriefingCatalog c)
	{
		c.m_aLore.Insert("In the early morning hours of October 12th, motorized vanguard elements of the 7th Guards Airborne Division crossed the southern perimeter into the valley under dense low-hanging mist. With primary communications corridors pre-sighted and electronic countermeasures active across the operational sector, defending forces must establish perimeter fortifications and maintain forward observation outposts before the primary mechanized assault arrives.");
	}

	//------------------------------------------------------------------------------------------------
	protected static void BuildParams(TBD_BriefingCatalog c)
	{
		c.m_aParams.Insert(new TBD_ParamInfo("visibility", "View Distance", "2,500 m"));
		c.m_aParams.Insert(new TBD_ParamInfo("shield", "Safe Start Duration", "3 min", TBD_EUITint.WARNING));
		c.m_aParams.Insert(new TBD_ParamInfo("schedule", "Mission Start Time", "07:00"));
		c.m_aParams.Insert(new TBD_ParamInfo("hourglass_bottom", "Mission Duration", "60 min"));
		c.m_aParams.Insert(new TBD_ParamInfo("thermostat", "Thermals (TI)", "Disabled"));
	}

	//------------------------------------------------------------------------------------------------
	protected static void BuildAssets(TBD_BriefingCatalog c)
	{
		// Friendly (Soviet motor pool, as the lobby's kits).
		TBD_AssetTypeInfo btr = Type("BTR-70", VEH_BTR70, 2, "80 km/h", "Yes", "10 km/h", "Crew 2 + Dismount 7 Troops");
		btr.m_bExpanded = true;
		btr.m_aWeapons.Insert("14.5mm KPVT Heavy MG");
		btr.m_aWeapons.Insert("Coax 7.62mm PKT");
		btr.m_aInstances.Insert(Apc("Alpha 1-1", 5030, 4410));
		btr.m_aInstances.Insert(Apc("Alpha 1-2", 5046, 4402));
		c.m_aFriendlyAssets.Insert(btr);

		TBD_AssetTypeInfo brdm = Type("BRDM-2", VEH_BRDM2, 1, "95 km/h", "Yes", "9 km/h", "Crew 2 + 2 Troops");
		brdm.m_aWeapons.Insert("14.5mm KPVT Heavy MG");
		brdm.m_aWeapons.Insert("Coax 7.62mm PKT");
		brdm.m_aInstances.Insert(Scout("Recon 1-1", 5090, 4380));
		c.m_aFriendlyAssets.Insert(brdm);

		TBD_AssetTypeInfo ural = Type("Ural-4320", VEH_URAL, 3, "85 km/h", "No", "", "Crew 1 + 16 Troops");
		ural.m_aInstances.Insert(Truck("Logistics 1"));
		ural.m_aInstances.Insert(Truck("Logistics 2"));
		ural.m_aInstances.Insert(Truck("Logistics 3"));
		c.m_aFriendlyAssets.Insert(ural);

		TBD_AssetTypeInfo uaz = Type("UAZ-469", VEH_UAZ469, 2, "100 km/h", "No", "", "Crew 1 + 3 Troops");
		uaz.m_aInstances.Insert(Truck("HQ 1"));
		uaz.m_aInstances.Insert(Truck("HQ 2"));
		c.m_aFriendlyAssets.Insert(uaz);

		c.m_aFriendlyAssets.Insert(new TBD_AssetTypeInfo("Crates", "", 5));

		// Enemy (the other side's motor pool; the mockup's enemy panel is the same layout in red).
		TBD_AssetTypeInfo m1025 = Type("M1025 (M2HB)", VEH_M1025, 2, "105 km/h", "No", "", "Crew 2 + 2 Troops");
		m1025.m_bExpanded = true;
		m1025.m_aWeapons.Insert("12.7mm M2HB Heavy MG");
		m1025.m_aInstances.Insert(Scout("Bravo 1-1"));
		m1025.m_aInstances.Insert(Scout("Bravo 1-2"));
		c.m_aEnemyAssets.Insert(m1025);

		TBD_AssetTypeInfo m923 = Type("M923A1", VEH_M923, 3, "90 km/h", "No", "", "Crew 1 + 16 Troops");
		m923.m_aInstances.Insert(Truck("Supply 1"));
		m923.m_aInstances.Insert(Truck("Supply 2"));
		m923.m_aInstances.Insert(Truck("Supply 3"));
		c.m_aEnemyAssets.Insert(m923);

		TBD_AssetTypeInfo m151 = Type("M151A2 (M2HB)", VEH_M151, 2, "105 km/h", "No", "", "Crew 2 + 2 Troops");
		m151.m_aWeapons.Insert("12.7mm M2HB Heavy MG");
		m151.m_aInstances.Insert(Scout("Charlie 1-1"));
		m151.m_aInstances.Insert(Scout("Charlie 1-2"));
		c.m_aEnemyAssets.Insert(m151);
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_AssetTypeInfo Type(string name, ResourceName prefab, int count, string speedRoad, string amphibious, string speedWater, string crew)
	{
		TBD_AssetTypeInfo type = new TBD_AssetTypeInfo(name, prefab, count);
		type.Specs(speedRoad, amphibious, speedWater, crew);
		return type;
	}

	//------------------------------------------------------------------------------------------------
	//! The mockup's fully stocked APC.
	protected static TBD_AssetInstanceInfo Apc(string callsign, float x, float z)
	{
		TBD_AssetInstanceInfo v = new TBD_AssetInstanceInfo(callsign, x, z);
		v.m_aAmmo.Insert(new TBD_KitEntry("14.5mm KPVT Belts", "500 rnds"));
		v.m_aAmmo.Insert(new TBD_KitEntry("Coax 7.62mm PKT", "2,000 rnds"));
		Stock(v);
		return v;
	}

	protected static TBD_AssetInstanceInfo Scout(string callsign, float x = 0, float z = 0)
	{
		TBD_AssetInstanceInfo v = new TBD_AssetInstanceInfo(callsign, x, z);
		v.m_aAmmo.Insert(new TBD_KitEntry("Heavy MG Belts", "600 rnds"));
		Stock(v);
		return v;
	}

	protected static TBD_AssetInstanceInfo Truck(string callsign)
	{
		TBD_AssetInstanceInfo v = new TBD_AssetInstanceInfo(callsign);
		Stock(v);
		return v;
	}

	protected static void Stock(TBD_AssetInstanceInfo v)
	{
		v.m_aInvAmmo.Insert(new TBD_KitEntry("AK-74 5.45 Mag", "", 30));
		v.m_aInvAmmo.Insert(new TBD_KitEntry("PKM 7.62 Belt", "", 8));
		v.m_aInvAmmo.Insert(new TBD_KitEntry("RPG-7 HEAT", "", 6));
		v.m_aInvWeapons.Insert(new TBD_KitEntry("AKS-74U", "", 2));
		v.m_aInvWeapons.Insert(new TBD_KitEntry("RPG-7V2", "", 1));
		v.m_aInvGrenades.Insert(new TBD_KitEntry("F-1 Grenades", "", 12));
		v.m_aInvGrenades.Insert(new TBD_KitEntry("Smoke (White)", "", 12));
		v.m_aInvGrenades.Insert(new TBD_KitEntry("Smoke (Red)", "", 4));
		v.m_aInvMedical.Insert(new TBD_KitEntry("Surgical Kit", "", 1));
		v.m_aInvMedical.Insert(new TBD_KitEntry("Blood (500ml)", "", 4));
		v.m_aInvMedical.Insert(new TBD_KitEntry("Bandages", "", 10));
		v.m_aInvMedical.Insert(new TBD_KitEntry("Tourniquets", "", 6));
		v.m_aInvMisc.Insert(new TBD_KitEntry("Toolkit", "", 1));
		v.m_aInvMisc.Insert(new TBD_KitEntry("Spare Barrel", "", 1));
		v.m_aInvMisc.Insert(new TBD_KitEntry("E-Tool", "", 2));
	}

	//------------------------------------------------------------------------------------------------
	protected static void BuildUniforms(TBD_BriefingCatalog c)
	{
		TBD_UniformInfo cdf = new TBD_UniformInfo("CDF (12th Mechanized)", "kit:sov_rifleman", "", "TTsKO / Dubok");
		cdf.m_aChips.Insert("MAG");
		cdf.m_aChips.Insert("AK");
		c.m_aFriendlyUniforms.Insert(cdf);

		TBD_UniformInfo guard = new TBD_UniformInfo("CDF National Guard", "kit:fia_rifleman", "", "Olive Drab / BDU");
		guard.m_aChips.Insert("AK");
		c.m_aFriendlyUniforms.Insert(guard);

		TBD_UniformInfo vdv = new TBD_UniformInfo("Russian VDV (Airborne)", "kit:us_rifleman", "", "VSR-93 / Flora");
		vdv.m_aChips.Insert("MAG");
		vdv.m_aChips.Insert("AK");
		c.m_aEnemyUniforms.Insert(vdv);

		TBD_UniformInfo chdkz = new TBD_UniformInfo("ChDKZ (Separatist Militia)", "kit:fia_sl", "", "Gorka-3 / Khaki");
		chdkz.m_aChips.Insert("AK");
		c.m_aEnemyUniforms.Insert(chdkz);
	}

	//------------------------------------------------------------------------------------------------
	protected static void BuildPlans(TBD_BriefingCatalog c)
	{
		c.m_aPlans.Insert(new TBD_PlanInfo("plan_alpha", "Plan Alpha - Main Axis of Advance"));
		c.m_aPlans.Insert(new TBD_PlanInfo("plan_bravo", "Plan Bravo - Flank & Cutoff"));
		c.m_aPlans.Insert(new TBD_PlanInfo("plan_charlie", "Plan Charlie - Defense in Depth"));
		c.m_aPlans.Insert(new TBD_PlanInfo("plan_recon", "Plan Recon - Screen & Observation"));
	}
}
