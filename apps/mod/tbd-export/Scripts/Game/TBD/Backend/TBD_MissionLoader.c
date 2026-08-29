//! Minimal parsed mission header - expanded in Phase 1 as loader hardens.
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
//! == MEASURED LANDMINE - a null check is NOT a presence test =================================
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
//! T-181.18 - `polygon` is modelled as of this slice. It was deliberately absent before, which
//! meant every polygon-shaped zone in an authored mission parsed to a shape with nothing in it
//! and was silently unusable. The nested container type is the schema's own shape: an array of
//! `[x, z]` pairs (`#/$defs/polygon`, minItems 3, inner arrays exactly 2 numbers).
//!
//! **Proven, not assumed** (T-181.18): `ref array<ref array<float>>` is a legal Enfusion field
//! type and `JsonLoadContext.ReadValue` compiles against a class containing one - compile probe
//! with a failing negative control (`ref array<ref array<TypeThatDoesNotExist>>` -> `Unknown type`,
//! so the compiler really does check the inner generic argument). Runtime population was then
//! confirmed on a live world boot; see `TBD_ZoneRegistry` for the diagnostic that proves it in
//! every run rather than only in the one that was measured.
//!
//! Nothing here validates the polygon - a ring with fewer than 3 vertices or a pair that is not
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

//! T-181.18 - the zone `rules` object, as much of it as a TYPED parser can see.
//!
//! -- The problem, stated honestly ------------------------------------------------------------
//! `mission.schema.json#/$defs/zoneRules` is a CLOSED 16-key vocabulary (`additionalProperties:
//! false`, T-241). Schema validation rejects undeclared keys; Enfusion's `JsonLoadContext` still
//! only maps JSON keys onto named class fields, so a key this class does not declare is invisible
//! at runtime - not rejected, not logged, simply absent. That is a real limitation of the typed
//! reader and it is written down here rather than papered over. Objective keys
//! (`captureSeconds`, `holdSeconds`, ...) are schema-legal and are read by the second pass in
//! `Objectives/TBD_ObjectiveRules.c`, not here.
//!
//! -- The decision ----------------------------------------------------------------------------
//! Carry the rules the framework actually consumes as named fields with ABSENT sentinels, and make
//! the absence detectable. `TBD_ZoneRegistry.ResolveRules` then reports, per zone and by id:
//!   * a `rules` object that parsed but yielded no key this build understands  -> WARNING
//!   * a key present but out of range                                          -> ERROR + default
//!   * a `penalty` string this build does not recognise                        -> ERROR + default
//! so an authored rule that cannot be honoured is LOUD, never a silent default. What it cannot do
//! is name an unknown key, because a typed parser never sees one. Adding a rule to the vocabulary
//! is a field here (or in `TBD_ObjectiveRulesStruct`) plus a `#/$defs/zoneRules` property plus a
//! case in `ResolveRules` - that is the whole cost (T-241).
//!
//! -- The play-area vocabulary (closed; `#/$defs/zoneRules`, `additionalProperties: false`) ---
//! `graceSeconds`     number >= 0 - seconds a player may remain in violation before the penalty.
//! `warnEverySeconds` number >  0 - how often the player is told, while in violation.
//! `penalty`          string      - "none" | "warn" | "kill". See TBD_EZonePenalty for why the
//!                                  default is deliberately NOT "kill" under one life.
//! @contract mission.schema.json#/$defs/zoneRules
class TBD_MissionZoneRulesStruct
{
	//! Sentinel for "key absent from JSON". Same device `TBD_MissionSlotStruct.Y_ABSENT` uses and
	//! for the same reason: `JsonLoadContext` leaves a missing key at its field initializer, and
	//! standard JSON cannot carry NaN. No sane authored value approaches -1e6, so the initializer
	//! doubles as the presence flag - which is what lets a bad authored value (a NEGATIVE grace,
	//! say) be reported as an error instead of being mistaken for "not authored".
	static const float ABSENT = -1000000;

	float graceSeconds = ABSENT;
	float warnEverySeconds = ABSENT;
	string penalty;   //!< Empty string = absent (JsonLoadContext leaves it at the initializer).
}

