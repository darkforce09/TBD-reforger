/**
 * @file TBD_LobbySlotInfo.c
 * @brief One seat of the Lobby screen: role, weapon and tag chips, state, holder and kit key.
 *
 * Role: the presentation of one seat.  Position: created by TBD_LobbySquadInfo.AddSlot; read by
 * TBD_LobbySlotRowComponent and TBD_KitInspectorPanel; mutated by TBD_LobbyCatalog.Claim and
 * Release.
 * State: plain data.  Invariants: m_sState is the wire vocabulary verbatim, OPEN, HELD or DEAD; a
 * held seat carries its holder's name.
 */

//! One seat; m_sState reads OPEN, HELD or DEAD.
class TBD_LobbySlotInfo
{
	string m_sKey; //!< durable slot key, the string Claim takes
	int m_iIndex; //!< 1-based position in the squad (`1: Platoon Commander`)
	string m_sRole; //!< role display text, e.g. `Platoon Commander`
	ref array<string> m_aWeapons; //!< weapon chips, e.g. `AK-74`, `RPG-7`
	ref array<string> m_aTags; //!< role tags, e.g. `MED`, `ENG`
	string m_sState = "OPEN"; //!< OPEN, HELD or DEAD; default OPEN
	string m_sHolder; //!< holder display name; empty when OPEN
	bool m_bOwn; //!< the viewer holds this seat
	string m_sKitKey; //!< TBD_LobbyCatalog.GetKit key

	//! Create an OPEN seat with no chips.
	void TBD_LobbySlotInfo(string key, int index, string role, string kitKey)
	{
		m_sKey = key;
		m_iIndex = index;
		m_sRole = role;
		m_sKitKey = kitKey;
		m_aWeapons = {};
		m_aTags = {};
	}

	//! @return true when nobody holds the seat
	bool IsOpen()
	{
		return m_sState == "OPEN";
	}

	//! @return true when the holder spent their life
	bool IsDead()
	{
		return m_sState == "DEAD";
	}

	//! @return `<index>: <role>`, e.g. `8: Rifleman (AT)`, the roster row and kit inspector headline
	string Headline()
	{
		return string.Format("%1: %2", m_iIndex, m_sRole);
	}
}
