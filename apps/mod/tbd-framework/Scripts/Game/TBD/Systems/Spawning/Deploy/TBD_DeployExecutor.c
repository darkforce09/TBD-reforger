/**
 * @file TBD_DeployExecutor.c
 * @brief The one door into the world: every deploy of a framework mission ends here.
 *
 * Role: decides one deploy (ONE LIFE, stage, loadout boundary, seat, platform gate, body reuse or
 * rematerialize) and hands the player onto the chosen body through vanilla's POSSESS request.
 * Position: owned by TBD_SpawnManager; called by every deploy path (stage waves, join, retry,
 * death redeploy, admin respawn, Ready and Continue, lobby claims, the vanilla pull path).
 * State: the per-player deploy-requested latch and the last deploy result, server only.
 * Invariants: a spent life is DENIED before anything else unless the admin override is set, and
 * only DeployPlayerInternal accepts that override; a body is reused only alive and by the identity
 * it was handed to; a spawn ticket names the chosen body before the possess request is made.
 */

//! Deploy decisions and the body takeover of the spawn manager.
class TBD_DeployExecutor : Managed
{
	protected TBD_SpawnManager m_Spawn; //!< owning manager
	protected ref map<int, bool> m_mDeployRequested; //!< players handed a body and not since killed, re-armed or gone
	protected ref map<int, TBD_EDeployResult> m_mLastDeployResult; //!< playerId to its last DeployPlayerEx or AdminRespawn result

	//! Bind the executor to its manager.
	void TBD_DeployExecutor(TBD_SpawnManager spawn)
	{
		m_Spawn = spawn;
		m_mDeployRequested = new map<int, bool>();
		m_mLastDeployResult = new map<int, TBD_EDeployResult>();
	}

	//! Cancel the pending spawn announcements.
	void CancelCallbacks()
	{
		GetGame().GetCallqueue().Remove(FinalizeSpawnWhenControlled);
	}

	//! True when `playerId` was handed a body that is still theirs.
	bool HasRequested(int playerId)
	{
		return m_mDeployRequested.Contains(playerId);
	}

	//! Re-arm `playerId` so the next deploy runs instead of answering ALREADY.
	void ForgetRequest(int playerId)
	{
		m_mDeployRequested.Remove(playerId);
	}

	//! Deploy `playerId` through the one-life boundary, recording the result.
	//! @authority server
	TBD_EDeployResult DeployPlayerEx(int playerId)
	{
		TBD_EDeployResult result = DeployPlayerInternal(playerId, false, false);
		RecordDeployResult(playerId, result);
		return result;
	}

	//! True when the last deploy of `playerId` waits on the platform or could not be authorized now.
	bool AwaitsPlatform(int playerId)
	{
		TBD_EDeployResult last;
		if (!m_mLastDeployResult.Find(playerId, last))
			return false;

		return last == TBD_EDeployResult.AUTHORIZING || last == TBD_EDeployResult.UNAUTHORIZED;
	}

	//! The last deploy result of `playerId`, or NOT_MINE when none is recorded.
	TBD_EDeployResult LastDeployResult(int playerId)
	{
		TBD_EDeployResult last = TBD_EDeployResult.NOT_MINE;
		m_mLastDeployResult.Find(playerId, last);
		return last;
	}

	//! Drop the recorded result, so the next one read comes from a later deploy.
	void ForgetDeployResult(int playerId)
	{
		m_mLastDeployResult.Remove(playerId);
	}

	//! Record `result` as the last deploy result of `playerId`.
	void RecordDeployResult(int playerId, TBD_EDeployResult result)
	{
		m_mLastDeployResult.Set(playerId, result);
	}

