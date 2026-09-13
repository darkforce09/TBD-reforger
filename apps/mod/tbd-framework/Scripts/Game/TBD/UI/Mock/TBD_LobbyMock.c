//! Pre-game rebuild (2026-09-13) — the mock catalog behind the Lobby.
//!
//! Dataset is the Stitch pre-game mockup (lobby_sidebar / orbat_panel_blufor /
//! slot_kit_inspector): BLUFOR Defending (92 seats) vs OPFOR Attacking (95), Spectators (10);
//! BLUFOR squads Alpha 1-1 UAZ-3151, Alpha 2-1 BMP-2, Alpha 2-2 BTR-80, Bravo 1-1 T-72B with the
//! drawn roles, weapons, tags and holders; OPFOR mirrors them under Soviet callsigns. Three kits
//! (`rifleman_at` = the mockup's `8: RIFLEMAN (AT)` in full, `medic`, `crew`) keyed by role.
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
}
