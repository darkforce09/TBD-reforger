/**
 * @file TBD_SpawnDeparture.c
 * @brief What a disconnect leaves behind: the seat, the life, the body and every per-id row.
 *
 * Role: tears a departing player out of the spawn manager's state.
 * Position: TBD_SpawnManager.OnPlayerDisconnected calls it on the authority, before vanilla deletes the
 * player's controlled entity.
 * State: none of its own.
 * Invariants: the spectator streaming host is released first, so the corpse is the controlled
 * entity again; a spent life's seat is retained (the side stays fielded, the seat off the market);
 * a live player's identity-to-slot pairing is remembered for a reconnect; a NUMERIC death mark is
 * dropped so the next holder of the id does not inherit it; every row keyed on the numeric id,
 * the key cache and the connection epoch go, so a recycled id inherits nothing.
 */

//! Disconnect handling of the spawn manager.
class TBD_SpawnDeparture : Managed
{
	protected TBD_SpawnManager m_Spawn; //!< owning manager

	//! Bind the departure handling to its manager.
	void TBD_SpawnDeparture(TBD_SpawnManager spawn)
	{
		m_Spawn = spawn;
	}

	//! Tear `playerId` out of the spawn state, in the order the invariants above require.
	//! @authority server
	void OnPlayerDisconnected(int playerId)
	{
		TBD_SpectatorHost.ReleaseFor(playerId, "player disconnected");

		TBD_SpawnIdentityKeys identity = m_Spawn.GetIdentity();
		TBD_OneLifeLedger lives = m_Spawn.GetLives();
		TBD_SlotClaimBook slots = m_Spawn.GetSlots();

		string bindKey = identity.PlayerBindKey(playerId);
		bool lifeSpent = m_Spawn.IsOneLife() && lives.IsBindKeyDead(bindKey);

		TBD_MissionSlotStruct slot = m_Spawn.GetAssignedSlot(playerId);
		string slotKey = "-";
		if (slot)
		{
			slotKey = slot.Key();
			if (TBD_SpawnIdentityKeys.IsDurableKey(bindKey))
				slots.RememberReclaim(bindKey, slot.id);
		}

		ForgetBodyVanillaIsAboutToTake(playerId, slot);

		m_Spawn.GetDeployExecutor().ForgetRequest(playerId);
		m_Spawn.GetDeployWaves().ForgetHolder(playerId);
		m_Spawn.GetDeployWaves().ForgetRetries(playerId);
		m_Spawn.GetDeployWatchdog().ForgetSpawnSeen(playerId);
		lives.ClearAdminRespawnPending(playerId);
		m_Spawn.GetTickets().ForgetPlayer(playerId);

		if (lifeSpent && slot)
			slots.RetainDepartedSeat(slot, bindKey, TBD_SpawnIdentityKeys.IsIdentityKey(bindKey));

		if (!TBD_SpawnIdentityKeys.IsIdentityKey(bindKey) && lives.IsBindKeyDead(bindKey))
		{
			lives.ForgetBindKey(bindKey);
			Print(string.Format("[TBD][JIP] player=%1 left on a NUMERIC key (%2) -- one-life mark dropped so the next holder of that id does not inherit this death. ONE LIFE IS NOT ENFORCEABLE ON THIS HOST; fix the dedicated server's backend config.",
				playerId, bindKey), LogLevel.WARNING);
		}

		slots.Unassign(playerId);
		identity.ForgetCachedKey(playerId);
		m_Spawn.GetJoinAudit().ForgetPlayer(playerId);
		m_Spawn.GetEpochs().CloseEpoch(playerId);

		if (lifeSpent)
			Print(string.Format("[TBD][JIP] player=%1 left DEAD -- seat %2 retained under key %3 reclaimable=%4 (side stays fielded, seat off the market)",
				playerId, slotKey, bindKey, TBD_SpawnIdentityKeys.IsIdentityKey(bindKey)));
		else
			Print(string.Format("[TBD][JIP] player=%1 left ALIVE -- seat %2 released, reclaim recorded under key %3 keyMode=%4",
				playerId, slotKey, bindKey, TBD_SpawnIdentityKeys.KeyModeLabel(bindKey)));
	}

	//! Forget the slot body `playerId` stands on: vanilla deletes it now (no reconnect component)
	//! or after its reservation expires (the reconnect re-apply never runs on a framework world), so
	//! the next deploy on the slot rematerializes. A lobby holder who never deployed controls
	//! nothing and keeps the pristine lineup body.
	//! @authority server
	protected void ForgetBodyVanillaIsAboutToTake(int playerId, TBD_MissionSlotStruct slot)
	{
		if (!slot)
			return;

		IEntity controlled = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
		if (!controlled || m_Spawn.GetBodies().GetSlotBody(slot.Key()) != controlled)
			return;

		m_Spawn.GetLoadoutSettle().CancelLoadoutAppsFor(controlled);
		m_Spawn.GetBodies().ForgetBody(slot.Key());

		Print(string.Format("[TBD][JIP] player=%1 slot=%2 body released to vanilla teardown -- next deploy on this slot rematerializes a fresh dressed body",
			playerId, slot.Key()));
	}
}
