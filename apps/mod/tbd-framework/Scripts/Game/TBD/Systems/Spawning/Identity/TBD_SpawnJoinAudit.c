/**
 * @file TBD_SpawnJoinAudit.c
 * @brief The join door: what the spawn manager decides when a player's audit succeeds.
 *
 * Role: resolves the joiner's bind key, opens a fresh connection epoch, hands a returning spent
 * life its seat back, logs one join verdict and schedules the join-in-progress deploy.
 * Position: TBD_SpawnManager.OnPlayerAuditSuccess calls OnAudit on the authority; this is the only
 * join hook that fires on a framework world (the modded SCR_RespawnSystemComponent swallows
 * vanilla's).
 * State: the per-connection audited key and the session's seen keys, server only.
 * Invariants: idempotent per connection (vanilla can audit twice); a repeat audit is swallowed only
 * when the first resolved an identity key, so a NUMERIC first audit can upgrade; a spent life is
 * never deployed; lateness alone never refuses a deploy.
 */

//! Join-in-progress handling of the spawn manager.
class TBD_SpawnJoinAudit : Managed
{
	protected const int JIP_DEPLOY_DELAY_MS = 250; //!< delay (ms) between the audit and the join deploy, matching the stage-entry settle

	protected TBD_SpawnManager m_Spawn; //!< owning manager
	protected ref map<int, string> m_mAuditedKey; //!< playerId to the key this hook resolved for its connection; only written here, erased at disconnect
	protected ref map<string, bool> m_mSeenKeys; //!< identity keys that joined this session, for FIRST versus RECONNECT

	//! Bind the audit to its manager.
	void TBD_SpawnJoinAudit(TBD_SpawnManager spawn)
	{
		m_Spawn = spawn;
		m_mAuditedKey = new map<int, string>();
		m_mSeenKeys = new map<string, bool>();
	}

	//! Cancel the pending join deploys.
	void CancelCallbacks()
	{
		GetGame().GetCallqueue().Remove(DeployJoiner);
	}

	//! Forget the audited key of a departing player, so a recycled id is audited afresh.
	void ForgetPlayer(int playerId)
	{
		m_mAuditedKey.Remove(playerId);
	}

	//! Handle one successful audit: key, epoch, seat hand-back, verdict line and deploy decision.
	//! A spent life is refused, a mission's flow.jip policy can refuse, LOADING/END/DEBRIEF wait,
	//! LOBBY and seatless players go to the picker; everyone else deploys after a short delay.
	//! @authority server
	void OnAudit(int playerId)
	{
		TBD_SpawnIdentityKeys identity = m_Spawn.GetIdentity();
		TBD_EGameStage stage = m_Spawn.GetStage();

		string priorKey;
		if (m_mAuditedKey.Find(playerId, priorKey) && TBD_SpawnIdentityKeys.IsIdentityKey(priorKey))
		{
			Print(string.Format("[TBD][JIP] player=%1 duplicate audit ignored -- already joined this connection as %2", playerId, priorKey));
			return;
		}

		identity.ForgetCachedKey(playerId);
		string bindKey = identity.PlayerBindKey(playerId);
		m_mAuditedKey.Set(playerId, bindKey);

		int epoch = m_Spawn.GetEpochs().OpenFreshEpoch(playerId);

		bool seenBefore = false;
		if (TBD_SpawnIdentityKeys.IsIdentityKey(bindKey))
		{
			seenBefore = m_mSeenKeys.Contains(bindKey);
			m_mSeenKeys.Set(bindKey, true);
		}

		if (m_Spawn.IsOneLife() && !TBD_SpawnIdentityKeys.IsIdentityKey(bindKey) && TBD_SpawnIdentityGate.RequiresDurableIdentity(stage))
			m_Spawn.GetIdentityGate().NoteLateNonDurableJoin(playerId);

		bool reclaimed = m_Spawn.GetSlots().ReclaimDepartedSeat(playerId, bindKey);
		bool lifeSpent = m_Spawn.GetLives().IsBindKeyDead(bindKey);

		string action = "DEPLOY";
		bool deploy = true;

		if (m_Spawn.IsOneLife() && lifeSpent)
		{
			action = "DENIED-life-spent";
			deploy = false;
		}
		else if (!TBD_JipPolicy.AllowsJoinAtStage(stage))
		{
			action = "DENIED-jip-" + TBD_JipPolicy.Name();
			deploy = false;
		}
		else if (!IsStageDeployable())
		{
			action = "WAIT-stage-not-deployable";
			deploy = false;
		}
		else if (stage == TBD_EGameStage.LOBBY)
		{
			action = "PICKER-lobby";
			deploy = false;
		}
		else if (!m_Spawn.GetAssignedSlot(playerId))
		{
			action = "PICKER-no-slot";
			deploy = false;
		}
		else if (m_Spawn.GetDeployWaves().IsHolderDeployed(playerId))
		{
			action = "ALREADY-holder-deployed";
			deploy = false;
		}

		LogJoinVerdict(playerId, bindKey, seenBefore, reclaimed, lifeSpent, action);

		if (deploy)
			GetGame().GetCallqueue().CallLater(DeployJoiner, JIP_DEPLOY_DELAY_MS, false, playerId, epoch);
	}

	//! One line answering every question a reconnect test asks: join kind, key, key mode, life,
	//! seat, stage and the action taken.
	//! @authority server
	protected void LogJoinVerdict(int playerId, string bindKey, bool seenBefore, bool reclaimed, bool lifeSpent, string action)
	{
		string joinKind = "FIRST";
		if (seenBefore)
			joinKind = "RECONNECT";

		string lifeLabel = "OK";
		if (lifeSpent)
			lifeLabel = "SPENT";

		string seatLabel = "none-yet";
		TBD_MissionSlotStruct slot = m_Spawn.GetAssignedSlot(playerId);
		if (slot)
		{
			seatLabel = slot.Key();
			if (reclaimed)
				seatLabel = seatLabel + "(reclaimed)";
		}

		string line = string.Format("[TBD][JIP] player=%1 join=%2 key=%3 keyMode=%4",
			playerId, joinKind, bindKey, TBD_SpawnIdentityKeys.KeyModeLabel(bindKey));
		line = line + string.Format(" life=%1 seat=%2 stage=%3 action=%4",
			lifeLabel, seatLabel, typename.EnumToString(TBD_EGameStage, m_Spawn.GetStage()), action);
		Print(line);
	}

	//! True in the stages where putting a player in the world means something: LOBBY, BRIEFING,
	//! SAFE_START and LIVE.
	protected bool IsStageDeployable()
	{
		TBD_EGameStage stage = m_Spawn.GetStage();
		return stage == TBD_EGameStage.LOBBY
			|| stage == TBD_EGameStage.BRIEFING
			|| stage == TBD_EGameStage.SAFE_START
			|| stage == TBD_EGameStage.LIVE;
	}

	//! Deploy a player who joined mid-round, unless the connection moved on or a body is already
	//! requested or delivered. Deferred because the player controller is not reliably present at
	//! audit time.
	//! @authority server
	protected void DeployJoiner(int playerId, int epoch)
	{
		if (!m_Spawn.GetEpochs().IsSameConnection(playerId, epoch))
			return;

		if (m_Spawn.GetDeployExecutor().HasRequested(playerId) || m_Spawn.GetDeployWaves().IsHolderDeployed(playerId))
			return;

		m_Spawn.GetDeployWaves().DeployOnPath(playerId, "jip");
	}
}
