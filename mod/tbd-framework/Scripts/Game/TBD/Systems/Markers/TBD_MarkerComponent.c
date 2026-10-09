/**
 * @file TBD_MarkerComponent.c
 * @brief Game mode component that hosts the marker client's lifecycle.
 *
 * Role: starts `TBD_MarkerClient` on machines with a workspace and shuts it down with the world;
 * logs once whether the engine's marker system is reachable.  Position: on the game mode prefab
 * (`TBD_GameMode.et`) beside the framework, spawn, lobby and spectator components.
 * State: two pending call-queue callbacks, cancelled in `OnDelete`.  Invariants: the availability
 * report runs on every machine and logs at NORMAL or WARNING level only, since a headless server
 * legitimately has no map; the client starts only where `GetGame().GetWorkspace()` exists.
 */

[ComponentEditorProps(category: "TBD/Framework", description: "TBD map markers -- draws the mission JSON's per-faction briefing markers on the in-game map.")]
//! Component class of `TBD_MarkerComponent`; carries no data.
class TBD_MarkerComponentClass : SCR_BaseGameModeComponentClass {}

//! Hosts the marker client on the game mode.
class TBD_MarkerComponent : SCR_BaseGameModeComponent
{
	static const int START_DELAY_MS = 2500; //!< delay in ms before the client starts, past controller and slot setup
	static const int REPORT_DELAY_MS = 1000; //!< delay in ms before the marker-manager report, past sibling init

	//! Schedule the marker-manager report on every machine and, where a workspace exists (a
	//! dedicated server has none), the marker client's start.
	//! @param owner the game mode entity
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// The game mode inherits vanilla `GameMode_Plain.et`, so the manager's presence is read at
		// runtime rather than assumed.
		GetGame().GetCallqueue().CallLater(ReportMarkerManager, REPORT_DELAY_MS, false);

		if (!GetGame().GetWorkspace())
			return;

		GetGame().GetCallqueue().CallLater(TBD_MarkerClient.Start, START_DELAY_MS, false);
	}

	//! Cancel both pending callbacks and shut the marker client down; statics outlive a world
	//! inside one process, so the next world must not inherit this world's poll and map hook.
	//! @param owner the game mode entity
	override void OnDelete(IEntity owner)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
		{
			queue.Remove(ReportMarkerManager);
			queue.Remove(TBD_MarkerClient.Start);
		}

		TBD_MarkerClient.Shutdown();

		super.OnDelete(owner);
	}

	//! Log once whether the engine's marker system is reachable on this machine: a warning for a
	//! missing manager, config or PLACED_CUSTOM entry; otherwise `marker-manager ok` with the
	//! placed icon count. Never logs at ERROR level.
	protected void ReportMarkerManager()
	{
		SCR_MapMarkerManagerComponent mgr = TBD_MarkerClient.FindMarkerManager();
		if (!mgr)
		{
			TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
				"marker-manager: MISSING -- SCR_MapMarkerManagerComponent is not on the game mode entity, so mission markers cannot be drawn. Add it to TBD_GameMode.et.");
			return;
		}

		SCR_MapMarkerConfig cfg = mgr.GetMarkerConfig();
		if (!cfg)
		{
			TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
				"marker-manager: present but its marker config did not load -- placed markers will have no icon.");
			return;
		}

		SCR_MapMarkerEntryPlaced placed = SCR_MapMarkerEntryPlaced.Cast(
			cfg.GetMarkerEntryConfigByType(SCR_EMapMarkerType.PLACED_CUSTOM));
		if (!placed)
		{
			TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
				"marker-manager: present, but this game build's marker config has no PLACED_CUSTOM entry.");
			return;
		}

		int iconCount = 0;
		array<ref SCR_MarkerIconEntry> icons = placed.GetIconEntries();
		if (icons)
			iconCount = icons.Count();

		TBD_Log.Kv(TBD_MarkerService.CH_MARKERS, "marker-manager",
			string.Format("ok placedIcons=%1", iconCount));
	}
}
