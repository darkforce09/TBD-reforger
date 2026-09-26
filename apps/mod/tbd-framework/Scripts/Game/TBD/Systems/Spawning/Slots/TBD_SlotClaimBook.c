/**
 * @file TBD_SlotClaimBook.c
 * @brief Who holds which mission slot: claims, releases, automatic seating and retained seats.
 *
 * Role: the seat ledger of the spawn manager.  Position: owned by TBD_SpawnManager; the lobby
 * claims and releases through the manager, the deploy path seats through AssignSlotForPlayer, the
 * join and disconnect handlers hand seats back and retain them, the win conditions count them.
 * State: playerId to slot, identity to slot (reconnect reclaim), slot to departed seat, and the
 * round-robin cursor; server only.
 * Invariants: a slot has at most one holder; a connected holder, a spent-life holder and a departed
 * spent-life seat all block a slot; a spent life keeps its seat; the departed-seat table is keyed on
 * the slot, so two departed players with one bind key never overwrite each other.
 */

//! A seat whose holder spent their life and then left, keyed on the slot in TBD_SlotClaimBook.
class TBD_DepartedSeat
{
	string bindKey; //!< bind key of the player who left; matched to hand the seat back
	ref TBD_MissionSlotStruct slot; //!< the retained slot
	bool reclaimable; //!< false for a NUMERIC key: a lease on a number never matches its returning holder
}

//! The seat ledger of TBD_SpawnManager.
class TBD_SlotClaimBook : Managed
{
	protected TBD_SpawnManager m_Spawn; //!< owning manager
	protected ref map<int, ref TBD_MissionSlotStruct> m_mPlayerSlot; //!< playerId to its assigned slot
	protected ref map<string, string> m_mIdentityReclaim; //!< durable bind key to slot id, for a reconnect
	protected ref map<string, ref TBD_DepartedSeat> m_mDepartedSlots; //!< slot key to the seat a spent life left behind
	protected int m_iRoundRobin; //!< next slot index the automatic seating tries; default 0

	//! Bind the ledger to its manager.
	void TBD_SlotClaimBook(TBD_SpawnManager spawn)
	{
		m_Spawn = spawn;
		m_mPlayerSlot = new map<int, ref TBD_MissionSlotStruct>();
		m_mIdentityReclaim = new map<string, string>();
		m_mDepartedSlots = new map<string, ref TBD_DepartedSeat>();
	}

	//! Every assigned seat, playerId to slot. Callers read it and never change it.
	map<int, ref TBD_MissionSlotStruct> GetPlayerSlots()
	{
		return m_mPlayerSlot;
	}

	//! The slot assigned to `playerId`, or null.
	TBD_MissionSlotStruct GetAssignedSlot(int playerId)
	{
		return m_mPlayerSlot.Get(playerId);
	}

	//! True when a spent life that left still holds `slotKey`.
	bool IsSeatDeparted(string slotKey)
	{
		return m_mDepartedSlots.Contains(slotKey);
	}

	//! True when `slotKey` is somebody else's seat: a connected holder, a holder who spent their
	//! life, or a departed spent life (NUMERIC departures block everyone).
	//! @authority server
	protected bool IsSlotHeldByAnother(string slotKey, int playerId)
	{
		foreach (int otherId, TBD_MissionSlotStruct assigned : m_mPlayerSlot)
		{
			if (!assigned || otherId == playerId || assigned.Key() != slotKey)
				continue;

			if (GetGame().GetPlayerManager().GetPlayerController(otherId))
				return true;

			if (m_Spawn.IsOneLife() && m_Spawn.IsPlayerDead(otherId))
				return true;
		}

		TBD_DepartedSeat departed;
		if (m_mDepartedSlots.Find(slotKey, departed) && departed)
		{
			if (!departed.reclaimable)
				return true;

			if (departed.bindKey != m_Spawn.GetIdentity().PlayerBindKey(playerId))
				return true;
		}

		return false;
	}

	//! Seat `playerId` automatically, once: a departed seat of theirs, else their reclaimed slot,
	//! else their roster slot, else the next free slot round-robin. Refuses (logged) when every slot
	//! is held, so two players never share a body.
	//! @authority server
	void AssignSlotForPlayer(int playerId)
	{
		if (m_mPlayerSlot.Contains(playerId))
			return;

		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots || slots.IsEmpty())
		{
			Print("[TBD] SpawnManager: no mission slots -- cannot assign player " + playerId, LogLevel.ERROR);
			return;
		}

		string bindKey = m_Spawn.GetIdentity().PlayerBindKey(playerId);

		if (ReclaimDepartedSeat(playerId, bindKey))
			return;