//! One entry from the mission `zones[]` array (spawn, objective, boundary, ...).
//! @contract mission.schema.json#/$defs/zone
class TBD_MissionZoneStruct
{
	string id;
	string type;
	//! Human name authored in the editor ("Levie Bridge"). OPTIONAL in the schema - a zone that
	//! omits it parses to an empty string, which is why callers fall back to `type`+`id` rather
	//! than assuming a label exists.
	string label;
	string faction;
	ref TBD_MissionShapeStruct shape;
	//! T-181.18 - OPTIONAL in the schema; null when the zone authored no rules at all. A non-null
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

//! T-181.13 - how the round ends, straight from the mission JSON.
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
//! marker that exists at all is complete - there is no partial-marker case to defend against.
//! @contract mission.schema.json#/$defs/marker
class TBD_MissionMarkerStruct
{
	float x;      //!< World X, metres.
	float z;      //!< World Z, metres.
	string icon;  //!< Icon key authored in the editor ("objective", "defend", "destroy").
	string label; //!< Marker caption ("OBJ BRIDGE").
}

//! T-181.23 - one faction's WRITTEN ORDERS. This is the Arma-3 briefing text the whole briefing
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

//! T-181.38 - the mission `flow` block: the numbers that PACE an event.
//!
//! Every golden mission authors all four fields and, until this slice, the block was not in
//! `TBD_MissionDocumentStruct` at all - so `safeStartSeconds` was overridden by a hardcoded 300,
//! `timeLimitSeconds` was authored by every mission and evaluated by none, and `jip: "disabled"`
//! was silently violated. "JSON is the contract" (TBD_MOD_DESIGN.md S2) is exactly what that
//! failed.
//!
//! == EVERY FIELD IS OPTIONAL, AND `0` IS NOT `absent` ===========================================
//! `flow` itself is schema-required, but it declares no `required` PROPERTIES, so a mission may
//! author `"flow": {}` - `golden-missions/empty-warning-fields.json` does exactly that, and it must
//! behave identically to a build that never saw a flow block.
//!
//! `JsonLoadContext` ALLOCATES a nested `ref <class>` field even when the JSON key is ABSENT
//! (measured 2026-07-25 - see the landmine header on `TBD_MissionShapeStruct` above), so
//! `if (doc.flow)` is ALWAYS TRUE and is not a presence test. Worse, an absent integer key would
//! read back as `0` - and `0` is a REAL authored value here: `timeLimitSeconds: 0` means "no time
//! limit", a deliberate statement rather than silence. A plain zero-test would erase the
//! distinction between "the author said no limit" and "the author said nothing".
//!
//! Hence the ABSENT sentinel, the same device `TBD_MissionSlotStruct.Y_ABSENT` and
//! `TBD_MissionZoneRulesStruct.ABSENT` use and for the same reason: `JsonLoadContext` leaves a
//! missing key at its field INITIALIZER, and standard JSON cannot carry NaN. No sane authored
//! duration approaches -1e6, so the initializer doubles as the presence flag - which is also what
//! lets a NEGATIVE authored value (illegal under the schema's `minimum: 0`) be reported as a fault
//! instead of mistaken for "not authored".
//!
//! `jip` uses the empty string for the same purpose, exactly like
//! `TBD_MissionZoneRulesStruct.penalty`.
//!
//! Nothing here validates. An out-of-range value parses fine and is caught, named and reported
//! where it is APPLIED - `TBD_MissionFlow` / `TBD_FrameworkManager.ApplyMissionFlow`, which is the
//! only place these turn into behaviour.
//! @contract mission.schema.json#/$defs/flow
class TBD_MissionFlowStruct
{
	//! "key absent from JSON". A presence flag, not a magic default - see the header.
	static const int ABSENT = -1000000;

