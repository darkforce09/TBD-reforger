//! T-181.9.1 - the lobby roster: what the server knows about every seat, shaped for a screen.
//! UI reorg 2026-09-12: models only. Server builder + wire Serialise/Parse live in `TBD_LobbyService.c`.
//!
//! -- The fact this design is built on --------------------------------------------------------
//! **A client has no mission document and no slot assignment.** `TBD_FrameworkManager.OnPostInit`
//! returns early for `RplMode.Client` *before* `TBD_MissionLoader.BeginLoad()`, so
//! `TBD_MissionLoader.GetMission()` is null on every client, forever; and `TBD_SpawnManager`'s
//! `m_mPlayerSlot` is a plain `map`, not an `RplProp`, so `GetAssignedSlot()` is server-only.
//!
//! The slot picker is a client-side menu. It therefore cannot read the roster at all - it can only
//! render what the server chose to send it. Same wall the briefing hit at T-181.9.2, and the same
//! answer: build on the server, ship one string over an owner-scoped RPC, rebuild on the client.
//!
//! -- The roster is NOT re-derived here -------------------------------------------------------
//! `TBD_SpawnManager.BuildSlotRoster()` (T-181.9, shipped) is the authority's own answer to
//! "who holds what", emitted as TSV precisely so a picker could bind to it:
//!
//!     <slotKey> \t <faction> \t <group> \t <role> \t <state> \t <holderPlayerId>
//!     state in OPEN | HELD | DEAD
//!
//! This service **parses that and nothing else**. It does not consult `m_mPlayerSlot`, does not
//! re-implement `IsSlotHeldByAnother`, and cannot disagree with the authority about who holds a
//! seat - because it never forms an opinion. What it adds is the two things a screen needs and a
//! playerId cannot supply: the holder's **display name**, and the faction's **display name**.
//!
//! `DEAD` with holder `-1` is a seat whose occupant spent their life and then quit. It reads as
//! DEAD, not OPEN, and it is not selectable - exactly as the authority reports it.
//!
//! -- Side discipline: deliberately NOT applied here ------------------------------------------
//! The briefing filters the other side out at the wire. The lobby must not: you cannot pick a side
//! you cannot see, and Arma 3 - the parity target - shows the full ORBAT of both sides with who is
//! in each seat. Side discipline begins the moment you have a seat, which is the briefing screen.
//! This is a deliberate divergence between two neighbouring screens, recorded so a later reader
//! does not "fix" one to match the other.

//! One seat. `m_bIsOwn` is resolved on the SERVER against the reader's own assignment, so the
//! client never has to work out which seat is theirs (it could not - it has no assignment).
class TBD_LobbySlot
{
	string m_sKey;    //!< durable slot key - the exact string ClaimSlot() takes
	string m_sRole;
	string m_sState;  //!< OPEN | HELD | DEAD, verbatim from the authority
	string m_sHolder; //!< display name; empty for OPEN and for a departed DEAD holder
	bool m_bIsOwn;

	//------------------------------------------------------------------------------------------------
	void TBD_LobbySlot(string key, string role, string state, string holder, bool isOwn)
	{
		m_sKey = key;
		m_sRole = role;
		m_sState = state;
		m_sHolder = holder;
		m_bIsOwn = isOwn;
	}

	//------------------------------------------------------------------------------------------------
	//! The one question the picker asks of a row. A dead seat is never selectable - that is the
	//! whole point of the DEAD state existing separately from HELD.
	bool IsOpen()
	{
		return m_sState == TBD_LobbyService.STATE_OPEN;
	}

	//------------------------------------------------------------------------------------------------
	bool IsDead()
	{
		return m_sState == TBD_LobbyService.STATE_DEAD;
	}
}

//! One squad. Seats and the open count are carried explicitly so a collapsed group can still say
//! how much room it has - that is what makes side -> group -> slot navigable without opening
//! everything to find out.
class TBD_LobbyGroup
{
	string m_sCallsign;
	int m_iOpen;
	bool m_bHasOwn; //!< the reader's seat is in this squad - the disclosure default
	ref array<ref TBD_LobbySlot> m_aSlots;

	//------------------------------------------------------------------------------------------------
	void TBD_LobbyGroup(string callsign)
	{
		m_sCallsign = callsign;
		m_aSlots = {};
	}

	//------------------------------------------------------------------------------------------------
	int Seats()
	{
		return m_aSlots.Count();
	}
}

