/**
 * @file TBD_SpawnManager.c
 * @brief The game-mode component that owns slot bodies, seats, ONE LIFE and every deploy.
 *
 * Role: the spawn authority of a framework world: stands one dressed body per mission slot, seats
 * players, deploys them onto their slot body through vanilla's POSSESS request, and enforces ONE
 * LIFE.  Position: a component on the TBD game mode (frozen class name); engine hooks and the public
 * surface live here, the work lives in the helpers it owns (Slots/, Identity/, Deploy/, Lives/,
 * Manager/TBD_ConnectionEpochs); callers reach it through GetInstance.
 * State: the three editor attributes, the cached round stage and one owned instance of each helper;
 * server only.  Invariants: every deploy passes TBD_DeployExecutor.DeployPlayerInternal, the one-life
 * boundary; only an admin respawn bypasses it; vanilla spawns only on a NOT_MINE result.
 */

//! Editor class of TBD_SpawnManager.
[ComponentEditorProps(category: "TBD/Framework", description: "Server-only: slot-body materialization + claim/bind deploy from mission JSON.")]
class TBD_SpawnManagerClass : SCR_BaseGameModeComponentClass {}

//! The spawn authority of a framework world. Deploy = claim a seat, then hand the player onto the
//! pre-materialized slot body through vanilla's POSSESS request, which creates no second body.
//! @authority server
class TBD_SpawnManager : SCR_BaseGameModeComponent
{
	static const string IDENTITY_OVERRIDE_PHRASE = "I-ACCEPT-NO-ONE-LIFE"; //!< exact phrase an admin types to accept an unenforceable ONE LIFE; `#tbd` splits on spaces

	[Attribute("1", desc: "T-941.2: when ON, also seat unclaimed players at BRIEFING (PIE). Claimed holders always deploy on LOBBY to BRIEFING. Never fires during LOBBY. TBD_GameMode.et leaves claimed-holder briefing deploy on regardless.")]
	protected bool m_bAutoDeploy; //!< default 1: the BRIEFING holder deploy also seats every unclaimed player (PIE); the game mode prefab sets 0

	[Attribute("5000", desc: "Delay (ms) between death and automatic redeploy (auto-deploy worlds only).")]
	protected int m_iRedeployDelayMs; //!< default 5000 (ms): respawn beat before the automatic redeploy (ONE LIFE off)

	[Attribute("1", desc: "ONE LIFE: death is terminal; only an admin (AdminRespawn) can put a player back in.")]
	protected bool m_bOneLife; //!< default 1: death is terminal, the seat stays claimed, only AdminRespawn brings a player back

	protected TBD_EGameStage m_eStage = TBD_EGameStage.LOADING; //!< round stage cached from OnStageChanged; LOADING until the first change

	protected ref TBD_ConnectionEpochs m_Epochs; //!< connection epochs
	protected ref TBD_SpawnIdentityKeys m_Identity; //!< bind keys
	protected ref TBD_SpawnIdentityGate m_IdentityGate; //!< stage gate and waiver
	protected ref TBD_SpawnJoinAudit m_JoinAudit; //!< join door
	protected ref TBD_SlotClaimBook m_Slots; //!< seat ledger
	protected ref TBD_SlotBodyMaterializer m_Bodies; //!< slot bodies
	protected ref TBD_SlotLoadoutSettle m_LoadoutSettle; //!< spawn boundary
	protected ref TBD_PossessTicketLedger m_Tickets; //!< spawn tickets
	protected ref TBD_DeployExecutor m_DeployExecutor; //!< deploy decisions
	protected ref TBD_DeployWaves m_DeployWaves; //!< scheduled deploys and retries
	protected ref TBD_DeployWatchdog m_DeployWatchdog; //!< post-deploy watchdog
	protected ref TBD_ReadyDeploy m_ReadyDeploy; //!< Ready and Continue
	protected ref TBD_OneLifeLedger m_Lives; //!< spent lives
	protected ref TBD_DeathRespawnFlow m_DeathFlow; //!< deaths and admin respawns
	protected ref TBD_SpawnDeparture m_Departure; //!< disconnects

