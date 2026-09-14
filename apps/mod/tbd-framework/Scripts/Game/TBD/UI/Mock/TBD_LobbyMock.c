//! Pre-game rebuild (2026-09-13) — the mock catalog behind the Lobby.
//!
//! Dataset is the Stitch pre-game mockup (lobby_sidebar / orbat_panel_blufor /
//! slot_kit_inspector): BLUFOR Defending (92 seats) vs OPFOR Attacking (95), Spectators (10);
//! BLUFOR squads Alpha 1-1 UAZ-3151, Alpha 2-1 BMP-2, Alpha 2-2 BTR-80, Bravo 1-1 T-72B with the
//! drawn roles, weapons, tags and holders; OPFOR mirrors them under Soviet callsigns. Three kits
//! (`rifleman_at` = the mockup's `8: RIFLEMAN (AT)` in full, `medic`, `crew`) keyed by role.
//!
//! Kit-preview pass (2026-09-13): every kit also carries its WIRE half — kit alias + a real
//! `TBD_SlotLoadoutStruct` of GUID-pinned vanilla prefabs (read off golden-missions/
//! slot-loadout-coverage.json, Character_USSR_AT.et, Character_USSR_SL.et) — so the 3D doll wears
//! exactly what the server pass would spawn. `crew` is deliberately kit-only (no loadout).
//!
//! Counts the UI shows (`0 / 92`, `2/3`) are COMPUTED by the screen from these rows. Consumed only
//! through `TBD_LobbyCatalog.Get()`; replaces the retired `TBD_LobbyMockData` (voice channels
//! return with the voice-panel pass, from its own mockup).
class TBD_LobbyMock
{
	//------------------------------------------------------------------------------------------------
	static TBD_LobbyCatalog Build()
	{
		TBD_LobbyCatalog catalog = new TBD_LobbyCatalog();
		catalog.m_sMissionId = "wog_187_chollima_on_the_wing_10";
		catalog.m_Identity = new TBD_SessionIdentity("Mission Maker", "ADMIN", 1);

		catalog.m_aFactions.Insert(new TBD_LobbyFactionInfo("BLUFOR", "BLUFOR", "DEFENDING", TBD_EUITint.BLUFOR, 92));
		catalog.m_aFactions.Insert(new TBD_LobbyFactionInfo("OPFOR",  "OPFOR",  "ATTACKING", TBD_EUITint.OPFOR, 95));
		catalog.m_aFactions.Insert(new TBD_LobbyFactionInfo("SPEC",   "Spectators", "", TBD_EUITint.NEUTRAL, 10, true));

		catalog.m_mSquads.Insert("BLUFOR", BuildBlufor());
		catalog.m_mSquads.Insert("OPFOR", BuildOpfor());
		catalog.m_mSquads.Insert("SPEC", {});

		catalog.m_mKits.Insert("rifleman_at", BuildRiflemanAtKit());
		catalog.m_mKits.Insert("medic", BuildMedicKit());
		catalog.m_mKits.Insert("crew", BuildCrewKit());
		catalog.m_mKits.Insert("rifleman", BuildRiflemanKit());
		return catalog;
	}

