//! T-181.39 - the OBJECTIVE half of `zones[].rules`, read by a second typed pass over the very
//! same mission JSON the loader already parsed.
//!
//! == Why a second parse, and why that is not duplication ======================================
//! `mission.schema.json#/$defs/zoneRules` is a CLOSED 16-key vocabulary (`additionalProperties:
//! false`, T-241). Schema validation is the only place a misspelled rule key is caught: Enfusion's
//! `JsonLoadContext` maps JSON keys onto NAMED class fields, so a key no class declares is
//! invisible at runtime - not rejected, not logged, simply absent. The mission loader's
//! `TBD_MissionZoneRulesStruct` therefore declares exactly the three keys the PLAY-AREA subsystem
//! consumes (`graceSeconds`, `warnEverySeconds`, `penalty`) and is structurally blind to
//! `captureSeconds`, `contestable`, `holdSeconds`, `targetAlias` and `points` - the keys the
//! golden missions actually author on objective zones. Those keys are still schema-legal; they are
//! simply read by THIS pass.
//!
//! Two ways to fix that:
//!   1. add the objective keys to `TBD_MissionZoneRulesStruct` in `Backend/TBD_MissionLoader.c`;
//!   2. run a SECOND `JsonLoadContext` pass over `TBD_MissionLoader.GetRawJson()` with a struct
//!      that declares only what this subsystem needs.
//!
//! (2) is what this file does, for two reasons. The mechanical one: `Backend/**` belongs to another
//! slice's lane and a cross-lane edit to a shared parser is exactly the kind of change that turns
//! into a merge conflict nobody reviews. The design one: it keeps the objective vocabulary NEXT TO
//! the code that interprets it, so adding a rule is one file rather than two (plus the matching
//! `#/$defs/zoneRules` property - T-241), and it makes the subsystem's dependency on the document
//! explicit rather than smuggled through a shared struct that grows a field per consumer. The cost
//! is one extra parse of a document that is at most 8 MB and is parsed exactly ONCE per world -
//! off the hot path entirely.
//!
//! If a later slice consolidates the two, the seam is `TBD_ObjectiveRulesReader.ForZone()`: point
//! it at `TBD_MissionZoneStruct.rules` instead and delete the structs below. Nothing else changes.
//!
//! == The presence problem, and why bools are the exception ====================================
//! `JsonLoadContext` ALLOCATES a nested `ref <class>` field even when the JSON key is absent
//! (measured - see the landmine on `TBD_MissionShapeStruct`), so `if (zone.rules)` is ALWAYS true
//! and is not a presence test. Numeric and string fields therefore carry an ABSENT sentinel exactly
//! like `TBD_MissionZoneRulesStruct.ABSENT` does.
//!
//! **Bools cannot carry a sentinel** - there is no third value - so "absent" and "authored false"
//! are indistinguishable here and always will be. That is stated rather than worked around, and it
//! costs NOTHING behaviourally because every bool below is initialised to the value an author gets
//! by writing nothing:
//!   * `contestable = true`   - absent -> true, authored `true` -> true, authored `false` -> false.
//!   * `pauseOnEnemy = true`  - same shape.
//!   * `resetOnEnemy = false` / `requireHolderPresent = false` - absent and authored-false agree.
//! All three cases are handled correctly. The only thing lost is the ability to say "you did not
//! author this key", which is a diagnostic, not a behaviour.
//! @contract mission.schema.json#/$defs/zoneRules

//------------------------------------------------------------------------------------------------
//! The objective half of the CLOSED `zoneRules` vocabulary (`additionalProperties: false`, T-241).
//! Every key below is declared in `#/$defs/zoneRules`; undeclared keys fail schema validation.
//! Adding a rule is a field here PLUS a property there - not an open-object free-for-all.
//!
//! -- objective_capture -----------------------------------------------------------------------
//! `captureSeconds`       number > 0  - uninterrupted presence needed to take a NEUTRAL objective.
//! `neutralizeSeconds`    number >= 0 - presence needed to tear a held objective back to neutral.
//!                                      Defaults to `captureSeconds` (a symmetric 1:1 rate).
//!                                      `0` means instant: a single-stage capture.
//! `contestable`          bool        - does an enemy inside stop the capture? See TBD_Objective.
//! `onEmpty`              "hold"|"decay" - partial progress when NOBODY is inside.
//! `decayRate`            number > 0  - progress-seconds lost per real second while decaying.
//! `announceEverySeconds` number > 0  - how often a player standing on it is told the progress.
//! `points`               number >= 0 - carried and reported; NOT consumed by any end trigger here.
//!
//! -- objective_hold_until --------------------------------------------------------------------
//! `holdSeconds`          number > 0  - REQUIRED. How long the zone's faction must hold.
//! `pauseOnEnemy`         bool        - an enemy inside pauses the hold clock.
//! `resetOnEnemy`         bool        - an enemy inside resets the hold clock to zero.
//! `requireHolderPresent` bool        - the clock only runs while a friendly is inside.
//! `announceEverySeconds` number > 0
//! `points`               number >= 0
//!
//! -- objective_destroy -----------------------------------------------------------------------
//! `targetAlias`          string      - REQUIRED. Registry alias of the thing to destroy.
//! `targetCount`          int >= 0    - how many must die. `0`/absent = all of them.
//! `points`               number >= 0
class TBD_ObjectiveRulesStruct
{
	//! Sentinel for "key absent from JSON". Same device and same reasoning as
	//! `TBD_MissionZoneRulesStruct.ABSENT`: `JsonLoadContext` leaves a missing key at its field
	//! initializer and standard JSON cannot carry NaN, so an initializer no sane author would type
	//! doubles as the presence flag. This is what lets a NEGATIVE authored value be reported as an
	//! error instead of being mistaken for "not authored".
	static const float ABSENT = -1000000;

