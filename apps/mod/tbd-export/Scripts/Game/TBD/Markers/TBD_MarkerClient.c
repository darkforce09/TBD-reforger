//! T-181.19 - the CLIENT half of map markers: asking for them, and putting them on the map.
//!
//! -- Which map -------------------------------------------------------------------------------
//! The real one. This does not draw anything itself: it hands rows to Reforger's own placed-marker
//! system (`SCR_MapMarkerManagerComponent` + `SCR_MapMarkerBase`, type `PLACED_CUSTOM`), which is
//! what the in-game map already uses for player-placed markers. Consequences worth stating:
//!   * no new `.layout` and no new `.conf`, so the `resourceDatabase.rdb` blocker that gates every
//!     modded menu preset does not apply to this slice at all;
//!   * the markers pan, zoom, label and layer-fade exactly like every other marker on that map,
//!     because they ARE that kind of marker.
//!
//! -- Markers are inserted LOCAL, on purpose --------------------------------------------------
//! `InsertStaticMarker(marker, isLocal: true)` keeps the marker on this client and out of the
//! replication path entirely. That is the second lock on side discipline: the server already sends
//! a player only their own side's rows (`TBD_MarkerData.c`), and a local insert guarantees those
//! rows cannot then be re-broadcast to anyone else by the vanilla marker sync component.
//!
//! -- Late joiners (the defect this slice refuses to repeat) ----------------------------------
//! T-181.28 records the briefing shipping push-only and therefore silently missing anyone who
//! joins while the round is already sitting in BRIEFING. Markers are PULL-driven instead, and the
//! pull has two independent triggers, either of which alone is enough for a late joiner:
//!   1. a poll that runs until the server gives an authoritative answer, and then stops. A player
//!      who joins unslotted keeps asking; the moment they take a seat, the next tick serves them.
//!   2. every time the player OPENS THE MAP. This costs nothing when idle, and it is also what
//!      picks up an admin mission switch or a re-slot to the other side at exactly the moment the
//!      player would otherwise notice stale markers.
//!
//! The poll is deliberate, not laziness: the client has no locally readable signal for "my slot was
//! assigned" - `m_mPlayerSlot` is a plain map on the server, not an `RplProp` - so there is nothing
//! to catch up ON. This is the same call the lobby made, and the lobby is the JIP-safe screen.
//!
//! -- Two consequences of riding the vanilla marker system, stated not hidden -----------------
//!  1. **UGC restriction.** `SCR_MapMarkerManagerComponent.CheckMarkersUserRestrictions()` runs on
//!     every map open and blocks EVERY static marker on an account without the
//!     `EUserInteraction.UserGeneratedContent` privilege - it cannot tell a mission briefing from a
//!     player scribble. A console account with UGC off would therefore see no mission markers. Not
//!     fixable without leaving this system; recorded so nobody reports it as a mod bug.
//!  2. **Removing a marker while the map is CLOSED.** `RemoveStaticMarker` -> `OnDelete()` ->
//!     `m_wRoot.RemoveFromHierarchy()` on a widget whose parent map frame may already have been
//!     destroyed. `m_wRoot` is a `ref Widget` so the object is still alive, and removing an
//!     already-orphaned widget should be a no-op - but vanilla only ever removes markers FROM the
//!     open map, so this ordering is ours alone and is UNVERIFIED on this lane.
class TBD_MarkerClient
{
	//! How often an unserved client re-asks. Stops entirely once served, so the steady state of a
	//! full server is ZERO marker traffic.
	static const int POLL_MS = 5000;

	//! Floor between two map-open requests, so hammering the map key cannot spam the server.
	static const float MAP_REQUEST_MIN_GAP_MS = 3000;

	//! Set once `Start()` has armed the poll and the map hook, so a double start is a no-op.
	protected static bool s_bRunning;

	//! The server has given an authoritative answer at least once for the CURRENT mission+side.
	//! Note this is true even when the answer was "your side authored none" - that is an answer.
	protected static bool s_bServed;

	//! What is currently on this player's map, so a re-apply can take it back off. Strong refs on
	//! both sides (the manager holds one too) - the same shape CRF uses for its own marker handles.
	protected static ref array<ref SCR_MapMarkerBase> s_aApplied;

	//! What the applied set is FOR. A change in either means the map is stale and must be rebuilt.
	protected static string s_sAppliedMissionId;
	protected static string s_sAppliedFaction;

	//! World time of the last map-open-triggered request, for MAP_REQUEST_MIN_GAP_MS.
	protected static float s_fLastMapRequestMs;

