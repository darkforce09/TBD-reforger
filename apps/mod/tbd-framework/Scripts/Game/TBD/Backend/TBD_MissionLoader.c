//! Minimal parsed mission header — expanded in Phase 1 as loader hardens.
//! @contract mission.schema.json#/$defs/meta (subset: id/name/terrain).
class TBD_MissionMetaStruct
{
	string id;
	string name;
	string terrain;
}

//! One playable faction from the mission `factions[]` array.
//! @contract mission.schema.json#/$defs/faction
class TBD_MissionFactionStruct
{
	string key;
	string displayName;
	string presetId;
}

//! Circle shape (metres, world XZ + radius) used by spawn/objective zones.
//! @contract mission.schema.json#/$defs/circle
class TBD_MissionCircleStruct
{
	float x;
	float z;
	float r;
}

//! Zone shape wrapper. The schema is `oneOf` circle | polygon, so exactly one of the two is
//! AUTHORED on a well-formed zone.
//!
//! ══ MEASURED LANDMINE — a null check is NOT a presence test ═════════════════════════════════
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref <class>` field even when the JSON key is
//! absent. Measured 2026-07-25 on a live world boot against
//! `packages/tbd-schema/golden-missions/bridgehead-at-levie.json`, whose zone `z4` authors a
//! polygon and NO circle: `shape.circle` came back non-null with `x=0 z=0 r=0`, and zone `z5`,
//! which authors no `rules` key at all, came back with a non-null `rules` object full of
//! sentinels. So `if (shape.circle)` is ALWAYS TRUE and tells you nothing.
//!
//! The only reliable presence test for a nested object is a SCALAR SENTINEL on one of its fields
//! (`circle.r > 0`; `TBD_MissionZoneRulesStruct.ABSENT`) or a container's element count. Every
//! consumer in this file and in `TBD_ZoneRegistry` tests for CONTENT, never for non-null. This
//! generalises: any future struct added here needs the same treatment.
//!
//! T-181.18 — `polygon` is modelled as of this slice. It was deliberately absent before, which
//! meant every polygon-shaped zone in an authored mission parsed to a shape with nothing in it
//! and was silently unusable. The nested container type is the schema's own shape: an array of
//! `[x, z]` pairs (`#/$defs/polygon`, minItems 3, inner arrays exactly 2 numbers).
//!
//! **Proven, not assumed** (T-181.18): `ref array<ref array<float>>` is a legal Enfusion field
//! type and `JsonLoadContext.ReadValue` compiles against a class containing one — compile probe
//! with a failing negative control (`ref array<ref array<TypeThatDoesNotExist>>` -> `Unknown type`,
//! so the compiler really does check the inner generic argument). Runtime population was then
//! confirmed on a live world boot; see `TBD_ZoneRegistry` for the diagnostic that proves it in
//! every run rather than only in the one that was measured.
//!
//! Nothing here validates the polygon — a ring with fewer than 3 vertices or a pair that is not
//! exactly 2 numbers is legal as far as this struct is concerned and is caught (and reported by
//! zone id) when `TBD_ZoneRegistry` prepares the zone for use.
//! @contract mission.schema.json#/$defs/shape
class TBD_MissionShapeStruct
{
	ref TBD_MissionCircleStruct circle;
	//! Outer array = vertices; inner array = exactly [x, z] in world metres. Null when the zone
	//! authored a circle instead (the two are mutually exclusive per the schema's `oneOf`).
	ref array<ref array<float>> polygon;
}

