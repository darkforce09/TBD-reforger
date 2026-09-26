/**
 * @file TBD_MarkerClient.c
 * @brief Client half of mission map markers: asks the server for this player's markers.
 *
 * Role: pulls the player's side-scoped marker set and hands each served answer to
 * `TBD_MarkerApplier`.  Position: `TBD_MarkerComponent` calls `Start` and `Shutdown`; replies arrive
 * through the modded `SCR_PlayerController` (`TBD_RpcDo_Markers`, or in place on a listen host).
 * State: static pull state on the client: running flag, served flag, the mission and side the map
 * shows, the map-open rate limit and the last logged outcome.
 * Invariants: the pull has two independent triggers, so a late joiner is always served: a poll
 * every `POLL_MS` that stops once the server answers authoritatively (the slot map is not
 * replicated, so the client has no local signal to wait on), and every map open, at most once per
 * `MAP_REQUEST_MIN_GAP_MS`. An unserved answer clears the map and re-arms the poll. Markers are
 * inserted local-only (`InsertStaticMarker(marker, true)`), so they never enter replication.
 * Known limits: `SCR_MapMarkerManagerComponent.CheckMarkersUserRestrictions` hides every static
 * marker on an account without the `UserGeneratedContent` privilege; removing a marker while the
 * map is closed takes an ordering vanilla never uses and is unverified.
 */

//! Client-side marker pull.
//! @authority client
class TBD_MarkerClient
{
	static const int POLL_MS = 5000; //!< re-ask period in milliseconds while unserved
	static const float MAP_REQUEST_MIN_GAP_MS = 3000; //!< floor between two map-open requests in milliseconds

	protected static bool s_bRunning; //!< true once `Start` armed the poll and the map hook
	protected static bool s_bServed; //!< true once the server answered for the current mission and side, rows or not
	protected static string s_sAppliedMissionId; //!< mission id of the markers on the map; empty when unserved
	protected static string s_sAppliedFaction; //!< faction key of the markers on the map; empty when unserved
	protected static float s_fLastMapRequestMs; //!< world time in ms of the last map-open request; 0 before the first
	protected static string s_sLastLoggedOutcome; //!< `mission|faction|count` of the last outcome logged

	//! Arm the pull: subscribe to map open, start the poll and ask once now (a listen host answers
	//! synchronously). A second call is a no-op. Runs on any machine with a workspace.
	//! @authority client
	static void Start()
	{
		if (s_bRunning)
			return;

		s_bRunning = true;

		SCR_MapEntity.GetOnMapOpen().Insert(OnMapOpen);
		ArmPoll();
		Request();
	}

	//! Start the unserved poll; any armed poll is removed first, so two calls never stack timers.
	protected static void ArmPoll()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
			return;

