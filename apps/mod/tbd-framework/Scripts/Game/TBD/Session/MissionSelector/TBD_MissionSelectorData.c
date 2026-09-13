//! Pre-game rebuild (2026-09-12) — what the Mission Selector screen reads. Plain models plus the
//! one read surface (`TBD_MissionCatalog`) the screen talks to.
//!
//! Role (UI_STRUCTURE.md table): `TBD_XData.c`, runs on both sides, holds no wire code. Today the
//! catalog is filled by `UI/Mock/TBD_MissionSelectorMock.c`; when the mission library reaches the
//! wire, a `TBD_MissionSelectorClient` hands `TBD_MissionCatalog.Set()` a real one and nothing in
//! `UI/` changes. That swap point is the reason this file exists apart from the mock.
//!
//! Field names mirror the Stitch inspector top-to-bottom: hero (title, tag, author, versions),
//! REQUIRED MODSET & MODS, MISSION SUMMARY, ORBAT OVERVIEW (per faction: role, slots, vehicles),
//! OBJECTIVES (per faction).

//! One entry of the TERRAINS list.
class TBD_TerrainInfo
{
	string m_sKey;           //!< "everon" — matches TBD_MissionSummary.m_sTerrainKey
	string m_sName;          //!< "Everon"
	string m_sIcon;          //!< TBD_UIIcons key ("water", "landscape", "ac_unit")
	//! Inspector hero texture (TBD_UILayouts.HERO_*). Empty = the topo-grid art.
	ResourceName m_sHeroImage;

	void TBD_TerrainInfo(string key, string name, string icon, ResourceName heroImage = "")
	{
		m_sKey = key;
		m_sName = name;
		m_sIcon = icon;
		m_sHeroImage = heroImage;
	}
}

//! One mode / tag a mission carries and the browser filters on (COOP, PvP, Warlords, RHS, Zeus).
class TBD_MissionMode
{
	string m_sKey;           //!< "PVP" — matches TBD_MissionSummary.m_sTag
	string m_sLabel;         //!< "PvP"
	TBD_EUITint m_eTint;     //!< chip colour on the card

	void TBD_MissionMode(string key, string label, TBD_EUITint tint)
	{
		m_sKey = key;
		m_sLabel = label;
		m_eTint = tint;
	}
}

//! One selectable build of a mission.
class TBD_MissionVersion
{
	string m_sLabel;         //!< "v2.14.99"
	string m_sBadge;         //!< "LATEST", "STABLE", "" — drives the badge chip
	string m_sNote;          //!< "Release build"

	void TBD_MissionVersion(string label, string badge, string note)
	{
		m_sLabel = label;
		m_sBadge = badge;
		m_sNote = note;
	}
}

//! One required addon.
class TBD_MissionMod
{
	string m_sName;          //!< "@CRF_Framework"
	string m_sVersion;       //!< "v2.4.0"
	bool m_bSynced = true;   //!< client has it

	void TBD_MissionMod(string name, string version, bool synced = true)
	{
		m_sName = name;
		m_sVersion = version;
		m_bSynced = synced;
	}
}

//! A counted asset line in a faction's ORBAT column ("2x  BMP-2").
class TBD_MissionAsset
{
	string m_sName;
	int m_iCount;

	void TBD_MissionAsset(string name, int count)
	{
		m_sName = name;
		m_iCount = count;
	}
}

//! One objective line in a faction's OBJECTIVES column.
class TBD_MissionObjective
{
	string m_sTitle;         //!< "Defend Sector 1"
	string m_sIcon;          //!< "shield" / "flag"

	void TBD_MissionObjective(string title, string icon)
	{
		m_sTitle = title;
		m_sIcon = icon;
	}
}

//! Per-faction slice of a mission: header (tag, role, slots) + vehicles + objectives.
class TBD_MissionFactionSummary
{
	string m_sKey;           //!< "BLUFOR"
	string m_sRole;          //!< "Defending" / "Attacking"
	int m_iSlots;
	TBD_EUITint m_eTint;     //!< BLUFOR / OPFOR
	ref array<ref TBD_MissionAsset> m_aVehicles;
	ref array<ref TBD_MissionObjective> m_aObjectives;

	void TBD_MissionFactionSummary(string key, string role, int slots, TBD_EUITint tint)
	{
		m_sKey = key;
		m_sRole = role;
		m_iSlots = slots;
		m_eTint = tint;
		m_aVehicles = {};
		m_aObjectives = {};
	}

	TBD_MissionFactionSummary AddVehicle(string name, int count)
	{
		m_aVehicles.Insert(new TBD_MissionAsset(name, count));
		return this;
	}

	TBD_MissionFactionSummary AddObjective(string title, string icon)
	{
		m_aObjectives.Insert(new TBD_MissionObjective(title, icon));
		return this;
	}
}

//! Everything the browser card and the inspector show for one mission.
class TBD_MissionSummary
{
	string m_sId;            //!< stable id, also the catalog tag ("everon/pvp_test_1")
	string m_sTitle;         //!< "PVP Test 1"
	string m_sTag;           //!< mode key: "PVP", "COOP", …
	string m_sTerrainKey;    //!< "everon"
	int m_iSlots;            //!< 48
	string m_sAuthor;        //!< "Bohemia Interactive"
	string m_sModsetName;    //!< "TBD CORE COMPETITIVE V1.8"
	string m_sSummary;       //!< SITREP paragraph
	ref array<ref TBD_MissionVersion> m_aVersions;
	ref array<ref TBD_MissionMod> m_aMods;
	ref array<ref TBD_MissionFactionSummary> m_aFactions;