//! T-181.18 — the zone `rules` object, as much of it as a TYPED parser can see.
//!
//! ── The problem, stated honestly ────────────────────────────────────────────────────────────
//! `mission.schema.json#/$defs/zone/rules` is `{"type":"object","additionalProperties":true}` with
//! **no declared properties at all**. Enfusion's `JsonLoadContext` maps JSON keys onto named class
//! fields; it cannot model an open object, so it can only ever see keys this class declares by
//! name. Keys it does not declare are invisible — not rejected, not logged, simply absent. That is
//! a real limitation of the lane and it is written down here rather than papered over.
//!
//! ── The decision ────────────────────────────────────────────────────────────────────────────
//! Carry the rules the framework actually consumes as named fields with ABSENT sentinels, and make
//! the absence detectable. `TBD_ZoneRegistry.ResolveRules` then reports, per zone and by id:
//!   * a `rules` object that parsed but yielded no key this build understands  -> WARNING
//!   * a key present but out of range                                          -> ERROR + default
//!   * a `penalty` string this build does not recognise                        -> ERROR + default
//! so an authored rule that cannot be honoured is LOUD, never a silent default. What it cannot do
//! is name an unknown key, because a typed parser never sees one. Adding a rule to the vocabulary
//! is a field here plus a case in `ResolveRules` — that is the whole cost.
//!
//! ── The vocabulary (additive; legal under `additionalProperties: true`) ─────────────────────
//! `graceSeconds`     number >= 0 — seconds a player may remain in violation before the penalty.
//! `warnEverySeconds` number >  0 — how often the player is told, while in violation.
//! `penalty`          string      — "none" | "warn" | "kill". See TBD_EZonePenalty for why the
//!                                  default is deliberately NOT "kill" under one life.
//! @contract mission.schema.json#/$defs/zone (rules)
class TBD_MissionZoneRulesStruct
{
	//! Sentinel for "key absent from JSON". Same device `TBD_MissionSlotStruct.Y_ABSENT` uses and
	//! for the same reason: `JsonLoadContext` leaves a missing key at its field initializer, and
	//! standard JSON cannot carry NaN. No sane authored value approaches -1e6, so the initializer
	//! doubles as the presence flag — which is what lets a bad authored value (a NEGATIVE grace,
	//! say) be reported as an error instead of being mistaken for "not authored".
	static const float ABSENT = -1000000;

	float graceSeconds = ABSENT;
	float warnEverySeconds = ABSENT;
	string penalty;   //!< Empty string = absent (JsonLoadContext leaves it at the initializer).
}

//! One entry from the mission `zones[]` array (spawn, objective, boundary, …).
//! @contract mission.schema.json#/$defs/zone
class TBD_MissionZoneStruct
{
	string id;
	string type;
	//! Human name authored in the editor ("Levie Bridge"). OPTIONAL in the schema — a zone that
	//! omits it parses to an empty string, which is why callers fall back to `type`+`id` rather
	//! than assuming a label exists.
	string label;
	string faction;
	ref TBD_MissionShapeStruct shape;
	//! T-181.18 — OPTIONAL in the schema; null when the zone authored no rules at all. A non-null
	//! rules block whose every field is still at its sentinel means "authored, but nothing in it
	//! was legible to this build" and is reported as such.
	ref TBD_MissionZoneRulesStruct rules;
}

//! One ORBAT role line.
//! @contract mission.schema.json#/$defs/role
class TBD_MissionOrbatRoleStruct
{
	string slot;  //!< Role label within the squad ("Squad Leader"). Schema-required.
	string kit;   //!< Loadout alias (kit:<id>). Schema-required.
	int count;    //!< Number of slots to materialize for this role.
}

//! One ORBAT group (squad) and its roles.
//! @contract mission.schema.json#/$defs/group
class TBD_MissionOrbatGroupStruct
{
	string callsign; //!< Squad callsign ("Alpha"). Schema-required.
	string type;     //!< Group type label authored in the editor ("infantry_squad"). Schema-required.
	ref array<ref TBD_MissionOrbatRoleStruct> roles;
}

//! One faction's ORBAT (its groups), keyed by faction in the orbat map.
//! @contract mission.schema.json#/$defs/orbatFaction
class TBD_MissionOrbatFactionStruct
{
	ref array<ref TBD_MissionOrbatGroupStruct> groups;
}

