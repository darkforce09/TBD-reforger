//! Mock data models and static repository for the TBD Mission Selector UI.
//! Faithfully models the Stitch Google mockup dataset for 1:1 parity without live wire dependencies.

class TBD_MockTerrain
{
	string m_sName;
	int m_iCount;
	string m_sBadge;

	void TBD_MockTerrain(string name, int count, string badge = "")
	{
		m_sName = name;
		m_iCount = count;
		m_sBadge = badge;
	}
}

class TBD_MockMission
{
	string m_sTitle;
	string m_sTag;
	int m_iSlots;
	string m_sPlayersRange;
	string m_sSubtitle;
	string m_sRespawnRule;
	string m_sDifficulty;
	string m_sAiSkill;
	string m_sSynopsis;
	string m_sTerrain;
	string m_sAuthor;
	string m_sVersion;
	string m_sSystem;
	string m_sFastTravel;
	string m_sRedeployment;
	ref array<string> m_aRequiredMods;
	ref array<string> m_aModSizes;

	void TBD_MockMission(string title, string tag, int slots, string playersRange, string subtitle, string respawnRule, string difficulty, string aiSkill, string synopsis, string terrain = "Altis")
	{
		m_sTitle = title;
		m_sTag = tag;
		m_iSlots = slots;
		m_sPlayersRange = playersRange;
		m_sSubtitle = subtitle;
		m_sRespawnRule = respawnRule;
		m_sDifficulty = difficulty;
		m_sAiSkill = aiSkill;
		m_sSynopsis = synopsis;
		m_sTerrain = terrain;
		m_sAuthor = "by Bohemia Interactive";
		m_sVersion = "VERSION 2.14.99";
		m_sSystem = "CP Capture & Spend";
		m_sFastTravel = "Conquered Sectors";
		m_sRedeployment = "FOB / Base Node";

		m_aRequiredMods = {};
		m_aModSizes = {};

		m_aRequiredMods.Insert("ACE3 Reforger Core  v1.8.2 - Advanced Ballistics & Medical");
		m_aModSizes.Insert("2.4 GB");

		m_aRequiredMods.Insert("RHS: Status Quo (AFRF & USAF)  v0.6.1 - Modern Weapons");
		m_aModSizes.Insert("7.1 GB");

		m_aRequiredMods.Insert("TFAR - Tactical Radio Comms  v1.2 - Long-Range Audio");
		m_aModSizes.Insert("420 MB");

		m_aRequiredMods.Insert("CBA_A3 NextGen Framework  v3.15 - Community Base Addons");
		m_aModSizes.Insert("180 MB");

		m_aRequiredMods.Insert("Project Reality Altis HD Assets - Structures, Fortifications");
		m_aModSizes.Insert("4.7 GB");
	}
}

class TBD_MissionSelectorData
{
	//------------------------------------------------------------------------------------------------
	//! Build the complete list of 15 terrains featured in the Stitch mockup.
	static ref array<ref TBD_MockTerrain> GetMockTerrains()
	{
		ref array<ref TBD_MockTerrain> list = {};
		list.Insert(new TBD_MockTerrain("!Virtual Reality", 1, "DEV"));
		list.Insert(new TBD_MockTerrain("Altis", 30));
		list.Insert(new TBD_MockTerrain("Anizay", 8));
		list.Insert(new TBD_MockTerrain("Beketov", 14));
		list.Insert(new TBD_MockTerrain("Bukovina", 6));
		list.Insert(new TBD_MockTerrain("Chernarus", 42));
		list.Insert(new TBD_MockTerrain("Everon", 27));
		list.Insert(new TBD_MockTerrain("Gabreta", 9));
		list.Insert(new TBD_MockTerrain("Kolguyev", 11));
		list.Insert(new TBD_MockTerrain("Livonia", 23));
		list.Insert(new TBD_MockTerrain("Malden", 18));
		list.Insert(new TBD_MockTerrain("Sahrani", 15));
		list.Insert(new TBD_MockTerrain("Takistan", 22));
		list.Insert(new TBD_MockTerrain("Tanoa", 19));
		list.Insert(new TBD_MockTerrain("Zargabad", 12));
		return list;
	}

