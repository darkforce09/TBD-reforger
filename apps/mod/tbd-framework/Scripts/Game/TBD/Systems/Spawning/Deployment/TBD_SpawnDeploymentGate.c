/**
 * @file TBD_SpawnDeploymentGate.c
 * @brief The platform's say on a slot deploy, and its decisions continuing or refusing one.
 *
 * Role: asks TBD_DeploymentAuthorization whether a deploy into an event roster seat may proceed,
 * and applies the platform's later decision to the waiting deploy.  Position: TBD_DeployExecutor
 * calls Admits and Refusal once a seat is assigned and before any body is prepared;
 * TBD_DeploymentAuthorization calls OnAuthorized and OnRefused when a decision arrives.
 * State: the answer of the last refusal (static, read by the deploy immediately after Admits).
 * Invariants: a mission without deployment authorization admits every deploy; a player with no
 * connection epoch is RETRY; a decision for a connection that moved on does nothing; a refused
 * admin respawn keeps the player dead in their seat.
 */

//! Deployment authorization gate of the slot deploy.
class TBD_SpawnDeploymentGate
{
	protected static TBD_EDeployResult s_eRefusal = TBD_EDeployResult.RETRY; //!< answer of the last Admits that did not admit; default RETRY

	//! May `playerId` deploy into `slot` now? False leaves the answer for Refusal: RETRY while the
	//! player has no connection epoch, AUTHORIZING while the platform decides, UNAUTHORIZED when it
	//! cannot be authorized now (the seat is kept).
	//! @authority server
	static bool Admits(int playerId, notnull TBD_MissionSlotStruct slot)
	{
		s_eRefusal = TBD_EDeployResult.RETRY;
		if (!TBD_DeploymentAuthorization.AppliesToThisMission())
			return true;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return false;

		int epoch = spawn.ConnectionEpochFor(playerId);
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

	//! The deploy's answer when there is no seat yet (RETRY) or the last Admits did not admit.
	static TBD_EDeployResult Refusal(TBD_MissionSlotStruct slot)
	{
		if (!slot)
			return TBD_EDeployResult.RETRY;

		return s_eRefusal;
	}

	//! The platform allowed the deployment a deploy of `playerId` waited on: the deploy continues
	//! through the retry step, which carries an admin respawn's override, marks the holder deployed
	//! and settles the admin respawn.
	//! @authority server
	static void OnAuthorized(notnull TBD_SpawnManager spawn, int playerId, int epoch)
	{
		if (!spawn.GetEpochs().IsSameConnection(playerId, epoch))
			return;

		spawn.GetDeployWaves().ForgetRetries(playerId);
		spawn.GetDeployWaves().RetryDeploy(playerId, epoch);
	}

	//! The deployment a deploy of `playerId` waited on was refused and the player told why. A dead
	//! player waiting on an admin respawn stays dead in their seat; anyone else goes back to slot
	//! selection when the platform denied the seat (`seatDenied`), and keeps it otherwise.
	//! @authority server
	static void OnRefused(notnull TBD_SpawnManager spawn, int playerId, int epoch, bool seatDenied)
	{
		if (!spawn.GetEpochs().IsSameConnection(playerId, epoch))
			return;

		if (spawn.GetLives().IsAdminRespawnPending(playerId))
		{
			spawn.GetDeathFlow().FinishAdminRespawn(playerId, TBD_EDeployResult.UNAUTHORIZED, "platform decision");
			return;
		}

		if (!seatDenied)
			return;

		// Give the seat back so the player picks another from the lobby. ReleaseSlot keeps a spent
		// life's seat (ONE LIFE) and a body the player already stands in.
		TBD_MissionSlotStruct slot = spawn.GetAssignedSlot(playerId);
		if (slot && spawn.ReleaseSlot(playerId))
			Print(string.Format("[TBD][Spawn] player=%1 seat %2 given back after the platform denied the deployment - back to slot selection", playerId, slot.Key()));
	}
}
