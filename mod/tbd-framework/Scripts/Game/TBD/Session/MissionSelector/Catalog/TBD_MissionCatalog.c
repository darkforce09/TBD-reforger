/**
 * @file TBD_MissionCatalog.c
 * @brief The Mission Selector's one read surface: terrains, modes, missions and the session identity.
 *
 * Role: holds the catalog in force and answers the screens' lookups and counts.  Position: Get
 * builds it from TBD_MissionSelectorMock until Set installs another (a client cache or a test
 * fixture); TBD_MissionSelectorScreen and its three panels read it. Runs on both sides and holds no
 * wire code.
 * State: the static catalog in force, created on first Get.  Invariants: the arrays are never
 * null; lookups return null (or NEUTRAL, or the key itself) for unknown keys; mission order is
 * catalog order.
 */

//! The catalog the screens read. Screens hold one of these and never a raw array, so replacing the
//! mock changes nothing in them.
class TBD_MissionCatalog
{
	protected static ref TBD_MissionCatalog s_Instance; //!< the catalog in force; null until the first Get or Set

	ref array<ref TBD_TerrainInfo> m_aTerrains; //!< TERRAINS list, in display order
	ref array<ref TBD_MissionMode> m_aModes; //!< mode tags, in filter order
	ref array<ref TBD_MissionSummary> m_aMissions; //!< every mission, in catalog order
	ref TBD_SessionIdentity m_Identity; //!< the session identity the top bar shows; may be null

	//! An empty catalog.
	void TBD_MissionCatalog()
	{
		m_aTerrains = {};
		m_aModes = {};
		m_aMissions = {};
	}

	//! @return the catalog in force; the mock until Set installs another
	static TBD_MissionCatalog Get()
	{
		if (!s_Instance)
			s_Instance = TBD_MissionSelectorMock.Build();

		return s_Instance;
	}

	//! Replace the catalog in force (a client cache or a test fixture).
	//! @param catalog the new catalog; null restores the mock on the next Get
	static void Set(TBD_MissionCatalog catalog)
	{
		s_Instance = catalog;
	}

	//! @return the terrains, in display order
	array<ref TBD_TerrainInfo> GetTerrains()
	{
		return m_aTerrains;
	}

	//! @return the terrain with key `key`, or null when unknown
	TBD_TerrainInfo GetTerrain(string key)
	{
		foreach (TBD_TerrainInfo terrain : m_aTerrains)
		{
			if (terrain.m_sKey == key)
				return terrain;
		}

		return null;
	}

	//! @return the mode tags, in filter order
	array<ref TBD_MissionMode> GetModes()
	{
		return m_aModes;
	}

	//! @return the session identity, or null
	TBD_SessionIdentity GetIdentity()
	{
		return m_Identity;
	}

	//! Append the missions on one terrain, in catalog order.
	//! @param terrainKey the terrain
	//! @param outMissions receives the missions
	//! @return how many were appended
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

	//! @return how many missions are on terrain `terrainKey`
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

	//! @return how many missions on `terrainKey` carry `modeKey`, the count beside each Modes checkbox
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

	//! @return the mission with id `id`, or null
	TBD_MissionSummary FindMission(string id)
	{
		foreach (TBD_MissionSummary mission : m_aMissions)
		{
			if (mission.m_sId == id)
				return mission;
		}

		return null;
	}

	//! @return the terrain with key `key`, or null
	TBD_TerrainInfo FindTerrain(string key)
	{
		foreach (TBD_TerrainInfo terrain : m_aTerrains)
		{
			if (terrain.m_sKey == key)
				return terrain;
		}

		return null;
	}

	//! @return the mode with key `key`, or null
	TBD_MissionMode FindMode(string key)
	{
		foreach (TBD_MissionMode mode : m_aModes)
		{
			if (mode.m_sKey == key)
				return mode;
		}

		return null;
	}

	//! @return the tint of mode `key`, NEUTRAL when the mode is unknown
	TBD_EUITint ModeTint(string key)
	{
		TBD_MissionMode mode = FindMode(key);
		if (!mode)
			return TBD_EUITint.NEUTRAL;

		return mode.m_eTint;
	}

	//! @return the display label of mode `key`, the key itself when unknown
	string ModeLabel(string key)
	{
		TBD_MissionMode mode = FindMode(key);
		if (!mode)
			return key;

		return mode.m_sLabel;
	}
}
