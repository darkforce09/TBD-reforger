/**
 * @file TBD_PlayersCatalog.c
 * @brief Who is connected, shaped for the briefing's PLAYERS panel and the slotted-count badges.
 *
 * Role: holds the players and the seat capacity per faction and answers totals, capacities, the
 * slotted players of a side and the players in a state.
 * Position: screens read Get(); TBD_PlayersMock builds the catalog on first use; Set() replaces it,
 * the seam for a live source (the player manager and TBD_SpawnManager's slot map exist only on the
 * server, so a live source needs an owner-scoped RPC).
 * State: the process-wide instance, and per instance the player list and capacity map; client.
 * Invariants: screens read player data only through Get(); lists keep roster order.
 */

//! The players catalog. One process-wide instance.
class TBD_PlayersCatalog
{
	protected static ref TBD_PlayersCatalog s_Instance; //!< the catalog Get returns; built from the mock on first use

	ref array<ref TBD_PlayerInfo> m_aPlayers; //!< every connected player, roster order
	ref map<string, int> m_mCapacity; //!< faction key -> seats

	//! An empty catalog.
	void TBD_PlayersCatalog()
	{
		m_aPlayers = {};
		m_mCapacity = new map<string, int>();
	}

	//! @return the catalog, built from TBD_PlayersMock on first use
	static TBD_PlayersCatalog Get()
	{
		if (!s_Instance)
			s_Instance = TBD_PlayersMock.Build();

		return s_Instance;
	}

	//! Replace the catalog every screen reads.
	//! @param catalog the new catalog
	static void Set(TBD_PlayersCatalog catalog)
	{
		s_Instance = catalog;
	}


	//! @return the number of connected players
	int Total()
	{
		return m_aPlayers.Count();
	}

	//! @param factionKey "BLUFOR" or "OPFOR"
	//! @return the seats on that side, 0 when unknown
	int Capacity(string factionKey)
	{
		int seats;
		if (m_mCapacity.Find(factionKey, seats))
			return seats;

		return 0;
	}

	//! Everyone slotted on a side, roster order.
	//! @param factionKey "BLUFOR" or "OPFOR"
	//! @return a new list
	array<TBD_PlayerInfo> GetSlotted(string factionKey)
	{
		array<TBD_PlayerInfo> lane = {};
		foreach (TBD_PlayerInfo player : m_aPlayers)
		{
			if (player.m_eState == TBD_EPlayerState.SLOTTED && player.m_sFactionKey == factionKey)
				lane.Insert(player);
		}

		return lane;
	}

	//! Everyone in a state, roster order.
	//! @param state the state to select
	//! @return a new list
	array<TBD_PlayerInfo> GetByState(TBD_EPlayerState state)
	{
		array<TBD_PlayerInfo> lane = {};
		foreach (TBD_PlayerInfo player : m_aPlayers)
		{
			if (player.m_eState == state)
				lane.Insert(player);
		}

		return lane;
	}

	//! @param factionKey "BLUFOR" or "OPFOR"
	//! @return the number slotted on that side
	int CountSlotted(string factionKey)
	{
		return GetSlotted(factionKey).Count();
	}

	//! @return the number slotted on every side
	int CountSlottedAll()
	{
		return GetByState(TBD_EPlayerState.SLOTTED).Count();
	}

	//! @return the seats on every side
	int CapacityAll()
	{
		int total;
		foreach (string key, int seats : m_mCapacity)
		{
			total += seats;
		}

		return total;
	}
}
