/**
 * @file TBD_FrameworkManager.c
 * @brief The game mode component that loads the mission and owns the round's stage machine.
 *
 * Role: owns the one round stage and the one way to change it (SetStage), fans each transition out
 * to the subsystems and the local UI, and holds the replicated authored settings and END banner.
 * Position: a component on TBD_GameMode.et (frozen class name); engine hooks, the replicated
 * fields and the public surface live here, the work lives in the helpers it owns (Stage/,
 * TBD_FrameworkRollCall, TBD_MissionFlowReport); callers reach it through GetInstance.
 * State: six replicated fields (stage, spectator policy, night vision, end winner, end reason,
 * debrief board), the last refusal reason and one instance of each helper; mission load and the
 * stage machine run on the server, the local UI hook on every machine with a workspace.
 * Invariants: the stage changes only through SetStage and a round ends only through
 * SetStage(END); every replicated write is followed by Replication.BumpMe on this component;
 * OnDelete cancels every call-queue entry the component and its helpers armed.
 */

//! Editor class of TBD_FrameworkManager.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD platform game mode manager -- mission load and stage machine.")]
class TBD_FrameworkManagerClass : SCR_BaseGameModeComponentClass {}

//! The round's stage machine and mission load of a framework world.
class TBD_FrameworkManager : SCR_BaseGameModeComponent
{
	//! @replicated m_Stage
	[RplProp(onRplName: "OnStageReplicated")]
	protected TBD_EGameStage m_Stage = TBD_EGameStage.LOADING; //!< current round stage; server-owned, LOADING until the load settles

	//! @replicated m_sSpectatorPolicy
	[RplProp()]
	protected string m_sSpectatorPolicy; //!< authored `settings.spectatorPolicy`; empty when absent; read by the client spectator controller

	//! @replicated m_bNightVision
	[RplProp()]
	protected bool m_bNightVision; //!< authored `settings.nightVision`; false when absent; false strips night-vision gadgets on spawn

	//! @replicated m_sEndWinner
	[RplProp()]
	protected string m_sEndWinner; //!< END banner winner key; empty when none was named

	//! @replicated m_sEndReason
	[RplProp()]
	protected string m_sEndReason; //!< END banner reason (`faction_eliminated`, `time_limit`, an objective trigger, `admin`)

	//! @replicated m_sDebriefBoard
	[RplProp()]
	protected string m_sDebriefBoard; //!< packed scoreboard (`kills\tfaction\trole\tname` per line); empty before END

	protected string m_sLastStageRefusal; //!< why the last SetStage refused; empty when it did not

	protected ref TBD_LoadingGate m_LoadingGate; //!< LOADING to LOBBY
	protected ref TBD_StageEnvironment m_Environment; //!< wind direction and night-vision strip
	protected ref TBD_EndBanner m_EndBanner; //!< END banner decision and kill tally
	protected ref TBD_FactionElimination m_FactionElimination; //!< objective and elimination end check
	protected ref TBD_RoundClock m_RoundClock; //!< authored round clock

	//! Create the helpers.
	void TBD_FrameworkManager(IEntityComponentSource src, IEntity ent, IEntity parent)
	{
		m_LoadingGate = new TBD_LoadingGate(this);
		m_Environment = new TBD_StageEnvironment(this);
		m_EndBanner = new TBD_EndBanner();
		m_FactionElimination = new TBD_FactionElimination(this);
		m_RoundClock = new TBD_RoundClock(this);
	}

	//! The framework manager on the loaded world, resolved off the live game mode on every call:
	//! statics outlive a world when a fleet command restarts the mission in-process.
	//! @return the manager, or null when the world has none
	static TBD_FrameworkManager GetInstance()
	{
		SCR_BaseGameMode gameMode = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
		if (!gameMode)
			return null;

		return TBD_FrameworkManager.Cast(gameMode.FindComponent(TBD_FrameworkManager));
	}

	//! Whether the loaded world runs the TBD framework; every vanilla-suppressing modded class asks
	//! this before standing vanilla down.
	//! @return true when the game mode carries this component
	static bool IsFrameworkWorld()
	{
		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
		if (!gm)
			return false;

		return gm.FindComponent(TBD_FrameworkManager) != null;
	}

	//! The current round stage.
	//! @return the stage
	TBD_EGameStage GetStage()
	{
		return m_Stage;
	}

	//! Why the last SetStage refused, for the admin surfaces.
	//! @return the reason, or empty when the last call was not refused
	string GetLastStageRefusal()
	{
		return m_sLastStageRefusal;
	}

	//! The END banner's winner key.
	//! @return the key, or empty
	string GetEndWinner()
	{
		return m_sEndWinner;
	}