	//! Create the helpers.
	void TBD_SpawnManager(IEntityComponentSource src, IEntity ent, IEntity parent)
	{
		m_Epochs = new TBD_ConnectionEpochs();
		m_Identity = new TBD_SpawnIdentityKeys();
		m_IdentityGate = new TBD_SpawnIdentityGate(this);
		m_JoinAudit = new TBD_SpawnJoinAudit(this);
		m_Slots = new TBD_SlotClaimBook(this);
		m_Bodies = new TBD_SlotBodyMaterializer(this);
		m_LoadoutSettle = new TBD_SlotLoadoutSettle(this);
		m_Tickets = new TBD_PossessTicketLedger();
		m_DeployExecutor = new TBD_DeployExecutor(this);
		m_DeployWaves = new TBD_DeployWaves(this);
		m_DeployWatchdog = new TBD_DeployWatchdog(this);
		m_ReadyDeploy = new TBD_ReadyDeploy(this);
		m_Lives = new TBD_OneLifeLedger(this);
		m_DeathFlow = new TBD_DeathRespawnFlow(this);
		m_Departure = new TBD_SpawnDeparture(this);
	}

	//! The spawn manager on the currently loaded world's game mode, or null. Resolved on every call:
	//! a static would outlive a world restarted in-process.
	static TBD_SpawnManager GetInstance()
	{
		SCR_BaseGameMode gameMode = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
		if (!gameMode)
			return null;

		return TBD_SpawnManager.Cast(gameMode.FindComponent(TBD_SpawnManager));
	}

	//! True when ONE LIFE is on.
	bool IsOneLife()
	{
		return m_bOneLife;
	}

	//! True when automatic deploy is on.
	bool IsAutoDeploy()
	{
		return m_bAutoDeploy;
	}

	//! Delay (ms) between a death and the automatic redeploy.
	int RedeployDelayMs()
	{
		return m_iRedeployDelayMs;
	}

	//! The round stage cached from OnStageChanged.
	TBD_EGameStage GetStage()
	{
		return m_eStage;
	}

	//! Connection epochs.
	TBD_ConnectionEpochs GetEpochs() { return m_Epochs; }
	//! Bind keys.
	TBD_SpawnIdentityKeys GetIdentity() { return m_Identity; }
	//! Identity stage gate.
	TBD_SpawnIdentityGate GetIdentityGate() { return m_IdentityGate; }
	//! Join door.
	TBD_SpawnJoinAudit GetJoinAudit() { return m_JoinAudit; }
	//! Seat ledger.
	TBD_SlotClaimBook GetSlots() { return m_Slots; }
	//! Slot bodies.
	TBD_SlotBodyMaterializer GetBodies() { return m_Bodies; }
	//! Loadout settle.
	TBD_SlotLoadoutSettle GetLoadoutSettle() { return m_LoadoutSettle; }
	//! Spawn tickets.
	TBD_PossessTicketLedger GetTickets() { return m_Tickets; }
	//! Deploy decisions.
	TBD_DeployExecutor GetDeployExecutor() { return m_DeployExecutor; }
	//! Scheduled deploys.
	TBD_DeployWaves GetDeployWaves() { return m_DeployWaves; }
	//! Post-deploy watchdog.
	TBD_DeployWatchdog GetDeployWatchdog() { return m_DeployWatchdog; }
	//! Spent lives.
	TBD_OneLifeLedger GetLives() { return m_Lives; }
	//! Deaths and admin respawns.
	TBD_DeathRespawnFlow GetDeathFlow() { return m_DeathFlow; }