	int briefingSeconds = ABSENT;  //!< Intended length of the BRIEFING stage. Schema: integer >= 0.
	int safeStartSeconds = ABSENT; //!< Safestart countdown length. Schema: integer >= 0.
	int timeLimitSeconds = ABSENT; //!< Round length; an authored 0 means "no limit". Schema: >= 0.
	string jip;                    //!< "disabled" | "until_safestart_end" | "always". Empty = absent.
}

//! One mission-placed world object (`mission.schema.json#/$defs/entity`) - T-254.
//! Field names must equal the JSON keys. OPTIONAL inventory is ignored here (no consumer yet).
//! @contract mission.schema.json#/$defs/entity
class TBD_MissionEntityStruct
{
	string alias;      //!< Registry alias (`prop:`/`comp:`/...). Schema-required.
	float x;           //!< World X metres. Schema-required.
	float z;           //!< World Z metres. Schema-required.
	float headingDeg;  //!< Yaw degrees 0..360. OPTIONAL - 0 when absent (JsonLoadContext default).
	string faction;    //!< Faction key. OPTIONAL - empty when absent.
}

//! Mission policy block (`mission.schema.json#/$defs/settings`) - T-259.
//!
//! All three fields are OPTIONAL. Empty string on `respawn` / `spectatorPolicy` means the key was
//! absent (JsonLoadContext leaves string fields at `""`). `nightVision` defaults to `false`, so
//! "absent" and "authored false" are indistinguishable - that matches the schema's boolean with
//! no presence flag, and the default is "NVG off".
//!
//! `JsonLoadContext` ALLOCATES this nested `ref` even when the JSON key is ABSENT - same trap as
//! `flow`. Callers that care about "was settings authored?" must not null-check this reference;
//! they read field values (or the raw JSON via `GetRawJson`).
//! @contract mission.schema.json#/$defs/settings
class TBD_MissionSettingsStruct
{
	string respawn;          //!< "none" | "tickets" | "wave". Empty = absent.
	string spectatorPolicy;  //!< "none" | "own_side_delayed_60s" | "free". Empty = absent.
	bool nightVision;        //!< Authored NVG policy. Default false = off / absent.
}

//! Full mission document parsed from the backend - the canonical contract the loader
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
	//! T-254 - mission-placed world objects (`entities[]`). OPTIONAL: missions authored before
	//! this field existed carry none, so null here is legal. When present, `SpawnMissionEntities`
	//! places them so destroy-alias resolution can find them in the world.
	//! @contract mission.schema.json#/properties/entities
	ref array<ref TBD_MissionEntityStruct> entities;
	ref TBD_MissionWinConditionsStruct winConditions;         //!< T-181.13 round-end triggers.
	//! T-181.38 - event pacing. ALWAYS non-null after a parse, even for a mission with no `flow`
	//! key: `JsonLoadContext` allocates it regardless. Test its FIELDS against
	//! `TBD_MissionFlowStruct.ABSENT`, never this reference against null.
	ref TBD_MissionFlowStruct flow;
	//! T-181.23 - written orders keyed by faction key, exactly like `orbat`. OPTIONAL: the block
	//! is not in the schema's top-level `required` list and every mission authored before it
	//! existed has none, so this stays null and that is legal.
	ref map<string, ref TBD_MissionBriefingStruct> briefings;
	//! T-181.40 / T-293 - the radio plan. ALWAYS non-null after a parse even when the JSON key is
	//! absent - `JsonLoadContext` allocates nested `ref` fields regardless. Presence is a CONTENT
	//! test on `nets` (count / `freqMHz` sentinel) inside `TBD_RadioPlan`, never a null check here.
	//! An unauthored plan (e.g. `empty-warning-fields.json`) is legal: empty/absent nets, no error.
	//! @contract mission.schema.json#/$defs/radioPlan
	ref TBD_MissionRadioPlanStruct radioPlan;
	//! T-259 - mission policy (respawn / spectator / NVG). ALWAYS non-null after a parse even
	//! when the JSON key is absent - `JsonLoadContext` allocates nested `ref` fields regardless.
	//! Read field values (empty string / false), do not null-check this reference.
	//! @contract mission.schema.json#/$defs/settings
	ref TBD_MissionSettingsStruct settings;
}

//! Loads Mission JSON from backend REST or $profile fallback.
//! @route GET /api/v1/missions/{id}/compiled (service-token tier; body = this canonical document, T-092.2).
class TBD_MissionLoader
{
	//! Hard cap on a mission body (profile file or REST `/compiled` payload). A profile file
	//! over this would silently truncate in Read() and then fail JSON parse with a misleading
	//! error - reject it up front (T-130.4 F1-16). T-456 applies the same ceiling to
	//! `OnBackendFetchSuccess` so a compromised/stale API path cannot hand the mod an oversized
	//! JSON that skips the profile gate.
	//! T-450: the SAME ceiling is pinned on mission.schema.json as `x-tbd-missionFileMaxBytes`
	//! (8388608) and enforced by `validate_mission_document` before `/compiled` serves a body.
	//! Do not change this constant without updating the schema keyword and the API/xtask checks.
	protected static const int MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024;