	//! Why the round ended.
	//! @return the reason, or empty before the first END
	string GetEndReason()
	{
		return m_sEndReason;
	}

	//! The packed DEBRIEF scoreboard.
	//! @return the packed rows, or empty before END
	string GetDebriefBoard()
	{
		return m_sDebriefBoard;
	}

	//! Kills credited to a player this round; authority only.
	//! @param playerId the player
	//! @return the kill count, 0 when unknown
	int GetKills(int playerId)
	{
		return m_EndBanner.GetKills(playerId);
	}

	//! Authored `settings.spectatorPolicy`, read by the client spectator controller.
	//! @return the policy, or empty when the mission omitted it
	string GetSpectatorPolicy()
	{
		return m_sSpectatorPolicy;
	}

	//! Authored `settings.nightVision`.
	//! @return true when night-vision gadgets are allowed; false when unauthored
	bool IsNightVisionAllowed()
	{
		return m_bNightVision;
	}

	//! Subscribe the night-vision strip, schedule the roll-call; on the server start the mission load.
	//! @param owner the game mode entity
	//! @authority server
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(owner);
		if (gm)
			gm.GetOnPlayerSpawned().Insert(m_Environment.OnPlayerSpawnedApplyNvg);

		// One frame later, so sibling components are constructed before the roll-call asks.
		GetGame().GetCallqueue().CallLater(PrintComponentRollCall, 0);

		if (TBD_Authority.IsClient())
			return;