//! T-181.13 — how the round ends, straight from the mission JSON.
//! `endOn` values are the schema enum: time_limit | all_objectives_captured |
//! faction_eliminated | objective_destroyed | hold_expired. TBD one-life events evaluate
//! `faction_eliminated` today; the others are parsed and ignored until their slice lands, so
//! an authored mission never silently loses a condition it declared.
//! @contract mission.schema.json#/$defs/winConditions
class TBD_MissionWinConditionsStruct
{
	string mode;               //!< Free-form mode label from the editor.
	ref array<string> endOn;   //!< One or more end triggers.
}

//! One map marker authored on a faction's briefing. All four keys are schema-required, so a
//! marker that exists at all is complete — there is no partial-marker case to defend against.
//! @contract mission.schema.json#/$defs/marker
class TBD_MissionMarkerStruct
{
	float x;      //!< World X, metres.
	float z;      //!< World Z, metres.
	string icon;  //!< Icon key authored in the editor ("objective", "defend", "destroy").
	string label; //!< Marker caption ("OBJ BRIDGE").
}

//! T-181.23 — one faction's WRITTEN ORDERS. This is the Arma-3 briefing text the whole briefing
//! screen exists to display; before this slice the document modelled the mission's structure but
//! not a word of its prose.
//!
//! Every field is OPTIONAL in the schema (`briefing` declares no `required`), so a partially
//! authored briefing parses to empty strings rather than failing. Callers must treat an empty
//! string as "not authored" and render nothing, never a blank heading.
//!
//! `markers` is modelled rather than dropped: the mod's standing rule is that an authored mission
//! never silently loses data it declared (same reasoning as `winConditions.endOn`). It is null
//! when the briefing omits the key.
//! @contract mission.schema.json#/$defs/briefing
class TBD_MissionBriefingStruct
{
	string situation;  //!< What is happening.
	string mission;    //!< What this side must achieve.
	string execution;  //!< How they are to do it.
	ref array<ref TBD_MissionMarkerStruct> markers; //!< Optional map markers; null when absent.
}

//! T-181.38 — the mission `flow` block: the numbers that PACE an event.
//!
//! Every golden mission authors all four fields and, until this slice, the block was not in
//! `TBD_MissionDocumentStruct` at all — so `safeStartSeconds` was overridden by a hardcoded 300,
//! `timeLimitSeconds` was authored by every mission and evaluated by none, and `jip: "disabled"`
//! was silently violated. "JSON is the contract" (TBD_MOD_DESIGN.md §2) is exactly what that
//! failed.
//!
//! ══ EVERY FIELD IS OPTIONAL, AND `0` IS NOT `absent` ═══════════════════════════════════════════
//! `flow` itself is schema-required, but it declares no `required` PROPERTIES, so a mission may
//! author `"flow": {}` — `golden-missions/empty-warning-fields.json` does exactly that, and it must
//! behave identically to a build that never saw a flow block.
//!
//! `JsonLoadContext` ALLOCATES a nested `ref <class>` field even when the JSON key is ABSENT
//! (measured 2026-07-25 — see the landmine header on `TBD_MissionShapeStruct` above), so
//! `if (doc.flow)` is ALWAYS TRUE and is not a presence test. Worse, an absent integer key would
//! read back as `0` — and `0` is a REAL authored value here: `timeLimitSeconds: 0` means "no time
//! limit", a deliberate statement rather than silence. A plain zero-test would erase the
//! distinction between "the author said no limit" and "the author said nothing".
//!
//! Hence the ABSENT sentinel, the same device `TBD_MissionSlotStruct.Y_ABSENT` and
//! `TBD_MissionZoneRulesStruct.ABSENT` use and for the same reason: `JsonLoadContext` leaves a
//! missing key at its field INITIALIZER, and standard JSON cannot carry NaN. No sane authored
//! duration approaches -1e6, so the initializer doubles as the presence flag — which is also what
//! lets a NEGATIVE authored value (illegal under the schema's `minimum: 0`) be reported as a fault
//! instead of mistaken for "not authored".
//!
//! `jip` uses the empty string for the same purpose, exactly like
//! `TBD_MissionZoneRulesStruct.penalty`.
//!
//! Nothing here validates. An out-of-range value parses fine and is caught, named and reported
//! where it is APPLIED — `TBD_MissionFlow` / `TBD_FrameworkManager.ApplyMissionFlow`, which is the
//! only place these turn into behaviour.
//! @contract mission.schema.json#/$defs/flow
class TBD_MissionFlowStruct
{
	//! "key absent from JSON". A presence flag, not a magic default — see the header.
	static const int ABSENT = -1000000;