	//! The deploy decision. In order: not the authority or no framework mission -> NOT_MINE; a spent
	//! life -> DENIED; LOBBY -> FAILED; an unplayable lineup -> DENIED; lineup or roster not ready ->
	//! RETRY; already bound -> ALREADY; no seat or the platform gate -> its refusal; a body that is
	//! missing, dead, forced fresh or handed to another identity is rematerialized; a body still being
	//! dressed -> RETRY, unplayable -> FAILED; no controller -> RETRY; else DEPLOYED.
	//! @param forceFreshBody always rematerialize (admin respawn)
	//! @param adminOverride bypass ONE LIFE and the LOBBY refusal; passed true only by
	//! TBD_DeathRespawnFlow.AdminRespawn and TBD_DeployWaves.RetryDeploy for a pending admin respawn;
	//! every other caller goes through DeployPlayerEx
	//! @authority server
	TBD_EDeployResult DeployPlayerInternal(int playerId, bool forceFreshBody, bool adminOverride)
	{
		if (TBD_Authority.IsClient())
			return TBD_EDeployResult.NOT_MINE;

		if (!TBD_MissionLoader.IsLoaded() || !TBD_MissionLoader.IsValid())
			return TBD_EDeployResult.NOT_MINE;

		if (m_Spawn.IsOneLife() && !adminOverride && m_Spawn.IsPlayerDead(playerId))
		{
			Print(string.Format("[TBD][Spawn] deploy DENIED player=%1 key=%2 -- one life spent (admin respawn is the only way back)",
				playerId, m_Spawn.GetIdentity().PlayerBindKey(playerId)), LogLevel.WARNING);
			return TBD_EDeployResult.DENIED;
		}

		if (m_Spawn.GetStage() == TBD_EGameStage.LOBBY && !adminOverride)
		{
			Print(string.Format("[TBD][Spawn] deploy FAILED player=%1 -- bodies wait for BRIEFING", playerId), LogLevel.WARNING);
			return TBD_EDeployResult.FAILED;
		}

		if (m_Spawn.GetLoadoutSettle().IsRefused())
		{
			Print(string.Format("[TBD][Spawn] deploy DENIED player=%1 -- one or more slot bodies are UNPLAYABLE (blocking loadout failure at the spawn boundary)",
				playerId), LogLevel.ERROR);
			return TBD_EDeployResult.DENIED;
		}

		if (!m_Spawn.GetBodies().AreMaterialized() || !TBD_RosterLoader.IsLoaded())
			return TBD_EDeployResult.RETRY;

		if (m_mDeployRequested.Contains(playerId))
			return TBD_EDeployResult.ALREADY;

		m_Spawn.GetSlots().AssignSlotForPlayer(playerId);

		TBD_MissionSlotStruct slot = m_Spawn.GetAssignedSlot(playerId);
		if (!slot || !TBD_SpawnDeploymentGate.Admits(playerId, slot))
			return TBD_SpawnDeploymentGate.Refusal(slot);

		string bindKey = m_Spawn.GetIdentity().PlayerBindKey(playerId);
		IEntity body = m_Spawn.GetBodies().GetSlotBody(slot.Key());
		string remakeReason;
		if (!body)
		{
			remakeReason = "no standing body";
		}
		else if (forceFreshBody)
		{
			remakeReason = "fresh body enforced by the caller (admin respawn)";
		}
		else if (TBD_CharacterUtil.IsDead(body, true))
		{
			remakeReason = "previous life spent";
		}
		else if (m_Spawn.GetBodies().IsBoundToAnother(slot.Key(), bindKey))
		{
			remakeReason = "body already used by another occupant";
		}

		if (!remakeReason.IsEmpty())
		{
			if (body && !TBD_CharacterUtil.IsDead(body, true))
				Print(string.Format("[TBD][Slots] slot=%1 abandoning a LIVE body (%2) -- it stays in the world", slot.Key(), remakeReason), LogLevel.WARNING);

			m_Spawn.GetLoadoutSettle().CancelLoadoutAppsFor(body);

			body = m_Spawn.GetBodies().SpawnSlotBody(slot, 0);
			if (!body)
				return TBD_EDeployResult.FAILED;
			m_Spawn.GetBodies().SetSlotBody(slot.Key(), body);
			Print(string.Format("[TBD][Slots] rematerialized body for slot %1 (%2) -- freshly dressed from mission JSON", slot.Key(), remakeReason));
		}

		TBD_LoadoutApplication bodyApp = m_Spawn.GetLoadoutSettle().FindLoadoutAppFor(body);
		if (bodyApp)
		{
			if (!bodyApp.IsDone())
				return TBD_EDeployResult.RETRY;
			if (bodyApp.HasBlockingFailure())
			{
				Print(string.Format("[TBD][Spawn] deploy FAILED player=%1 slot=%2 -- this slot body is UNPLAYABLE: %3",
					playerId, slot.Key(), bodyApp.BlockingSummary()), LogLevel.ERROR);
				return TBD_EDeployResult.FAILED;
			}
			if (bodyApp.HasShortfall())
				Print(string.Format("[TBD][Spawn] deploy player=%1 slot=%2 -- deploying with a loadout SHORTFALL (playable, but not carrying what the mission authored): %3",
					playerId, slot.Key(), bodyApp.ShortfallBrief()), LogLevel.WARNING);
		}

		SCR_PlayerController pc = SCR_PlayerController.Cast(
			GetGame().GetPlayerManager().GetPlayerController(playerId));
		if (!pc)
		{
			Print("[TBD] SpawnManager: no player controller for player " + playerId, LogLevel.ERROR);
			return TBD_EDeployResult.RETRY;
		}

		string engineKey = TBD_SlotBodyMaterializer.EngineFactionKey(slot.faction);
		if (engineKey.IsEmpty())
			engineKey = m_Spawn.GetBodies().BodyFactionKey(body);

		m_Spawn.GetBodies().BindBody(slot.Key(), bindKey);

		HandPlayerOntoBody(pc, body, playerId, engineKey, string.Format("slot=%1 faction=%2", slot.id, slot.faction));
		Print(string.Format("[TBD] SpawnManager: bound player %1 to slot %2 body (kit %3)", playerId, slot.Key(), slot.kit));
		return TBD_EDeployResult.DEPLOYED;
	}