	//------------------------------------------------------------------------------------------------
	protected static array<ref TBD_LobbySquadInfo> BuildBlufor()
	{
		array<ref TBD_LobbySquadInfo> squads = {};

		TBD_LobbySquadInfo hq = new TBD_LobbySquadInfo("Alpha 1-1", "UAZ-3151");
		hq.AddSlot("Platoon Commander", "rifleman", "AK-74", "", "[1stID] Miller");
		hq.AddSlot("Platoon Sergeant",  "rifleman", "AK-74");
		hq.AddSlot("Platoon Medic",     "medic",    "AK-74", "MED", "CPL Davis, R.");
		squads.Insert(hq);

		TBD_LobbySquadInfo a21 = new TBD_LobbySquadInfo("Alpha 2-1", "BMP-2");
		a21.AddSlot("Squad Leader",        "rifleman",    "AK-74",    "",    "SSG Vance, D.");
		a21.AddSlot("Team Leader",         "rifleman",    "AK-74",    "",    "SGT Kowalski, P.");
		a21.AddSlot("Automatic Rifleman",  "rifleman",    "RPK-74",   "",    "SPC Ramos, C.");
		a21.AddSlot("Grenadier",           "rifleman",    "AK-74 GL", "",    "PFC Harris, M.");
		a21.AddSlot("Team Leader",         "rifleman",    "AK-74",    "",    "SPC Clark, H.");
		a21.AddSlot("Machine Gunner",      "rifleman",    "PKM",      "",    "PFC Jones, T.");
		a21.AddSlot("Combat Medic",        "medic",       "AK-74",    "MED");
		a21.AddSlot("Rifleman (AT)",       "rifleman_at", "AK-74,RPG-7", "", "CPL Smith, K.");
		squads.Insert(a21);

		TBD_LobbySquadInfo a22 = new TBD_LobbySquadInfo("Alpha 2-2", "BTR-80");
		a22.AddSlot("Squad Leader",        "rifleman",    "AK-74",    "",    "SSG Ortiz, L.");
		a22.AddSlot("Team Leader",         "rifleman",    "AK-74",    "",    "SGT Nakamura, K.");
		a22.AddSlot("Automatic Rifleman",  "rifleman",    "RPK-74",   "",    "SPC Baker, J.");
		a22.AddSlot("Grenadier",           "rifleman",    "AK-74 GL");
		a22.AddSlot("Team Leader",         "rifleman",    "AK-74",    "",    "SPC Reyes, A.");
		a22.AddSlot("Machine Gunner",      "rifleman",    "PKM",      "",    "PFC Wu, D.");
		a22.AddSlot("Combat Medic",        "medic",       "AK-74",    "MED", "PFC Okafor, S.");
		a22.AddSlot("Rifleman (AT)",       "rifleman_at", "AK-74,RPG-7");
		squads.Insert(a22);

		TBD_LobbySquadInfo b11 = new TBD_LobbySquadInfo("Bravo 1-1", "T-72B");
		b11.AddSlot("Tank Commander", "crew", "AKS-74U", "",    "1LT Stone, B.");
		b11.AddSlot("Gunner",         "crew", "AKS-74U", "",    "SGT Becker, T.");
		b11.AddSlot("Driver",         "crew", "AKS-74U", "ENG", "SPC Volkov, Y.");
		squads.Insert(b11);

		return squads;
	}

	//------------------------------------------------------------------------------------------------
	protected static array<ref TBD_LobbySquadInfo> BuildOpfor()
	{
		array<ref TBD_LobbySquadInfo> squads = {};

		TBD_LobbySquadInfo hq = new TBD_LobbySquadInfo("Sokol 1-1", "UAZ-3151");
		hq.AddSlot("Platoon Commander", "rifleman", "AK-74", "", "ST LT Petrov, I.");
		hq.AddSlot("Platoon Sergeant",  "rifleman", "AK-74", "", "SSGT Ivanov, D.");
		hq.AddSlot("Platoon Medic",     "medic",    "AK-74", "MED");
		squads.Insert(hq);

		TBD_LobbySquadInfo s21 = new TBD_LobbySquadInfo("Sokol 2-1", "BMP-2");
		s21.AddSlot("Squad Leader",       "rifleman",    "AK-74",    "",    "SGT Morozov, A.");
		s21.AddSlot("Team Leader",        "rifleman",    "AK-74");
		s21.AddSlot("Automatic Rifleman", "rifleman",    "RPK-74",   "",    "PVT Sokolov, N.");
		s21.AddSlot("Grenadier",          "rifleman",    "AK-74 GL", "",    "PVT Lebedev, K.");
		s21.AddSlot("Team Leader",        "rifleman",    "AK-74",    "",    "JR SGT Kozlov, V.");
		s21.AddSlot("Machine Gunner",     "rifleman",    "PKM");
		s21.AddSlot("Combat Medic",       "medic",       "AK-74",    "MED", "PVT Novikov, E.");
		s21.AddSlot("Rifleman (AT)",      "rifleman_at", "AK-74,RPG-7", "", "PVT Fedorov, M.");
		squads.Insert(s21);

		TBD_LobbySquadInfo v11 = new TBD_LobbySquadInfo("Vympel 1-1", "T-72B");
		v11.AddSlot("Tank Commander", "crew", "AKS-74U", "",    "LT Orlov, S.");
		v11.AddSlot("Gunner",         "crew", "AKS-74U");
		v11.AddSlot("Driver",         "crew", "AKS-74U", "ENG", "PVT Popov, R.");
		squads.Insert(v11);

		return squads;
	}

