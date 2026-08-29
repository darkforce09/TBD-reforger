//! T-181.40 - where the radio slice's lifecycle is hosted, and where the ONE question this lane
//! could not answer offline gets answered on every boot.
//!
//! The seat is a component on the game mode prefab - the same one `TBD_FrameworkManager`,
//! `TBD_SpawnManager`, `TBD_LobbyComponent`, `TBD_SpectatorComponent` and `TBD_MarkerComponent`
//! already occupy (`Prefabs/Systems/TBD_GameMode.et`). `TBD_MarkerComponent` sets the precedent and
//! this is a deliberately close copy of it.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD radio nets - assigns and displays the mission JSON's per-faction radioPlan nets, and tunes the player's radio where the world supports it.")]
class TBD_RadioComponentClass : SCR_BaseGameModeComponentClass {}

class TBD_RadioComponent : SCR_BaseGameModeComponent
{
	//! The game mode component graph is up well before the local player has a controller or the
	//! server has assigned a slot, so the client start is nudged past init rather than racing it.
	//! Nothing is lost by being late: the client polls until it is served.
	static const int START_DELAY_MS = 2500;

	//! The report is nudged past init so the answer is not "missing" merely because we asked first.
	//!
	//! 1500 ms and not longer, MEASURED: `world-boot.sh` breaks its wait as soon as the roll-call
	//! (or the mission verdict) appears and then settles for only `TBD_WORLDBOOT_SETTLE` seconds,
	//! default 4. A 3000 ms fuse landed inside that window on a plain boot and fell OFF THE END of
	//! a `--mission=` boot, so the single most important diagnostic in this slice was missing from
	//! exactly the run that had a radio plan to report. The world's entities are created during
	//! `Game::LoadEntities`, which the boot log shows completing BEFORE the game mode entity is
	//! constructed, so there is nothing left to wait for anyway.
	static const int REPORT_DELAY_MS = 1500;

	//------------------------------------------------------------------------------------------------
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// Runs on EVERY machine, headless included, because this is the line that answers the
		// question no oracle in this repo could: whether this world can support radio at all.
		// `world-boot.sh` prints the answer, so it is a fact in the boot log rather than an
		// assumption in a comment.
		GetGame().GetCallqueue().CallLater(ReportRadio, REPORT_DELAY_MS, false);

		// T-181.49 - this was `if (!GetGame().GetWorkspace())`, on the belief that a dedicated
		// server has no workspace. It does: `GetGame().GetWorkspace()` is MEASURED NON-NULL on the
		// headless dedicated server `world-boot.sh` runs (engine 1.7.0.54), so this guard let the
		// client-side radio poll start on the server and excluded nothing at all. The replication
		// mode is the real answer to "am I a machine with a player at a screen", and it is what
		// `TBD_FrameworkManager`, `TBD_AdminService` and `TBD_RadioController` already ask.
		// No screen, nobody to show a net list to.
		if (RplSession.Mode() == RplMode.Dedicated)
			return;

		GetGame().GetCallqueue().CallLater(TBD_RadioClient.Start, START_DELAY_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process (recorded landmine), so both callbacks, the
	//! client's timers and the parsed plan must be released or the next world starts with a poll
	//! belonging to a world that no longer exists and a radio plan from the previous mission.
	override void OnDelete(IEntity owner)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
		{
			queue.Remove(ReportRadio);
			queue.Remove(TBD_RadioClient.Start);
		}

		TBD_RadioClient.Shutdown();
		TBD_RadioService.Reset();

		super.OnDelete(owner);
	}

	//------------------------------------------------------------------------------------------------
	//! One line, once, saying whether this world can support radio at all.
	//!
	//! -- Why this is the load-bearing diagnostic of the whole slice --------------------------
	//! The engine already says it, once, buried in a wall of world-load output and attributed to
	//! whichever prop happened to carry the first `BaseRadioComponent`:
	//!
	//!     DEFAULT (W): World doesn't contain RadioManagerEntity to support any BaseRadioComponent.
	//!
	//! That line is easy to miss and impossible to grep for by feature. This asks
	//! `ChimeraWorld.GetRadioManager()` directly, tags the answer `[TBD][Radio]`, and states the
	//! consequence in the same breath - so "the radio half is not working" is never something an
	//! operator has to infer from silence. Silence about a feature that never runs is exactly the
	//! failure mode `world-boot.sh` was built to catch for prefab components.
	//!
	//! NORMAL / WARNING only, never ERROR: a world without a radio backbone is a legitimate world,
	//! and `world-boot.sh` triages any TBD-owned `SCRIPT (E)` line as a gate failure.
	protected void ReportRadio()
	{
		ReportBackbone();
		ReportPlan();
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server - how many nets this mission actually authored, once, at boot.
	//!
	//! This is what makes the `radioPlan` parse a RUNTIME fact rather than a compile-time hope. The
	//! parse is otherwise lazy - nothing touches it until a player asks - so a headless boot with
	//! zero players would exercise none of it, and `world-boot.sh --mission=<golden>` would pass
	//! while the projection quietly bound nothing. Asking here means the gate reads
	//! `plan mission=msn_8f3a2c authored=4 accepted=4 rejected=0` off a real golden document.
	//!
	//! Clients hold no mission document, so they have nothing to report and say nothing.
	protected void ReportPlan()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!TBD_MissionLoader.IsValid())
		{
			// Ordinary on a boot with no configured mission - the plan is parsed lazily the moment
			// one loads, so there is nothing to fix and nothing to warn about.
			TBD_Log.Kv(TBD_RadioPlan.CH_RADIO, "plan", "no mission loaded yet - radio plan will parse on load.");
			return;
		}

		// The call itself is what triggers `EnsureParsed`, which emits the detailed `plan` line
		// (and one warning per rejected net). The count is logged too so the two can be compared.
		int nets = TBD_RadioPlan.GetTotalNetCount();
		TBD_Log.Kv(TBD_RadioPlan.CH_RADIO, "plan-ready", string.Format("usableNets=%1", nets));
	}

	//------------------------------------------------------------------------------------------------
	protected void ReportBackbone()
	{
		if (TBD_RadioTuner.IsBackboneAvailable())
		{
			TBD_Log.Kv(TBD_RadioPlan.CH_RADIO, "backbone",
				"ok - world has a RadioManagerEntity; mission frequencies will be tuned into carried radios.");
			return;
		}

		TBD_Log.Warn(TBD_RadioPlan.CH_RADIO,
			"backbone: MISSING - this world has no RadioManagerEntity, so the engine supports NO BaseRadioComponent on it and no frequency can be set from script. Net ASSIGNMENT and DISPLAY still work; automatic TUNING does not. Fix is a world edit in Workbench (place a RadioManagerEntity in worlds/TBD_Dev_POC.ent), not a script change.");
	}
}
