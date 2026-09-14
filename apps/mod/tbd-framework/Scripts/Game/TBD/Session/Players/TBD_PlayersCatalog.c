//! Briefing pass (2026-09-14) — who is connected, shaped for the PLAYERS modal and the nav badges.
//!
//! `UI_STRUCTURE.md` reserved this module for shared player data. Presentation catalog, mock until
//! an adapter fills it from the server (player manager + `TBD_SpawnManager` slot map, both
//! authority-side today — same wall as the lobby roster, same answer: one owner-scoped RPC).
//! Screens read `TBD_PlayersCatalog.Get()` only; the mock lives in `UI/Mock/TBD_PlayersMock.c`.
enum TBD_EPlayerState
{
	SLOTTED,
	SPECTATOR,
	UNSLOTTED
}

class TBD_PlayerInfo
{
	string m_sName;
	string m_sFactionKey;  //!< "BLUFOR" / "OPFOR"; empty for spectators and the unslotted
	int m_iPing;           //!< ms
	string m_sTag;         //!< "ADMIN"; empty = no chip
	TBD_EPlayerState m_eState;

	void TBD_PlayerInfo(string name, string factionKey, int ping, TBD_EPlayerState state, string tag = "")
	{
		m_sName = name;
		m_sFactionKey = factionKey;
		m_iPing = ping;
		m_eState = state;
		m_sTag = tag;
	}
}

class TBD_PlayersCatalog
{
	protected static ref TBD_PlayersCatalog s_Instance;

	ref array<ref TBD_PlayerInfo> m_aPlayers;
	ref map<string, int> m_mCapacity; //!< faction key -> seats

	//------------------------------------------------------------------------------------------------
	void TBD_PlayersCatalog()
	{
		m_aPlayers = {};
		m_mCapacity = new map<string, int>();
	}

	static TBD_PlayersCatalog Get()
	{
		if (!s_Instance)
			s_Instance = TBD_PlayersMock.Build();

		return s_Instance;
	}

	static void Set(TBD_PlayersCatalog catalog)
	{
		s_Instance = catalog;
	}

	// ── Reads ───────────────────────────────────────────────────────────────────────────────

	int Total()
	{
		return m_aPlayers.Count();
	}

	int Capacity(string factionKey)
	{
		int seats;
		if (m_mCapacity.Find(factionKey, seats))
			return seats;

		return 0;
	}

	//! Everyone slotted on a side, roster order.
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

	int CountSlotted(string factionKey)
	{
		return GetSlotted(factionKey).Count();
	}

	int CountSlottedAll()
	{
		return GetByState(TBD_EPlayerState.SLOTTED).Count();
	}

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
