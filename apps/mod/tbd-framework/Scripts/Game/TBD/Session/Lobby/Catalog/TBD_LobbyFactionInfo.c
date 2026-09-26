/**
 * @file TBD_LobbyFactionInfo.c
 * @brief One row of the Lobby FACTIONS column: name, role label, tint and seat count.
 *
 * Role: the presentation of one side, or of the spectators.  Position: built by TBD_LobbyMock into
 * TBD_LobbyCatalog; read by TBD_LobbyFactionPanel and TBD_PlayersPanel.
 * State: plain data.  Invariants: m_sKey matches TBD_LobbySide.m_sKey; an empty role label shows no
 * chip; m_bSpectators rows go to the spectator dock.
 */

//! One faction row, or the Spectators row when m_bSpectators is set.
class TBD_LobbyFactionInfo
{
	string m_sKey; //!< faction key, e.g. `BLUFOR`; matches TBD_LobbySide.m_sKey
	string m_sName; //!< display name, e.g. `BLUFOR` or `Spectators`
	string m_sRoleLabel; //!< e.g. `DEFENDING`; empty = no chip
	TBD_EUITint m_eTint; //!< BLUFOR, OPFOR or NEUTRAL
	int m_iSeats; //!< seats on the side
	bool m_bSpectators; //!< the Spectators row; default false

	//! Create a faction row.
	void TBD_LobbyFactionInfo(string key, string name, string roleLabel, TBD_EUITint tint, int seats, bool spectators = false)
	{
		m_sKey = key;
		m_sName = name;
		m_sRoleLabel = roleLabel;
		m_eTint = tint;
		m_iSeats = seats;
		m_bSpectators = spectators;
	}
}
