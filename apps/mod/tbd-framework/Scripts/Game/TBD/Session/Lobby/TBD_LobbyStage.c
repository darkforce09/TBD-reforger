/**
 * @file TBD_LobbyStage.c
 * @brief Client stage watcher that raises the pre-game screens in LOBBY and closes them after.
 *
 * Role: polls TBD_FrameworkManager.GetStage() every POLL_MS; on entering LOBBY resets
 * TBD_LobbyClient and raises the Mission Selector (the Lobby and Briefing tabs are reached from its
 * top bar); while the stage stays in LOBBY raises it again when no pre-game screen is open, the
 * player has no body and has not been told to stand down; on any other stage closes the selector
 * and the lobby.  Position: started and shut down by TBD_LobbyComponent; opens screens through
 * TBD_MenuStack. The replicated stage's own hook drives the end overlays, not this watcher.
 * State: the running flag, last stage, arm attempts, last raise outcome and the preset-unavailable
 * latch, as process-wide statics; client only.
 * Invariants: Start refuses on a dedicated server and logs it; arming retries every ARM_RETRY_MS up
 * to ARM_MAX_ATTEMPTS and logs the give-up; the first raise of a round is unconditional, only the
 * re-raise is gated; every raise outcome logs once per change, never per poll; statics are reset
 * on every Start and Shutdown, because a world can be torn down without its OnDelete.
 */

//! Stage watcher of the pre-game screens.
class TBD_LobbyStage
{
	static const int POLL_MS = 500; //!< ms between stage polls

	static const int ARM_RETRY_MS = 250; //!< ms between IsFrameworkWorld retries while arming
	static const int ARM_MAX_ATTEMPTS = 60; //!< arming retries before a logged give-up: 60 x 250 ms = 15 s

	static const string LOG_TAG = "[TBD][Lobby] "; //!< prefix of every watcher line, so one grep returns the picker's lifecycle

	static const int RAISE_UNSEEN        = 0; //!< raise outcome: nothing decided yet on this world
	static const int RAISE_OPENED        = 1; //!< raise outcome: the selector opened
	static const int RAISE_NO_CONTROLLER = 2; //!< raise outcome: deferred, no local controller yet
	static const int RAISE_PRESET_DEAD   = 3; //!< raise outcome: refused, the preset is latched unavailable
	static const int RAISE_ALREADY_OPEN  = 4; //!< raise outcome: skipped, a pre-game screen is up
	static const int RAISE_OPEN_FAILED   = 5; //!< raise outcome: TBD_MenuStack.Open returned null

	protected static bool s_bRunning; //!< Tick is armed
	protected static TBD_EGameStage s_LastStage; //!< stage seen by the last Tick; LOADING when reset

	protected static bool s_bPresetUnavailable; //!< TBD_MenuStack.Open returned null this round; stops the re-raise from logging every poll

	protected static int s_iArmAttempts; //!< IsFrameworkWorld retries so far
	protected static int s_iLastRaiseOutcome; //!< RAISE_* of the last Raise; RAISE_UNSEEN when reset

	//! Print LOG_TAG + `message` at NORMAL through PrintFormat.
	protected static void Log(string message)
	{
		PrintFormat("%1", LOG_TAG + message, level: LogLevel.NORMAL);
	}

	//! Print LOG_TAG + `message` at WARNING: the world-boot gate fails on a TBD ERROR line, so a state the gate can reach is never an ERROR.
	protected static void LogWarn(string message)
	{
		PrintFormat("%1", LOG_TAG + message, level: LogLevel.WARNING);
	}

	//! Reset every static and TBD_MenuStack, then retry TryArm every ARM_RETRY_MS; refuses and logs on a dedicated server or without a call queue. A delayed one-shot check would race TBD_FrameworkManager's own deferred start, hence the retry.
	//! @authority client
	static void Start()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
		{
			LogWarn("Start refused - no call queue on this machine. The slot picker cannot arm.");
			return;
		}

		// Idempotent re-arm: never two timers, and a previous world that failed to tear down cannot
		// latch the watcher off for the process.
		queue.Remove(Tick);
		queue.Remove(TryArm);

		s_bRunning = false;
		s_LastStage = TBD_EGameStage.LOADING;
		s_bPresetUnavailable = false;
		s_iArmAttempts = 0;
		s_iLastRaiseOutcome = RAISE_UNSEEN;

		// A menu the engine destroyed during teardown without OnMenuClose leaves a stale entry that
		// makes IsOpen true forever; before this world opens anything is the safe moment to clear it.
		TBD_MenuStack.Reset();

		// The mode test, not the workspace: a headless dedicated server has a workspace.
		if (RplSession.Mode() == RplMode.Dedicated)
		{
			Log("Start refused - dedicated server (RplSession.Mode()==Dedicated). The picker is a client screen; nothing to raise here.");
			return;
		}

		Log(string.Format("Start - arming watcher: retrying IsFrameworkWorld() every %1 ms, up to %2 attempts.",
			ARM_RETRY_MS, ARM_MAX_ATTEMPTS));