		queue.Remove(Tick);
		queue.CallLater(Tick, POLL_MS, true);
	}

	//! Release the map hook, the poll, every applied marker and the pull state. Statics outlive a
	//! world inside one process, so a world restart without this would keep a poll firing against
	//! a dead world. A call while not running is a no-op.
	//! @authority client
	static void Shutdown()
	{
		if (!s_bRunning)
			return;

		s_bRunning = false;

		SCR_MapEntity.GetOnMapOpen().Remove(OnMapOpen);

		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Tick);

		TBD_MarkerApplier.Clear();

		s_bServed = false;
		s_sAppliedMissionId = string.Empty;
		s_sAppliedFaction = string.Empty;
		s_fLastMapRequestMs = 0;
		s_sLastLoggedOutcome = string.Empty;
		TBD_MarkerApplier.RearmAreaFillNote();

		TBD_MarkerIcons.ResetForWorld();
	}

	//! Ask the server for this player's markers. The request carries no arguments, so a client has
	//! no faction to forge (see `TBD_MarkerService`). Does nothing without a player controller.
	//! @authority client
	static void Request()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		pc.TBD_RequestMarkers();
	}

	//! Take one answer: unserved clears the map (and re-arms the poll when it was served); served
	//! replaces every applied marker with the new rows. The rows carry no ids, so every apply is a
	//! full replace.
	//! @param xs world X per marker, followed by the `TBD_MarkerStyleCodec` style trailer
	//! @param zs world Z per marker
	//! @param icons authored icon per marker
	//! @param labels caption per marker; empty is legal
	//! @param factionKey the side the rows belong to
	//! @param missionId the mission the rows belong to
	//! @param served false when the server had no authoritative answer
	//! @authority client
	static void Accept(array<int> xs, array<int> zs, array<string> icons, array<string> labels,
		string factionKey, string missionId, bool served)
	{
		if (!served)
		{
			// A player who lost their seat must not keep the old side's markers.
			bool wasServed = s_bServed;

			if (wasServed || TBD_MarkerApplier.Count() > 0)
				TBD_MarkerApplier.Clear();

			s_bServed = false;
			s_sAppliedMissionId = string.Empty;
			s_sAppliedFaction = string.Empty;
			s_sLastLoggedOutcome = string.Empty;

			// `Tick` cancelled itself when served, so a lost seat re-arms it.
			if (wasServed)
			{
				TBD_Log.Event(TBD_MarkerService.CH_MARKERS,
					"lost the authoritative marker answer (no slot?) -- cleared the map and resumed asking.");
				ArmPoll();
			}

			return;
		}

		s_bServed = true;

		// A new mission reports its own authoring mistakes again.
		if (missionId != s_sAppliedMissionId)
		{
			TBD_MarkerIcons.ResetReported();
			TBD_MarkerApplier.RearmAreaFillNote();
		}

		s_sAppliedMissionId = missionId;
		s_sAppliedFaction = factionKey;

		TBD_MarkerApplier.Clear();

		// Served with no rows is a legal answer, not an error.
		if (!zs || zs.IsEmpty())
		{
			if (NoteOutcome(missionId, factionKey, 0))
			{
				TBD_Log.Event(TBD_MarkerService.CH_MARKERS,
					string.Format("faction '%1' has no map markers in mission '%2'.", factionKey, missionId));
			}

			return;
		}

		array<int> sizeFp = new array<int>();
		array<int> rotationFp = new array<int>();
		array<int> shapeIdx = new array<int>();
		array<int> brushIdx = new array<int>();
		array<int> colorRgb = new array<int>();
		array<int> alpha255 = new array<int>();
		if (!TBD_MarkerStyleCodec.UnpackFromX(xs, zs, sizeFp, rotationFp, shapeIdx, brushIdx, colorRgb, alpha255))
		{
			TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
				"marker style trailer was malformed -- drew icon-only defaults.");
		}

		int unknownIcons;
		if (!TBD_MarkerApplier.ApplyRows(xs, zs, icons, labels, sizeFp, rotationFp, shapeIdx, brushIdx,
			colorRgb, alpha255, unknownIcons))
			return;

		if (NoteOutcome(missionId, factionKey, TBD_MarkerApplier.Count()))
		{
			TBD_Log.Kv(TBD_MarkerService.CH_MARKERS, "applied", string.Format(
				"mission=%1 faction=%2 markers=%3 unknownIcons=%4",
				missionId, factionKey, TBD_MarkerApplier.Count(), unknownIcons));
		}
	}

	//! The marker manager: `SCR_MapMarkerManagerComponent.GetInstance()` first (set in the
	//! component's `OnPostInit`), then a lookup on the live game mode, so the result does not
	//! depend on init order.
	//! @return the manager, or null on a machine that has none
	static SCR_MapMarkerManagerComponent FindMarkerManager()
	{
		SCR_MapMarkerManagerComponent mgr = SCR_MapMarkerManagerComponent.GetInstance();
		if (mgr)
			return mgr;

		BaseGameMode gameMode = GetGame().GetGameMode();
		if (!gameMode)
			return null;

		return SCR_MapMarkerManagerComponent.Cast(gameMode.FindComponent(SCR_MapMarkerManagerComponent));
	}

	//! @return how many mission markers this client has on its map
	static int AppliedCount()
	{
		return TBD_MarkerApplier.Count();
	}

	//! @return true once the server answered authoritatively for the current mission and side
	static bool IsServed()
	{
		return s_bServed;
	}

	//! One poll step: re-ask while unserved, cancel the poll once served.
	protected static void Tick()
	{
		if (s_bServed)
		{
			GetGame().GetCallqueue().Remove(Tick);
			return;
		}

		Request();
	}

	//! Re-ask on map open, at most once per `MAP_REQUEST_MIN_GAP_MS`. Only asks, never inserts:
	//! an insert here would race `SCR_MapMarkersUI.OnMapOpen`, which builds every static marker's
	//! widget; the reply lands after that walk, and `InsertStaticMarker` builds the widget itself
	//! on an open map.
	//! @param config the opened map's configuration; unused
	protected static void OnMapOpen(MapConfiguration config)
	{
		ChimeraWorld world = ChimeraWorld.CastFrom(GetGame().GetWorld());
		if (world)
		{
			float now = world.GetWorldTime();
			if (s_fLastMapRequestMs > 0 && (now - s_fLastMapRequestMs) < MAP_REQUEST_MIN_GAP_MS)
				return;

			s_fLastMapRequestMs = now;
		}

		Request();
	}

	//! Record an outcome for the log gate.
	//! @return true when the outcome differs from the last one logged
	protected static bool NoteOutcome(string missionId, string factionKey, int count)
	{
		string outcome = string.Format("%1|%2|%3", missionId, factionKey, count);
		if (outcome == s_sLastLoggedOutcome)
			return false;

		s_sLastLoggedOutcome = outcome;
		return true;
	}
}
