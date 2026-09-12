//! Lobby feature module - CLIENT stage watcher that raises/drops the lobby screen. Split out of TBD_LobbyController.c (UI reorg 2026-09-12); logic unchanged.
//!
//! CLIENT - watches the replicated game stage and raises/drops the lobby.
//!
//! -- Why a poll, and why it is hosted on a game-mode component ------------------------------
//! `TBD_FrameworkManager.m_Stage` is an `[RplProp(onRplName: "OnStageReplicated")]`, so the VALUE
//! is replicated and `GetStage()` is correct on a client. But `OnStageReplicated()` is an empty
//! stub, and `TBD_FrameworkManager.c` belongs to another slice this wave (T-181.23) - so this
//! slice must not write into it. A 500 ms poll of the replicated value is the self-contained way
//! to be correct today; it costs one enum compare per tick.
//!
//! It is started and stopped by `TBD_LobbyComponent` (a component on the game mode prefab) rather
//! than from a `SCR_PlayerController` override, for the reason `TBD_SpectatorComponent` gives in
//! its own header: statics outlive a world inside one process, so a watcher needs a lifetime tied
//! to the world, and the game-mode component graph is exactly that lifetime. It also means this
//! slice adds no third override of a vanilla method.
//!
//! `OnStageChanged` is public and side-effect-complete precisely so the eventual real hook is a
//! one-line call - see the slice report.
class TBD_LobbyStage
{
	static const int POLL_MS = 500;

	//! T-181.49 - the arming retry. `Start()` used to be one-shot, so losing a race it had no
	//! reason to expect to win was PERMANENT. See `Start` for the measurement.
	static const int ARM_RETRY_MS = 250;
	static const int ARM_MAX_ATTEMPTS = 60; //!< 60 x 250 ms = 15 s, then give up LOUDLY.

	//! Greppable prefix. One vocabulary for the whole raise path, so
	//! `grep '\[TBD\]\[Lobby\]' console.log` returns the entire lifecycle of the picker.
	static const string LOG_TAG = "[TBD][Lobby] ";

	//! Outcome of the last `Raise()`. `Raise` runs on a 500 ms poll, so an unlatched log line
	//! there would emit twice a second forever and bury the signal it exists to carry. Logging
	//! only on a CHANGE of outcome gives exactly one line per transition, which is what an
	//! operator actually needs: not "it refused", but "it started refusing, for this reason".
	static const int RAISE_UNSEEN        = 0; //!< nothing decided yet on this world.
	static const int RAISE_OPENED        = 1;
	static const int RAISE_NO_CONTROLLER = 2;
	static const int RAISE_PRESET_DEAD   = 3;
	static const int RAISE_ALREADY_OPEN  = 4;
	static const int RAISE_OPEN_FAILED   = 5;

	protected static bool s_bRunning;
	protected static TBD_EGameStage s_LastStage;

	//! `TBD_MenuStack.Open` returned null once - the preset is not registered (the known
	//! `resourceDatabase.rdb` blocker). Latched so the re-raise below logs ONE error for the round
	//! instead of one every 500 ms forever. A log flood would bury the very line an operator greps
	//! for to know whether the Workbench pass worked.
	protected static bool s_bPresetUnavailable;

	protected static int s_iArmAttempts;
	protected static int s_iLastRaiseOutcome;

