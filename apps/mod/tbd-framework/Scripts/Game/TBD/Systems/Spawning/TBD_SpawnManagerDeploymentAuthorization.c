//! TBD_SpawnManager's side of platform deployment authorization (TBD_DeploymentAuthorization): the
//! deploy results a platform decision adds, the gate every slot deploy passes once a slot is
//! assigned and before any body is prepared for it, the platform decisions that continue or refuse a
//! waiting deploy, and the life ends the platform hears about - a seat changed or given back, a
//! death, a disconnect, the round or the world ending.
//! @authority server

//! The deploy results a platform decision adds.
modded enum TBD_EDeployResult
{
	//! The platform is deciding whether this player may deploy into this event roster slot. Its
	//! decision continues the deploy or returns the player to slot selection by itself, so callers
	//! neither retry nor fall through to vanilla.
	AUTHORIZING,
	//! This deployment into an event seat cannot be authorized now (no roster slot table, no runtime
	//! session, no game identity): the player has been told why and keeps the seat. Distinct from
	//! DENIED, which is a spent life.
	UNAUTHORIZED,
}

//! The platform's say on a slot deploy, asked by TBD_SpawnManager.DeployPlayerInternal once the slot
//! is assigned and before any body is prepared for it.
class TBD_SpawnDeploymentGate
{
	//! What the last `Admits` that did not admit answers.
	protected static TBD_EDeployResult s_eRefusal = TBD_EDeployResult.RETRY;