	//------------------------------------------------------------------------------------------------
	//! The slot_kit_inspector mockup, verbatim.
	protected static TBD_KitInfo BuildRiflemanAtKit()
	{
		TBD_KitInfo kit = new TBD_KitInfo("rifleman_at");
		DressRiflemanAt(kit);
		AddGear(kit);
		kit.m_aGear.Insert(new TBD_KitEntry("Backpack", "RPG-7 Rocket Pack (Backpack)"));

		TBD_KitWeapon ak = new TBD_KitWeapon("WEAPON SLOT 1", "AK-74", "7 MAGS");
		ak.m_aAttachments.Insert(new TBD_KitEntry("Optic / Sight", "1P29 Tulip (4x)"));
		ak.m_aAttachments.Insert(new TBD_KitEntry("Muzzle Device", "Standard 2-Chamber Brake"));
		ak.m_aAttachments.Insert(new TBD_KitEntry("Handguard / Rail", "Plum Polymer"));
		ak.m_aAttachments.Insert(new TBD_KitEntry("Tactical Light", "None"));
		ak.m_aAmmo.Insert(new TBD_KitEntry("5.45x39mm 7N6 (Standard Ball)", "", 5));
		ak.m_aAmmo.Insert(new TBD_KitEntry("5.45x39mm 7T3 (Green Tracer)", "", 2));
		kit.m_aWeapons.Insert(ak);

		TBD_KitWeapon pm = new TBD_KitWeapon("WEAPON SLOT 2", "Makarov PM", "3 MAGS");
		pm.m_aAttachments.Insert(new TBD_KitEntry("Optic / Sight", "Fixed Iron Sights"));
		pm.m_aAttachments.Insert(new TBD_KitEntry("Muzzle Device", "None"));
		pm.m_aAttachments.Insert(new TBD_KitEntry("Grip", "Bakelite Grips"));
		pm.m_aAttachments.Insert(new TBD_KitEntry("Light / Laser", "None"));
		pm.m_aAmmo.Insert(new TBD_KitEntry("9x18mm 57-N-181S (Standard Ball)", "", 3));
		kit.m_aWeapons.Insert(pm);

		TBD_KitWeapon rpg = new TBD_KitWeapon("WEAPON SLOT 3", "RPG-7", "4 ROCKETS");
		rpg.m_aAttachments.Insert(new TBD_KitEntry("Optic / Sight", "PGO-7V (2.7x)"));
		rpg.m_aAttachments.Insert(new TBD_KitEntry("Warhead Mtd", "PG-7VL HEAT", 0, TBD_EUITint.WARNING));
		rpg.m_aAttachments.Insert(new TBD_KitEntry("Bipod", "None"));
		rpg.m_aAttachments.Insert(new TBD_KitEntry("Sling", "Standard Canvas"));
		rpg.m_aAmmo.Insert(new TBD_KitEntry("PG-7VL HEAT", "", 3));
		rpg.m_aAmmo.Insert(new TBD_KitEntry("OG-7V Fragmentation", "", 1));
		kit.m_aWeapons.Insert(rpg);

		AddGrenades(kit);
		AddGadgets(kit, "R-148 (VHF Squad Net)", "R-107M Backpack Radio");
		AddTools(kit);
		AddMedical(kit, 4, 2, 1);
		AddMisc(kit);
		return kit;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_KitInfo BuildRiflemanKit()
	{
		TBD_KitInfo kit = new TBD_KitInfo("rifleman");
		DressRifleman(kit);
		AddGear(kit);
		kit.m_aGear.Insert(new TBD_KitEntry("Backpack", "RD-54 Assault Pack"));
		kit.m_aWeapons.Insert(BuildAk("WEAPON SLOT 1", 8));
		kit.m_aWeapons.Insert(BuildMakarov("WEAPON SLOT 2"));
		AddGrenades(kit);
		AddGadgets(kit, "R-148 (VHF Squad Net)", "None");
		AddTools(kit);
		AddMedical(kit, 4, 2, 1);
		AddMisc(kit);
		return kit;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_KitInfo BuildMedicKit()
	{
		TBD_KitInfo kit = new TBD_KitInfo("medic");
		DressMedic(kit);
		AddGear(kit);
		kit.m_aGear.Insert(new TBD_KitEntry("Backpack", "Medical Backpack (Large)"));
		kit.m_aWeapons.Insert(BuildAk("WEAPON SLOT 1", 5));
		kit.m_aWeapons.Insert(BuildMakarov("WEAPON SLOT 2"));
		kit.m_aGrenades.Insert(new TBD_KitEntry("RDG-2 Smoke (White)", "", 4));
		AddGadgets(kit, "R-148 (VHF Squad Net)", "None");
		AddTools(kit);
		AddMedical(kit, 20, 8, 6);
		kit.m_aMedical.Insert(new TBD_KitEntry("Saline Bag", "", 4));
		kit.m_aMedical.Insert(new TBD_KitEntry("Splint", "", 4));
		AddMisc(kit);
		return kit;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_KitInfo BuildCrewKit()
	{
		TBD_KitInfo kit = new TBD_KitInfo("crew");
		DressCrew(kit);
		kit.m_aGear.Insert(new TBD_KitEntry("Helmet", "TSh-4 Tanker Helmet"));
		kit.m_aGear.Insert(new TBD_KitEntry("Vest", "None"));
		kit.m_aGear.Insert(new TBD_KitEntry("Jacket", "Tanker Coverall (Black)"));
		kit.m_aGear.Insert(new TBD_KitEntry("Pants", "Tanker Coverall (Black)"));
		kit.m_aGear.Insert(new TBD_KitEntry("Boots", "Combat Boots (Leather)"));
		kit.m_aGear.Insert(new TBD_KitEntry("Gloves", "Leather Gloves"));
		kit.m_aGear.Insert(new TBD_KitEntry("Backpack", "None"));

		TBD_KitWeapon aks = new TBD_KitWeapon("WEAPON SLOT 1", "AKS-74U", "4 MAGS");
		aks.m_aAttachments.Insert(new TBD_KitEntry("Optic / Sight", "Fixed Iron Sights"));
		aks.m_aAttachments.Insert(new TBD_KitEntry("Muzzle Device", "Booster"));
		aks.m_aAttachments.Insert(new TBD_KitEntry("Stock", "Folding Skeleton"));
		aks.m_aAttachments.Insert(new TBD_KitEntry("Tactical Light", "None"));
		aks.m_aAmmo.Insert(new TBD_KitEntry("5.45x39mm 7N6 (Standard Ball)", "", 4));
		kit.m_aWeapons.Insert(aks);
		kit.m_aWeapons.Insert(BuildMakarov("WEAPON SLOT 2"));

		kit.m_aGrenades.Insert(new TBD_KitEntry("RDG-2 Smoke (White)", "", 2));
		AddGadgets(kit, "R-148 (VHF Squad Net)", "R-123M Vehicle Set");
		kit.m_aTools.Insert(new TBD_KitEntry("Entrenching Tool", "None"));
		kit.m_aTools.Insert(new TBD_KitEntry("Vehicle Toolkit", "Armour Repair Kit", 1));
		kit.m_aTools.Insert(new TBD_KitEntry("Wirecutters", "None"));
		kit.m_aTools.Insert(new TBD_KitEntry("Marker Kit", "None"));
		AddMedical(kit, 4, 1, 1);
		AddMisc(kit);
		return kit;
	}

	// ── Shared pieces ───────────────────────────────────────────────────────────────────────

	protected static void AddGear(TBD_KitInfo kit)
	{
		kit.m_aGear.Insert(new TBD_KitEntry("Helmet", "SSh-68 Steel Helmet"));
		kit.m_aGear.Insert(new TBD_KitEntry("Vest", "6B5 Ballistic Body Armor"));
		kit.m_aGear.Insert(new TBD_KitEntry("Jacket", "VSR-93 Flora Jacket"));
		kit.m_aGear.Insert(new TBD_KitEntry("Pants", "VSR-93 Flora Trousers"));
		kit.m_aGear.Insert(new TBD_KitEntry("Boots", "Combat Boots (Leather)"));
		kit.m_aGear.Insert(new TBD_KitEntry("Gloves", "Shooting Gloves"));
	}

	protected static TBD_KitWeapon BuildAk(string slotLabel, int mags)
	{
		TBD_KitWeapon ak = new TBD_KitWeapon(slotLabel, "AK-74", string.Format("%1 MAGS", mags));
		ak.m_aAttachments.Insert(new TBD_KitEntry("Optic / Sight", "Fixed Iron Sights"));
		ak.m_aAttachments.Insert(new TBD_KitEntry("Muzzle Device", "Standard 2-Chamber Brake"));
		ak.m_aAttachments.Insert(new TBD_KitEntry("Handguard / Rail", "Plum Polymer"));
		ak.m_aAttachments.Insert(new TBD_KitEntry("Tactical Light", "None"));
		ak.m_aAmmo.Insert(new TBD_KitEntry("5.45x39mm 7N6 (Standard Ball)", "", mags - 2));
		ak.m_aAmmo.Insert(new TBD_KitEntry("5.45x39mm 7T3 (Green Tracer)", "", 2));
		return ak;
	}

	protected static TBD_KitWeapon BuildMakarov(string slotLabel)
	{
		TBD_KitWeapon pm = new TBD_KitWeapon(slotLabel, "Makarov PM", "3 MAGS");
		pm.m_aAttachments.Insert(new TBD_KitEntry("Optic / Sight", "Fixed Iron Sights"));
		pm.m_aAttachments.Insert(new TBD_KitEntry("Muzzle Device", "None"));
		pm.m_aAttachments.Insert(new TBD_KitEntry("Grip", "Bakelite Grips"));
		pm.m_aAttachments.Insert(new TBD_KitEntry("Light / Laser", "None"));
		pm.m_aAmmo.Insert(new TBD_KitEntry("9x18mm 57-N-181S (Standard Ball)", "", 3));
		return pm;
	}

	protected static void AddGrenades(TBD_KitInfo kit)
	{
		kit.m_aGrenades.Insert(new TBD_KitEntry("F-1 Fragmentation", "", 2));
		kit.m_aGrenades.Insert(new TBD_KitEntry("RDG-2 Smoke (White)", "", 2));
		kit.m_aGrenades.Insert(new TBD_KitEntry("RDG-2 Smoke (Orange)", "", 1));
	}

	protected static void AddGadgets(TBD_KitInfo kit, string srRadio, string lrRadio)
	{
		kit.m_aGadgets.Insert(new TBD_KitEntry("Binoculars", "B8x30 (With Reticle)"));
		kit.m_aGadgets.Insert(new TBD_KitEntry("Compass", "Adrianov Compass"));
		kit.m_aGadgets.Insert(new TBD_KitEntry("Map", "Topographic Tactical Map"));
		kit.m_aGadgets.Insert(new TBD_KitEntry("Map Tools", "Protractor"));
		kit.m_aGadgets.Insert(new TBD_KitEntry("Night Vision", "None"));
		kit.m_aGadgets.Insert(new TBD_KitEntry("GPS", "None"));
		kit.m_aGadgets.Insert(new TBD_KitEntry("SR Radio", srRadio));
		kit.m_aGadgets.Insert(new TBD_KitEntry("LR Radio", lrRadio));
	}

	protected static void AddTools(TBD_KitInfo kit)
	{
		kit.m_aTools.Insert(new TBD_KitEntry("Entrenching Tool", "E-Tool (MPL-50)", 1));
		kit.m_aTools.Insert(new TBD_KitEntry("Vehicle Toolkit", "None"));
		kit.m_aTools.Insert(new TBD_KitEntry("Wirecutters", "None"));
		kit.m_aTools.Insert(new TBD_KitEntry("Marker Kit", "SWT Marker Kit", 1));
	}

	protected static void AddMedical(TBD_KitInfo kit, int bandages, int tourniquets, int morphine)
	{
		kit.m_aMedical.Insert(new TBD_KitEntry("Bandages", "", bandages));
		kit.m_aMedical.Insert(new TBD_KitEntry("Tourniquet", "", tourniquets));
		kit.m_aMedical.Insert(new TBD_KitEntry("Morphine", "", morphine));
	}

	protected static void AddMisc(TBD_KitInfo kit)
	{
		kit.m_aMisc.Insert(new TBD_KitEntry("Explosive / Demo", "None"));
		kit.m_aMisc.Insert(new TBD_KitEntry("Chemlights", "Chemlight (Green)", 2));
		kit.m_aMisc.Insert(new TBD_KitEntry("Signal Flare", "RSP-30 Flare (Green)", 1));
		kit.m_aMisc.Insert(new TBD_KitEntry("Field Utility", "Earplugs", 1));
	}

	// ── The wire half of each kit ────────────────────────────────────────────────────────────
	// GUID-pinned vanilla prefabs, all read off files on record (see the header). The Makarov the
	// text card lists has no GUID on record, so `handgun` stays empty until one is pinned.
	static const string KIT_SOV_RIFLEMAN = "kit:sov_rifleman";
	static const ResourceName PREFAB_SOV_RIFLEMAN = "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et";
	static const string AK74            = "{43497A18DD888667}Prefabs/Weapons/Rifles/AK74/Rifle_AK74_base.et";
	static const string AK74N_1P29      = "{EB404DC9E1BCB750}Prefabs/Weapons/Rifles/AK74/Rifle_AK74N_1P29.et";
	static const string AKS74U          = "{BFEA719491610A45}Prefabs/Weapons/Rifles/AKS74U/Rifle_AKS74U.et";
	static const string RPG7_PGO7       = "{E8A55396050E1762}Prefabs/Weapons/Launchers/RPG7/Launcher_RPG7_PGO7.et";
	static const string RGD5            = "{645C73791ECA1698}Prefabs/Weapons/Grenades/Grenade_RGD5.et";
	static const string OPTIC_1P29      = "{ACDF49FACD0701A8}Prefabs/Weapons/Attachments/Optics/Optic_1P29/Optic_1P29.et";
	static const string OPTIC_PSO1      = "{C850A33226B8F9C1}Prefabs/Weapons/Attachments/Optics/Optic_PSO1/Optic_PSO1.et";
	static const string MAG_AK_30       = "{63C1E699345B24F9}Prefabs/Weapons/Magazines/Magazine_545x39_AK_30rnd_Base.et";
	static const string MAG_RPK_45      = "{BC74DAC891D48540}Prefabs/Weapons/Magazines/Magazine_545x39_RPK_45rnd_Ball.et";
	static const string HELMET_SSH68_NET  = "{22963D69CA50EB9E}Prefabs/Characters/HeadGear/Helmet_SSh68_01/Helmet_SSh68_01_net.et";
	static const string HELMET_SSH68_CAMO = "{66196D85AB93D2BE}Prefabs/Characters/HeadGear/Helmet_SSh68_01/Helmet_SSh68_01_camo.et";
	static const string VEST_HARNESS    = "{08155E701A949620}Prefabs/Characters/Vests/Vest_SovietHarness/Variants/Vest_SovietHarness_rifleman.et";
	static const string VEST_LIFCHIK_GL = "{C8516078375CBE45}Prefabs/Characters/Vests/Vest_Lifchik/Vest_Lifchik_GL.et";
	static const string VEST_6B2        = "{ADE19B33DCBB9005}Prefabs/Characters/Vests/Vest_6B2/Vest_6B2.et";
	static const string JACKET_M88      = "{9F546CCA2582D16F}Prefabs/Characters/Uniforms/Jacket_M88.et";
	static const string PANTS_M88       = "{DCF980831E880F6A}Prefabs/Characters/Uniforms/Pants_M88.et";
	static const string BOOTS_SOVIET    = "{4C6029AB8BF5C044}Prefabs/Characters/Footwear/CombatBoots_Soviet_01_Dirty.et";
	static const string BACKPACK_RPG    = "{0D39750E5695B9D8}Prefabs/Items/Equipment/Backpacks/Backpack_RPG_Gunner.et";
	static const string TOURNIQUET_USSR = "{80E75A71C29190DB}Prefabs/Items/Medicine/Tourniquet_01/Tourniquet_USSR_01.et";

	//------------------------------------------------------------------------------------------------
	protected static void SetBase(TBD_KitInfo kit)
	{
		kit.m_sKitAlias = KIT_SOV_RIFLEMAN;
		kit.m_sBasePrefab = PREFAB_SOV_RIFLEMAN;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_SlotGearStruct NewLoadout(TBD_KitInfo kit)
	{
		kit.m_Loadout = new TBD_SlotLoadoutStruct();
		kit.m_Loadout.gear = new TBD_SlotGearStruct();
		kit.m_Loadout.cargo = {};
		return kit.m_Loadout.gear;
	}

	//------------------------------------------------------------------------------------------------
	protected static void AddCargo(TBD_KitInfo kit, string container, string item, int qty)
	{
		TBD_SlotCargoStruct row = new TBD_SlotCargoStruct();
		row.container = container;
		row.item = item;
		row.qty = qty;
		kit.m_Loadout.cargo.Insert(row);
	}

	//------------------------------------------------------------------------------------------------
	//! The full mockup kit: AK-74 + 1P29 in hand, RPG-7 slung, rocket pack, SSh-68, harness.
	protected static void DressRiflemanAt(TBD_KitInfo kit)
	{
		SetBase(kit);
		TBD_SlotGearStruct gear = NewLoadout(kit);
		gear.primary = AK74;
		gear.optic = OPTIC_1P29;
		gear.magazine = MAG_AK_30;
		gear.launcher = RPG7_PGO7;
		gear.throwable = RGD5;
		gear.helmet = HELMET_SSH68_NET;
		gear.vest = VEST_HARNESS;
		gear.uniform = JACKET_M88;
		gear.pants = PANTS_M88;
		gear.boots = BOOTS_SOVIET;
		gear.backpack = BACKPACK_RPG;
		AddCargo(kit, "vest", MAG_AK_30, 5);
		AddCargo(kit, "jacket", TOURNIQUET_USSR, 1);
	}

	//------------------------------------------------------------------------------------------------
	//! Partial loadout: weapon + head + vest authored, everything else is the kit prefab's own.
	protected static void DressRifleman(TBD_KitInfo kit)
	{
		SetBase(kit);
		TBD_SlotGearStruct gear = NewLoadout(kit);
		gear.primary = AK74N_1P29;
		gear.optic = OPTIC_PSO1;
		gear.magazine = MAG_RPK_45;
		gear.helmet = HELMET_SSH68_CAMO;
		gear.vest = VEST_LIFCHIK_GL;
	}

	//------------------------------------------------------------------------------------------------
	protected static void DressMedic(TBD_KitInfo kit)
	{
		SetBase(kit);
		TBD_SlotGearStruct gear = NewLoadout(kit);
		gear.primary = AKS74U;
		gear.vest = VEST_6B2;
		gear.uniform = JACKET_M88;
		gear.pants = PANTS_M88;
		gear.boots = BOOTS_SOVIET;
	}

	//------------------------------------------------------------------------------------------------
	//! Kit-only: no JSON loadout, the doll is the prefab as shipped. This is the cache-reset proof —
	//! picked after `rifleman_at`, every garment must fall back to the prefab's own.
	protected static void DressCrew(TBD_KitInfo kit)
	{
		SetBase(kit);
	}
}