	//! Cap on a backend error body echoed into the log (T-181.44). Print() discards a line over
	//! 1024 bytes rather than truncating it, so the body must be cut here or it is never seen.
	//! Sized so the cut body plus the `[TBD][Mission] ... http=... body=` prefix and the truncation
	//! marker still clear 1024 - the whole point is that this line SURVIVES.
	protected static const int ERROR_BODY_LOG_MAX_BYTES = 900;

	//! T-181.54 - the one status that means "this id was never a backend mission". The API validates
	//! the id's SHAPE before looking anything up, so a non-uuid (a golden's `msn_*`) comes back 400
	//! `{"error":"invalid id"}` rather than 404. That distinguishes a mission deliberately staged on
	//! disk from a stale cache, which is the difference between a normal run and a real fault.
	//! MEASURED against the live API 2026-07-25, not assumed: 400 for `msn_8f3a2c`.
	protected static const int HTTP_BAD_REQUEST = 400;

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
			// B1 - uid-aware lookup: durable uid matches first, display id stays valid.
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
	//! T-181.13 - true when the mission declared this end trigger. Missions authored before
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
	//! T-181.23 - the written orders for ONE faction, or null when this mission authored none for
	//! that side.
	//!
	//! Faction-keyed exactly like `orbat`, so a caller passes the same key it already resolved from
	//! the player's slot - which is what keeps side discipline enforceable: the server hands out one
	//! side's orders and never the other's.
	//!
	//! Returns null (not an empty struct) so a caller can tell "this mission has no orders for me"
	//! apart from "orders exist but are blank". Absent `briefings` is LEGAL - the block is optional
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
	//! T-181.18 - the raw `zones[]` array, or null when no VALID mission is loaded.
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
	//! T-254 - the raw `entities[]` array, or null when no VALID mission is loaded / none authored.
	//! @contract mission.schema.json#/properties/entities
	static array<ref TBD_MissionEntityStruct> GetEntities()
	{
		if (!s_Valid || !s_Mission)
			return null;

		return s_Mission.entities;
	}

	//------------------------------------------------------------------------------------------------
	//! T-259 - the mission policy block (`settings`), or null when no VALID mission is loaded.
	//!
	//! The reference itself is ALWAYS allocated after a successful parse (JsonLoadContext), even
	//! when the JSON key was absent - read `respawn` / `spectatorPolicy` (empty = absent) and
	//! `nightVision` rather than null-checking the return. Returns null only when no valid
	//! mission is loaded, mirroring `GetEntities` / `GetZones`.
	//! @contract mission.schema.json#/$defs/settings
	static TBD_MissionSettingsStruct GetSettings()
	{
		if (!s_Valid || !s_Mission)
			return null;

		return s_Mission.settings;
	}

	//------------------------------------------------------------------------------------------------
	//! T-254 - spawn every authored `entities[]` row into the world so destroy-alias resolution
	//! (`TBD_ObjectiveRegistry.ArmDestroyTargets`) can find matching prefabs inside its zone.
	//!
	//! Resolves each `alias` through `TBD_Registry` (auto-loads if needed). Rows whose alias is not
	//! in `Data/registry.json` are skipped with a warning - extending that file with `prop:`/`comp:`
	//! rows is outside this loader's owns when the Objects palette synthesises new aliases.
	//! Idempotent for a given load: call once after a valid parse (see `ParseMissionJson`).
	static void SpawnMissionEntities()
	{
		array<ref TBD_MissionEntityStruct> entities = GetEntities();
		if (!entities || entities.Count() == 0)
			return;

		int spawned = 0;
		int skipped = 0;
		foreach (TBD_MissionEntityStruct ent : entities)
		{
			if (!ent || ent.alias.IsEmpty())
			{
				skipped++;
				continue;
			}

			bool ok;
			ResourceName prefab = TBD_Registry.Resolve(ent.alias, ok);
			if (!ok || prefab.IsEmpty())
			{
				Print(string.Format("[TBD][Entities] skip alias='%1' - not in registry", ent.alias), LogLevel.WARNING);
				skipped++;
				continue;
			}

			Resource resource = Resource.Load(prefab);
			if (!resource || !resource.IsValid())
			{
				Print(string.Format("[TBD][Entities] Resource.Load failed for alias='%1' prefab=%2", ent.alias, prefab), LogLevel.ERROR);
				skipped++;
				continue;
			}

			float surfaceY = GetGame().GetWorld().GetSurfaceY(ent.x, ent.z);
			vector pos = Vector(ent.x, surfaceY, ent.z);

			EntitySpawnParams params = new EntitySpawnParams();
			params.TransformMode = ETransformMode.WORLD;
			Math3D.MatrixIdentity4(params.Transform);
			params.Transform[3] = pos;

			float yawRad = ent.headingDeg * Math.DEG2RAD;
			params.Transform[0] = Vector(Math.Cos(yawRad), 0, Math.Sin(yawRad));
			params.Transform[2] = Vector(-Math.Sin(yawRad), 0, Math.Cos(yawRad));

			IEntity body = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
			if (!body)
			{
				Print(string.Format("[TBD][Entities] SpawnEntityPrefab failed for alias='%1'", ent.alias), LogLevel.ERROR);
				skipped++;
				continue;
			}

			spawned++;
			Print(string.Format("[TBD][Entities] spawned alias='%1' at %2 heading=%3", ent.alias, pos.ToString(), ent.headingDeg));
		}

		Print(string.Format("[TBD][Entities] spawn done spawned=%1 skipped=%2", spawned, skipped));
	}

