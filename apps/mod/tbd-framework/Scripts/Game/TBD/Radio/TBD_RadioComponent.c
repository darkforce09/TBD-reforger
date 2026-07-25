//! T-181.40 — where the radio slice's lifecycle is hosted, and where the ONE question this lane
//! could not answer offline gets answered on every boot.
//!
//! The seat is a component on the game mode prefab — the same one `TBD_FrameworkManager`,
//! `TBD_SpawnManager`, `TBD_LobbyComponent`, `TBD_SpectatorComponent` and `TBD_MarkerComponent`
//! already occupy (`Prefabs/Systems/TBD_GameMode.et`). `TBD_MarkerComponent` sets the precedent and
//! this is a deliberately close copy of it.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD radio nets — assigns and displays the mission JSON's per-faction radioPlan nets, and tunes the player's radio where the world supports it.")]
class TBD_RadioComponentClass : SCR_BaseGameModeComponentClass {}

class TBD_RadioComponent : SCR_BaseGameModeComponent
{
	//! The game mode component graph is up well before the local player has a controller or the
	//! server has assigned a slot, so the client start is nudged past init rather than racing it.
	//! Nothing is lost by being late: the client polls until it is served.
	static const int START_DELAY_MS = 2500;

	//! The world's entity graph is still being built during `OnPostInit`, so the backbone question
	//! is asked a beat later — otherwise a world that DOES place a `RadioManagerEntity` could be
	//! reported as lacking one purely because we asked first. Same reasoning as the marker slice's
	//! availability report, longer fuse because this one waits on world entities and not on a
	//! sibling component.
	static const int REPORT_DELAY_MS = 3000;

	//------------------------------------------------------------------------------------------------
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// Runs on EVERY machine, headless included, because this is the line that answers the
		// question no oracle in this repo could: whether this world can support radio at all.
		// `world-boot.sh` prints the answer, so it is a fact in the boot log rather than an
		// assumption in a comment.
		GetGame().GetCallqueue().CallLater(ReportBackbone, REPORT_DELAY_MS, false);

		// A dedicated server has no workspace at all (measured — see TBD_UILayouts). That is the
		// cleanest available "am I a machine with a screen" test, and the one the rest of the UI
		// framework already trusts. No screen, nobody to show a net list to.
		if (!GetGame().GetWorkspace())
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
			queue.Remove(ReportBackbone);
			queue.Remove(TBD_RadioClient.Start);
		}

		TBD_RadioClient.Shutdown();
		TBD_RadioService.Reset();

		super.OnDelete(owner);
	}

	//------------------------------------------------------------------------------------------------
	//! One line, once, saying whether this world can support radio at all.
	//!
	//! ── Why this is the load-bearing diagnostic of the whole slice ──────────────────────────
	//! The engine already says it, once, buried in a wall of world-load output and attributed to
	//! whichever prop happened to carry the first `BaseRadioComponent`:
	//!
	//!     DEFAULT (W): World doesn't contain RadioManagerEntity to support any BaseRadioComponent.
	//!
	//! That line is easy to miss and impossible to grep for by feature. This asks
	//! `ChimeraWorld.GetRadioManager()` directly, tags the answer `[TBD][Radio]`, and states the
	//! consequence in the same breath — so "the radio half is not working" is never something an
	//! operator has to infer from silence. Silence about a feature that never runs is exactly the
	//! failure mode `world-boot.sh` was built to catch for prefab components.
	//!
	//! NORMAL / WARNING only, never ERROR: a world without a radio backbone is a legitimate world,
	//! and `world-boot.sh` triages any TBD-owned `SCRIPT (E)` line as a gate failure.
	protected void ReportBackbone()
	{
		if (TBD_RadioTuner.IsBackboneAvailable())
		{
			TBD_Log.Kv(TBD_RadioPlan.CH_RADIO, "backbone",
				"ok — world has a RadioManagerEntity; mission frequencies will be tuned into carried radios.");
			return;
		}

		TBD_Log.Warn(TBD_RadioPlan.CH_RADIO,
			"backbone: MISSING — this world has no RadioManagerEntity, so the engine supports NO BaseRadioComponent on it and no frequency can be set from script. Net ASSIGNMENT and DISPLAY still work; automatic TUNING does not. Fix is a world edit in Workbench (place a RadioManagerEntity in worlds/TBD_Dev_POC.ent), not a script change.");
	}
}