	//! `mission|faction|count` of the last outcome written to the log. Every map open re-requests,
	//! so without this a player who checks their map twenty times writes twenty identical lines.
	protected static string s_sLastLoggedOutcome;

	//! One warning per world when an area shape arrives: the placed-marker widget cannot fill.
	protected static bool s_bAreaFillNoted;

	//------------------------------------------------------------------------------------------------
	//! @authority client - arm the pull. Called by `TBD_MarkerComponent` on any machine with a
	//! workspace (a dedicated server has none and has no map to draw on).
	static void Start()
	{
		if (s_bRunning)
			return;

		s_bRunning = true;

		SCR_MapEntity.GetOnMapOpen().Insert(OnMapOpen);
		ArmPoll();

		// Ask immediately as well as on the timer: on a listen host the answer is synchronous, so
		// a slotted host player sees their markers without waiting out a poll interval.
		Request();
	}

	//------------------------------------------------------------------------------------------------
	//! (Re)start the unserved poll. `Remove` first so arming twice cannot stack two timers -
	//! `ScriptCallQueue.Remove` cancels by FUNCTION, which is precisely the semantics wanted here.
	protected static void ArmPoll()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
			return;

		queue.Remove(Tick);
		queue.CallLater(Tick, POLL_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process (a recorded landmine in this program), so every
	//! timer, invoker and cached marker has to be released here. Without this an in-process
	//! scenario restart leaves a poll firing against a dead world and marker objects pointing at
	//! widgets that no longer exist.
	static void Shutdown()
	{
		if (!s_bRunning)
			return;

		s_bRunning = false;

		SCR_MapEntity.GetOnMapOpen().Remove(OnMapOpen);

		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
		{
			// `ScriptCallQueue.Remove` cancels BY FUNCTION, not by arguments. That is exactly right
			// here and only here: there is one client, one Tick, and no per-player argument that
			// could be cancelled for the wrong person.
			queue.Remove(Tick);
		}

		ClearApplied();

		s_bServed = false;
		s_sAppliedMissionId = string.Empty;
		s_sAppliedFaction = string.Empty;
		s_fLastMapRequestMs = 0;
		s_sLastLoggedOutcome = string.Empty;
		s_bAreaFillNoted = false;

		TBD_MarkerIcons.ResetForWorld();
	}

	//------------------------------------------------------------------------------------------------
	//! Ask the server for THIS player's markers. No arguments - see `TBD_MarkerData.c` on why the
	//! request being unparameterised is what makes side discipline structural.
	static void Request()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		pc.TBD_RequestMarkers();
	}

	//------------------------------------------------------------------------------------------------
	//! A payload arrived (or was built in place on a listen host).
	//!
	//! Re-application is a full replace, never a merge: the arrays are positional and carry no ids,
	//! so there is nothing to diff against and a replace cannot leave a stale marker behind.
	static void Accept(array<int> xs, array<int> zs, array<string> icons, array<string> labels,
		string factionKey, string missionId, bool served)
	{
		if (!served)
		{
			// The server has nothing authoritative for us (usually: no slot yet). Drop whatever we
			// were showing - a player who has lost their seat must not keep the old side's
			// intelligence on their map.
			bool wasServed = s_bServed;

			if (wasServed || (s_aApplied && !s_aApplied.IsEmpty()))
				ClearApplied();

			s_bServed = false;
			s_sAppliedMissionId = string.Empty;
			s_sAppliedFaction = string.Empty;
			s_sLastLoggedOutcome = string.Empty;

			// Going served -> unserved has to RE-ARM the poll. `Tick()` cancels itself the moment it
			// is served, so without this a player who loses and retakes a seat would only recover
			// their markers by opening the map - which is exactly the kind of "works if you happen
			// to do the right thing" gap T-181.28 records against the briefing.
			if (wasServed)
			{
				TBD_Log.Event(TBD_MarkerService.CH_MARKERS,
					"lost the authoritative marker answer (no slot?) - cleared the map and resumed asking.");
				ArmPoll();
			}

			return;
		}

		s_bServed = true;

		// A new mission means a new set of authoring mistakes worth hearing about.
		if (missionId != s_sAppliedMissionId)
		{
			TBD_MarkerIcons.ResetReported();
			s_bAreaFillNoted = false;
		}

		s_sAppliedMissionId = missionId;
		s_sAppliedFaction = factionKey;

		ClearApplied();

		// Served-but-empty is a legal, common answer: `briefings` absent, `markers` absent, and
		// `markers: []` are three different states and none of them is an error.
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

		ApplyRows(xs, zs, icons, labels, sizeFp, rotationFp, shapeIdx, brushIdx, colorRgb,
			alpha255, factionKey, missionId);
	}

