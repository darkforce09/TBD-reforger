/**
 * @file TBD_RadioComponent.c
 * @brief Game mode component that hosts the radio lifecycle and its boot report.
 *
 * Role: starts `TBD_RadioClient` on machines with a player, resets the radio statics with the
 * world, and logs once whether the world has a radio backbone and how many nets the mission
 * authored.  Position: on the game mode prefab (`TBD_GameMode.et`) beside the framework, spawn,
 * lobby, spectator and marker components.
 * State: two pending call-queue callbacks, cancelled in `OnDelete`; the once-per-world backbone
 * report flag.  Invariants: the report runs on every machine and logs at NORMAL or WARNING level
 * only; the client never starts on a dedicated server (`RplMode.Dedicated`; a dedicated server
 * does have a workspace, so the workspace is not the test).
 */

//! Component class of `TBD_RadioComponent`; carries no data.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD radio nets -- assigns and displays the mission JSON's per-faction radioPlan nets, and tunes the player's radio where the world supports it.")]
class TBD_RadioComponentClass : SCR_BaseGameModeComponentClass {}

//! Hosts the radio client and the radio boot report on the game mode.
class TBD_RadioComponent : SCR_BaseGameModeComponent
{
	static const int START_DELAY_MS = 2500; //!< delay in ms before the client starts, past controller and slot setup

	protected static bool s_bBackboneReported; //!< true once the backbone line was logged this world; reset in OnDelete

	//! Delay in ms before the boot report. It must land inside the `world-boot.sh` settle window
	//! (default 4 s after the roll-call); world entities already exist when the game mode is built.
	static const int REPORT_DELAY_MS = 1500; //!< milliseconds

	//! Schedule the boot report on every machine and, except on a dedicated server, the radio
	//! client's start.
	//! @param owner the game mode entity
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		GetGame().GetCallqueue().CallLater(ReportRadio, REPORT_DELAY_MS, false);
		ScheduleClientStart();
	}

	//! Schedule `TBD_RadioClient.Start` after `START_DELAY_MS`, except on a dedicated server,
	//! which has nobody to show a net list to.
	//! @authority client
	protected void ScheduleClientStart()
	{
		if (RplSession.Mode() == RplMode.Dedicated)
			return;

		GetGame().GetCallqueue().CallLater(TBD_RadioClient.Start, START_DELAY_MS, false);
	}

	//! Cancel both pending callbacks, shut the client down and reset the service and plan; statics
	//! outlive a world inside one process.
	//! @param owner the game mode entity
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
		s_bBackboneReported = false;

		super.OnDelete(owner);
	}

	//! Log the backbone and the plan report. The engine's own missing-backbone warning is buried
	//! in world-load output under an arbitrary prop, so this tags the answer `[TBD][Radio]`.
	protected void ReportRadio()
	{
		ReportBackbone();
		ReportPlan();
	}

	//! Force the lazy `radioPlan` parse at boot, so a headless boot with no players still logs the
	//! `plan` line, then log the usable net count. Silent on a client (no mission document).
	//! @authority server
	protected void ReportPlan()
	{
		if (TBD_Authority.IsClient())
			return;

		if (!TBD_MissionLoader.IsValid())
		{
			// Ordinary on a boot without a mission: the plan parses when one loads.
			TBD_Log.Kv(TBD_RadioPlan.CH_RADIO, "plan", "no mission loaded yet -- radio plan will parse on load.");
			return;
		}

		// The call triggers `EnsureParsed`, which logs the `plan` line and each rejected net.
		int nets = TBD_RadioPlan.GetTotalNetCount();
		TBD_Log.Kv(TBD_RadioPlan.CH_RADIO, "plan-ready", string.Format("usableNets=%1", nets));
	}

	//! Log once per world whether the world has a `RadioManagerEntity`: `backbone ok`, or a warning
	//! naming the world and the script-side channel table in use.
	protected void ReportBackbone()
	{
		if (s_bBackboneReported)
			return;

		s_bBackboneReported = true;

		if (TBD_RadioTuner.IsBackboneAvailable())
		{
			TBD_Log.Kv(TBD_RadioPlan.CH_RADIO, "backbone",
				"ok -- world has a RadioManagerEntity; mission frequencies will be tuned into carried radios.");
			return;
		}

		string world = TBD_RadioTuner.WorldFileName();
		string fallback = TBD_RadioTuner.FallbackSourceName();
		TBD_Log.Warn(TBD_RadioPlan.CH_RADIO, string.Format(
			"backbone: MISSING -- world='%1' has no RadioManagerEntity; using script-side channel table (%2). Add RadioManagerEntity in Workbench (worlds/TBD_Dev_POC.ent) to restore the engine backbone. T-941.7 fallback is in use; the world edit is on the operator checklist.",
			world, fallback));
	}
}
