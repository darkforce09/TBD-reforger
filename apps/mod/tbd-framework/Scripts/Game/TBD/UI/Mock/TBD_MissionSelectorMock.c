//! Pre-game rebuild (2026-09-12) — the mock catalog behind the Mission Selector.
//!
//! Dataset is the Stitch pre-game mockup (terrain_selector / scenario_browser /
//! mission_inspector): Everon · Arland · Kolguyev, three missions each so every filter has
//! something to do, the PVP Test 1 inspector (v2.14.99 LATEST, TBD CORE COMPETITIVE V1.8 with six
//! mods, BLUFOR Defending 24 vs OPFOR Attacking 24, two objectives a side), identity
//! `Mission Maker` / `ADMIN`, one player connected.
//!
//! Counts in the UI (`N AVAILABLE`, the Modes badge, per-mode checkbox counts) are COMPUTED from
//! this data by the screen, never typed in — the mockup's `30 Available` is a picture, the screen
//! is data-driven. Consumed only through `TBD_MissionCatalog.Get()`.
class TBD_MissionSelectorMock
{
	//------------------------------------------------------------------------------------------------
	static TBD_MissionCatalog Build()
	{
		TBD_MissionCatalog catalog = new TBD_MissionCatalog();

		catalog.m_Identity = new TBD_SessionIdentity("Mission Maker", "ADMIN", 1);

		catalog.m_aTerrains.Insert(new TBD_TerrainInfo("everon",   "Everon",   "water",     TBD_UILayouts.HERO_EVERON));
		catalog.m_aTerrains.Insert(new TBD_TerrainInfo("arland",   "Arland",   "landscape", TBD_UILayouts.HERO_TOPO));
		catalog.m_aTerrains.Insert(new TBD_TerrainInfo("kolguyev", "Kolguyev", "ac_unit",   TBD_UILayouts.HERO_TOPO));

		catalog.m_aModes.Insert(new TBD_MissionMode("COOP",     "COOP",     TBD_EUITint.SUCCESS));
		catalog.m_aModes.Insert(new TBD_MissionMode("WARLORDS", "Warlords", TBD_EUITint.TERTIARY));
		catalog.m_aModes.Insert(new TBD_MissionMode("PVP",      "PvP",      TBD_EUITint.PRIMARY));
		catalog.m_aModes.Insert(new TBD_MissionMode("RHS",      "RHS Mod",  TBD_EUITint.NEUTRAL));
		catalog.m_aModes.Insert(new TBD_MissionMode("ZEUS",     "Zeus",     TBD_EUITint.WARNING));

		// ── Everon ──────────────────────────────────────────────────────────────────────────
		TBD_MissionSummary pvp1 = AddMission("everon/pvp_test_1", "PVP Test 1", "PVP", "everon", 48, catalog);
		pvp1.m_sSummary = "BLUFOR mechanized infantry forces establish fortified defensive perimeters across Sector 1 and 2 to repel a coordinated dawn assault by OPFOR armored spearheads. Victory requires either holding all active control zones until extraction or completely eliminating opposing combatants.";
		TBD_MissionFactionSummary blu = AddFaction(pvp1, "BLUFOR", "Defending", 24, TBD_EUITint.BLUFOR);
		blu.AddVehicle("BMP-2", 2).AddVehicle("UAZ-3151", 1).AddVehicle("BRDM-2", 1);
		blu.AddObjective("Defend Sector 1", "shield").AddObjective("Defend Sector 2", "shield");
		TBD_MissionFactionSummary opf = AddFaction(pvp1, "OPFOR", "Attacking", 24, TBD_EUITint.OPFOR);
		opf.AddVehicle("BMP-2", 2).AddVehicle("UAZ-3151", 1).AddVehicle("BRDM-2", 1);
		opf.AddObjective("Capture Sector 1", "flag").AddObjective("Capture Sector 2", "flag");

		TBD_MissionSummary coop1 = AddMission("everon/coop_test_1", "Co-op Test 1", "COOP", "everon", 4, catalog);
		coop1.m_sSummary = "A four-man reconnaissance team infiltrates the Montignac ridge at dusk to mark artillery targets and exfiltrate before the garrison reacts.";
		TBD_MissionFactionSummary blu1 = AddFaction(coop1, "BLUFOR", "Attacking", 4, TBD_EUITint.BLUFOR);
		blu1.AddVehicle("UAZ-3151", 1);
		blu1.AddObjective("Mark the mortar pit", "flag");
		blu1.AddObjective("Reach the exfil boat", "flag");
		TBD_MissionFactionSummary opf2 = AddFaction(coop1, "OPFOR", "Defending", 0, TBD_EUITint.OPFOR);
		opf2.AddVehicle("BTR-70", 1);
		opf2.AddObjective("Hold Montignac", "shield");

		TBD_MissionSummary coop2 = AddMission("everon/coop_test_2", "Co-op Test 2", "COOP", "everon", 12, catalog);
		coop2.m_sSummary = "Platoon-minus assault on the Levie airfield: breach the perimeter, clear the tower, hold until the relief convoy arrives.";
		TBD_MissionFactionSummary blu3 = AddFaction(coop2, "BLUFOR", "Attacking", 12, TBD_EUITint.BLUFOR);
		blu3.AddVehicle("M151A2", 2);
		blu3.AddVehicle("M923A1", 1);
		blu3.AddObjective("Breach the airfield", "flag");
		blu3.AddObjective("Clear the tower", "flag");
		blu3.AddObjective("Hold for relief", "shield");
		TBD_MissionFactionSummary opf4 = AddFaction(coop2, "OPFOR", "Defending", 0, TBD_EUITint.OPFOR);
		opf4.AddVehicle("BRDM-2", 1);
		opf4.AddObjective("Hold Levie airfield", "shield");

		// ── Arland ──────────────────────────────────────────────────────────────────────────
		TBD_MissionSummary war = AddMission("arland/warlords_32", "Warlords 32 Arland", "WARLORDS", "arland", 32, catalog);
		war.m_sSummary = "Sector-control warfare across the whole island. Capture, spend, advance; the base that falls first loses.";
		TBD_MissionFactionSummary blu5 = AddFaction(war, "BLUFOR", "Attacking", 16, TBD_EUITint.BLUFOR);
		blu5.AddVehicle("M113", 2);
		blu5.AddVehicle("M151A2", 3);
		blu5.AddObjective("Take the harbour", "flag");
		blu5.AddObjective("Raid the OPFOR base", "flag");
		TBD_MissionFactionSummary opf6 = AddFaction(war, "OPFOR", "Attacking", 16, TBD_EUITint.OPFOR);
		opf6.AddVehicle("BTR-70", 2);
		opf6.AddVehicle("UAZ-3151", 3);
		opf6.AddObjective("Take the quarry", "flag");
		opf6.AddObjective("Raid the BLUFOR base", "flag");

		TBD_MissionSummary patrol = AddMission("arland/coop_patrol_10", "Co-op Arland Patrol", "COOP", "arland", 10, catalog);
		patrol.m_sSummary = "Two fireteams sweep the northern villages for a downed pilot while insurgent cells shadow the road net.";
		TBD_MissionFactionSummary blu7 = AddFaction(patrol, "BLUFOR", "Attacking", 10, TBD_EUITint.BLUFOR);
		blu7.AddVehicle("M998", 2);
		blu7.AddObjective("Locate the pilot", "flag");
		blu7.AddObjective("Escort to the LZ", "shield");
		TBD_MissionFactionSummary opf8 = AddFaction(patrol, "OPFOR", "Defending", 0, TBD_EUITint.OPFOR);
		opf8.AddVehicle("Ural-4320", 1);
		opf8.AddObjective("Deny the road net", "shield");

		TBD_MissionSummary zeus = AddMission("arland/zeus_sandbox_18", "Zeus Arland Sandbox", "ZEUS", "arland", 18, catalog);
		zeus.m_sSummary = "Curated game-master sandbox: two squads, one Zeus, objectives issued live.";
		TBD_MissionFactionSummary blu9 = AddFaction(zeus, "BLUFOR", "Attacking", 16, TBD_EUITint.BLUFOR);
		blu9.AddVehicle("M113", 1);
		blu9.AddVehicle("M151A2", 2);
		blu9.AddObjective("As tasked by Zeus", "flag");
		TBD_MissionFactionSummary opf10 = AddFaction(zeus, "OPFOR", "Defending", 2, TBD_EUITint.OPFOR);
		opf10.AddVehicle("BMP-1", 1);
		opf10.AddObjective("As tasked by Zeus", "shield");

		// ── Kolguyev ────────────────────────────────────────────────────────────────────────
		TBD_MissionSummary frontier = AddMission("kolguyev/pvp_frontier_40", "PVP Kolguyev Frontier", "PVP", "kolguyev", 40, catalog);
		frontier.m_sSummary = "Meeting engagement along the frozen river line. Three contested crossings, one hour, no respawns.";
		TBD_MissionFactionSummary blu11 = AddFaction(frontier, "BLUFOR", "Attacking", 20, TBD_EUITint.BLUFOR);
		blu11.AddVehicle("M113", 2);
		blu11.AddVehicle("M998", 2);
		blu11.AddObjective("Hold two crossings", "shield");
		TBD_MissionFactionSummary opf12 = AddFaction(frontier, "OPFOR", "Attacking", 20, TBD_EUITint.OPFOR);
		opf12.AddVehicle("BTR-70", 2);
		opf12.AddVehicle("UAZ-3151", 2);
		opf12.AddObjective("Hold two crossings", "shield");

		TBD_MissionSummary rhs = AddMission("kolguyev/rhs_assault_24", "RHS Kolguyev Assault", "RHS", "kolguyev", 24, catalog);
		rhs.m_sSummary = "Modern-kit mechanized push on the radar station. RHS AFRF versus RHS USAF.";
		TBD_MissionFactionSummary blu13 = AddFaction(rhs, "BLUFOR", "Attacking", 12, TBD_EUITint.BLUFOR);
		blu13.AddVehicle("M2A2", 2);
		blu13.AddObjective("Seize the radar", "flag");
		TBD_MissionFactionSummary opf14 = AddFaction(rhs, "OPFOR", "Defending", 12, TBD_EUITint.OPFOR);
		opf14.AddVehicle("BMP-2", 2);
		opf14.AddObjective("Hold the radar", "shield");

		TBD_MissionSummary recon = AddMission("kolguyev/coop_recon_8", "Co-op Kolguyev Recon", "COOP", "kolguyev", 8, catalog);
		recon.m_sSummary = "Night insertion by boat; photograph the coastal battery and get out unseen.";
		TBD_MissionFactionSummary blu15 = AddFaction(recon, "BLUFOR", "Attacking", 8, TBD_EUITint.BLUFOR);
		blu15.AddVehicle("RHIB", 2);
		blu15.AddObjective("Photograph the battery", "flag");
		blu15.AddObjective("Exfil unseen", "shield");
		TBD_MissionFactionSummary opf16 = AddFaction(recon, "OPFOR", "Defending", 0, TBD_EUITint.OPFOR);
		opf16.AddVehicle("UAZ-3151", 2);
		opf16.AddObjective("Guard the battery", "shield");

		return catalog;
	}