		TBD_MissionSlotStruct slot;
		if (TBD_SpawnIdentityKeys.IsDurableKey(bindKey))
		{
			string reclaimId;
			if (m_mIdentityReclaim.Find(bindKey, reclaimId))
				slot = TBD_MissionLoader.GetSlotById(reclaimId);
		}
		if (slot && IsSlotHeldByAnother(slot.Key(), playerId))
		{
			Print(string.Format("[TBD][Spawn] player=%1 reclaim of slot %2 refused -- held by someone else", playerId, slot.Key()), LogLevel.WARNING);
			slot = null;
		}

		if (!slot)
		{
			string slotId = ResolveSlotIdForPlayer(playerId);
			slot = TBD_MissionLoader.GetSlotById(slotId);
			if (slot && IsSlotHeldByAnother(slot.Key(), playerId))
			{
				Print(string.Format("[TBD][Spawn] player=%1 roster slot %2 already held -- falling back to a free slot", playerId, slot.Key()), LogLevel.WARNING);
				slot = null;
			}
		}

		if (!slot)
		{
			int count = slots.Count();
			for (int i = 0; i < count; i++)
			{
				TBD_MissionSlotStruct candidate = slots[(m_iRoundRobin + i) % count];
				if (!candidate || IsSlotHeldByAnother(candidate.Key(), playerId))
					continue;
				slot = candidate;
				m_iRoundRobin = (m_iRoundRobin + i + 1) % count;
				break;
			}
		}

		if (!slot)
		{
			Print(string.Format("[TBD][Spawn] player=%1 could not be seated -- every mission slot is held (%2 slots)", playerId, slots.Count()), LogLevel.ERROR);
			return;
		}