	//------------------------------------------------------------------------------------------------
	//! The marker manager, or null on a machine that has none.
	//!
	//! Two routes because they can disagree: `GetInstance()` is a static assigned in the
	//! component's own `OnPostInit`, while `FindComponent` walks the live game-mode entity - which
	//! is the route vanilla's own `SCR_BaseTutorialStage.CreateMarkerCustom()` uses. Trying both
	//! costs nothing and removes an init-order assumption this lane cannot test.
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

	//------------------------------------------------------------------------------------------------
	//! How many markers this client currently has on its map. Exists so a live check (or a future
	//! test screen) can read the outcome instead of inferring it from the log.
	static int AppliedCount()
	{
		if (!s_aApplied)
			return 0;

		return s_aApplied.Count();
	}

	//------------------------------------------------------------------------------------------------
	static bool IsServed()
	{
		return s_bServed;
	}

	//------------------------------------------------------------------------------------------------
	//! The poll. Runs only while unserved - the moment the server gives a real answer this
	//! cancels itself, so a settled server carries no marker traffic at all.
	protected static void Tick()
	{
		if (s_bServed)
		{
			GetGame().GetCallqueue().Remove(Tick);
			return;
		}

		Request();
	}

	//------------------------------------------------------------------------------------------------
	//! The player opened the map. Re-ask, rate-limited.
	//!
	//! Only ASKS - never inserts. Inserting synchronously from here would race
	//! `SCR_MapMarkersUI.OnMapOpen`, which walks every static marker and builds its widget; a
	//! marker inserted mid-walk could end up with two root widgets, one of them orphaned. The RPC
	//! round trip lands well after that walk, and `InsertStaticMarker` builds the widget itself
	//! when the map is already open, so both orderings are covered without any special casing.
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

	//------------------------------------------------------------------------------------------------
	//! Build one `SCR_MapMarkerBase` per row and hand it to the engine.
	//!
	//! The four-setter recipe is not invented - it is exactly what vanilla's own
	//! `SCR_BaseTutorialStage.CreateMarkerCustom()` does for a placed custom marker
	//! (`SetType(PLACED_CUSTOM)` / `SetIconEntry` / `SetColorEntry` / `SetCustomText`).
	protected static void ApplyRows(array<int> xs, array<int> zs, array<string> icons,
		array<string> labels, array<int> sizeFp, array<int> rotationFp, array<int> shapeIdx,
		array<int> brushIdx, array<int> colorRgb, array<int> alpha255,
		string factionKey, string missionId)
	{
		SCR_MapMarkerManagerComponent mgr = FindMarkerManager();
		if (!mgr)
		{
			// WARNING, not ERROR: a machine with no marker manager (or no map) is not a broken
			// mission, and `world-boot.sh` fails closed on any TBD-owned `SCR (E)` line.
			TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
				string.Format("no SCR_MapMarkerManagerComponent on the game mode - %1 marker(s) cannot be drawn.",
					xs.Count()));
			return;
		}

		EnsureApplied();

		int unknownIcons = 0;