	//------------------------------------------------------------------------------------------------
	//! T-259 - apply authored `settings` through the smallest published seams this loader can
	//! reach without editing files outside its owns.
	//!
	//! `spectatorPolicy`:
	//!   - `"free"`                 -> `TBD_SpectatorTargets.SetFactionRestricted(false)`
	//!   - `"own_side_delayed_60s"` -> `SetFactionRestricted(true)` (own-side follow-cam; the 60 s
	//!     delay itself is owned by SpectatorController and is NOT switched here)
	//!   - `"none"` / empty         -> leave the SpectatorTargets default (restricted ON). Full
	//!     "black screen, no spectator" needs SpectatorController entry points - outside owns.
	//!
	//! `respawn` and `nightVision` have no published setter reachable from this file. Logged so
	//! an authored value is visible in the boot log rather than silently ignored.
	//! @authority server
	protected static void ApplyMissionSettings()
	{
		TBD_MissionSettingsStruct s = GetSettings();
		if (!s)
			return;

		if (s.spectatorPolicy == "free")
		{
			TBD_SpectatorTargets.SetFactionRestricted(false);
			Print("[TBD][Settings] spectatorPolicy=free -> faction restriction OFF", LogLevel.NORMAL);
		}
		else if (s.spectatorPolicy == "own_side_delayed_60s")
		{
			TBD_SpectatorTargets.SetFactionRestricted(true);
			Print("[TBD][Settings] spectatorPolicy=own_side_delayed_60s -> faction restriction ON (delay owned by SpectatorController)", LogLevel.NORMAL);
		}
		else if (s.spectatorPolicy == "none")
		{
			Print("[TBD][Settings] spectatorPolicy=none - no black-screen seam in MissionLoader owns; SpectatorTargets left at default", LogLevel.WARNING);
		}
		else if (!s.spectatorPolicy.IsEmpty())
		{
			Print(string.Format("[TBD][Settings] spectatorPolicy='%1' unrecognised - SpectatorTargets left at default", s.spectatorPolicy), LogLevel.WARNING);
		}

		if (!s.respawn.IsEmpty())
			Print(string.Format("[TBD][Settings] respawn='%1' authored but no respawn-pool setter in MissionLoader owns", s.respawn), LogLevel.WARNING);

		// nightVision: bool defaults false, so only log the authored-true case as the interesting
		// one. Authored false and absent both read as false - no NVG seam here either.
		if (s.nightVision)
			Print("[TBD][Settings] nightVision=true authored but no NVG setter in MissionLoader owns", LogLevel.WARNING);
	}