	//------------------------------------------------------------------------------------------------
	//! Build the complete library of 9 Altis missions from the Stitch mockup.
	static ref array<ref TBD_MockMission> GetMockMissions()
	{
		ref array<ref TBD_MockMission> list = {};

		list.Insert(new TBD_MockMission(
			"SC 48 Warlords (Whole Island)",
			"WARLORDS",
			48,
			"1 - 48",
			"Whole Island Domination - Altis",
			"BASE ONLY",
			"REGULAR",
			"STANDARD / 3P ON",
			"Capture sectors. Through them, advance to the enemy base and raid it. Parameters allow you to change various rules within the mission, including initial Command Points, AI garrisons, vehicle availability, fast travel routes, and automated logistical drops."
		));

		list.Insert(new TBD_MockMission(
			"COOP 04 Firing From Vehicles",
			"COOP",
			4,
			"1 - 4",
			"Altis Proving Grounds",
			"RESPAWN TENT",
			"REGULAR",
			"STANDARD / 3P ON",
			"Engage fast-moving hostile technicals and motorized reconnaissance patrols from mounted transport vehicles across western Altis."
		));

		list.Insert(new TBD_MockMission(
			"COOP 12 Combat Patrol",
			"COOP",
			12,
			"1 - 12",
			"Randomized Tactical Objectives",
			"FOB",
			"REGULAR",
			"HARDENED / 3P OFF",
			"Cooperative dynamic sandbox patrol against guerrilla garrisons occupying military outposts with procedural objectives."
		));

		list.Insert(new TBD_MockMission(
			"End Game 16 Kavala",
			"PvP",
			16,
			"2 - 16",
			"Kavala Coastal Sector",
			"RESPAWN POINT",
			"REGULAR",
			"VETERAN / 3P ON",
			"Two opposing teams race to locate intel, secure key blueprints, and upload data at the central Kavala communications facility."
		));

		list.Insert(new TBD_MockMission(
			"Escape 10 Altis",
			"COOP",
			10,
			"1 - 10",
			"Wilderness Survival & Evasion",
			"DISABLED",
			"VETERAN",
			"EXPERT / 1P ONLY",
			"POWs stranded without gear must scavenge equipment, evade high-threat patrols, and secure an extraction boat or helicopter."
		));

		list.Insert(new TBD_MockMission(
			"RHS BECTI 32 - Altis",
			"RHS",
			32,
			"4 - 32",
			"Combined Arms Warfare & Base Building",
			"MHQ / BASE",
			"REGULAR",
			"STANDARD / 3P ON",
			"Full-scale combined arms warfare with base building, factory logistics, resource procurement, and modern US / Russian weapon systems."
		));

		list.Insert(new TBD_MockMission(
			"Support 04 Rodopoli",
			"COOP",
			4,
			"1 - 4",
			"CAS Flight & Logistics Relay",
			"AIRFIELD",
			"REGULAR",
			"STANDARD / 3P ON",
			"Provide close air support, medical evacuation, and artillery relay for allied AI ground platoons under heavy siege."
		));

		list.Insert(new TBD_MockMission(
			"Vanguard 50 Syrta",
			"PvP",
			50,
			"10 - 50",
			"Shrinking Zone Sector Defense",
			"WAVE RESPAWN",
			"REGULAR",
			"STANDARD / 3P ON",
			"High-intensity competitive infantry assault across shrinking tactical zones centered around the fortified Syrta compound."
		));

		list.Insert(new TBD_MockMission(
			"Zeus 16+2 Master Altis (NATO)",
			"ZEUS",
			18,
			"2 - 18",
			"Curated Sandbox Operations",
			"ZEUS DISCRETION",
			"REGULAR",
			"CUSTOM / 3P ON",
			"Expanded dual-Zeus command environment supervising multi-squad tactical NATO combined arms operations across Altis."
		));

		return list;
	}
}
