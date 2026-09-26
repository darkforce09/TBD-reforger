/**
 * @file TBD_LobbyRoster.c
 * @brief The lobby roster models: every seat of both sides, who holds it, and the last verdict.
 *
 * Role: sides, squads and seats as the server reports them, plus the reader's own seat, life,
 * in-world state and the verdict of their last action.  Position: built on the server by
 * TBD_LobbyService, carried by TBD_LobbyRosterWire, cached and edited optimistically by
 * TBD_LobbyClient on the client.
 * State: plain data owned by whoever built or parsed it.
 * Invariants: counts and the own-seat key are derived from the seats by Recount, never carried on
 * the wire; a DEAD seat is neither open nor selectable; a reply replaces a roster, never merges.
 */

//! One seat. m_bIsOwn is resolved on the server against the reader's own assignment, which a client does not have.
class TBD_LobbySlot
{
	string m_sKey; //!< durable slot key, the exact string ClaimSlot takes
	string m_sRole; //!< role display text, sanitised
	string m_sState; //!< OPEN, HELD or DEAD, verbatim from the authority
	string m_sHolder; //!< holder display name; empty for OPEN and for a departed DEAD holder
	bool m_bIsOwn; //!< the reader holds this seat

	//! Create a seat from its wire fields.
	void TBD_LobbySlot(string key, string role, string state, string holder, bool isOwn)
	{
		m_sKey = key;
		m_sRole = role;
		m_sState = state;
		m_sHolder = holder;
		m_bIsOwn = isOwn;
	}

	//! @return true when nobody holds the seat; a DEAD seat is never open
	bool IsOpen()
	{
		return m_sState == TBD_LobbyService.STATE_OPEN;
	}

	//! @return true when the holder spent their life
	bool IsDead()
	{
		return m_sState == TBD_LobbyService.STATE_DEAD;
	}
}

//! One squad; the open count and own flag let a folded squad say how much room it has.
class TBD_LobbyGroup
{
	string m_sCallsign; //!< squad callsign, sanitised
	int m_iOpen; //!< open seats; set by Recount
	bool m_bHasOwn; //!< the reader's seat is in this squad; set by Recount
	ref array<ref TBD_LobbySlot> m_aSlots; //!< seats in roster order

	//! Create an empty squad.
	void TBD_LobbyGroup(string callsign)
	{
		m_sCallsign = callsign;
		m_aSlots = {};
	}

	//! @return the number of seats in the squad
	int Seats()
	{
		return m_aSlots.Count();
	}
}

//! One side: the same shape as a squad, one level up.
class TBD_LobbySide
{
	string m_sKey; //!< faction key
	string m_sName; //!< faction display name, sanitised
	int m_iSeats; //!< seats on the side; set by Recount
	int m_iOpen; //!< open seats on the side; set by Recount
	bool m_bHasOwn; //!< the reader's seat is on this side; set by Recount
	ref array<ref TBD_LobbyGroup> m_aGroups; //!< squads in roster order

	//! Create an empty side.
	void TBD_LobbySide(string key, string name)
	{
		m_sKey = key;
		m_sName = name;
		m_aGroups = {};
	}
}

//! Everything the picker draws: built on the server, shipped as one string, rebuilt on the client.
class TBD_LobbyRoster
{
	string m_sMissionName; //!< mission display name, sanitised
	string m_sTerrain; //!< terrain name, sanitised
	string m_sStage; //!< TBD_EGameStage name at build time; empty without a framework manager

	string m_sOwnKey; //!< the reader's own seat; empty = no seat yet, which disables DEPLOY; set by Recount
	string m_sOwnLabel; //!< `<callsign> . <role>` of the own seat, ready to print; set by Recount

	bool m_bLifeSpent; //!< the reader spent their life: the authority refuses claim and deploy

	bool m_bInWorld; //!< the reader controls a body, however it arrived (their own deploy, the BRIEFING holder deploy, a join-in-progress deploy or an admin respawn); refreshed per roster, never latched

	ref array<ref TBD_LobbySide> m_aSides; //!< sides in roster order

	string m_sUnavailableReason; //!< why the server could not build a roster; empty when available

	string m_sAction; //!< CLAIM, RELEASE or DEPLOY; empty for a plain refresh
	bool m_bActionOk; //!< the action succeeded
	string m_sActionReason; //!< the sentence explaining the verdict
	string m_sActionKey; //!< the slot a claim was about (the rejection highlight); the TBD_EDeployResult name for DEPLOY

	//! Create an empty roster.
	void TBD_LobbyRoster()
	{
		m_aSides = {};
	}

	//! @return true when the server could build the roster
	bool IsAvailable()
	{
		return m_sUnavailableReason.IsEmpty();
	}

	//! @return true when the reader holds a seat
	bool HasOwnSlot()
	{
		return !m_sOwnKey.IsEmpty();
	}

	//! @return seats across every side
	int TotalSeats()
	{
		int n = 0;
		foreach (TBD_LobbySide side : m_aSides)
		{
			n += side.m_iSeats;
		}

		return n;
	}

	//! @return open seats across every side
	int TotalOpen()
	{
		int n = 0;
		foreach (TBD_LobbySide side : m_aSides)
		{
			n += side.m_iOpen;
		}

		return n;
	}

	//! The seat with `key`, walked linearly: a lobby list is searched once per interaction, not per frame.
	//! @return the seat, or null for an empty or unknown key
	TBD_LobbySlot FindSlot(string key)
	{
		if (key.IsEmpty())
			return null;

		foreach (TBD_LobbySide side : m_aSides)
		{
			foreach (TBD_LobbyGroup group : side.m_aGroups)
			{
				foreach (TBD_LobbySlot slot : group.m_aSlots)
				{
					if (slot.m_sKey == key)
						return slot;
				}
			}
		}

		return null;
	}

	//! Recount every side and squad, and the reader's own seat and label, from the seats; runs after every parse and every optimistic edit so headline counts match the rows.
	void Recount()
	{
		m_sOwnKey = string.Empty;
		m_sOwnLabel = string.Empty;

		foreach (TBD_LobbySide side : m_aSides)
		{
			side.m_iSeats = 0;
			side.m_iOpen = 0;
			side.m_bHasOwn = false;

			foreach (TBD_LobbyGroup group : side.m_aGroups)
			{
				group.m_iOpen = 0;
				group.m_bHasOwn = false;

				foreach (TBD_LobbySlot slot : group.m_aSlots)
				{
					side.m_iSeats++;

					if (slot.IsOpen())
					{
						group.m_iOpen++;
						side.m_iOpen++;
					}

					if (!slot.m_bIsOwn)
						continue;

					group.m_bHasOwn = true;
					side.m_bHasOwn = true;
					m_sOwnKey = slot.m_sKey;
					m_sOwnLabel = string.Format("%1 . %2", group.m_sCallsign, slot.m_sRole);
				}
			}
		}
	}
}