//! One side. Same shape as a group, one level up.
class TBD_LobbySide
{
	string m_sKey;
	string m_sName;
	int m_iSeats;
	int m_iOpen;
	bool m_bHasOwn;
	ref array<ref TBD_LobbyGroup> m_aGroups;

	//------------------------------------------------------------------------------------------------
	void TBD_LobbySide(string key, string name)
	{
		m_sKey = key;
		m_sName = name;
		m_aGroups = {};
	}
}

//! Everything the picker draws. Built on the server, shipped as one string, rebuilt on the client.
class TBD_LobbyRoster
{
	string m_sMissionName;
	string m_sTerrain;
	string m_sStage;

	//! The reader's own seat. Empty key = no seat yet, which is what disables DEPLOY.
	string m_sOwnKey;
	string m_sOwnLabel; //!< "ALPHA . SL", ready to print

	//! ONE LIFE: this reader has already spent theirs. Claim and deploy will both be refused by
	//! the authority, so the screen says so up front instead of letting them find out by clicking.
	bool m_bLifeSpent;

	//! T-181.29 - the authority's answer to "does this reader already have a body?".
	//!
	//! -- Why the client cannot answer this for itself, and why the screen needed it -----------
	//! The screen stands down on `TBD_LobbyClient.IsDeployed()`, which is latched by exactly ONE
	//! event: a DEPLOY verdict the player's own click asked for. Every OTHER door into the world is
	//! server-side and silent to this client - `TBD_SpawnManager`'s LOBBY auto-deploy wave
	//! (`m_bAutoDeploy`, still 1), the JIP `DeployJoiner` path, and `AdminRespawn`. A player any of
	//! those put in the world therefore had a live character AND the picker sitting on top of it,
	//! with nothing that would ever take it down.
	//!
	//! So the fact travels the same way every other fact this screen draws travels: computed on the
	//! authority in `BuildForPlayer`, carried on the wire, rebuilt on the client. It sits next to
	//! `m_bLifeSpent` because it is the same KIND of fact - something only the server can know about
	//! this player, which changes what the screen is allowed to offer.
	//!
	//! -- Deliberately NOT latched ------------------------------------------------------------
	//! `m_bDeployed` latches for good; this does not. It is a per-roster observation, so a reading
	//! that turns out to be wrong is corrected by the very next refresh (2 s at worst) and the
	//! picker comes back on `TBD_LobbyStage.Tick`'s unconditional re-raise. That property is what
	//! makes closing on it safe: the worst case is a picker that flickers, never one that is gone
	//! for good - which is the failure `Raise()` is deliberately built to avoid.
	bool m_bInWorld;

	ref array<ref TBD_LobbySide> m_aSides;

	//! Set when the server could not answer. The screen shows this instead of an empty frame -
	//! design law: an empty state says why, it never shows a void.
	string m_sUnavailableReason;

	// -- The last thing the server did on this player's behalf, if anything. ------------------
	// Every server reply carries a whole fresh roster, so a claim/release/deploy answer and a
	// plain refresh are the SAME message shape. That is what makes optimistic reconciliation a
	// wholesale replace rather than a merge: there is never a partial update to apply.
	string m_sAction;  //!< CLAIM | RELEASE | DEPLOY, empty for a plain refresh
	bool m_bActionOk;
	string m_sActionReason;
	string m_sActionKey; //!< the slot the action was about (for the rejection highlight)

	//------------------------------------------------------------------------------------------------
	void TBD_LobbyRoster()
	{
		m_aSides = {};
	}

	//------------------------------------------------------------------------------------------------
	bool IsAvailable()
	{
		return m_sUnavailableReason.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	bool HasOwnSlot()
	{
		return !m_sOwnKey.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	int TotalSeats()
	{
		int n = 0;
		foreach (TBD_LobbySide side : m_aSides)
		{
			n += side.m_iSeats;
		}

		return n;
	}

	//------------------------------------------------------------------------------------------------
	int TotalOpen()
	{
		int n = 0;
		foreach (TBD_LobbySide side : m_aSides)
		{
			n += side.m_iOpen;
		}

		return n;
	}

	//------------------------------------------------------------------------------------------------
	//! Slot by key, or null. Linear because a lobby list is walked once per interaction, not per
	//! frame - a map would cost more to keep in step than it saves.
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

	//------------------------------------------------------------------------------------------------
	//! Recount every side and group from the slots. Called after a local optimistic edit so the
	//! headline counts a collapsed row shows cannot drift from the rows underneath it.
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