	int briefingSeconds = ABSENT;  //!< Intended length of the BRIEFING stage. Schema: integer >= 0.
	int safeStartSeconds = ABSENT; //!< Safestart countdown length. Schema: integer >= 0.
	int timeLimitSeconds = ABSENT; //!< Round length; an authored 0 means "no limit". Schema: >= 0.
	string jip;                    //!< "disabled" | "until_safestart_end" | "always". Empty = absent.
}

//! Full mission document parsed from the backend — the canonical contract the loader
//! consumes. schemaVersion is the canonical STRING ("1.0"/"1.1"/"1.2"), distinct from the
//! website's integer editor/export version. Field names must equal the JSON keys.
//! @contract mission.schema.json#/
class TBD_MissionDocumentStruct
{
	string schemaVersion;                                      //!< Canonical contract version ("1.0"/"1.1"/"1.2").
	ref TBD_MissionMetaStruct meta;                            //!< Mission header.
	ref array<ref TBD_MissionFactionStruct> factions;         //!< Playable factions.
	ref array<ref TBD_MissionZoneStruct> zones;               //!< Spawn/objective/boundary zones.
	ref map<string, ref TBD_MissionOrbatFactionStruct> orbat; //!< ORBAT keyed by faction.
	ref array<ref TBD_MissionSlotStruct> slots;               //!< Flattened spawn slots (schema 1.1).
	ref TBD_MissionWinConditionsStruct winConditions;         //!< T-181.13 round-end triggers.
	//! T-181.38 — event pacing. ALWAYS non-null after a parse, even for a mission with no `flow`
	//! key: `JsonLoadContext` allocates it regardless. Test its FIELDS against
	//! `TBD_MissionFlowStruct.ABSENT`, never this reference against null.
	ref TBD_MissionFlowStruct flow;
	//! T-181.23 — written orders keyed by faction key, exactly like `orbat`. OPTIONAL: the block
	//! is not in the schema's top-level `required` list and every mission authored before it
	//! existed has none, so this stays null and that is legal.
	ref map<string, ref TBD_MissionBriefingStruct> briefings;
}

//! Loads Mission JSON from backend REST or $profile fallback.
//! @route GET /api/v1/missions/{id}/compiled (service-token tier; body = this canonical document, T-092.2).
class TBD_MissionLoader
{
	//! Hard cap on a profile mission file. A file over this would silently truncate in
	//! Read() and then fail JSON parse with a misleading error — reject it up front (T-130.4 F1-16).
	protected static const int MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024;

	protected static ref TBD_MissionDocumentStruct s_Mission;
	protected static string s_RawJson;
	protected static bool s_Loaded;
	protected static bool s_Valid;
	protected static bool s_LoadInFlight;

	protected static ref RestCallback s_RestCallback;