	void TBD_MissionSummary(string id, string title, string tag, string terrainKey, int slots, string author)
	{
		m_sId = id;
		m_sTitle = title;
		m_sTag = tag;
		m_sTerrainKey = terrainKey;
		m_iSlots = slots;
		m_sAuthor = author;
		m_aVersions = {};
		m_aMods = {};
		m_aFactions = {};
	}

	//! Total slots across factions, or m_iSlots when no faction data is attached.
	int GetSlotTotal()
	{
		if (m_aFactions.IsEmpty())
			return m_iSlots;

		int total;
		foreach (TBD_MissionFactionSummary faction : m_aFactions)
		{
			total += faction.m_iSlots;
		}

		return total;
	}

	int GetObjectiveTotal()
	{
		int total;
		foreach (TBD_MissionFactionSummary faction : m_aFactions)
		{
			total += faction.m_aObjectives.Count();
		}

		return total;
	}

	int GetSyncedModCount()
	{
		int synced;
		foreach (TBD_MissionMod mod : m_aMods)
		{
			if (mod.m_bSynced)
				synced++;
		}

		return synced;
	}
}

//! What the player picked in the Scenario Browser, for the screens after it (the lobby's mono
//! title). Static because the selector screen is gone by the time the lobby reads it.
class TBD_SessionSelection
{
	static string s_sMissionId;
	static string s_sTitle;
	static string s_sTerrainKey;
	static string s_sVersionLabel;

	static void Set(TBD_MissionSummary mission, string versionLabel)
	{
		if (!mission)
			return;

		s_sMissionId = mission.m_sId;
		s_sTitle = mission.m_sTitle;
		s_sTerrainKey = mission.m_sTerrainKey;
		s_sVersionLabel = versionLabel;
	}
}

//! The read surface. Screens hold one of these and never a raw array, so the mock -> client swap
//! is invisible to them.
class TBD_MissionCatalog
{
	protected static ref TBD_MissionCatalog s_Instance;

	ref array<ref TBD_TerrainInfo> m_aTerrains;
	ref array<ref TBD_MissionMode> m_aModes;
	ref array<ref TBD_MissionSummary> m_aMissions;
	ref TBD_SessionIdentity m_Identity;

	//------------------------------------------------------------------------------------------------
	void TBD_MissionCatalog()
	{
		m_aTerrains = {};
		m_aModes = {};
		m_aMissions = {};
	}

	//------------------------------------------------------------------------------------------------
	//! The catalog in force. Mock until a client cache calls Set().
	static TBD_MissionCatalog Get()
	{
		if (!s_Instance)
			s_Instance = TBD_MissionSelectorMock.Build();

		return s_Instance;
	}

	//------------------------------------------------------------------------------------------------
	//! Replace the catalog (a live client cache, or a test fixture). Null restores the mock on the
	//! next Get().
	static void Set(TBD_MissionCatalog catalog)
	{
		s_Instance = catalog;
	}

	// ── Reads ───────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	array<ref TBD_TerrainInfo> GetTerrains()
	{
		return m_aTerrains;
	}

	//------------------------------------------------------------------------------------------------
	//! Terrain by key, null when unknown.
	TBD_TerrainInfo GetTerrain(string key)
	{
		foreach (TBD_TerrainInfo terrain : m_aTerrains)
		{
			if (terrain.m_sKey == key)
				return terrain;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	array<ref TBD_MissionMode> GetModes()
	{
		return m_aModes;
	}

	//------------------------------------------------------------------------------------------------
	TBD_SessionIdentity GetIdentity()
	{
		return m_Identity;
	}

	//------------------------------------------------------------------------------------------------
	//! Missions on one terrain, in catalog order. Returns the count appended.
	int GetMissions(string terrainKey, notnull array<TBD_MissionSummary> outMissions)
	{
		int added;
		foreach (TBD_MissionSummary mission : m_aMissions)
		{
			if (mission.m_sTerrainKey != terrainKey)
				continue;

			outMissions.Insert(mission);
			added++;
		}

		return added;
	}

	//------------------------------------------------------------------------------------------------
	int CountMissions(string terrainKey)
	{
		int count;
		foreach (TBD_MissionSummary mission : m_aMissions)
		{
			if (mission.m_sTerrainKey == terrainKey)
				count++;
		}

		return count;
	}

	//------------------------------------------------------------------------------------------------
	//! How many missions on `terrainKey` carry `modeKey` — the count beside each Modes checkbox.
	int CountMode(string terrainKey, string modeKey)
	{
		int count;
		foreach (TBD_MissionSummary mission : m_aMissions)
		{
			if (mission.m_sTerrainKey == terrainKey && mission.m_sTag == modeKey)
				count++;
		}

		return count;
	}

	//------------------------------------------------------------------------------------------------
	TBD_MissionSummary FindMission(string id)
	{
		foreach (TBD_MissionSummary mission : m_aMissions)
		{
			if (mission.m_sId == id)
				return mission;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	TBD_TerrainInfo FindTerrain(string key)
	{
		foreach (TBD_TerrainInfo terrain : m_aTerrains)
		{
			if (terrain.m_sKey == key)
				return terrain;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	TBD_MissionMode FindMode(string key)
	{
		foreach (TBD_MissionMode mode : m_aModes)
		{
			if (mode.m_sKey == key)
				return mode;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! Tint for a mode key, NEUTRAL when the mode is unknown.
	TBD_EUITint ModeTint(string key)
	{
		TBD_MissionMode mode = FindMode(key);
		if (!mode)
			return TBD_EUITint.NEUTRAL;

		return mode.m_eTint;
	}

	//------------------------------------------------------------------------------------------------
	//! Display label for a mode key, the key itself when unknown.
	string ModeLabel(string key)
	{
		TBD_MissionMode mode = FindMode(key);
		if (!mode)
			return key;

		return mode.m_sLabel;
	}
}