	//------------------------------------------------------------------------------------------------
	//! One line, one shape, one grep. `PrintFormat` and NEVER `Print(localVariable)` - MEASURED in
	//! this codebase: `Print` emits the DECLARATION of a local, not its value, which is why the
	//! roll-call assertion in `world-boot.sh` has to strip a trailing quote.
	protected static void Log(string message)
	{
		PrintFormat("%1", LOG_TAG + message, level: LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	//! WARNING, not ERROR, and deliberately: `world-boot.sh` triages any TBD-owned `SCRIPT (E)`
	//! line as a gate failure, so shouting at ERROR about a state the gate can legitimately reach
	//! would turn a diagnostic into a false red. MEASURED this slice: WARNING lines DO reach
	//! `console.log` (`[TBD][Radio] backbone: MISSING` is one), so nothing is lost by the level.
	protected static void LogWarn(string message)
	{
		PrintFormat("%1", LOG_TAG + message, level: LogLevel.WARNING);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority client - only a machine with a local player may open a menu.
	//!
	//! -- T-181.49: this used to be a one-shot with three silent exits -------------------------
	//! `TBD_LobbyComponent.OnPostInit` arms this with `CallLater(..., false)` at +2000 ms. That
	//! made `IsFrameworkWorld()` a COIN FLIP, and losing it was permanent AND invisible:
	//!
	//!   MEASURED 2026-07-25, `world-boot.sh --mission=bridgehead-at-levie`, engine 1.7.0.54:
	//!     21:02:13.963  [TBD][Lobby] wire self-check PASS   <- TBD_LobbyComponent.OnPostInit
	//!     21:02:15.96   (Start fires: +2000 ms from the line above)
	//!     21:02:16.201  [TBD] roll-call: ... Lobby=ok       <- TBD_FrameworkManager's CallLater(..., 0)
	//!
	//! `Start` fires ~240 ms BEFORE the framework manager's own deferred roll-call. The call queue
	//! does not tick during world load, so both callbacks are flushed together when it starts and
	//! their relative order is not something this class gets to choose. Ask `IsFrameworkWorld()`
	//! once, at that instant, and the answer is whatever the flush order happened to be.
	//!
	//! So it now RETRIES. The bound exists so a genuinely broken world says so instead of spinning
	//! forever, and the give-up is logged - the point of this whole slice is that no exit on this
	//! path is silent.
	//!
	//! -- Statics are reset HERE, not only in Shutdown() ---------------------------------------
	//! `TBD_GameMode` is constructed TWICE per Workbench session (once for the World Editor, once
	//! for Play) while every static below survives between them. If the editor instance's
	//! `OnDelete` is skipped, `Shutdown()` never runs and the Play instance inherits `s_bRunning`
	//! true and a stale `TBD_MenuStack` entry - and the old `if (s_bRunning) return;` turned that
	//! into a picker that never opens again for the life of the process. A new world arming its
	//! watcher is unambiguous proof the previous world is gone, so that is the moment to clear.
	//! `Shutdown()` keeps doing it too; belt and braces, not one or the other.
	static void Start()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
		{
			LogWarn("Start refused - no call queue on this machine. The slot picker cannot arm.");
			return;
		}

		// Idempotent re-arm. Removing first is strictly safer than the `if (s_bRunning) return;`
		// this replaces: it cannot leave two timers running, and it cannot latch the watcher OFF
		// for the rest of the process when a previous world failed to tear itself down.
		queue.Remove(Tick);
		queue.Remove(TryArm);

		s_bRunning = false;
		s_LastStage = TBD_EGameStage.LOADING;
		s_bPresetUnavailable = false;
		s_iArmAttempts = 0;
		s_iLastRaiseOutcome = RAISE_UNSEEN;

		// A menu the engine destroyed during world teardown without firing `OnMenuClose` leaves a
		// stale weak entry in `TBD_MenuStack`'s static array, and `Raise`'s `IsOpen` check then
		// returns true forever - silently, without even latching `s_bPresetUnavailable`. Nothing
		// else in the addon calls `Reset()`; this is its one caller and this is the safe moment
		// for it, before anything on THIS world has opened a screen.
		TBD_MenuStack.Reset();

		// -- The real authority test ----------------------------------------------------------
		// NOT `GetGame().GetWorkspace()`, which this line used to ask: that is MEASURED NON-NULL
		// on the headless dedicated server `world-boot.sh` runs, so it never excluded anything.
		// `RplSession.Mode() == RplMode.Dedicated` is what both oracles use for this question and
		// what the rest of this addon already uses (TBD_FrameworkManager, TBD_AdminService, ...).
		if (RplSession.Mode() == RplMode.Dedicated)
		{
			Log("Start refused - dedicated server (RplSession.Mode()==Dedicated). The picker is a client screen; nothing to raise here.");
			return;
		}

		Log(string.Format("Start - arming watcher: retrying IsFrameworkWorld() every %1 ms, up to %2 attempts.",
			ARM_RETRY_MS, ARM_MAX_ATTEMPTS));

		queue.CallLater(TryArm, ARM_RETRY_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Retry until this world admits it is a framework world, then promote to the real `Tick` and
	//! cancel this. Bounded so a world that will never qualify says so once and stops.
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

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process, so this MUST run on world teardown or the next
	//! round starts with a tick pointed at a framework manager that no longer exists.
	//!
	//! Deliberately does NOT call `TBD_MenuStack.Reset()`: teardown is exactly when the briefing
	//! and spectator screens may still be legitimately stacked, and wiping their entries here
	//! would strand THEIR bookkeeping to fix ours. The arm path is the safe place for that, and it
	//! is where it now lives.
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

		// Cleared with the world: the next round gets a fresh chance to open the preset, which
		// matters precisely because the Workbench pass that registers it may land between rounds.
		s_bPresetUnavailable = false;

		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UILobby);

		TBD_LobbyClient.Reset();
	}

	//------------------------------------------------------------------------------------------------
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

		// -- LOBBY is a phase you are IN, not a screen you visit -----------------------------
		// Esc closes any menu and no script can stop it. For the briefing that is fine - you can
		// re-open your orders from the lobby. For the LOBBY it is a dead end: a player who
		// dismisses the picker has no seat, no way to get one, and one life to lose by missing
		// the round. So the picker comes back.
		//
		// Deliberately a soft modal and nothing stronger: it costs one `IsOpen` check per tick,
		// it stops the moment the round leaves LOBBY or the player reaches the world, and it
		// cannot flood the log because a preset that will not open is latched off after the first
		// failure.
		//
		// "In the world" is asked of the LOCAL controlled entity, not just of our own deploy flag,
		// and that is what makes this safe next to `TBD_SpawnManager.m_bAutoDeploy` - the PIE wave
		// this picker exists to replace, which still defaults ON (see the slice report). A player
		// the wave already deployed has a body, so the re-raise stands down and Esc dismisses the
		// picker for good, instead of trapping someone who is already playing behind it.
		//
		// Note this guards only the RE-raise. The first open (on the transition into LOBBY) is
		// unconditional on purpose: if this test were ever wrong, gating the initial open would
		// mean the picker silently never appears, which is a far worse failure than one that can
		// be dismissed.
		//
		// -- T-181.29: this guard was only ever HALF the answer -------------------------------
		// Standing the re-raise down does nothing for a screen that is ALREADY OPEN, and that is the
		// case the auto-deploy wave produces: the wave fires ~250 ms into LOBBY, this watcher raises
		// the picker on the same transition, and the result was a picker sitting over a live
		// character with only Esc to get rid of it. The close now lives where it belongs - on the
		// roster, in `TBD_LobbyScreen.OnRosterChanged` via `TBD_LobbyClient.ShouldStandDown()`.
		//
		// `ShouldStandDown()` replaces the bare `IsDeployed()` here so the re-raise and the close
		// test the same predicate. It is a strict superset of what this line asked before, so it can
		// only suppress a raise that used to happen, never permit one that did not - and every input
		// to it either latches for the round or self-corrects on the next 2 s refresh.
		if (stage != TBD_EGameStage.LOBBY)
			return;

		if (TBD_LobbyClient.ShouldStandDown() || SCR_PlayerController.GetLocalControlledEntity())
			return;

		Raise();
	}

	//------------------------------------------------------------------------------------------------
	//! Put the picker up, at most once per round if the preset cannot resolve.
	//!
	//! -- T-181.42: this is where "do I have a screen" is actually decided --------------------
	//! **`GetGame().GetWorkspace()` is NON-NULL on a headless dedicated server** (engine 1.7.0.54).
	//! It is not a dedicated-server test, and this class used to treat it as one. MEASURED in this
	//! repo: `world-boot.sh --mission=bridgehead-at-levie` with `TBD_WORLDBOOT_SETTLE=12` failed
	//! **3/3** with `SCRIPT (E): [TBD][ui] preset 60 did not open`, ~1000 ms (two poll ticks) after
	//! `LOADING -> LOBBY`, on a boot with ZERO players. For that line to be reachable at all, BOTH
	//! workspace guards then on the path (`TBD_LobbyComponent.OnPostInit` and `Start`) must have
	//! passed on a headless machine - the failing log is its own proof. The default 4 s settle
	//! usually ended before the watcher fired, which is why this read as an intermittent gate flake.
	//! (Both of those guards are gone as of T-181.49; `Start` now refuses on `RplMode.Dedicated`
	//! and says so, so a headless boot no longer reaches this function at all.)
	//!
	//! The reliable test is a null LOCAL PLAYER CONTROLLER - the idiom `TBD_MissionBrowser.c:285`
	//! already uses. It goes HERE rather than in `Start()` deliberately: `Tick` polls every 500 ms,
	//! so gating the raise is self-healing - a client whose controller is not up yet simply raises
	//! on a later tick. Gating `Start()` would be a one-shot test with a race, and losing that race
	//! would mean the picker silently NEVER appears, which is far worse than raising it late. Same
	//! reasoning the `Tick` comment already gives for keeping the first open unconditional.
	//!
	//! -- T-181.49: all four exits are now observable ------------------------------------------
	//! Every one of these used to `return` in silence. Nine of the eleven guards on the whole
	//! raise path did, which made "the picker did not open" a fact with no evidence attached -
	//! the real defect this slice fixes. `LogOutcome` latches on the OUTCOME so the 500 ms poll
	//! emits one line per transition, never a flood.
	protected static void Raise()
	{
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

		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
		{
			LogOutcome(RAISE_ALREADY_OPEN, "raise skipped - TBD_UILobby is already on the stack.");
			return;
		}

		if (!TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UILobby))
		{
			s_bPresetUnavailable = true;
			LogOutcome(RAISE_OPEN_FAILED, "raise FAILED - TBD_MenuStack.Open returned null. Latched off for this round; see the [TBD][ui] error above for the preset id.");
			return;
		}

		LogOutcome(RAISE_OPENED, "picker OPEN - TBD_UILobby raised.");
	}