		for (int i = 0, count = zs.Count(); i < count; i++)
		{
			// Defensive against a malformed parallel set. The four arrays are built together and
			// sent together so they cannot legitimately differ in length, but a short array here
			// would be an out-of-range read rather than a missing caption.
			if (!xs.IsIndexValid(i) || !icons.IsIndexValid(i) || !labels.IsIndexValid(i))
				break;

			bool recognised;
			int iconEntry = TBD_MarkerIcons.Resolve(icons[i], recognised);
			if (!recognised)
			{
				unknownIcons++;
				TBD_MarkerIcons.ReportUnknown(icons[i]);
			}

			iconEntry = TBD_MarkerIcons.ClampToLoadedConfig(iconEntry);

			int rowSizeFp = -1;
			int rowRotFp = -1;
			int rowShapeIdx = 0;
			int rowBrushIdx = -1;
			int rowRgb = -1;
			int rowAlpha255 = -1;
			if (sizeFp && sizeFp.IsIndexValid(i))
				rowSizeFp = sizeFp[i];
			if (rotationFp && rotationFp.IsIndexValid(i))
				rowRotFp = rotationFp[i];
			if (shapeIdx && shapeIdx.IsIndexValid(i))
				rowShapeIdx = shapeIdx[i];
			if (brushIdx && brushIdx.IsIndexValid(i))
				rowBrushIdx = brushIdx[i];
			if (colorRgb && colorRgb.IsIndexValid(i))
				rowRgb = colorRgb[i];
			if (alpha255 && alpha255.IsIndexValid(i))
				rowAlpha255 = alpha255[i];

			NoteAreaShape(rowShapeIdx, rowBrushIdx);

			bool scaleIcon = false;
			if (rowSizeFp >= 0 && rowSizeFp != 100)
				scaleIcon = true;

			bool fadeIcon = false;
			if (rowAlpha255 >= 0 && rowAlpha255 != 255)
				fadeIcon = true;

			SCR_MapMarkerBase marker;
			TBD_StyledMapMarker styled;
			if (scaleIcon || fadeIcon)
			{
				styled = new TBD_StyledMapMarker();
				if (scaleIcon)
					styled.m_iTbdSizeFp = rowSizeFp;
				if (fadeIcon)
					styled.m_iTbdAlpha255 = rowAlpha255;
				marker = styled;
			}
			else
			{
				marker = new SCR_MapMarkerBase();
			}

			marker.SetType(SCR_EMapMarkerType.PLACED_CUSTOM);
			marker.SetWorldPos(xs[i], zs[i]);
			marker.SetIconEntry(iconEntry);

			int colorEntry = TBD_MarkerIcons.MARKER_COLOR;
			if (rowRgb >= 0)
				colorEntry = NearestColorEntry(rowRgb);
			marker.SetColorEntry(colorEntry);

			if (rowRotFp >= 0)
				marker.SetRotation((rowRotFp + 50) / 100);

			marker.SetCustomText(labels[i]);

			// A mission marker is briefing material, not a scribble: the player must not be able to
			// delete their own orders off the map with the vanilla remove action.
			marker.SetCanBeRemovedByOwner(false);

			// isLocal -- this client only. Never enters replication, so it cannot leak sideways.
			mgr.InsertStaticMarker(marker, true);

			s_aApplied.Insert(marker);
		}

		if (NoteOutcome(missionId, factionKey, s_aApplied.Count()))
		{
			TBD_Log.Kv(TBD_MarkerService.CH_MARKERS, "applied", string.Format(
				"mission=%1 faction=%2 markers=%3 unknownIcons=%4",
				missionId, factionKey, s_aApplied.Count(), unknownIcons));
		}
	}

	protected static bool NoteOutcome(string missionId, string factionKey, int count)
	{
		string outcome = string.Format("%1|%2|%3", missionId, factionKey, count);
		if (outcome == s_sLastLoggedOutcome)
			return false;

		s_sLastLoggedOutcome = outcome;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Take every marker this slice put on the map back off it.
	protected static void ClearApplied()
	{
		if (!s_aApplied || s_aApplied.IsEmpty())
		{
			EnsureApplied();
			return;
		}

		SCR_MapMarkerManagerComponent mgr = FindMarkerManager();

		foreach (SCR_MapMarkerBase marker : s_aApplied)
		{
			if (!marker)
				continue;

			// Local markers keep `GetMarkerID() == -1`, which is the branch in
			// `RemoveStaticMarker` that deletes the widget and drops the manager's reference. We
			// never assign a marker id, so we can never take the synchronised branch and can never
			// ask the server to delete anything.
			if (mgr)
				mgr.RemoveStaticMarker(marker);
		}

		s_aApplied.Clear();
	}

	//------------------------------------------------------------------------------------------------
	protected static void EnsureApplied()
	{
		if (!s_aApplied)
			s_aApplied = new array<ref SCR_MapMarkerBase>();
	}

	//------------------------------------------------------------------------------------------------
	//! T-673 -- area shape/brush is on the wire but SCR_MapMarkerBase has no fill surface.
	//! The icon still carries the caption; one note per world, not per row.
	protected static void NoteAreaShape(int rowShapeIdx, int rowBrushIdx)
	{
		if (s_bAreaFillNoted)
			return;

		if (rowShapeIdx <= 0)
			return;

		s_bAreaFillNoted = true;

		TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
			string.Format("area marker shapeIdx=%1 brushIdx=%2 -- placed-marker widget cannot fill a shape; icon drawn with authored colour/rotation/size/alpha.",
				rowShapeIdx, rowBrushIdx));
	}

	//------------------------------------------------------------------------------------------------
	//! Parse `#rrggbb` to 0xRRGGBB. Empty -> -1 (absent). Malformed -> -2.
	//------------------------------------------------------------------------------------------------
	//! Map an authored 0xRRGGBB onto the nearest `SCR_EScenarioFrameworkMarkerCustomColor`
	//! index (the placed-marker palette). Hex stays on TBD_MissionMarkerStruct; SetColorEntry
	//! takes the enum index. Unreadable colour or an empty palette falls back to MARKER_COLOR.
	protected static int NearestColorEntry(int rgb)
	{
		if (rgb < 0)
			return TBD_MarkerIcons.MARKER_COLOR;

		int wantR = rgb / 65536;
		int wantG = (rgb / 256) % 256;
		int wantB = rgb % 256;

		SCR_MapMarkerManagerComponent mgr = FindMarkerManager();
		if (!mgr)
			return TBD_MarkerIcons.MARKER_COLOR;

		SCR_MapMarkerConfig cfg = mgr.GetMarkerConfig();
		if (!cfg)
			return TBD_MarkerIcons.MARKER_COLOR;

		SCR_MapMarkerEntryPlaced placed = SCR_MapMarkerEntryPlaced.Cast(
			cfg.GetMarkerEntryConfigByType(SCR_EMapMarkerType.PLACED_CUSTOM));
		if (!placed)
			return TBD_MarkerIcons.MARKER_COLOR;

		array<ref SCR_MarkerColorEntry> entries = placed.GetColorEntries();
		if (!entries || entries.IsEmpty())
			return TBD_MarkerIcons.MARKER_COLOR;

		int bestIndex = TBD_MarkerIcons.MARKER_COLOR;
		int bestDist = 2147483647;

		for (int i = 0, n = entries.Count(); i < n; i++)
		{
			SCR_MarkerColorEntry entry = entries[i];
			if (!entry)
				continue;

			Color palette = entry.GetColor();
			if (!palette)
				continue;

			int packed = palette.PackToInt();
			int pr = (packed / 65536) % 256;
			int pg = (packed / 256) % 256;
			int pb = packed % 256;

			int dr = pr - wantR;
			int dg = pg - wantG;
			int db = pb - wantB;
			int dist = (dr * dr) + (dg * dg) + (db * db);
			if (dist < bestDist)
			{
				bestDist = dist;
				bestIndex = i;
			}
		}

		return bestIndex;
	}

}