	//------------------------------------------------------------------------------------------------
	//! May `playerId` deploy into `slot` now? False leaves the deploy's answer for `Refusal`: RETRY
	//! while the player has no connection to stamp a request with, AUTHORIZING while the platform
	//! decides, UNAUTHORIZED when it cannot be authorized now (the seat is kept).
	static bool Admits(int playerId, notnull TBD_MissionSlotStruct slot)
	{
		s_eRefusal = TBD_EDeployResult.RETRY;
		if (!TBD_DeploymentAuthorization.AppliesToThisMission())
			return true;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return false;

		int epoch = spawn.ConnectEpochFor(playerId);
		if (epoch == 0)
			return false;

		TBD_EDeploymentGate gate = TBD_DeploymentAuthorization.Check(playerId, epoch, slot);
		if (gate == TBD_EDeploymentGate.PROCEED)
			return true;

		if (gate == TBD_EDeploymentGate.WAIT)
			s_eRefusal = TBD_EDeployResult.AUTHORIZING;
		else
			s_eRefusal = TBD_EDeployResult.UNAUTHORIZED;

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! The deploy's answer when there is no slot yet (RETRY) or the last `Admits` did not admit.
	static TBD_EDeployResult Refusal(TBD_MissionSlotStruct slot)
	{
		if (!slot)
			return TBD_EDeployResult.RETRY;

		return s_eRefusal;
	}
}

modded class TBD_SpawnManager
{
	//! The last deploy result of each player (DeployPlayerEx, AdminRespawn): a platform decision in
	//! progress, or impossible now, shapes what DeployOnReady and the admin powers answer.
	protected ref map<int, TBD_EDeployResult> m_mLastDeployResult;

	//------------------------------------------------------------------------------------------------
	//! The player's connection stamp, created on first use; 0 while the player has no connection.
	int ConnectEpochFor(int playerId)
	{
		return EnsureConnectEpoch(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! True when the player's last deploy waits on the platform, or could not be authorized now.
	bool AwaitsPlatform(int playerId)
	{
		TBD_EDeployResult last;
		if (!m_mLastDeployResult || !m_mLastDeployResult.Find(playerId, last))
			return false;

		return last == TBD_EDeployResult.AUTHORIZING || last == TBD_EDeployResult.UNAUTHORIZED;
	}

	//------------------------------------------------------------------------------------------------
	//! The player's last deploy result, or NOT_MINE when none is recorded.
	TBD_EDeployResult LastDeployResult(int playerId)
	{
		TBD_EDeployResult last = TBD_EDeployResult.NOT_MINE;
		if (m_mLastDeployResult)
			m_mLastDeployResult.Find(playerId, last);

		return last;
	}

	//------------------------------------------------------------------------------------------------
	//! Drop the player's recorded result, so the next one read is from a deploy that follows.
	void ForgetDeployResult(int playerId)
	{
		if (m_mLastDeployResult)
			m_mLastDeployResult.Remove(playerId);
	}

	//------------------------------------------------------------------------------------------------
	protected void RecordDeployResult(int playerId, TBD_EDeployResult result)
	{
		if (!m_mLastDeployResult)
			m_mLastDeployResult = new map<int, TBD_EDeployResult>();

		m_mLastDeployResult.Set(playerId, result);
	}

	//------------------------------------------------------------------------------------------------
	override TBD_EDeployResult DeployPlayerEx(int playerId)
	{
		TBD_EDeployResult result = super.DeployPlayerEx(playerId);
		RecordDeployResult(playerId, result);
		return result;
	}

	//------------------------------------------------------------------------------------------------
	override TBD_EDeployResult AdminRespawn(int playerId, string byAdmin = "unknown")
	{
		TBD_EDeployResult result = super.AdminRespawn(playerId, byAdmin);
		RecordDeployResult(playerId, result);
		return result;
	}

	//------------------------------------------------------------------------------------------------
	//! The platform decides an event seat in every mode: while it decides, or cannot, the player is
	//! told so, and no walk-on stands in for its decision.
	override bool DeployOnReady(int playerId, out string why)
	{
		ForgetDeployResult(playerId);
		bool deployed = super.DeployOnReady(playerId, why);
		if (deployed || !AwaitsPlatform(playerId))
			return deployed;

		why = "checking your seat with the TBD platform - you deploy once it allows it";
		if (LastDeployResult(playerId) == TBD_EDeployResult.UNAUTHORIZED)
			why = "your seat cannot be authorized right now - the reason is in your chat";

		Print(string.Format("[TBD][Spawn] ready player=%1 -> path=slot result=%2 (%3)", playerId,
			typename.EnumToString(TBD_EDeployResult, LastDeployResult(playerId)), why));
		return false;
	}

	//------------------------------------------------------------------------------------------------
	override protected IEntity SpawnWalkOnBody(int playerId, out string why)
	{
		if (AwaitsPlatform(playerId))
		{
			why = "the TBD platform decides this seat - no walk-on";
			return null;
		}

		return super.SpawnWalkOnBody(playerId, why);
	}

	//------------------------------------------------------------------------------------------------
	//! TBD_DeploymentAuthorization: the platform allowed the deployment a deploy of this player
	//! waited on. The deploy continues through the retry step, which carries an admin respawn's
	//! override, marks the holder deployed and settles the admin respawn.
	void OnDeploymentAuthorized(int playerId, int epoch)
	{
		if (!IsSameConnection(playerId, epoch))
			return;

		m_mRetryCount.Remove(playerId);
		RetryDeploy(playerId, epoch);
	}

	//------------------------------------------------------------------------------------------------
	//! TBD_DeploymentAuthorization: the deployment a deploy of this player waited on was refused, and
	//! the player has been told why. A dead player waiting on an admin respawn stays dead in their
	//! seat; anyone else goes back to slot selection when the platform denied them the seat
	//! (`seatDenied`), and keeps it when the refusal was not about the seat.
	void OnDeploymentRefused(int playerId, int epoch, bool seatDenied)
	{
		if (!IsSameConnection(playerId, epoch))
			return;

		if (m_mAdminRespawnPending.Contains(playerId))
		{
			FinishAdminRespawn(playerId, TBD_EDeployResult.UNAUTHORIZED, "platform decision");
			return;
		}

		if (!seatDenied)
			return;

		// Give the seat back so the player picks another from the lobby. ReleaseSlot keeps a spent
		// life's seat (ONE LIFE) and a body the player already stands in.
		TBD_MissionSlotStruct slot = GetAssignedSlot(playerId);
		if (slot && ReleaseSlot(playerId))
			Print(string.Format("[TBD][Spawn] player=%1 seat %2 given back after the platform denied the deployment - back to slot selection", playerId, slot.Key()));
	}

	//------------------------------------------------------------------------------------------------
	//! The platform's decision continues an AUTHORIZING respawn, so it keeps the admin override
	//! meanwhile; every other outcome settles as before.
	override protected void FinishAdminRespawn(int playerId, TBD_EDeployResult r, string byAdmin)
	{
		if (r != TBD_EDeployResult.AUTHORIZING)
		{
			super.FinishAdminRespawn(playerId, r, byAdmin);
			return;
		}

		m_mAdminRespawnPending.Set(playerId, true);
		Print(string.Format("[TBD][Admin] respawn player=%1 by=%2 - awaiting the platform's deployment decision, player stays DEAD until it allows the new life", playerId, byAdmin));
	}

	//------------------------------------------------------------------------------------------------
	//! A different seat ends the life, or the request, of the old one. A request for the new seat,
	//! asked while the claim deployed the player, stays.
	override bool ClaimSlot(int playerId, string slotKey)
	{
		TBD_MissionSlotStruct previous = GetAssignedSlot(playerId);
		bool claimed = super.ClaimSlot(playerId, slotKey);

		TBD_MissionSlotStruct current = GetAssignedSlot(playerId);
		if (claimed && previous && current && previous.Key() != current.Key())
			TBD_DeploymentAuthorization.OnSeatChanged(playerId, previous.uid);

		return claimed;
	}

	//------------------------------------------------------------------------------------------------
	//! A seat given back takes an allowed life that never reached the world with it.
	override bool ReleaseSlot(int playerId)
	{
		bool released = super.ReleaseSlot(playerId);
		if (released)
			TBD_DeploymentAuthorization.EndLife(playerId, "gave the seat back");

		return released;
	}

	//------------------------------------------------------------------------------------------------
	//! The round is over: every life it opened ends on the platform too.
	override void OnStageChanged(TBD_EGameStage stage)
	{
		super.OnStageChanged(stage);

		if (stage == TBD_EGameStage.END || stage == TBD_EGameStage.DEBRIEF || stage == TBD_EGameStage.LOADING)
			TBD_DeploymentAuthorization.EndAllLives("round over at " + typename.EnumToString(TBD_EGameStage, stage));
	}

	//------------------------------------------------------------------------------------------------
	//! The platform learns that this life is over before anything else runs for the death, so a
	//! later life in the seat is always asked for anew.
	override void OnPlayerKilled(notnull SCR_InstigatorContextData instigatorContextData)
	{
		if (RplSession.Mode() != RplMode.Client)
		{
			int playerId = instigatorContextData.GetVictimPlayerID();
			if (playerId > 0)
				TBD_DeploymentAuthorization.EndLife(playerId, "killed");
		}

		super.OnPlayerKilled(instigatorContextData);
	}

	//------------------------------------------------------------------------------------------------
	override void OnPlayerDisconnected(int playerId, KickCauseCode cause, int timeout)
	{
		super.OnPlayerDisconnected(playerId, cause, timeout);

		if (RplSession.Mode() == RplMode.Client)
			return;

		ForgetDeployResult(playerId);
		TBD_DeploymentAuthorization.EndLife(playerId, "disconnected");
	}

	//------------------------------------------------------------------------------------------------
	//! The world is ending: every life it opened ends on the platform. The runtime session ends after
	//! the game mode components (TBD_RuntimeSessionLifecycle).
	override void OnGameEnd()
	{
		super.OnGameEnd();

		if (RplSession.Mode() == RplMode.Client)
			return;

		TBD_DeploymentAuthorization.EndAllLives("world ended");
	}
}