	//------------------------------------------------------------------------------------------------
	//! World-space spawn point for a faction key. Returns vector.Zero if no spawn zone exists.
	//!
	//! T-181.18 note: still CIRCLE-ONLY on purpose. Polygons now parse (see
	//! `TBD_MissionShapeStruct`), but teaching spawn placement to pick a point inside a polygon is
	//! a different problem from testing containment - it needs a point that is on navigable ground,
	//! not just inside the ring, and a centroid is not that for a concave AO. Out of scope for the
	//! play-area slice; a polygon spawn zone remains unusable here, and it now says so instead of
	//! quietly answering with the wrong coordinates.
	//!
	//! T-181.18 BUG FIX (found by the runtime probe, latent until now): the old guard was
	//! `if (!zone.shape || !zone.shape.circle) continue;`. `shape.circle` is ALWAYS non-null - see
	//! the landmine on `TBD_MissionShapeStruct` - so a polygon-shaped spawn zone slipped past the
	//! guard and this returned `Vector(0, GetSurfaceY(0,0), 0)`: the corner of the map. It was
	//! latent only because no shipped mission yet authors a polygon spawn zone. The guard now tests
	//! the radius, which is real data. The matching dead guard in
	//! `TBD_MissionValidator.CheckZones` (same shape, same reason it can never fire) is reported to
	//! the command center rather than edited from here - that file is not this slice's to write.
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
			Print("[TBD] missionId not configured - cannot load mission.", LogLevel.ERROR);
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
			Print("[TBD] RestApi unavailable - trying profile fallback.", LogLevel.WARNING);
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
		// not an Authorization bearer - same header the /ingest telemetry endpoints use.
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
			// 0 = the backend ANSWERED, so this is not a shape rejection; a cache here may be stale.
			TryProfileFallbackAfterRestFailure(0);
			return;
		}

		// T-456 - REST path must honour the same MISSION_FILE_MAX_BYTES ceiling as profile load.
		// A compromised/stale API could otherwise hand the mod an oversized body that skips the
		// profile FileHandle.GetLength() gate and still reaches ParseMissionJson.
		if (!IsMissionBodyWithinCap(data))
		{
			Print(string.Format("[TBD] Backend mission body too large (%1 B > %2 B cap) - refusing to parse.",
				data.Length(), MISSION_FILE_MAX_BYTES), LogLevel.ERROR);
			TryProfileFallbackAfterRestFailure(0);
			return;
		}

		if (!ParseMissionJson(data))
		{
			TryProfileFallbackAfterRestFailure(0);
			return;
		}

		string missionId = TBD_BackendConfig.GetMissionId();
		CacheToProfile(missionId, data);
		s_Loaded = true;
		LogLoaded("backend");
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.44 - the response body is the ONLY place the reason lives, and this used to bin it.
	//!
	//! `/compiled` validates the document before serving it (T-181.31) and answers 500 with the
	//! deduped, capped findings when it does not hold - `/slots/3/groupCallsign does not match
	//! wireSafeString` for a callsign somebody typed a TAB into, `winConditions.endOn declares
	//! faction_eliminated but only 1 faction has slots`, a dangling kit alias. That is EVERY schema
	//! rejection, not one class of them. Discarding it left the operator with the symptom alone -
	//! "the server is still running the previous mission" - and the cause in an API log on another
	//! host, which is the definition of undiagnosable.
	//!
	//! `RestCallback.GetData()` is documented as readable from the error callback ("you can access
	//! data if any were provided by the RestApi"). On a transport failure it is empty; that is still
	//! information, and the message says which case it is rather than printing a bare blank.
	//! `TBD_ResultsReporter.OnSendError` already logs its body for the same reason.
	protected static void OnBackendFetchError(RestCallback cb)
	{
		s_LoadInFlight = false;

		int httpCode = cb.GetHttpCode();
		TBD_Log.Error(TBD_Log.CH_MISSION, string.Format(
			"backend refused the mission fetch - http=%1 body=%2", httpCode, FetchFailureBody(cb.GetData())));

		TryProfileFallbackAfterRestFailure(httpCode);
	}

	//------------------------------------------------------------------------------------------------
	//! The error body, made safe to log. `Print()` drops a line over 1024 bytes ENTIRELY rather
	//! than truncating it, so an uncapped body would log nothing at all - the exact failure mode
	//! this is here to fix. The cap leaves room for the `[TBD][Mission] ... http=... body=` prefix.
	protected static string FetchFailureBody(string body)
	{
		if (body.IsEmpty())
			return "<none - the request did not reach the API, or it answered with nothing>";

		if (body.Length() > ERROR_BODY_LOG_MAX_BYTES)
			return body.Substring(0, ERROR_BODY_LOG_MAX_BYTES) + "...<truncated>";

		return body;
	}

	//------------------------------------------------------------------------------------------------
	//! @param httpCode the status the backend answered with, or 0 when there was no HTTP failure
	//!        (unreachable API, empty body, unparseable document). It selects the message below,
	//!        so pass the real code - a wrong one mislabels a deliberate stage as a stale cache.
	protected static void TryProfileFallbackAfterRestFailure(int httpCode)
	{
		string missionId = TBD_BackendConfig.GetMissionId();
		if (LoadFromProfileFile(missionId))
		{
			s_Loaded = true;

			// T-181.44 - name what the operator is actually looking at, instead of an
			// indistinguishable `source=profile`. T-181.54 CORRECTS what that message claimed.
			//
			// The old text said the document is "not the one configured". That is NEVER true:
			// `LoadFromProfileFile` reads `$profile:missions/<missionId>.json`, so the file is
			// KEYED BY the configured id and the mission identity always matches. What can differ
			// is the document's VERSION - a cache holds whatever the backend last served for that
			// id, which may be older than what it would serve now.
			//
			// And there is a second, entirely legitimate way to arrive here that the old text
			// mislabelled as a fault: a mission STAGED on disk on purpose (a golden via
			// `scripts/mod/test-mission.sh`, or `world-boot.sh --mission=`). A golden's `msn_*` id
			// is not a uuid, so the backend rejects its SHAPE with 400 before ever looking for it.
			// That is the discriminator - a 400 means this id was never a backend mission, so the
			// profile file is the intended source, not a stale leftover.
			if (httpCode == HTTP_BAD_REQUEST)
			{
				TBD_Log.Event(TBD_Log.CH_MISSION,
					"loaded a mission STAGED ON DISK - the backend rejected this id's shape (400), so it was never a backend mission and this file is the intended source. NOTE: this path applies NO json-schema validation, only the more permissive TBD_MissionValidator.");
			}
			else
			{
				TBD_Log.Warn(TBD_Log.CH_MISSION,
					"RUNNING A CACHED MISSION - same mission id as configured, but this document is whatever the backend last served for it and may be an OLDER VERSION. Fix the failure logged above and restart the server.");
			}

			LogLoaded("profile-fallback");
		}
		else
		{
			TBD_Log.Error(TBD_Log.CH_MISSION, "load failed (REST + profile) - server stays in LOADING");
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
			Print(string.Format("[TBD] Profile mission file too large (%1 B > %2 B cap): %3 - refusing to parse a truncated read.",
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
	//! T-456 - shared body ceiling against `MISSION_FILE_MAX_BYTES` (REST path; profile uses the
	//! same constant via `FileHandle.GetLength()` before Read so a truncated read never lands).
	//! `string.Length()` is byte-counted in Enforce (same unit as the file-size gate).
	protected static bool IsMissionBodyWithinCap(string data)
	{
		return data.Length() <= MISSION_FILE_MAX_BYTES;
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

		// T-181.14 - one validation pass over the whole document. It reports EVERY problem
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

		// T-254 - place authored entities[] so destroy targets can exist in the world. Registry
		// Resolve auto-loads; unknown aliases are warned and skipped inside SpawnMissionEntities.
		SpawnMissionEntities();

		// T-259 - hand spectatorPolicy to the published SpectatorTargets seam. Respawns and NVG
		// have no published setter inside this file's owns; see ApplyMissionSettings.
		ApplyMissionSettings();

		// T-181.13.1 - a valid mission document is the earliest moment an end-of-round results
		// report could mean anything, and this is a server-only path (BeginLoad is reached only
		// after TBD_FrameworkManager.OnPostInit returns early for RplMode.Client). Arm() is
		// idempotent, so the REST-then-profile fallback calling this twice is harmless.
		TBD_ResultsReporter.Arm();

		// T-181.35 - the OTHER half of the same contract. The results POST can only join on
		// `users.arma_id`, and nothing writes that column until a player confirms a link code in
		// game. Armed on the same server-only path, for the same reason, and idempotent for the
		// same reason. Arming also puts one line in every boot's log saying whether linking can
		// work on this host at all.
		TBD_IdentityLink.Arm();

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.14 - one structured line per successful load:
	//! `[TBD][Mission] loaded id=... name='...' slots=... source=...`.
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
			// May already exist - not fatal.
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
