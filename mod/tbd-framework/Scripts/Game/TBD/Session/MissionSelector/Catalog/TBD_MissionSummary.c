/**
 * @file TBD_MissionSummary.c
 * @brief Everything the scenario browser card and the mission inspector show for one mission.
 *
 * Role: one mission's hero, modset, summary and per-faction rows, with its versions and required
 * mods.  Position: TBD_MissionSelectorMock builds them into TBD_MissionCatalog; the browser cards,
 * TBD_MissionInspectorPanel, TBD_MissionInspectorCards and TBD_SessionSelection read them.
 * State: none beyond its fields.  Invariants: the arrays are never null; GetSlotTotal falls back to
 * m_iSlots only when no faction rows are attached.
 */

//! Everything the browser card and the inspector show for one mission.
class TBD_MissionSummary
{
	string m_sId;            //!< stable id, also the catalog tag ("everon/pvp_test_1")
	string m_sTitle;         //!< "PVP Test 1"
	string m_sTag;           //!< mode key: "PVP", "COOP", ...
	string m_sTerrainKey;    //!< "everon"
	int m_iSlots;            //!< 48
	string m_sAuthor;        //!< "Bohemia Interactive"
	string m_sModsetName;    //!< "TBD CORE COMPETITIVE V1.8"
	string m_sSummary;       //!< SITREP paragraph
	ref array<ref TBD_MissionVersion> m_aVersions; //!< selectable builds, newest first
	ref array<ref TBD_MissionMod> m_aMods; //!< required addons
	ref array<ref TBD_MissionFactionSummary> m_aFactions; //!< per-faction rows

	//! Build a summary with empty version, mod and faction lists.
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

	//! @return total slots across factions, or m_iSlots when no faction rows are attached
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

	//! @return objectives across all factions
	int GetObjectiveTotal()
	{
		int total;
		foreach (TBD_MissionFactionSummary faction : m_aFactions)
		{
			total += faction.m_aObjectives.Count();
		}

		return total;
	}

	//! @return required mods the client has
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

//! One selectable build of a mission.
class TBD_MissionVersion
{
	string m_sLabel;         //!< "v2.14.99"
	string m_sBadge;         //!< "LATEST", "STABLE", "" -- drives the badge chip
	string m_sNote;          //!< "Release build"

	//! Build a version entry.
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

	//! Build a required-mod entry.
	void TBD_MissionMod(string name, string version, bool synced = true)
	{
		m_sName = name;
		m_sVersion = version;
		m_bSynced = synced;
	}
}
