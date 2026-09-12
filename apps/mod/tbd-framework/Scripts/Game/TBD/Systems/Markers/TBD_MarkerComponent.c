//! T-181.19 — where the marker client's lifecycle is hosted.
//!
//! Markers are a CLIENT feature that needs a seat which starts with the world and dies with it. In
//! this codebase that seat is a component on the game mode prefab — the same one
//! `TBD_FrameworkManager`, `TBD_SpawnManager`, `TBD_LobbyComponent` and `TBD_SpectatorComponent`
//! already occupy (`Prefabs/Systems/TBD_GameMode.et`). `TBD_LobbyComponent` sets the precedent and
//! this is a deliberately close copy of it.
//!
//! It also carries the one diagnostic this lane could not answer offline — see
//! `ReportMarkerManager()`.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD map markers — draws the mission JSON's per-faction briefing markers on the in-game map.")]
class TBD_MarkerComponentClass : SCR_BaseGameModeComponentClass {}

class TBD_MarkerComponent : SCR_BaseGameModeComponent
{
	//! The game mode component graph is up well before the local player has a controller or the
	//! server has assigned a slot, so the start is nudged past init rather than racing it. Nothing
	//! is lost by being late: the client polls until it is served.
	static const int START_DELAY_MS = 2500;

	//! The marker manager is a sibling component and may be initialised after us, so the
	//! availability report waits a beat too. Same reasoning, shorter fuse.
	static const int REPORT_DELAY_MS = 1000;

	//------------------------------------------------------------------------------------------------
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// Runs on EVERY machine, headless included, because this is the line that answers a
		// question no oracle in this repo could: whether `SCR_MapMarkerManagerComponent` is
		// actually present on the game mode entity at runtime. The prefab inherits from
		// `GameMode_Plain.et`, which is packed vanilla data — not readable offline, and adding a
		// second copy of a component the parent already carries would give the marker system two
		// instances fighting over one static. So the mod ASKS instead of assuming, and
		// `world-boot.sh` prints the answer.
		GetGame().GetCallqueue().CallLater(ReportMarkerManager, REPORT_DELAY_MS, false);

		// A dedicated server has no workspace at all (measured — see TBD_UILayouts). That is the
		// cleanest available "am I a machine with a screen" test, and the one the rest of the UI
		// framework already trusts. No screen, no map, no markers to draw.
		if (!GetGame().GetWorkspace())
			return;

		GetGame().GetCallqueue().CallLater(TBD_MarkerClient.Start, START_DELAY_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process (recorded landmine), so both callbacks and the
	//! client's own timers/invokers must be released or the next world starts with a poll and a
	//! map hook belonging to a world that no longer exists.
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

	//------------------------------------------------------------------------------------------------
	//! One line, once, saying whether the engine's marker system is reachable on this machine.
	//!
	//! Why it exists: a marker inserted with no manager is not an error the player can see — it is
	//! a feature that silently never runs, which is precisely the failure mode `world-boot.sh` was
	//! built to catch for prefab components. This makes the marker system's availability a fact in
	//! the boot log rather than an assumption in a comment.
	//!
	//! NORMAL / WARNING only, never ERROR: a headless server legitimately has no map, and
	//! `world-boot.sh` triages any TBD-owned `SCRIPT (E)` line as a gate failure.
	protected void ReportMarkerManager()
	{
		SCR_MapMarkerManagerComponent mgr = TBD_MarkerClient.FindMarkerManager();
		if (!mgr)
		{
			TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
				"marker-manager: MISSING — SCR_MapMarkerManagerComponent is not on the game mode entity, so mission markers cannot be drawn. Add it to TBD_GameMode.et.");
			return;
		}

		SCR_MapMarkerConfig cfg = mgr.GetMarkerConfig();
		if (!cfg)
		{
			TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
				"marker-manager: present but its marker config did not load — placed markers will have no icon.");
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