	//! True once the slot lineup stands and passed the loadout settle.
	bool AreSlotBodiesMaterialized() { return m_Bodies.AreMaterialized(); }
	//! True while the slot loadouts are still being assessed.
	bool IsLoadoutSettlePending() { return m_LoadoutSettle.IsPending(); }
	//! The connection epoch of `playerId`, opened on demand for a connected player; 0 when none.
	int ConnectionEpochFor(int playerId) { return m_Epochs.EnsureConnectEpoch(playerId); }
	//! True while the player on `playerId` is the one `epoch` was taken for.
	bool IsConnectionCurrent(int playerId, int epoch) { return m_Epochs.IsSameConnection(playerId, epoch); }
	//! Seat `playerId` automatically; see TBD_SlotClaimBook.AssignSlotForPlayer.
	void AssignSlotForPlayer(int playerId) { m_Slots.AssignSlotForPlayer(playerId); }
	//! The slot assigned to `playerId`, or null.
	TBD_MissionSlotStruct GetAssignedSlot(int playerId) { return m_Slots.GetAssignedSlot(playerId); }
	//! The body standing on `slotKey`, or null.
	IEntity GetSlotBody(string slotKey) { return m_Bodies.GetSlotBody(slotKey); }
	//! Claim a seat from the slot picker; see TBD_SlotClaimBook.ClaimSlot.
	bool ClaimSlot(int playerId, string slotKey) { return m_Slots.ClaimSlot(playerId, slotKey); }
	//! Give a seat back before deploying; see TBD_SlotClaimBook.ReleaseSlot.
	bool ReleaseSlot(int playerId) { return m_Slots.ReleaseSlot(playerId); }
	//! The lobby roster wire; see TBD_SlotRosterWire.
	array<string> BuildSlotRoster() { return TBD_SlotRosterWire.Build(this); }
	//! Stand the slot lineup; see TBD_SlotBodyMaterializer.MaterializeSlotBodies.
	void MaterializeSlotBodies() { m_Bodies.MaterializeSlotBodies(); }
	//! Why `stage` may not be entered, or empty; see TBD_SpawnIdentityGate.StageRefusal.
	static string StageRefusalFor(TBD_EGameStage stage) { return TBD_SpawnIdentityGate.StageRefusal(stage); }
	//! Sign the ONE LIFE identity waiver; see TBD_SpawnIdentityGate.AcceptNonDurableIdentity.
	bool AcceptNonDurableIdentity(string byAdmin, string phrase) { return m_IdentityGate.AcceptNonDurableIdentity(byAdmin, phrase); }
	//! Revoke the identity waiver.
	void RequireDurableIdentity(string byAdmin) { m_IdentityGate.RequireDurableIdentity(byAdmin); }
	//! The one-line identity verdict of this host.
	string IdentityStatusLine() { return m_IdentityGate.IdentityStatusLine(); }
	//! Deploy `playerId` through the one-life boundary; see TBD_DeployExecutor.
	TBD_EDeployResult DeployPlayerEx(int playerId) { return m_DeployExecutor.DeployPlayerEx(playerId); }
	//! Ready and Continue; see TBD_ReadyDeploy.
	bool DeployOnReady(int playerId, out string why) { return m_ReadyDeploy.DeployOnReady(playerId, why); }
	//! True when `playerId` has spent their one life.
	bool IsPlayerDead(int playerId) { return m_Lives.IsPlayerDead(playerId); }
	//! Seat holders of `factionKey` still alive.
	int CountAliveForFaction(string factionKey) { return m_Slots.CountAliveForFaction(factionKey); }
	//! Seats of `factionKey` claimed at all.
	int CountClaimedForFaction(string factionKey) { return m_Slots.CountClaimedForFaction(factionKey); }
	//! The last deploy result of `playerId`, or NOT_MINE.
	TBD_EDeployResult LastDeployResult(int playerId) { return m_DeployExecutor.LastDeployResult(playerId); }
	//! Drop the recorded deploy result of `playerId`.
	void ForgetDeployResult(int playerId) { m_DeployExecutor.ForgetDeployResult(playerId); }