	//------------------------------------------------------------------------------------------------
	//! One line per CHANGE of raise outcome. `Raise` is called from a 500 ms poll, so this latch is
	//! what keeps four honest diagnostics from becoming a log flood that hides them.
	protected static void LogOutcome(int outcome, string message)
	{
		if (s_iLastRaiseOutcome == outcome)
			return;

		s_iLastRaiseOutcome = outcome;
		Log(message);
	}

	//------------------------------------------------------------------------------------------------
	//! The single entry point for "the round changed phase" as far as the lobby is concerned.
	//! Kept public and complete so wiring it to the real replication hook is one line.
	//!
	//! -- T-181.29: the other OnStageChanged, and why m_bAutoDeploy is still 1 ----------------
	//! `TBD_SpawnManager.OnStageChanged` reacts to this same transition by scheduling
	//! `DeployAllConnectedPlayers` 250 ms out. Two handlers, one transition: this one raises the
	//! picker, that one puts everybody in the world. They race, and until this slice the race had no
	//! loser-recovery - whichever order they landed in, the picker stayed up.
	//!
	//! The obvious other fix is to flip `m_bAutoDeploy` to 0 and let the picker be the only way in.
	//! It is a real fix, it is almost certainly the right END state, and it is NOT taken here:
	//!
	//!   * The wave's stated reason for defaulting ON - "on a framework world this wave is currently
	//!     the ONLY working way into the world", because no screen could open - expired when
	//!     T-181.25 unblocked the menu presets. The premise is genuinely gone.
	//!   * But flipping it makes this picker LOAD-BEARING on its first ever live run. `TBD_GameMode.et`
	//!     does not override the attribute, so the `[Attribute("1")]` default in `TBD_SpawnManager`
	//!     is what ships - a one-character change takes effect immediately, and if the picker does
	//!     not open on a real client, nobody can deploy at all. `TBD_MenuStack.Open` returning null
	//!     is a failure this class already carries a latch for (`s_bPresetUnavailable`), which is
	//!     the measure of how plausible it still is.
	//!   * The fix in this slice is additive: it removes a screen that should not be there, and
	//!     changes nothing about how a player gets into the world. It is correct whether the wave
	//!     stays on or goes off, and it is correct whether or not the symptom it was filed for
	//!     turns out to be real.
	//!
	//! So: flip `TBD_SpawnManager.m_bAutoDeploy` to 0 in its own slice, gated on ONE observation -
	//! an operator confirming a live client sees the picker and can deploy from it. Until then the
	//! wave is the safety net and this slice is what stops the safety net from leaving a menu on
	//! the screen.
	static void OnStageChanged(TBD_EGameStage stage)
	{
		// T-181.49 - the transition itself, named. Without this line "the picker never appeared"
		// and "the watcher never saw LOBBY" are indistinguishable from the log, and they need
		// completely different fixes.
		Log(string.Format("stage -> %1", typename.EnumToString(TBD_EGameStage, stage)));

		if (stage == TBD_EGameStage.LOBBY)
		{
			TBD_LobbyClient.Reset();
			Raise();
			return;
		}

		// Any other phase: slotting is over. Closing through the stack hands input and focus back
		// correctly (TBD_MenuStack invariants 3 and 4).
		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UILobby);
	}
}