		m_mPlayerSlot.Insert(playerId, slot);
		Print(string.Format("[TBD] SpawnManager: assigned slot %1 to player %2 at (%3)", slot.id, playerId, slot.x.ToString() + "," + slot.z.ToString()));
	}

	//! Hand a returning spent life its departed seat back under the id it holds now. The life stays
	//! spent. Refused for a non-identity key and for a seat somebody else now holds.
	//! @return true when a seat was handed back
	//! @authority server
	bool ReclaimDepartedSeat(int playerId, string bindKey)
	{
		if (bindKey.IsEmpty() || m_mPlayerSlot.Contains(playerId))
			return false;

		if (!TBD_SpawnIdentityKeys.IsIdentityKey(bindKey))
			return false;

		string foundSlotKey;
		TBD_DepartedSeat found;
		foreach (string slotKey, TBD_DepartedSeat seat : m_mDepartedSlots)
		{
			if (!seat || !seat.reclaimable || seat.bindKey != bindKey || !seat.slot)
				continue;

			foundSlotKey = slotKey;
			found = seat;
			break;
		}

		if (!found)
			return false;

		if (IsSlotHeldByAnother(foundSlotKey, playerId))
		{
			Print(string.Format("[TBD][Spawn] player=%1 hand-back of departed slot %2 refused -- held by someone else (colliding bind key?)", playerId, foundSlotKey), LogLevel.WARNING);
			return false;
		}

		m_mDepartedSlots.Remove(foundSlotKey);
		m_mPlayerSlot.Insert(playerId, found.slot);
		Print(string.Format("[TBD][Spawn] player=%1 rejoined on a spent life -- slot %2 handed back (still dead)", playerId, foundSlotKey));
		return true;
	}

	//! The roster slot id of `playerId` when a roster is loaded and the key is durable, else empty.
	protected string ResolveSlotIdForPlayer(int playerId)
	{
		if (!TBD_RosterLoader.IsLoaded())
			return string.Empty;

		string bindKey = m_Spawn.GetIdentity().PlayerBindKey(playerId);
		if (!TBD_SpawnIdentityKeys.IsDurableKey(bindKey))
			return string.Empty;

		return TBD_RosterLoader.GetSlotForIdentity(bindKey);
	}

	//! Claim `slotKey` for `playerId` from the slot picker. After LOBBY a claim is a body: a first
	//! claim deploys, a different seat moves the holder. A changed seat ends the platform
	//! deployment of the old one.
	//! @return true when the seat is now theirs
	//! @authority server
	bool ClaimSlot(int playerId, string slotKey)
	{
		TBD_MissionSlotStruct previous = GetAssignedSlot(playerId);
		bool claimed = ClaimSeat(playerId, slotKey);

		TBD_MissionSlotStruct current = GetAssignedSlot(playerId);
		if (claimed && previous && current && previous.Key() != current.Key())
			TBD_DeploymentAuthorization.OnSeatChanged(playerId, previous.uid);

		return claimed;
	}

	//! The claim itself: refused for a spent life and for a seat held by another player.
	//! @authority server
	protected bool ClaimSeat(int playerId, string slotKey)
	{
		TBD_MissionSlotStruct slot = TBD_MissionLoader.GetSlotById(slotKey);
		if (!slot)
			return false;

		if (m_Spawn.IsOneLife() && m_Spawn.IsPlayerDead(playerId))
		{
			Print(string.Format("[TBD][Spawn] claim rejected player=%1 slot=%2 (one life spent)", playerId, slot.Key()), LogLevel.WARNING);
			return false;
		}

		if (IsSlotHeldByAnother(slot.Key(), playerId))
		{
			Print(string.Format("[TBD][Spawn] claim rejected player=%1 slot=%2 (held by another player)", playerId, slot.Key()));
			return false;
		}

		TBD_MissionSlotStruct previous = GetAssignedSlot(playerId);
		bool sameSeat = previous && previous.Key() == slot.Key();
		bool hadBody = m_Spawn.GetDeployExecutor().HasRequested(playerId);

		m_mPlayerSlot.Set(playerId, slot);
		Print(string.Format("[TBD][Spawn] claim player=%1 slot=%2", playerId, slot.Key()));

		if (m_Spawn.GetStage() != TBD_EGameStage.LOBBY && m_Spawn.GetStage() != TBD_EGameStage.LOADING)
		{
			if (hadBody && !sameSeat)
				m_Spawn.GetDeployWaves().RedeployHolderToClaimedSlot(playerId);
			else if (!hadBody)
				m_Spawn.GetDeployWaves().DeployOnPath(playerId, "post-lobby-claim");
		}

		return true;
	}

	//! Give a slot back before deploying. Refused for a spent life (ONE LIFE keeps the seat) and
	//! after deploy. A seat given back ends the platform life that never reached the world.
	//! @return true when the seat was released
	//! @authority server
	bool ReleaseSlot(int playerId)
	{
		bool released = ReleaseSeat(playerId);
		if (released)
			TBD_DeploymentAuthorization.EndLife(playerId, "gave the seat back");

		return released;
	}

	//! The release itself.
	//! @authority server
	protected bool ReleaseSeat(int playerId)
	{
		if (TBD_Authority.IsClient())
			return false;

		if (!m_mPlayerSlot.Contains(playerId))
			return false;

		if (m_Spawn.IsOneLife() && m_Spawn.IsPlayerDead(playerId))
		{
			Print(string.Format("[TBD][Spawn] release rejected player=%1 (one life spent -- slot retained)", playerId), LogLevel.WARNING);
			return false;
		}

		if (m_Spawn.GetDeployExecutor().HasRequested(playerId))
		{
			Print(string.Format("[TBD][Spawn] release rejected player=%1 (already deployed)", playerId), LogLevel.WARNING);
			return false;
		}

		TBD_MissionSlotStruct slot = m_mPlayerSlot.Get(playerId);
		m_mPlayerSlot.Remove(playerId);
		string key;
		if (slot)
			key = slot.Key();
		Print(string.Format("[TBD][Spawn] release player=%1 slot=%2", playerId, key));
		return true;
	}

	//! Players of `factionKey` holding a seat and not dead. A seat never deployed counts alive, so an
	//! unfielded player cannot end the round; departed seats are all spent lives and count zero.
	int CountAliveForFaction(string factionKey)
	{
		int alive;
		foreach (int playerId, TBD_MissionSlotStruct slot : m_mPlayerSlot)
		{
			if (!slot || slot.faction != factionKey)
				continue;
			if (!m_Spawn.IsPlayerDead(playerId))
				alive++;
		}
		return alive;
	}

	//! Seats of `factionKey` claimed at all, alive, dead or departed, so "eliminated" differs from
	//! "never fielded".
	int CountClaimedForFaction(string factionKey)
	{
		int claimed;
		foreach (int playerId, TBD_MissionSlotStruct slot : m_mPlayerSlot)
		{
			if (slot && slot.faction == factionKey)
				claimed++;
		}
		foreach (string slotKey, TBD_DepartedSeat departed : m_mDepartedSlots)
		{
			if (departed && departed.slot && departed.slot.faction == factionKey)
				claimed++;
		}
		return claimed;
	}

	//! Remember which slot a durable identity held, for a reconnect.
	void RememberReclaim(string bindKey, string slotId)
	{
		m_mIdentityReclaim.Set(bindKey, slotId);
	}

	//! Keep the seat of a spent life that left, off the market and counted as claimed.
	//! @param reclaimable false for a NUMERIC key, which never matches a returning holder
	void RetainDepartedSeat(notnull TBD_MissionSlotStruct slot, string bindKey, bool reclaimable)
	{
		TBD_DepartedSeat seat = new TBD_DepartedSeat();
		seat.bindKey = bindKey;
		seat.slot = slot;
		seat.reclaimable = reclaimable;
		m_mDepartedSlots.Set(slot.Key(), seat);
	}

	//! Drop the seat assignment of a departing player id.
	void Unassign(int playerId)
	{
		m_mPlayerSlot.Remove(playerId);
	}
}