	//! Admin respawn of a spent life; see TBD_DeathRespawnFlow.AdminRespawn.
	//! @authority server
	TBD_EDeployResult AdminRespawn(int playerId, string byAdmin = "unknown")
	{
		return m_DeathFlow.AdminRespawn(playerId, byAdmin);
	}

	//! Track the round stage: deploy the claimed holders on LOBBY to BRIEFING, and end every
	//! platform life when the round ends or the world reloads. TBD_FrameworkManager.SetStage calls
	//! it on every transition.
	void OnStageChanged(TBD_EGameStage stage)
	{
		TBD_EGameStage previous = m_eStage;
		m_eStage = stage;
		m_DeployWaves.OnStageChanged(previous, stage);

		if (stage == TBD_EGameStage.END || stage == TBD_EGameStage.DEBRIEF || stage == TBD_EGameStage.LOADING)
			TBD_DeploymentAuthorization.EndAllLives("round over at " + typename.EnumToString(TBD_EGameStage, stage));
	}

	//! Subscribe the watchdog to the game mode's spawn invoker (components have no spawn virtual).
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(owner);
		if (gm)
			gm.GetOnPlayerSpawned().Insert(m_DeployWatchdog.OnPlayerSpawnedHook);
	}

	//! Unsubscribe the spawn invoker and cancel every pending helper callback.
	override void OnDelete(IEntity owner)
	{
		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(owner);
		if (gm)
			gm.GetOnPlayerSpawned().Remove(m_DeployWatchdog.OnPlayerSpawnedHook);

		m_JoinAudit.CancelCallbacks();
		m_Bodies.CancelCallbacks();
		m_LoadoutSettle.CancelCallbacks();
		m_Tickets.CancelCallbacks();
		m_DeployExecutor.CancelCallbacks();
		m_DeployWaves.CancelCallbacks();
		m_DeployWatchdog.CancelCallbacks();
		m_DeathFlow.CancelCallbacks();

		super.OnDelete(owner);
	}

	//! The join hook that fires on a framework world; see TBD_SpawnJoinAudit.OnAudit.
	//! @authority server
	override void OnPlayerAuditSuccess(int playerId)
	{
		super.OnPlayerAuditSuccess(playerId);

		if (TBD_Authority.IsClient())
			return;

		m_JoinAudit.OnAudit(playerId);
	}

	//! End the platform life first, so a later life in the seat is asked for anew; then the death
	//! flow (TBD_DeathRespawnFlow.OnPlayerKilled).
	//! @authority server
	override void OnPlayerKilled(notnull SCR_InstigatorContextData instigatorContextData)
	{
		if (TBD_Authority.IsServer())
		{
			int playerId = instigatorContextData.GetVictimPlayerID();
			if (playerId > 0)
				TBD_DeploymentAuthorization.EndLife(playerId, "killed");
		}

		super.OnPlayerKilled(instigatorContextData);

		if (TBD_Authority.IsClient())
			return;

		m_DeathFlow.OnPlayerKilled(instigatorContextData);
	}

	//! Tear the player out of the spawn state (TBD_SpawnDeparture), then end their platform life.
	//! @authority server
	override void OnPlayerDisconnected(int playerId, KickCauseCode cause, int timeout)
	{
		super.OnPlayerDisconnected(playerId, cause, timeout);

		if (TBD_Authority.IsClient())
			return;

		m_Departure.OnPlayerDisconnected(playerId);
		m_DeployExecutor.ForgetDeployResult(playerId);
		TBD_DeploymentAuthorization.EndLife(playerId, "disconnected");
	}

	//! The world is ending: every life it opened ends on the platform. The runtime session ends after
	//! the game mode components (TBD_RuntimeSessionLifecycle).
	//! @authority server
	override void OnGameEnd()
	{
		super.OnGameEnd();

		if (TBD_Authority.IsClient())
			return;

		TBD_DeploymentAuthorization.EndAllLives("world ended");
	}
}