	//! The takeover shared by the slot path and the walk-on: faction affiliation, spectator host
	//! release, spawn ticket, possess (or direct bind), bookkeeping and the arrival watchdog.
	//! @param label names the caller in the affiliation warning
	//! @authority server
	void HandPlayerOntoBody(SCR_PlayerController pc, IEntity body, int playerId, string engineFactionKey, string label)
	{
		SCR_PlayerFactionAffiliationComponent factionComp = SCR_PlayerFactionAffiliationComponent.Cast(
			pc.FindComponent(SCR_PlayerFactionAffiliationComponent));
		if (factionComp)
		{
			if (!engineFactionKey.IsEmpty())
			{
				factionComp.SetAffiliatedFactionByKey(engineFactionKey);
				// Vanilla learns the affiliation only through the faction manager.
				SCR_FactionManager fm = SCR_FactionManager.Cast(GetGame().GetFactionManager());
				if (fm)
					fm.UpdatePlayerFaction_S(factionComp);
			}
			else
			{
				Print(string.Format("[TBD][Spawn] %1 has no engine mapping -- affiliation left untouched", label), LogLevel.WARNING);
			}
		}

		// A spectating dead player possesses a streaming host; release it only here, past every
		// refusal, so the possess pipeline sees the corpse as the controlled entity again.
		TBD_SpectatorHost.ReleaseFor(playerId, "deploying -- the player is going back into a real body");

		// The ticket names this body and is opened before the possess request asks for it.
		m_Spawn.GetTickets().AuthorizeSpawn(playerId, body);
		bool possessed = PossessSlotBody(pc, body, playerId);
		if (!possessed)
			pc.SetInitialMainEntity(body);

		m_mDeployRequested.Set(playerId, true);

		m_Spawn.GetDeployWaves().ForgetRetries(playerId);
		m_Spawn.GetDeployWatchdog().ForgetSpawnSeen(playerId);

		// The possess pipeline announces its own spawn; only the direct bind announces here.
		if (!possessed)
			NotifySpawnedManually(playerId);

		m_Spawn.GetDeployWatchdog().Arm(playerId);
	}

	//! Ask vanilla's possess spawn request to hand `body` to the player: it takes over an existing
	//! entity, so no second body is created, and it runs the finalize the client's loading screen
	//! waits on.
	//! @return false when the route is unavailable, so the caller binds directly
	protected bool PossessSlotBody(SCR_PlayerController pc, IEntity body, int playerId)
	{
		SCR_PossessSpawnRequestComponent request = SCR_PossessSpawnRequestComponent.Cast(
			pc.FindComponent(SCR_PossessSpawnRequestComponent));
		if (!request)
		{
			Print(string.Format("[TBD][Spawn] player=%1 has no possess request component -- falling back to direct bind", playerId), LogLevel.WARNING);
			return false;
		}

		SCR_PossessSpawnData data = SCR_PossessSpawnData.FromEntity(body);
		if (!data)
		{
			Print(string.Format("[TBD][Spawn] player=%1 possess data build failed -- falling back to direct bind", playerId), LogLevel.WARNING);
			return false;
		}

		if (!request.RequestRespawn(data))
		{
			Print(string.Format("[TBD][Spawn] player=%1 possess request refused -- falling back to direct bind", playerId), LogLevel.WARNING);
			return false;
		}

		Print(string.Format("[TBD][Spawn] player=%1 possess request accepted", playerId));
		return true;
	}

	//! Announce a direct-bind spawn through the game mode's spawn invoker once the player controls
	//! the body, so every listener (the client loading screen among them) hears it once.
	protected void NotifySpawnedManually(int playerId)
	{
		FinalizeSpawnWhenControlled(playerId, 0, m_Spawn.GetEpochs().EnsureConnectEpoch(playerId));
	}

	//! Poll every 200 ms, up to 25 times, until the player controls a body, then fire the spawn
	//! invoker. Does nothing once the connection moved on.
	protected void FinalizeSpawnWhenControlled(int playerId, int attempt, int epoch)
	{
		if (!m_Spawn.GetEpochs().IsSameConnection(playerId, epoch))
			return;

		IEntity controlled = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
		if (!controlled)
		{
			if (attempt < 25)
				GetGame().GetCallqueue().CallLater(FinalizeSpawnWhenControlled, 200, false, playerId, attempt + 1, epoch);
			else
				Print(string.Format("[TBD][Spawn] player=%1 never took control of its body -- spawn not announced", playerId), LogLevel.WARNING);
			return;
		}

		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(m_Spawn.GetOwner());
		if (gm)
			gm.GetOnPlayerSpawned().Invoke(playerId, controlled);
		else
			m_Spawn.GetDeployWatchdog().OnPlayerSpawnedHook(playerId, controlled);
	}
}