	//------------------------------------------------------------------------------------------------
	static bool IsLoaded()
	{
		return s_Loaded;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsValid()
	{
		return s_Valid;
	}

	//------------------------------------------------------------------------------------------------
	//! Flattened slot instances (null until loaded + validated).
	static array<ref TBD_MissionSlotStruct> GetSlots()
	{
		if (!s_Valid || !s_Mission)
			return null;

		return s_Mission.slots;
	}

	//------------------------------------------------------------------------------------------------
	static TBD_MissionSlotStruct GetSlotById(string slotId)
	{
		array<ref TBD_MissionSlotStruct> slots = GetSlots();
		if (!slots || slotId.IsEmpty())
			return null;

		foreach (TBD_MissionSlotStruct slot : slots)
		{
			// B1 — uid-aware lookup: durable uid matches first, display id stays valid.
			if (slot && (slot.id == slotId || (!slot.uid.IsEmpty() && slot.uid == slotId)))
				return slot;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	static TBD_MissionDocumentStruct GetMission()
	{
		return s_Mission;
	}

	//------------------------------------------------------------------------------------------------
	static string GetRawJson()
	{
		return s_RawJson;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.13 — true when the mission declared this end trigger. Missions authored before
	//! winConditions existed simply have none, so callers get `false` and the round runs until
	//! an admin ends it rather than ending unexpectedly.
	static bool HasEndTrigger(string trigger)
	{
		if (!s_Mission || !s_Mission.winConditions || !s_Mission.winConditions.endOn)
			return false;

		foreach (string t : s_Mission.winConditions.endOn)
		{
			if (t == trigger)
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.23 — the written orders for ONE faction, or null when this mission authored none for
	//! that side.
	//!
	//! Faction-keyed exactly like `orbat`, so a caller passes the same key it already resolved from
	//! the player's slot — which is what keeps side discipline enforceable: the server hands out one
	//! side's orders and never the other's.
	//!
	//! Returns null (not an empty struct) so a caller can tell "this mission has no orders for me"
	//! apart from "orders exist but are blank". Absent `briefings` is LEGAL — the block is optional
	//! and predates nothing; missions in the wild simply have none.
	static TBD_MissionBriefingStruct GetBriefingForFaction(string factionKey)
	{
		if (!s_Mission || !s_Mission.briefings || factionKey.IsEmpty())
			return null;

		TBD_MissionBriefingStruct briefing;
		if (!s_Mission.briefings.Find(factionKey, briefing))
			return null;

		return briefing;
	}

	//------------------------------------------------------------------------------------------------
	//! Playable factions parsed from the mission document (null until loaded).
	static array<ref TBD_MissionFactionStruct> GetFactions()
	{
		if (!s_Mission)
			return null;

		return s_Mission.factions;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.18 — the raw `zones[]` array, or null when no VALID mission is loaded.
	//!
	//! Gated on `s_Valid` (not merely `s_Mission`) exactly like `GetSlots()`: a document that
	//! failed validation must not hand out zones that a play-area enforcer would then confine
	//! players with. `GetSpawnZoneForFaction` below predates this and reads `s_Mission` directly;
	//! it is left alone rather than quietly tightened, because that is a behaviour change on a
	//! path this slice does not own.
	static array<ref TBD_MissionZoneStruct> GetZones()
	{
		if (!s_Valid || !s_Mission)
			return null;

		return s_Mission.zones;
	}

	//------------------------------------------------------------------------------------------------
	//! World-space spawn point for a faction key. Returns vector.Zero if no spawn zone exists.
	//!
	//! T-181.18 note: still CIRCLE-ONLY on purpose. Polygons now parse (see
	//! `TBD_MissionShapeStruct`), but teaching spawn placement to pick a point inside a polygon is
	//! a different problem from testing containment — it needs a point that is on navigable ground,
	//! not just inside the ring, and a centroid is not that for a concave AO. Out of scope for the
	//! play-area slice; a polygon spawn zone remains unusable here, and it now says so instead of
	//! quietly answering with the wrong coordinates.
	//!
	//! T-181.18 BUG FIX (found by the runtime probe, latent until now): the old guard was
	//! `if (!zone.shape || !zone.shape.circle) continue;`. `shape.circle` is ALWAYS non-null — see
	//! the landmine on `TBD_MissionShapeStruct` — so a polygon-shaped spawn zone slipped past the
	//! guard and this returned `Vector(0, GetSurfaceY(0,0), 0)`: the corner of the map. It was
	//! latent only because no shipped mission yet authors a polygon spawn zone. The guard now tests
	//! the radius, which is real data. The matching dead guard in
	//! `TBD_MissionValidator.CheckZones` (same shape, same reason it can never fire) is reported to
	//! the command center rather than edited from here — that file is not this slice's to write.
	static vector GetSpawnZoneForFaction(string factionKey)
	{
		if (!s_Mission || !s_Mission.zones)
		{
			Print("[TBD] GetSpawnZoneForFaction: no mission loaded.", LogLevel.ERROR);
			return vector.Zero;
		}

		foreach (TBD_MissionZoneStruct zone : s_Mission.zones)
		{
			if (!zone || zone.type != "spawn" || zone.faction != factionKey)
				continue;

			if (!zone.shape || !zone.shape.circle || zone.shape.circle.r <= 0)
				continue;

			float x = zone.shape.circle.x;
			float z = zone.shape.circle.z;
			return Vector(x, GetGame().GetWorld().GetSurfaceY(x, z), z);
		}

		Print("[TBD] No spawn zone for faction '" + factionKey + "'.", LogLevel.ERROR);
		return vector.Zero;
	}

	//------------------------------------------------------------------------------------------------
	//! Entry point: tries REST when backend config exists, else file only.
	static void BeginLoad()
	{
		if (s_Loaded || s_LoadInFlight)
			return;

		TBD_BackendConfig.Load();
		string missionId = TBD_BackendConfig.GetMissionId();
		if (missionId.IsEmpty())
		{
			Print("[TBD] missionId not configured — cannot load mission.", LogLevel.ERROR);
			return;
		}

		if (!TBD_BackendConfig.GetBackendUrl().IsEmpty() && !TBD_BackendConfig.GetServerToken().IsEmpty())
		{
			s_LoadInFlight = true;
			FetchFromBackend(missionId);
			return;
		}

		if (LoadFromProfileFile(missionId))
		{
			s_Loaded = true;
			LogLoaded("profile");
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static void FetchFromBackend(string missionId)
	{
		RestApi rest = GetGame().GetRestApi();
		if (!rest)
		{
			Print("[TBD] RestApi unavailable — trying profile fallback.", LogLevel.WARNING);
			s_LoadInFlight = false;
			if (LoadFromProfileFile(missionId))
				s_Loaded = true;
			return;
		}

		string baseUrl = TBD_BackendConfig.GetBackendUrl();
		if (baseUrl.EndsWith("/"))
			baseUrl = baseUrl.Substring(0, baseUrl.Length() - 1);

		RestContext ctx = rest.GetContext(baseUrl);
		if (!ctx)
		{
			Print("[TBD] RestContext failed for " + baseUrl, LogLevel.ERROR);
			s_LoadInFlight = false;
			if (LoadFromProfileFile(missionId))
				s_Loaded = true;
			return;
		}

		s_RestCallback = new RestCallback();
		s_RestCallback.SetOnSuccess(OnBackendFetchSuccess);
		s_RestCallback.SetOnError(OnBackendFetchError);

		// Backend guards the game-server tier with X-Service-Token (middleware.RequireServiceToken),
		// not an Authorization bearer — same header the /ingest telemetry endpoints use.
		string token = TBD_BackendConfig.GetServerToken();
		ctx.SetHeaders(string.Format("X-Service-Token,%1,Accept,application/json", token));

		string path = string.Format("/api/v1/missions/%1/compiled", missionId);
		Print("[TBD] Fetching mission " + missionId + " from " + baseUrl + path);
		ctx.GET(s_RestCallback, path);
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnBackendFetchSuccess(RestCallback cb)
	{
		s_LoadInFlight = false;
		string data = cb.GetData();
		if (data.IsEmpty())
		{
			Print("[TBD] Backend returned empty mission body.", LogLevel.ERROR);
			TryProfileFallbackAfterRestFailure();
			return;
		}

		if (!ParseMissionJson(data))
		{
			TryProfileFallbackAfterRestFailure();
			return;
		}

		string missionId = TBD_BackendConfig.GetMissionId();
		CacheToProfile(missionId, data);
		s_Loaded = true;
		LogLoaded("backend");
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnBackendFetchError(RestCallback cb)
	{
		s_LoadInFlight = false;
		Print("[TBD] Backend mission fetch failed — trying profile fallback.", LogLevel.WARNING);
		TryProfileFallbackAfterRestFailure();
	}

	//------------------------------------------------------------------------------------------------
	protected static void TryProfileFallbackAfterRestFailure()
	{
		string missionId = TBD_BackendConfig.GetMissionId();
		if (LoadFromProfileFile(missionId))
		{
			s_Loaded = true;
			LogLoaded("profile");
		}
		else
		{
			TBD_Log.Error(TBD_Log.CH_MISSION, "load failed (REST + profile) — server stays in LOADING");
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static bool LoadFromProfileFile(string missionId)
	{
		string path = string.Format("$profile:missions/%1.json", missionId);
		if (!FileIO.FileExists(path))
		{
			Print("[TBD] Profile mission file missing: " + path, LogLevel.ERROR);
			return false;
		}

		FileHandle handle = FileIO.OpenFile(path, FileMode.READ);
		if (!handle)
		{
			Print("[TBD] Could not open profile mission file: " + path, LogLevel.ERROR);
			return false;
		}

		int fileSize = handle.GetLength();
		if (fileSize > MISSION_FILE_MAX_BYTES)
		{
			Print(string.Format("[TBD] Profile mission file too large (%1 B > %2 B cap): %3 — refusing to parse a truncated read.",
				fileSize, MISSION_FILE_MAX_BYTES, path), LogLevel.ERROR);
			handle.Close();
			return false;
		}

		string data;
		handle.Read(data, MISSION_FILE_MAX_BYTES);
		handle.Close();

		return ParseMissionJson(data);
	}

	//------------------------------------------------------------------------------------------------
	protected static bool ParseMissionJson(string data)
	{
		s_RawJson = data;
		s_Valid = false;

		// Parse a JSON string: JsonLoadContext.LoadFromString (ImportFromString /
		// SCR_JsonLoadContext are both flagged obsolete by the engine).
		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(data))
		{
			Print("[TBD] Mission JSON parse failed.", LogLevel.ERROR);
			return false;
		}

		s_Mission = new TBD_MissionDocumentStruct();
		if (!ctx.ReadValue("", s_Mission))
		{
			Print("[TBD] Mission JSON schema mismatch (meta block).", LogLevel.ERROR);
			s_Mission = null;
			return false;
		}

		// T-181.14 — one validation pass over the whole document. It reports EVERY problem
		// (never just the first) as its own [TBD][Validate] line, then blocks on errors. The
		// meta.id, slots-required, orbat-parity and duplicate-key checks that used to live in
		// this file moved into TBD_MissionValidator so a broken mission surfaces all of its
		// faults in a single reload instead of one per fix.
		//
		// Blocking here is the fail-fast: s_Valid stays false, so TBD_FrameworkManager never
		// leaves LOADING and nobody is stranded in a half-built lobby.
		if (!TBD_MissionValidator.Run(s_Mission))
		{
			s_Mission = null;
			return false;
		}

		s_Valid = true;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.14 — one structured line per successful load:
	//! `[TBD][Mission] loaded id=… name='…' slots=… source=…`.
	protected static void LogLoaded(string source)
	{
		int slotCount = 0;
		if (s_Mission.slots)
			slotCount = s_Mission.slots.Count();

		TBD_Log.MissionLoaded(s_Mission.meta.id, s_Mission.meta.name, slotCount, source);
	}

	//------------------------------------------------------------------------------------------------
	protected static void CacheToProfile(string missionId, string data)
	{
		string dir = "$profile:missions";
		if (!FileIO.MakeDirectory(dir))
		{
			// May already exist — not fatal.
		}

		string path = string.Format("%1/%2.json", dir, missionId);
		FileHandle handle = FileIO.OpenFile(path, FileMode.WRITE);
		if (!handle)
		{
			Print("[TBD] Could not cache mission to " + path, LogLevel.WARNING);
			return;
		}

		handle.Write(data);
		handle.Close();
		Print("[TBD] Cached mission to " + path);
	}
}