//! T-673 -- placed custom marker that reapplies authored size/alpha after vanilla builds the widget.
//! Colour goes through SetColorEntry (nearest palette index) on the base class, not this override.
//! A default-styled row never instantiates this class, so icon-only markers keep the pre-T-673 path.
class TBD_StyledMapMarker : SCR_MapMarkerBase
{
	int m_iTbdSizeFp = -1;
	int m_iTbdAlpha255 = -1;

	protected static bool s_bSizeUnsupportedWarned;

	//------------------------------------------------------------------------------------------------
	override void OnCreateMarker(bool skipProfanityFilter = false)
	{
		super.OnCreateMarker(skipProfanityFilter);

		if (!m_wRoot)
			return;

		if (m_iTbdAlpha255 >= 0)
			m_wRoot.SetOpacity(m_iTbdAlpha255 / 255.0);

		if (m_iTbdSizeFp >= 0 && m_iTbdSizeFp != 100)
			GetGame().GetCallqueue().CallLater(ApplyTbdSize, 0, false);
	}

	//------------------------------------------------------------------------------------------------
	protected void ApplyTbdSize()
	{
		if (!m_wRoot)
			return;

		float factor = m_iTbdSizeFp / 100.0;
		int scaled = ScaleImagesUnder(m_wRoot, factor);

		if (scaled == 0 && !s_bSizeUnsupportedWarned)
		{
			s_bSizeUnsupportedWarned = true;
			TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
				"marker size could not be applied -- this build's marker layout has no scalable image; markers drew at default size.");
		}
	}

	//------------------------------------------------------------------------------------------------
	protected int ScaleImagesUnder(Widget parent, float factor)
	{
		int scaled = 0;
		if (!parent)
			return 0;

		Widget child = parent.GetChildren();
		while (child)
		{
			ImageWidget image = ImageWidget.Cast(child);
			if (image && FrameWidget.Cast(parent))
			{
				float w;
				float h;
				image.GetScreenSize(w, h);

				if (w > 0 && h > 0)
				{
					WorkspaceWidget workspace = GetGame().GetWorkspace();
					float baseW = workspace.DPIUnscale(w);
					float baseH = workspace.DPIUnscale(h);

					FrameSlot.SetSize(image, baseW * factor, baseH * factor);
					FrameSlot.SetPos(image,
						FrameSlot.GetPosX(image) - ((baseW * factor) - baseW) / 2,
						FrameSlot.GetPosY(image) - ((baseH * factor) - baseH) / 2);

					scaled++;
				}
			}

			scaled = scaled + ScaleImagesUnder(child, factor);
			child = child.GetSibling();
		}

		return scaled;
	}
}