		SetStage(TBD_EGameStage.LOADING);
		TBD_MissionLoader.BeginLoad();
		m_LoadingGate.Begin();
	}

	//! Cancel every call-queue entry this component and its helpers armed, because a fleet command
	//! restarts the mission in-process and a surviving timer would fire into the next world.
	//! @param owner the game mode entity
	override void OnDelete(IEntity owner)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(PrintComponentRollCall);

		m_LoadingGate.CancelCallbacks();
		m_FactionElimination.CancelCallbacks();
		m_RoundClock.CancelCallbacks();
		m_Environment.CancelCallbacks();

		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(owner);
		if (gm)
			gm.GetOnPlayerSpawned().Remove(m_Environment.OnPlayerSpawnedApplyNvg);

		super.OnDelete(owner);
	}

	//! Print the component roll-call of the owning game mode.
	protected void PrintComponentRollCall()
	{
		TBD_FrameworkRollCall.Report(GetOwner());
	}

	//! Latch the authored `settings` onto the replicated fields and log them.
	//! @authority server
	void LatchAuthoredSettings()
	{
		TBD_MissionSettingsStruct settings = TBD_MissionLoader.GetSettings();
		m_sSpectatorPolicy = string.Empty;
		m_bNightVision = false;
		if (settings)
		{
			m_sSpectatorPolicy = settings.spectatorPolicy;
			m_bNightVision = settings.nightVision;
		}
		Replication.BumpMe();

		if (settings)
			TBD_StageEnvironment.ReportSettings(m_sSpectatorPolicy, m_bNightVision);
	}

	//! Credit a player kill while LIVE; team kills, suicides and world or AI kills are ignored.
	//! @param instigatorContextData the engine's kill context
	//! @authority server
	override void OnPlayerKilled(notnull SCR_InstigatorContextData instigatorContextData)
	{
		super.OnPlayerKilled(instigatorContextData);

		if (TBD_Authority.IsClient() || m_Stage != TBD_EGameStage.LIVE)
			return;

		m_EndBanner.CreditKill(instigatorContextData.GetKillerPlayerID(), instigatorContextData.GetVictimPlayerID());
	}

	//! End the round with a named reason and winner, through SetStage(END).
	//! @param reason the end reason the banner shows
	//! @param winner the winning faction key, empty when none
	//! @authority server
	void EndRound(string reason, string winner)
	{
		m_EndBanner.SetPending(reason, winner);
		SetStage(TBD_EGameStage.END);
	}

	//! Move the round to `stage` unless refused (reason kept for GetLastStageRefusal): fix or clear
	//! the END banner, replicate, log, notify the subsystems and local UI, run the stage's hook.
	//! @param stage the target stage; the current stage is a no-op with an empty refusal
	//! @authority server
	void SetStage(TBD_EGameStage stage)
	{
		// Cleared before the same-stage early-out, so a no-op never replays an older refusal.
		m_sLastStageRefusal = string.Empty;

		if (m_Stage == stage)
			return;

		// SAFE_START promises that nobody can be hurt; without the component nothing keeps it.
		if (stage == TBD_EGameStage.SAFE_START && !TBD_SafestartManager.GetInstance())
		{
			m_sLastStageRefusal = "SAFE_START has no enforcement on this world (TBD_SafestartManager is missing from the game mode prefab) -- go straight to LIVE with '#tbd stage LIVE', and warn players that weapons are hot.";
			TBD_Log.Banner(TBD_Log.CH_SAFESTART,
				"SAFE_START REFUSED -- TBD_SafestartManager is not on the game mode; nothing would enforce damage-off",
				true);
			return;
		}

		// One life, identity and loadout delivery gates.
		m_sLastStageRefusal = TBD_SpawnManager.StageRefusalFor(stage);
		if (!m_sLastStageRefusal.IsEmpty())
			return;

		TBD_EGameStage previous = m_Stage;
		m_Stage = stage;
		if (stage == TBD_EGameStage.END)
		{
			string endWinner;
			string endReason;
			m_EndBanner.Resolve(endWinner, endReason);
			m_sEndWinner = endWinner;
			m_sEndReason = endReason;
			SnapshotDebriefBoard();
		}
		else if (stage == TBD_EGameStage.DEBRIEF)
		{
			SnapshotDebriefBoard();
		}
		else if (stage == TBD_EGameStage.LOADING || stage == TBD_EGameStage.LOBBY)
		{
			m_sEndWinner = string.Empty;
			m_sEndReason = string.Empty;
			m_sDebriefBoard = string.Empty;
			m_EndBanner.Clear();
		}
		Replication.BumpMe();

		// Before the fan-out; `[TBD] Stage` below is quoted by the staging runbook and remote-logs.
		TBD_Log.Stage(previous, stage);

		TBD_RadioBridgeStub.OnStageChanged(stage);

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm)
			sm.OnStageChanged(stage);

		// Every transition: LOBBY, BRIEFING and SAFE_START arm the shield, any other stage lifts it.
		TBD_SafestartManager safestart = TBD_SafestartManager.GetInstance();
		if (safestart)
			safestart.OnStageChanged(stage);

		Print("[TBD] Stage -> " + typename.EnumToString(TBD_EGameStage, stage));

		// The replication hook never fires on the authority, so a listen host is told here.
		NotifyLocalStageUI();

		if (stage == TBD_EGameStage.LOBBY)
			OnEnterLobby();
		else if (stage == TBD_EGameStage.BRIEFING)
			TBD_MissionFlowReport.AnnounceBriefing();
		else if (stage == TBD_EGameStage.LIVE)
			OnEnterLive();
		else if (stage == TBD_EGameStage.END)
			TBD_EndBanner.AnnounceEnd(m_sEndWinner, m_sEndReason);
	}

	//! Pack the scoreboard into the replicated board; the server only.
	//! @authority server
	protected void SnapshotDebriefBoard()
	{
		if (TBD_Authority.IsClient())
			return;

		m_sDebriefBoard = TBD_EndBanner.PackDebriefBoard();
	}

	//! LOBBY hook: preload the deployable mission list so admins can browse and deploy at once.
	protected void OnEnterLobby()
	{
		TBD_DeployableMissionList.Refresh();
	}

	//! LIVE hook: arm the end checks and the round clock, independently of each other.
	//! @authority server
	protected void OnEnterLive()
	{
		if (TBD_Authority.IsClient())
			return;

		m_FactionElimination.Arm();
		m_RoundClock.Arm();
	}

	//! Replication hook of m_Stage: drive this machine's local UI.
	//! @authority client
	void OnStageReplicated()
	{
		NotifyLocalStageUI();
	}

	//! Hand the stage to this machine's END and DEBRIEF overlays and player controller; called from
	//! OnStageReplicated and from SetStage (a listen host gets no hook). Local UI only.
	protected void NotifyLocalStageUI()
	{
		TBD_EndBanner.ApplyEndScreens(m_Stage);

		// No workspace: a dedicated server, which drives no menu.
		if (!GetGame().GetWorkspace())
			return;

		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		// Idempotent: TBD_OnStageChanged acts on transitions only.
		pc.TBD_OnStageChanged(m_Stage);
	}

	//! Admin stage command: `next` advances one stage (never past DEBRIEF), a stage name jumps to it,
	//! anything else is ignored.
	//! @param args `next` or a TBD_EGameStage name
	//! @authority server
	void HandleAdminStageCommand(string args)
	{
		if (args.IsEmpty())
			return;

		if (args == "next")
		{
			int next = m_Stage + 1;
			if (next > TBD_EGameStage.DEBRIEF)
				return;
			SetStage(next);
			return;
		}

		for (int i = TBD_EGameStage.LOADING; i <= TBD_EGameStage.DEBRIEF; i++)
		{
			string name = typename.EnumToString(TBD_EGameStage, i);
			if (args == name)
			{
				SetStage(i);
				return;
			}
		}
	}
}