		queue.CallLater(TryArm, ARM_RETRY_MS, true);
	}

	//! Once IsFrameworkWorld holds, start Tick every POLL_MS and log it; after ARM_MAX_ATTEMPTS, stop and log the give-up as a wiring failure.
	protected static void TryArm()
	{
		s_iArmAttempts++;

		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
			return;

		if (!TBD_FrameworkManager.IsFrameworkWorld())
		{
			if (s_iArmAttempts < ARM_MAX_ATTEMPTS)
				return;

			queue.Remove(TryArm);
			LogWarn(string.Format("Start GAVE UP - IsFrameworkWorld() still false after %1 attempts over %2 ms. The slot picker will NOT open on this world. This is a wiring failure, not a timing one: check that TBD_FrameworkManager is on the same game mode prefab as TBD_LobbyComponent.",
				s_iArmAttempts, s_iArmAttempts * ARM_RETRY_MS));
			return;
		}

		queue.Remove(TryArm);

		s_bRunning = true;
		s_LastStage = TBD_EGameStage.LOADING;

		queue.CallLater(Tick, POLL_MS, true);

		Log(string.Format("Tick ARMED after %1 attempt(s) (%2 ms) - polling the replicated stage every %3 ms.",
			s_iArmAttempts, s_iArmAttempts * ARM_RETRY_MS, POLL_MS));
	}

	//! Stop polling, reset the statics, close the selector and lobby, and reset TBD_LobbyClient. Leaves TBD_MenuStack alone: the briefing and spectator screens may still be stacked at teardown.
	//! @authority client
	static void Shutdown()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
		{
			queue.Remove(Tick);
			queue.Remove(TryArm);
		}

		s_bRunning = false;
		s_LastStage = TBD_EGameStage.LOADING;
		s_iArmAttempts = 0;
		s_iLastRaiseOutcome = RAISE_UNSEEN;

		s_bPresetUnavailable = false;

		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UIMissionSelector))
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UIMissionSelector);

		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UILobby);

		TBD_LobbyClient.Reset();
	}

	//! Hand a stage change to OnStageChanged; while in LOBBY, re-raise the selector when no pre-game screen is open, the player has no body and TBD_LobbyClient does not say stand down.
	protected static void Tick()
	{
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		TBD_EGameStage stage = fm.GetStage();

		if (stage != s_LastStage)
		{
			s_LastStage = stage;
			OnStageChanged(stage);
			return;
		}

		// Esc closes any menu. In LOBBY that is a dead end (no seat, one life), so the picker comes back:
		// a soft modal that stops once the round leaves LOBBY or the player has a body, asked of the
		// local controlled entity as well as TBD_LobbyClient.ShouldStandDown, the predicate
		// TBD_LobbyScreen closes on. A screen already open is closed there, not here.
		if (stage != TBD_EGameStage.LOBBY)
			return;

		if (TBD_LobbyClient.ShouldStandDown() || SCR_PlayerController.GetLocalControlledEntity())
			return;

		// Selector, Lobby or Briefing still up: only an empty stack brings the first screen back.
		if (IsPreGameScreenOpen())
			return;

		Raise();
	}

	//! @return true while the Mission Selector, Lobby or Briefing screen is on the TBD stack
	static bool IsPreGameScreenOpen()
	{
		return TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UIMissionSelector)
			|| TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby)
			|| TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UIBriefing);
	}

	//! Open the Mission Selector, logging each outcome once per change: deferred without a local controller, refused while the preset is latched, skipped when a pre-game screen is up, latched off when the open fails.
	protected static void Raise()
	{
		// No local controller yet: deferred, and the next poll retries, so gating here heals itself.
		if (!GetGame().GetPlayerController())
		{
			LogOutcome(RAISE_NO_CONTROLLER, "raise deferred - no local player controller yet. The 500 ms poll will retry; this is self-healing, not a failure.");
			return;
		}

		if (s_bPresetUnavailable)
		{
			LogOutcome(RAISE_PRESET_DEAD, "raise refused - preset latched unavailable for this round (TBD_MenuStack.Open returned null once). Cleared on world teardown or the next arm.");
			return;
		}

		if (IsPreGameScreenOpen())
		{
			LogOutcome(RAISE_ALREADY_OPEN, "raise skipped - a pre-game screen is already on the stack.");
			return;
		}

		if (!TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UIMissionSelector))
		{
			s_bPresetUnavailable = true;
			LogOutcome(RAISE_OPEN_FAILED, "raise FAILED - TBD_MenuStack.Open returned null. Latched off for this round; see the [TBD][ui] error above for the preset id.");
			return;
		}

		LogOutcome(RAISE_OPENED, "picker OPEN - TBD_UIMissionSelector raised (Lobby is the next tab).");
	}

	//! Log `message` only when `outcome` differs from the last one.
	protected static void LogOutcome(int outcome, string message)
	{
		if (s_iLastRaiseOutcome == outcome)
			return;

		s_iLastRaiseOutcome = outcome;
		Log(message);
	}

	//! React to a stage transition: in LOBBY reset TBD_LobbyClient and raise the selector; otherwise close the selector and the lobby through the stack. Logs `stage -> <name>`.
	static void OnStageChanged(TBD_EGameStage stage)
	{
		Log(string.Format("stage -> %1", typename.EnumToString(TBD_EGameStage, stage)));

		if (stage == TBD_EGameStage.LOBBY)
		{
			TBD_LobbyClient.Reset();
			Raise();
			return;
		}

		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UIMissionSelector))
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UIMissionSelector);

		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UILobby);
	}
}
