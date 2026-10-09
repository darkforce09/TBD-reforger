/**
 * @file TBD_PlayerInfo.c
 * @brief One connected player as the players panel shows them.
 *
 * Role: holds a player's name, faction key, ping, tag and state.
 * Position: TBD_PlayersMock creates it; TBD_PlayersCatalog holds it; TBD_PlayerLane renders it.
 * State: plain data, client.
 * Invariants: the faction key is empty for spectators and the unslotted.
 */

//! One connected player in the players catalog.
class TBD_PlayerInfo
{
	string m_sName; //!< display name
	string m_sFactionKey;  //!< "BLUFOR" / "OPFOR"; empty for spectators and the unslotted
	int m_iPing;           //!< ms
	string m_sTag;         //!< "ADMIN"; empty = no chip
	TBD_EPlayerState m_eState; //!< slotted, spectator or unslotted

	//! @param name display name
	//! @param factionKey "BLUFOR", "OPFOR" or empty
	//! @param ping round trip in ms
	//! @param state the player's state
	//! @param tag optional chip text such as "ADMIN"; empty = no chip
	void TBD_PlayerInfo(string name, string factionKey, int ping, TBD_EPlayerState state, string tag = "")
	{
		m_sName = name;
		m_sFactionKey = factionKey;
		m_iPing = ping;
		m_eState = state;
		m_sTag = tag;
	}
}