	//! Same idea for the one integer field. Kept separate from `ABSENT` because `targetCount: 0`
	//! is a MEANINGFUL authored value ("all of them"), so the sentinel must not be 0.
	static const int ABSENT_INT = -1;

	float captureSeconds = ABSENT;
	float neutralizeSeconds = ABSENT;
	float holdSeconds = ABSENT;
	float decayRate = ABSENT;
	float announceEverySeconds = ABSENT;
	float points = ABSENT;

	int targetCount = ABSENT_INT;

	string targetAlias;   //!< Empty = absent (JsonLoadContext leaves it at the initializer).
	string onEmpty;       //!< Empty = absent.

	bool contestable = true;
	bool pauseOnEnemy = true;
	bool resetOnEnemy = false;
	bool requireHolderPresent = false;
}

//------------------------------------------------------------------------------------------------
//! Just enough of a zone to join this pass onto the loader's. `shape` is deliberately NOT declared:
//! geometry belongs to T-181.18's `TBD_Zone` and re-parsing it here would create a second source of
//! truth for where an objective is.
class TBD_ObjectiveZoneStruct
{
	string id;
	string type;
	ref TBD_ObjectiveRulesStruct rules;
}

//------------------------------------------------------------------------------------------------
//! The document root for the *objective-rules* second pass: declares only `zones`. Every other
//! top-level key (meta, factions, orbat, slots, entities, ...) is deliberately not declared here so
//! this reader stays blind to them. That is NOT a claim about the primary loader -
//! `TBD_MissionDocumentStruct` models `entities[]` (T-254) and `SpawnMissionEntities` places them;
//! this struct only re-parses zone `rules.*` for objectives.
class TBD_ObjectiveDocStruct
{
	ref array<ref TBD_ObjectiveZoneStruct> zones;
}

//------------------------------------------------------------------------------------------------
//! Reads the objective rules once and hands them out by zone.
//!
//! -- Server-side only ------------------------------------------------------------------------
//! Reads `TBD_MissionLoader.GetRawJson()`, which is empty on a client (clients hold NO mission
//! document - `TBD_FrameworkManager.OnPostInit` returns early for `RplMode.Client` before
//! `BeginLoad()`). A client calling `Read()` gets a clean `false` and no objectives, which is the
//! correct answer for a machine that is not the authority.
//!
//! -- Static, and therefore explicitly cleared ------------------------------------------------
//! Statics OUTLIVE A WORLD inside one process (recorded landmine - `SelectMissionByNumber` restarts
//! the scenario in-process). `TBD_ObjectivesComponent.OnDelete` is required to call `Clear()` (via
//! `TBD_ObjectiveRegistry.Clear()`), or mission B's objectives would run on mission A's rules.
class TBD_ObjectiveRulesReader
{
	protected static ref array<ref TBD_ObjectiveZoneStruct> s_aZones;
	protected static bool s_bRead;
	protected static bool s_bOk;

	//------------------------------------------------------------------------------------------------
	static bool IsRead()
	{
		return s_bRead;
	}

	//------------------------------------------------------------------------------------------------
	//! True when the second pass actually produced a `zones[]` array. False means every objective
	//! runs on defaults, which is reported once by the caller rather than per zone.
	static bool IsOk()
	{
		return s_bOk;
	}

	//------------------------------------------------------------------------------------------------
	static void Clear()
	{
		s_aZones = null;
		s_bRead = false;
		s_bOk = false;
	}

	//------------------------------------------------------------------------------------------------
	//! How many zones this pass saw. Compared against the loader's count by the caller so a
	//! divergence between the two parses is caught rather than silently mis-joined.
	static int Count()
	{
		if (!s_aZones)
			return 0;

		return s_aZones.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! Parse. Idempotent: only the first call after a `Clear()` does work.
	static bool Read()
	{
		if (s_bRead)
			return s_bOk;

		s_bRead = true;
		s_bOk = false;

		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return false;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
			return false;

		TBD_ObjectiveDocStruct doc = new TBD_ObjectiveDocStruct();
		if (!ctx.ReadValue("", doc))
			return false;

		if (!doc.zones)
			return false;

		s_aZones = doc.zones;
		s_bOk = true;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! The rules for one zone, or null when this pass has nothing for it.
	//!
	//! -- The join, and why it is index-first -------------------------------------------------
	//! Both passes read the SAME JSON array in the same order, so `zones[i]` here is `zones[i]`
	//! there - that is a property of the parser, not a coincidence, and it is the only join that
	//! works for a zone whose `id` is blank. It is nevertheless VERIFIED against the id rather than
	//! trusted: `TBD_ZoneRegistry.Build()` skips null entries without inserting them, so a document
	//! containing `null` in `zones[]` would shift the loader's indices relative to this pass. When
	//! the ids disagree the join falls back to a by-id search, and only then gives up.
	//!
	//! Duplicate ids are possible (the schema requires `minLength: 1`, not uniqueness), which is
	//! precisely why the by-id search is the FALLBACK and not the primary.
	static TBD_ObjectiveRulesStruct ForZone(int index, string zoneId)
	{
		if (!s_aZones)
			return null;

		if (index >= 0 && index < s_aZones.Count())
		{
			TBD_ObjectiveZoneStruct atIndex = s_aZones[index];
			if (atIndex && atIndex.id == zoneId)
				return atIndex.rules;
		}

		if (zoneId.IsEmpty())
			return null;

		foreach (TBD_ObjectiveZoneStruct zone : s_aZones)
		{
			if (zone && zone.id == zoneId)
				return zone.rules;
		}

		return null;
	}
}