	// ── Builders ────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Every mock mission shares the inspector's version ladder and modset; the screen shows the
	//! same picture the mockup does whichever card is picked.
	protected static TBD_MissionSummary AddMission(string id, string title, string tag, string terrain, int slots, TBD_MissionCatalog catalog)
	{
		TBD_MissionSummary mission = new TBD_MissionSummary(id, title, tag, terrain, slots, "Bohemia Interactive");
		mission.m_sModsetName = "TBD CORE COMPETITIVE V1.8";

		mission.m_aVersions.Insert(new TBD_MissionVersion("v2.14.99", "LATEST", "Release build"));
		mission.m_aVersions.Insert(new TBD_MissionVersion("v2.14.02", "STABLE", "48 slots balance patch"));
		mission.m_aVersions.Insert(new TBD_MissionVersion("v2.13.80", "",       "Pre-framework update"));
		mission.m_aVersions.Insert(new TBD_MissionVersion("v2.12.10", "",       "Legacy staging build"));

		mission.m_aMods.Insert(new TBD_MissionMod("@CRF_Framework", "v2.4.0"));
		mission.m_aMods.Insert(new TBD_MissionMod("@RHSAFRF",       "v0.6.1"));
		mission.m_aMods.Insert(new TBD_MissionMod("@RHSUSAF",       "v0.6.1"));
		mission.m_aMods.Insert(new TBD_MissionMod("@ACE3_Reforger", "v1.8.2"));
		mission.m_aMods.Insert(new TBD_MissionMod("@TFAR_Enfusion", "v1.2.0"));
		mission.m_aMods.Insert(new TBD_MissionMod("@CBA_Reforger",  "v3.15.0"));

		catalog.m_aMissions.Insert(mission);
		return mission;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_MissionFactionSummary AddFaction(TBD_MissionSummary mission, string key, string role, int slots, TBD_EUITint tint)
	{
		TBD_MissionFactionSummary faction = new TBD_MissionFactionSummary(key, role, slots, tint);
		mission.m_aFactions.Insert(faction);
		return faction;
	}
}
