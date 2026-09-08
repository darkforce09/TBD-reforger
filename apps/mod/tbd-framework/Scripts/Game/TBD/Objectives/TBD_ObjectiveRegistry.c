//! T-181.39 — turns the mission document's objective zones into prepared `TBD_Objective`s, once,
//! and is the ONE authority on whether the round's objective-driven end conditions are met.
//!
//! ── What this closes ────────────────────────────────────────────────────────────────────────
//! `mission.schema.json` has offered `objective_capture`, `objective_destroy` and
//! `objective_hold_until` since the beginning, and `winConditions.endOn` has offered
//! `all_objectives_captured`, `objective_destroyed` and `hold_expired` alongside them. Nothing
//! captured, destroyed or timed out anything, so three of the five end triggers could never fire
//! and `TBD_FrameworkManager.TickWinConditions` implemented only `faction_eliminated`. Missions
//! that declared those triggers ran until an admin ended them.
//!
//! ── Built ON TOP of T-181.18, not beside it ─────────────────────────────────────────────────
//! `TBD_ZoneRegistry` already parses every zone in the document, resolves its shape, precomputes
//! its bounds and answers containment for circles and polygons against an independently-oracled
//! implementation. This registry consumes those prepared zones and adds only what T-181.18 has no
//! opinion about: objective rules, progress, ownership and completion. There is no second
//! containment test in this file and there must never be one.
//!
//! ── Server-side only ────────────────────────────────────────────────────────────────────────
//! Clients hold NO mission document (recorded landmine), so a client `Build()` would produce an
//! empty registry. `Build()` refuses rather than caching emptiness as an answer, and the only
//! caller — `TBD_ObjectivesComponent` — is authority-gated.
//!
//! ── Static, and therefore explicitly cleared ────────────────────────────────────────────────
//! Statics OUTLIVE A WORLD inside one process (recorded landmine — `SelectMissionByNumber` restarts
//! the scenario in-process). `TBD_ObjectivesComponent.OnDelete` MUST call `Clear()`, or mission B
//! inherits mission A's captured objectives and can win at kickoff.
//! ============================================================================================
//! T-212 -- TYPED PER-SIDE OBJECTIVES: the top-level `objectives[]` array
//!
//! == What was missing =========================================================================
//! T-706 put `objectives[]` on the wire and `#/$defs/objective` gave it the uniform attribute
//! spine (`id`, `type` in capture|destroy|hold|defend, `side`, `zoneId`, `label`, per-side
//! `framing`, `lock`, `autoLose`, `variantId`). Nothing read it. Measured on main before this
//! slice, the whole lane that owns objectives -- `Scripts/Game/TBD/Objectives` -- held ZERO
//! identifiers for `side`, `label`, `framing`, `lock`, `autoLose` and `variantId`: six of the
//! eleven spine properties, unreadable, because `JsonLoadContext` binds JSON keys onto
//! identically-named class MEMBERS and no class spelled them. An author could type per-side task
//! text and the round would run as if it were not there.
//!
//! == An OVERLAY, never a replacement ==========================================================
//! `zones[]` of `type: objective_*` stays the thing the runtime enforces. `Build()` still walks
//! prepared zones and nothing else; a typed row is joined onto a zone by `zoneId` and adds
//! identity, per-side framing and the two WOG scalars. An objective with no typed row behaves
//! EXACTLY as it did before this slice -- that is not a compatibility shim, it is the design:
//! geometry and capture rules have one owner and it is the zone.
//!
//! Where the two disagree -- a row saying `type: destroy` on an `objective_capture` zone -- the
//! ZONE WINS and the disagreement is reported by name at load. Resolving it silently in favour of
//! whichever was parsed last is how a player ends up with a task that does not match the rules
//! actually being applied to them.
//!
//! == ONE ENTITY, TWO FRAMINGS =================================================================
//! FNF v4's insight is that an objective reads differently to attacker and defender. Its shape --
//! two modules per objective -- is rejected by `#/$defs/objective` outright, and this reader
//! enforces that: two `objectives[]` rows naming the same `zoneId` is a WARNING quoting the ban,
//! and the first row wins deterministically. See `TBD_EObjectiveRole` for the four defects
//! (fnf_v4.md 14.4) that shape costs.
//!
//! == WOG caveat: `lock` and `autoLose` are CARRIED, NOT ENACTED ===============================
//! wog.md marks the SEMANTICS of the `WMT_Task_Point` parameters INFERRED: the addon that
//! implements `WMT_*` is absent from the corpus (wog.md:88-95), so parameter NAMES and observed
//! VALUES are hard evidence and the reading of what they DO is inference. T-685 took that caveat
//! and still chose TBD rules for the counts and heights, because those have an obvious and
//! testable meaning. These two do not, and the difference is worth stating:
//!
//!   * `_Lock` (observed 1 in 136 of 166) says "locked AT ROUND START". Honouring that needs an
//!     UNLOCK, and the corpus contains no evidence of what unlocks one. Implementing it as a
//!     permanent exclusion would not be a cautious reading of the parameter -- it would be a
//!     DIFFERENT parameter wearing its name, and a mission whose only capture objective was
//!     locked would become unwinnable with no line in the log saying why. So `lock` is parsed,
//!     counted and reported at load, and the round runs as if every objective were live. The
//!     board is left truthful rather than told to print LOCKED next to an objective that is not.
//!   * `_AutoLose` (-1 in 156 of 166, a live side in 10) names a faction that automatically LOSES
//!     if the objective falls. Acting on it means ENDING A ROUND, and the round-end authority is
//!     `TBD_FrameworkManager.TickWinConditions` reading `EvaluateEndTriggers()` -- one owner, in a
//!     file this slice does not own. Enacting an inferred rule from here would be two mistakes at
//!     once. It is validated against `factions[].key`, blanked when it names no side, reported,
//!     and exposed for the slice that adds the trigger.
//!
//! Neither is a write nobody reads: both are parsed, validated and named in the load log, which
//! is where an operator finds out that a document authored something this build does not run.
//!
//! == What this file CANNOT prove ==============================================================
//! The gate is `cargo xtask mod compile`. `flatten.rs` emits NO `objectives[]` on `/compiled`
//! (that is T-946.36, not this slice), so the only document that reaches this reader today is a
//! hand-staged schemaVersion 1.3 one -- `packages/tbd-schema/golden-missions/schema-1_3-wire-fields.json`
//! is the staged case, the same precedent T-685 shipped against. Whether an objective actually
//! READS differently to attacker and defender with two clients connected is a human checklist
//! item and is written up as one. Nothing here claims a live wire it does not have.
//! @contract mission.schema.json#/$defs/objective

//------------------------------------------------------------------------------------------------
//! One side's framing: `#/$defs/objectiveFraming`. Both keys optional, so both may be empty.
class TBD_ObjectiveFramingSideStruct
{
	string title;
	string text;
}

//------------------------------------------------------------------------------------------------
//! `objectives[].framing` -- the attacker's and defender's readings of ONE objective.
//!
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref <class>` field even when the JSON key is
//! ABSENT (measured -- see the landmine on `TBD_MissionShapeStruct`), so neither `attacker` nor
//! `defender` being non-null is a presence test. The STRINGS are the presence test, which is why
//! every consumer below asks `IsEmpty()` and never asks whether the ref exists.
class TBD_ObjectiveFramingStruct
{
	ref TBD_ObjectiveFramingSideStruct attacker;
	ref TBD_ObjectiveFramingSideStruct defender;
}

//------------------------------------------------------------------------------------------------
//! One `objectives[]` row: the uniform attribute spine of `#/$defs/objective`.
//!
//! Field names must equal the JSON keys -- `JsonLoadContext` maps by member name and a key no
//! class declares is invisible at runtime, not rejected, not logged, simply absent. `lock` is a
//! legal member name in this compiler (`TBD_VehicleWireStruct` already declares one); `type` is
//! too (`TBD_ObjectiveZoneStruct` already declares one).
//!
//! `lock` is a bool and therefore cannot carry an ABSENT sentinel -- there is no third value. That
//! costs nothing here: absent and authored-false both mean "not locked", which is the schema's own
//! default, and the only thing lost is the ability to say "you did not author this key". The same
//! limitation, stated the same way, as T-676's `repeat` and T-680's vehicle `lock`.
class TBD_ObjectiveEntityStruct
{
	string id;
	string type;
	string side;
	string zoneId;
	string label;
	ref TBD_ObjectiveFramingStruct framing;
	bool lock;
	string autoLose;
	string variantId;
}

//------------------------------------------------------------------------------------------------
//! The document root for the typed-objective pass: declares `objectives` and NOTHING else.
//!
//! `TBD_MissionDocumentStruct` has no `objectives` field and `TBD_MissionLoader.c` says so
//! outright ("objectives[] / editorTriggers[] are NOT on TBD_MissionDocumentStruct; their readers
//! re-parse GetRawJson()"). This is the third such pass in the tree, after
//! `TBD_ObjectiveRulesReader` (the objective half of `zoneRules`) and `TBD_TriggerRuntime`
//! (`editorTriggers[]`), and it is here for the same two reasons: the vocabulary stays next to the
//! code that interprets it, and growing a shared parser from another lane is the change nobody
//! reviews. The cost is one extra parse of a document parsed exactly ONCE per world.
class TBD_ObjectiveEntityDocStruct
{
	ref array<ref TBD_ObjectiveEntityStruct> objectives;
}

//------------------------------------------------------------------------------------------------
//! Reads the top-level `objectives[]` array once and hands rows out by `zoneId`.
//!
//! ── Server-side only ────────────────────────────────────────────────────────────────────────
//! Reads `TBD_MissionLoader.GetRawJson()`, which is empty on a client (clients hold NO mission
//! document). A client calling `Read()` gets a clean `false` and no rows, which is the correct
//! answer for a machine that is not the authority.
//!
//! ── Static, and therefore explicitly cleared ────────────────────────────────────────────────
//! Statics OUTLIVE A WORLD inside one process (recorded landmine -- `SelectMissionByNumber`
//! restarts the scenario in-process). `TBD_ObjectiveRegistry.Clear()` calls `Clear()` here, or
//! mission B's objectives inherit mission A's framing.
class TBD_ObjectiveEntityReader
{
	//! Log channel, kept apart from the registry's `Obj` so the typed pass is greppable on its own.
	static const string CH = "ObjTyped";

	//! `#/$defs/objective/type` -- the TASK vocabulary. Deliberately not the same strings as
	//! `TBD_ObjectiveRegistry.TYPE_*`, which are the `zones[].type` VOLUME vocabulary, nor
	//! `winConditions.endOn`, which is the round-end TRIGGER vocabulary. Three vocabularies, three
	//! sets of constants, so no call site can confuse a task kind with an enforcement volume.
	static const string TASK_CAPTURE = "capture";
	static const string TASK_DESTROY = "destroy";
	static const string TASK_HOLD    = "hold";
	static const string TASK_DEFEND  = "defend";

	protected static ref array<ref TBD_ObjectiveEntityStruct> s_aRows;
	//! Zone ids a prepared objective actually bound. Anything left over is a row pointing at
	//! nothing, which is reported rather than dropped.
	protected static ref array<string> s_aClaimedZoneIds;
	protected static bool s_bRead;
	protected static bool s_bOk;
	protected static int s_iGatedOut;

	//------------------------------------------------------------------------------------------------
	static void Clear()
	{
		s_aRows = null;
		s_aClaimedZoneIds = null;
		s_bRead = false;
		s_bOk = false;
		s_iGatedOut = 0;
	}

	//------------------------------------------------------------------------------------------------
	//! True when the pass produced an `objectives[]` array. False is NOT an error: every document
	//! authored before schemaVersion 1.3 has no such key, which is the whole shipped corpus.
	static bool IsOk()
	{
		return s_bOk;
	}

	//------------------------------------------------------------------------------------------------
	//! How many rows survived variant gating.
	static int Count()
	{
		if (!s_aRows)
			return 0;

		return s_aRows.Count();
	}

	//------------------------------------------------------------------------------------------------
	static int GatedOutCount()
	{
		return s_iGatedOut;
	}

	//------------------------------------------------------------------------------------------------
	//! Parse. Idempotent: only the first call after a `Clear()` does work.
	static bool Read()
	{
		if (s_bRead)
			return s_bOk;

		s_bRead = true;
		s_bOk = false;
		s_iGatedOut = 0;
		s_aRows = new array<ref TBD_ObjectiveEntityStruct>();
		s_aClaimedZoneIds = new array<string>();

		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return false;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
			return false;

		TBD_ObjectiveEntityDocStruct doc = new TBD_ObjectiveEntityDocStruct();
		if (!ctx.ReadValue("", doc))
			return false;

		if (!doc.objectives)
			return false;

		foreach (TBD_ObjectiveEntityStruct row : doc.objectives)
		{
			if (!row)
				continue;

			if (!VariantActive(row.variantId))
			{
				s_iGatedOut++;
				continue;
			}

			s_aRows.Insert(row);
		}

		s_bOk = true;
		WarnDuplicateZoneIds();
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! T-654 -- gate one row on the loader's ACTIVE VARIANT SET.
	//!
	//! `TBD_MissionLoader` filters the collections it owns and states that readers still parsing
	//! `GetRawJson()` -- objectives and editorTriggers -- must gate their rows on
	//! `GetActiveVariantIds()`. Null means variant selection is inert (no `variants[]` key), which
	//! is today's entire corpus, so everything runs. A row whose `variantId` is not in a non-null
	//! active set is EXCLUDED and counted.
	//!
	//! KNOWN LIMIT, stated rather than discovered: this cannot tell "deselected" from "DANGLING"
	//! (a `variantId` naming no `variants[]` row). The loader distinguishes the two because it
	//! walks the registry; that walk is `protected` and lives in a file this slice does not own, so
	//! excluded rows are counted together and the count is logged.
	protected static bool VariantActive(string variantId)
	{
		if (variantId.IsEmpty())
			return true;

		array<string> active = TBD_MissionLoader.GetActiveVariantIds();
		if (!active)
			return true;

		return active.Find(variantId) != -1;
	}

	//------------------------------------------------------------------------------------------------
	//! The row for one zone, or null when nothing named it.
	//!
	//! Joined by `zoneId` and NOT by index, unlike `TBD_ObjectiveRulesReader.ForZone`. That reader
	//! joins two passes over the SAME array, where index equality is a property of the parser. This
	//! one joins two DIFFERENT arrays -- `objectives[]` onto `zones[]` -- where index means nothing
	//! at all and `zoneId` is the only thing that can be right.
	static TBD_ObjectiveEntityStruct ForZone(string zoneId)
	{
		if (!s_aRows || zoneId.IsEmpty())
			return null;

		foreach (TBD_ObjectiveEntityStruct row : s_aRows)
		{
			if (row && row.zoneId == zoneId)
				return row;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! Record that a prepared objective took this zone's row.
	static void MarkClaimed(string zoneId)
	{
		if (!s_aClaimedZoneIds || zoneId.IsEmpty())
			return;

		if (s_aClaimedZoneIds.Find(zoneId) != -1)
			return;

		s_aClaimedZoneIds.Insert(zoneId);
	}

	//------------------------------------------------------------------------------------------------
	//! Every row that bound to no objective, by name. A row pointing at a zone that does not exist,
	//! or at one that is not an objective zone, is carried on the wire and does nothing -- exactly
	//! the failure the whole ticket exists to end, so it is reported rather than dropped.
	static void ReportUnclaimed()
	{
		if (!s_aRows)
			return;

		foreach (TBD_ObjectiveEntityStruct row : s_aRows)
		{
			if (!row)
				continue;

			if (row.zoneId.IsEmpty())
			{
				TBD_Log.Warn(CH, string.Format("objectives[] row id='%1' names no `zoneId`, so it has no geometry and nobody can ever be inside it. It is carried on the wire and does nothing. Point it at a `zones[].id` of an objective zone.",
					row.id));
				continue;
			}

			if (s_aClaimedZoneIds && s_aClaimedZoneIds.Find(row.zoneId) != -1)
				continue;

			TBD_Log.Warn(CH, string.Format("objectives[] row id='%1' names zoneId '%2', which no prepared objective zone matched -- either no zone carries that id, or that zone's type is not one of %3 / %4 / %5. The row is inert.",
				row.id, row.zoneId,
				TBD_ObjectiveRegistry.TYPE_CAPTURE, TBD_ObjectiveRegistry.TYPE_DESTROY, TBD_ObjectiveRegistry.TYPE_HOLD_UNTIL));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! The v4 defect the schema bans, caught by name. Two rows for one zone is "two modules that
	//! are the same objective", which is precisely what `#/$defs/objective` forbids; the FIRST row
	//! wins so the outcome is deterministic rather than parse-order luck.
	protected static void WarnDuplicateZoneIds()
	{
		if (!s_aRows)
			return;

		array<string> seen = new array<string>();

		foreach (TBD_ObjectiveEntityStruct row : s_aRows)
		{
			if (!row || row.zoneId.IsEmpty())
				continue;

			if (seen.Find(row.zoneId) != -1)
			{
				TBD_Log.Warn(CH, string.Format("objectives[] carries more than one row for zoneId '%1' (this one is id='%2'). mission.schema.json forbids that shape: ONE ENTITY, TWO FRAMINGS, never two rows for one objective. The FIRST row wins; author the other side under `framing.attacker` / `framing.defender` on that row instead.",
					row.zoneId, row.id));
				continue;
			}

			seen.Insert(row.zoneId);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Task type -> the zone kind that task belongs on. `defend` maps to CAPTURE because a defend
	//! task IS the far side of a capture: one enforcement volume, two framings.
	static TBD_EObjectiveKind KindOfTaskType(string taskType)
	{
		if (taskType == TASK_CAPTURE || taskType == TASK_DEFEND)
			return TBD_EObjectiveKind.CAPTURE;

		if (taskType == TASK_DESTROY)
			return TBD_EObjectiveKind.DESTROY;

		if (taskType == TASK_HOLD)
			return TBD_EObjectiveKind.HOLD_UNTIL;

		return TBD_EObjectiveKind.NONE;
	}

	//------------------------------------------------------------------------------------------------
	//! Does `objectives[].side` DEFEND under this task type? `hold` and `defend` are the defender
	//! framings; `capture` and `destroy` are the attacker framings.
	static bool IsDefenderFraming(string taskType)
	{
		return taskType == TASK_HOLD || taskType == TASK_DEFEND;
	}

	//------------------------------------------------------------------------------------------------
	//! Is `key` a declared `factions[].key`?
	//!
	//! A near-twin of `TBD_ZoneVolume.FactionExists`, which is `protected` and therefore out of
	//! reach from here. Duplicated rather than promoted: widening another slice's visibility to
	//! save twelve lines is the kind of cross-lane edit that turns into a conflict nobody reviews.
	static bool FactionExists(string key)
	{
		if (key.IsEmpty())
			return false;

		array<ref TBD_MissionFactionStruct> factions = TBD_MissionLoader.GetFactions();
		if (!factions)
			return false;

		foreach (TBD_MissionFactionStruct faction : factions)
		{
			if (faction && faction.key == key)
				return true;
		}

		return false;
	}
}

class TBD_ObjectiveRegistry
{
	//! Log channel. A literal rather than a `TBD_Log.CH_*` constant because `Core/TBD_Log.c` belongs
	//! to another slice's lane; keeping the string in one place here preserves the greppable-tag
	//! property the constants exist for. Fold it into `TBD_Log` when that file is next open.
	static const string CH = "Obj";

	//! Schema enum values (`mission.schema.json#/$defs/zone/type`) this module owns.
	static const string TYPE_CAPTURE    = "objective_capture";
	static const string TYPE_DESTROY    = "objective_destroy";
	static const string TYPE_HOLD_UNTIL = "objective_hold_until";

	//! `winConditions.endOn` values this module can drive.
	static const string TRIGGER_ALL_CAPTURED = "all_objectives_captured";
	static const string TRIGGER_DESTROYED    = "objective_destroyed";
	static const string TRIGGER_HOLD_EXPIRED = "hold_expired";

	//! `rules.onEmpty` vocabulary.
	static const string ON_EMPTY_HOLD  = "hold";
	static const string ON_EMPTY_DECAY = "decay";

	//! Defaults applied when a rule is absent, out of range, or unreadable. Every one is also named
	//! in the diagnostic that reports the fallback, so an operator reading the log never has to come
	//! here to find out what they got.
	static const float DEFAULT_CAPTURE_SECONDS = 120.0;
	static const float DEFAULT_DECAY_RATE = 1.0;
	static const float DEFAULT_CAPTURE_ANNOUNCE_SECONDS = 15.0;
	static const float DEFAULT_HOLD_ANNOUNCE_SECONDS = 60.0;

	//! Sanity ceiling on an authored duration. Pinned in schema as `maximum` 21600 on
	//! `captureSeconds` / `neutralizeSeconds` / `holdSeconds` (T-275 / mission.schema.json).
	//! Guard against a typo (`captureSeconds: 12000`) producing an objective nobody can take
	//! inside a 90-minute event. Deliberately generous: `last-stand-at-montfort.json`
	//! legitimately authors `holdSeconds: 2700`. Schema rejects over-ceiling values upstream;
	//! this remains the runtime fallback / inert path if a document reaches us out of band.
	static const float MAX_DURATION_SECONDS = 21600.0;

	//! Vertical extent of the destroy-target world query, in metres either side of sea level.
	//! Mission zones are XZ footprints with no Y at all (`TBD_ZoneGeometry` ignores Y throughout),
	//! so the box has to be tall enough to contain anything the terrain can hold. Everon's highest
	//! ground is a few hundred metres; 5 km either way is absurd overkill on purpose, because the
	//! cost of an over-tall box is nothing and the cost of an under-tall one is an objective that
	//! silently never finds its target.
	static const float QUERY_Y_EXTENT_M = 5000.0;

	protected static ref array<ref TBD_Objective> s_aObjectives;
	protected static bool s_bBuilt;
	protected static int s_iCaptureCount;
	protected static int s_iDestroyCount;
	protected static int s_iHoldCount;

	//! Scratch for the destroy-target world query. The query callback must be a static function
	//! with a fixed signature, so it has nowhere else to write. Single-threaded and never reentrant:
	//! `CountLiveTargets` sets these, runs the query to completion, and reads them back before any
	//! other call can begin.
	protected static ResourceName s_QueryResource;
	protected static TBD_Zone s_QueryZone;
	protected static int s_iQueryAlive;
	protected static int s_iQueryMatched;

	//------------------------------------------------------------------------------------------------
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//------------------------------------------------------------------------------------------------
	static int GetCaptureCount()
	{
		return s_iCaptureCount;
	}

	//------------------------------------------------------------------------------------------------
	static int GetDestroyCount()
	{
		return s_iDestroyCount;
	}

	//------------------------------------------------------------------------------------------------
	static int GetHoldCount()
	{
		return s_iHoldCount;
	}

	//------------------------------------------------------------------------------------------------
	//! Every prepared objective, including inert ones (kept so the summary count matches the
	//! document). Null until `Build()` has run.
	static array<ref TBD_Objective> GetAll()
	{
		return s_aObjectives;
	}

	//------------------------------------------------------------------------------------------------
	//! Drop everything. MUST be called on world teardown — see the class header.
	static void Clear()
	{
		s_aObjectives = null;
		s_bBuilt = false;
		s_iCaptureCount = 0;
		s_iDestroyCount = 0;
		s_iHoldCount = 0;
		s_QueryResource = string.Empty;
		s_QueryZone = null;
		TBD_ObjectiveRulesReader.Clear();
		TBD_ZoneVolume.Clear();
		TBD_ObjectiveEntityReader.Clear();
	}

	//------------------------------------------------------------------------------------------------
	//! Prepare every objective zone in the loaded mission. Safe to call repeatedly; only the first
	//! call after a `Clear()` does work.
	//!
	//! Returns false when there is no valid mission to build from — the caller keeps waiting rather
	//! than caching an empty registry as if it were the answer. This mirrors
	//! `TBD_ZoneRegistry.Build()` exactly, and for the same reason.
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		// The zone layer is the source of geometry. It is idempotent, so calling it here costs
		// nothing when `TBD_PlayAreaComponent` has already built it and makes objectives work when
		// that component is not on the prefab. Note this deliberately does NOT call
		// `TBD_ZoneRegistry.Clear()` — that belongs to its owner, and two components racing to tear
		// down one static is a coordination hazard for no gain. `TBD_Objective.m_Zone` is a strong
		// reference precisely so this file does not care who clears first.
		if (!TBD_ZoneRegistry.Build())
			return false;

		array<ref TBD_Zone> zones = TBD_ZoneRegistry.GetAll();
		if (!zones)
			return false;

		// The second typed pass over the same raw JSON. A failure here is not fatal: every objective
		// then runs on documented defaults, which is reported ONCE below rather than per zone.
		bool rulesOk = TBD_ObjectiveRulesReader.Read();
		TBD_ZoneVolume.Read();

		// T-212 -- the THIRD typed pass over the same raw JSON, for the top-level `objectives[]`
		// array the mission loader has no field for. A document with no such key is the normal
		// case and returns a clean false; every objective then runs untyped, exactly as before.
		TBD_ObjectiveEntityReader.Read();

		s_aObjectives = new array<ref TBD_Objective>();
		s_iCaptureCount = 0;
		s_iDestroyCount = 0;
		s_iHoldCount = 0;

		int usable = 0;

		foreach (int index, TBD_Zone zone : zones)
		{
			if (!zone)
				continue;

			TBD_EObjectiveKind kind = KindOf(zone.m_sType);
			if (kind == TBD_EObjectiveKind.NONE)
				continue;

			TBD_ObjectiveRulesStruct rules = TBD_ObjectiveRulesReader.ForZone(index, zone.m_sId);
			TBD_Objective objective = Prepare(zone, kind, rules, index);
			s_aObjectives.Insert(objective);

			if (objective.m_bUsable)
			{
				usable++;
				if (kind == TBD_EObjectiveKind.CAPTURE)
					s_iCaptureCount++;
				else if (kind == TBD_EObjectiveKind.DESTROY)
					s_iDestroyCount++;
				else
					s_iHoldCount++;
			}

			LogPrepared(objective);
		}

		s_bBuilt = true;

		if (!rulesOk && s_aObjectives.Count() > 0)
		{
			// Said out loud rather than left to be inferred from every objective sitting on a
			// default. This is the one failure mode of the second-parse design and it must never be
			// silent.
			TBD_Log.Warn(CH, "objective rules could not be re-read from the raw mission JSON — every objective below is running on documented defaults. See TBD_ObjectiveRulesReader.");
		}
		else if (rulesOk && TBD_ObjectiveRulesReader.Count() != zones.Count())
		{
			// The two passes disagree about how many zones the document has. The per-zone join
			// verifies ids and falls back to a by-id search, so this is a warning rather than a
			// refusal — but it means one of the two parses dropped something and an operator should
			// know before they wonder why an objective is on defaults.
			TBD_Log.Warn(CH, string.Format("zone count differs between the mission loader (%1) and the objective rules pass (%2) — rules are joined by id where the index disagrees",
				zones.Count(), TBD_ObjectiveRulesReader.Count()));
		}

		// One greppable summary line, always, even at zero — "this mission has no objectives" is
		// exactly as important to see in a boot log as "this mission has four".
		TBD_Log.Kv(CH, "built", string.Format("objectives=%1 usable=%2 capture=%3 destroy=%4 hold=%5",
			s_aObjectives.Count(), usable, s_iCaptureCount, s_iDestroyCount, s_iHoldCount));

		ReportTriggerCoverage();
		ReportTypedCoverage();

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Cross-check what the mission SAYS ends the round against what it actually contains.
	//!
	//! A mission that declares `all_objectives_captured` with no usable capture objective has an end
	//! trigger that can never fire — the round will run to the time limit and an author will call
	//! that a bug in the framework. Catching it at load, by name, costs one pass over a five-element
	//! array and turns a mid-event mystery into a line in the boot log.
	//!
	//! The inverse (an objective with no matching trigger) is also reported: it means the objective
	//! is decoration, which may well be intentional, so it is a note rather than a complaint.
	protected static void ReportTriggerCoverage()
	{
		if (TBD_MissionLoader.HasEndTrigger(TRIGGER_ALL_CAPTURED) && s_iCaptureCount == 0)
			TBD_Log.Warn(CH, string.Format("winConditions.endOn declares '%1' but this mission has no usable %2 zone — that trigger can NEVER fire",
				TRIGGER_ALL_CAPTURED, TYPE_CAPTURE));

		if (TBD_MissionLoader.HasEndTrigger(TRIGGER_DESTROYED) && s_iDestroyCount == 0)
			TBD_Log.Warn(CH, string.Format("winConditions.endOn declares '%1' but this mission has no usable %2 zone — that trigger can NEVER fire",
				TRIGGER_DESTROYED, TYPE_DESTROY));

		if (TBD_MissionLoader.HasEndTrigger(TRIGGER_HOLD_EXPIRED) && s_iHoldCount == 0)
			TBD_Log.Warn(CH, string.Format("winConditions.endOn declares '%1' but this mission has no usable %2 zone — that trigger can NEVER fire",
				TRIGGER_HOLD_EXPIRED, TYPE_HOLD_UNTIL));

		if (s_iCaptureCount > 0 && !TBD_MissionLoader.HasEndTrigger(TRIGGER_ALL_CAPTURED))
			TBD_Log.Kv(CH, "note", string.Format("%1 capture objective(s) but endOn does not declare '%2' — they are tracked and announced, and will not end the round",
				s_iCaptureCount, TRIGGER_ALL_CAPTURED));

		if (s_iDestroyCount > 0 && !TBD_MissionLoader.HasEndTrigger(TRIGGER_DESTROYED))
			TBD_Log.Kv(CH, "note", string.Format("%1 destroy objective(s) but endOn does not declare '%2' — tracked, will not end the round",
				s_iDestroyCount, TRIGGER_DESTROYED));

		if (s_iHoldCount > 0 && !TBD_MissionLoader.HasEndTrigger(TRIGGER_HOLD_EXPIRED))
			TBD_Log.Kv(CH, "note", string.Format("%1 hold objective(s) but endOn does not declare '%2' — tracked, will not end the round",
				s_iHoldCount, TRIGGER_HOLD_EXPIRED));
	}

	//------------------------------------------------------------------------------------------------
	//! Schema zone type -> objective kind. Anything else (spawn, boundary, base_protection) is
	//! `NONE` and is skipped: those belong to T-181.18.
	static TBD_EObjectiveKind KindOf(string zoneType)
	{
		if (zoneType == TYPE_CAPTURE)
			return TBD_EObjectiveKind.CAPTURE;
		if (zoneType == TYPE_DESTROY)
			return TBD_EObjectiveKind.DESTROY;
		if (zoneType == TYPE_HOLD_UNTIL)
			return TBD_EObjectiveKind.HOLD_UNTIL;

		return TBD_EObjectiveKind.NONE;
	}

	//------------------------------------------------------------------------------------------------
	//! One line per objective at load, plus its resolved rules.
	//!
	//! Two jobs, both borrowed from `TBD_ZoneRegistry.LogPrepared` because they are the right ones:
	//!   * OPERATIONAL — an operator reads the objective's extents and timings straight out of the
	//!     boot log and checks them against the map BEFORE an event, instead of discovering at
	//!     H-hour that the capture takes twenty minutes.
	//!   * PROOF — the numbers are computed from parsed floats, so a run printing plausible values
	//!     has demonstrated that the second JSON pass really did populate. A compile probe cannot
	//!     show that; this shows it in every single run.
	protected static void LogPrepared(notnull TBD_Objective objective)
	{
		string kind = typename.EnumToString(TBD_EObjectiveKind, objective.m_eKind);

		if (!objective.m_bUsable)
		{
			TBD_Log.Warn(CH, string.Format("objective id=%1 type=%2 is INERT: %3",
				objective.m_sId, kind, objective.m_sInertReason));
			return;
		}

		TBD_Log.Kv(CH, "objective", string.Format("id=%1 kind=%2 label='%3' faction='%4' bounds=[%5,%6 %7,%8]",
			objective.m_sId,
			kind,
			objective.m_sLabel,
			objective.m_sFaction,
			objective.m_Zone.m_fMinX, objective.m_Zone.m_fMinZ,
			objective.m_Zone.m_fMaxX, objective.m_Zone.m_fMaxZ));

		if (objective.m_eKind == TBD_EObjectiveKind.CAPTURE)
		{
			LogTyped(objective);
			TBD_Log.Kv(CH, "objectiveRules", string.Format("id=%1 capture=%2s neutralize=%3s contestable=%4 onEmpty=%5 decayRate=%6 points=%7",
				objective.m_sId,
				objective.m_fCaptureSeconds,
				objective.m_fNeutralizeSeconds,
				objective.m_bContestable,
				typename.EnumToString(TBD_EObjectiveOnEmpty, objective.m_eOnEmpty),
				objective.m_fDecayRate,
				objective.m_fPoints));
			return;
		}

		if (objective.m_eKind == TBD_EObjectiveKind.HOLD_UNTIL)
		{
			LogTyped(objective);
			TBD_Log.Kv(CH, "objectiveRules", string.Format("id=%1 hold=%2s holder='%3' pauseOnEnemy=%4 resetOnEnemy=%5 requireHolderPresent=%6 points=%7",
				objective.m_sId,
				objective.m_fHoldSeconds,
				objective.m_sFaction,
				objective.m_bPauseOnEnemy,
				objective.m_bResetOnEnemy,
				objective.m_bRequireHolderPresent,
				objective.m_fPoints));
			return;
		}

		LogTyped(objective);

		TBD_Log.Kv(CH, "objectiveRules", string.Format("id=%1 targetAlias='%2' targetCount=%3 points=%4",
			objective.m_sId,
			objective.m_sTargetAlias,
			objective.m_iTargetCount,
			objective.m_fPoints));
	}
	//------------------------------------------------------------------------------------------------
	//! T-212 -- the typed half of one objective's load line, or nothing when no row bound.
	//!
	//! Same two jobs as `LogPrepared`, and the second is the one that matters here. OPERATIONAL: an
	//! operator reads the per-side titles out of the boot log and checks that the attacker and the
	//! defender are actually being told different things, before an event rather than during one.
	//! PROOF: these strings are read off a parsed document, so a run that prints them has
	//! demonstrated that the third JSON pass really did populate. A compile cannot show that.
	protected static void LogTyped(notnull TBD_Objective objective)
	{
		if (!objective.m_bTyped)
			return;

		string sideRole = "attacks";
		if (objective.m_bSideDefends)
			sideRole = "defends";

		TBD_Log.Kv(CH, "objectiveTyped", string.Format("zone=%1 id='%2' type='%3' side='%4' (%5) lock=%6 autoLose='%7'",
			objective.m_sId,
			objective.m_sEntityId,
			objective.m_sTaskType,
			objective.m_sSide,
			sideRole,
			objective.m_bLocked,
			objective.m_sAutoLoseFaction));

		if (!objective.HasFraming())
			return;

		TBD_Log.Kv(CH, "objectiveFraming", string.Format("zone=%1 attacker='%2' | '%3'",
			objective.m_sId, objective.m_sAttackerTitle, objective.m_sAttackerText));

		TBD_Log.Kv(CH, "objectiveFraming", string.Format("zone=%1 defender='%2' | '%3'",
			objective.m_sId, objective.m_sDefenderTitle, objective.m_sDefenderText));
	}

	//------------------------------------------------------------------------------------------------
	//! Flatten one prepared zone plus its rules into a runnable objective, reporting every defect by
	//! zone id.
	protected static TBD_Objective Prepare(notnull TBD_Zone zone, TBD_EObjectiveKind kind, TBD_ObjectiveRulesStruct rules, int index)
	{
		TBD_Objective objective = new TBD_Objective();
		objective.m_Zone = zone;
		objective.m_eKind = kind;
		objective.m_sId = zone.m_sId;
		objective.m_sLabel = zone.m_sLabel;
		objective.m_sFaction = zone.m_sFaction;
		objective.m_bUsable = true;

		string subject = zone.m_sId;
		if (subject.IsEmpty())
			subject = string.Format("zones[%1]", index);

		// Defaults FIRST, so no field is ever left holding a sentinel even on the paths that
		// go inert below.
		objective.m_fCaptureSeconds = DEFAULT_CAPTURE_SECONDS;
		objective.m_fNeutralizeSeconds = DEFAULT_CAPTURE_SECONDS;
		objective.m_bContestable = true;
		objective.m_eOnEmpty = TBD_EObjectiveOnEmpty.HOLD;
		objective.m_fDecayRate = DEFAULT_DECAY_RATE;
		objective.m_fHoldSeconds = 0;
		objective.m_bPauseOnEnemy = true;
		objective.m_bResetOnEnemy = false;
		objective.m_bRequireHolderPresent = false;
		objective.m_iTargetCount = 0;
		objective.m_fPoints = 0;
		objective.m_fAnnounceEverySeconds = DEFAULT_CAPTURE_ANNOUNCE_SECONDS;
		if (kind == TBD_EObjectiveKind.HOLD_UNTIL)
			objective.m_fAnnounceEverySeconds = DEFAULT_HOLD_ANNOUNCE_SECONDS;

		// T-212 -- who DEFENDS, before any typed row is read. An untyped objective_hold_until is
		// held by its zone's faction, so that side defends; a capture or destroy zone names the side
		// told to take or break the ground, so that side attacks. A typed `type` may override it.
		objective.m_bSideDefends = kind == TBD_EObjectiveKind.HOLD_UNTIL;

		// T-212 -- bind the typed `objectives[]` row BEFORE the kind-specific rules, and before the
		// unusable-shape return, for two reasons. `ResolveHoldRules` refuses a hold objective that
		// names no holder, and an authored `side` IS a holder; and an objective that goes inert on
		// its geometry should still carry its identity into the log that says so.
		BindTypedEntity(objective, subject);

		// An objective with no usable shape can never contain anyone, so nothing can ever advance
		// it. `TBD_ZoneRegistry` has already reported the geometry defect; this reports the
		// CONSEQUENCE, which is the part an operator cares about.
		if (!zone.IsUsable())
		{
			objective.m_bUsable = false;
			objective.m_sInertReason = "the zone has no usable shape, so nobody can ever be inside it";
			return objective;
		}

		ResolveCommonRules(objective, rules, subject);

		if (kind == TBD_EObjectiveKind.CAPTURE)
			ResolveCaptureRules(objective, rules, subject);
		else if (kind == TBD_EObjectiveKind.HOLD_UNTIL)
			ResolveHoldRules(objective, rules, subject);
		else
			ResolveDestroyRules(objective, rules, subject);

		TBD_ZoneVolume.ApplyStartingOwner(objective);
		TBD_ZoneVolume.LogBound(objective);

		return objective;
	}

	//------------------------------------------------------------------------------------------------
	//! Rules every objective kind shares.
	protected static void ResolveCommonRules(notnull TBD_Objective objective, TBD_ObjectiveRulesStruct rules, string subject)
	{
		if (!rules)
			return;

		if (rules.points != TBD_ObjectiveRulesStruct.ABSENT)
		{
			if (rules.points < 0)
			{
				TBD_Log.Warn(CH, string.Format("objective '%1' rules.points=%2 is negative — using 0",
					subject, rules.points));
			}
			else
			{
				objective.m_fPoints = rules.points;
			}
		}

		if (rules.announceEverySeconds != TBD_ObjectiveRulesStruct.ABSENT)
		{
			if (rules.announceEverySeconds <= 0)
			{
				TBD_Log.Warn(CH, string.Format("objective '%1' rules.announceEverySeconds=%2 must be > 0 — using %3 s",
					subject, rules.announceEverySeconds, objective.m_fAnnounceEverySeconds));
			}
			else
			{
				objective.m_fAnnounceEverySeconds = rules.announceEverySeconds;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! `objective_capture`.
	//!
	//! ══ Why an absent `captureSeconds` DEFAULTS rather than going inert ══════════════════════
	//! The two failure modes are not symmetric. An objective that defaults to 120 s is playable and
	//! says so loudly in the log. An objective that goes inert takes `all_objectives_captured` with
	//! it — the trigger can then never fire and the round silently runs to the time limit. Between
	//! "the capture took a length nobody chose" and "the mission can no longer be won", the first is
	//! plainly the lesser harm, so this defaults and shouts. Contrast `objective_hold_until` below,
	//! where the reasoning runs the other way.
	protected static void ResolveCaptureRules(notnull TBD_Objective objective, TBD_ObjectiveRulesStruct rules, string subject)
	{
		bool captureAuthored = false;

		if (rules && rules.captureSeconds != TBD_ObjectiveRulesStruct.ABSENT)
		{
			if (rules.captureSeconds <= 0 || rules.captureSeconds > MAX_DURATION_SECONDS)
			{
				TBD_Log.Warn(CH, string.Format("objective '%1' rules.captureSeconds=%2 is outside 0..%3 — using the default %4 s",
					subject, rules.captureSeconds, MAX_DURATION_SECONDS, DEFAULT_CAPTURE_SECONDS));
			}
			else
			{
				objective.m_fCaptureSeconds = rules.captureSeconds;
				captureAuthored = true;
			}
		}

		if (!captureAuthored)
			TBD_Log.Warn(CH, string.Format("objective '%1' (%2) has no readable rules.captureSeconds — using the default %3 s. Either none was authored, or one was authored under a key this build does not declare and therefore cannot see; a typed JSON parser cannot tell those apart.",
				subject, TYPE_CAPTURE, DEFAULT_CAPTURE_SECONDS));

		// Teardown defaults to a symmetric 1:1 rate with the build.
		objective.m_fNeutralizeSeconds = objective.m_fCaptureSeconds;

		if (rules && rules.neutralizeSeconds != TBD_ObjectiveRulesStruct.ABSENT)
		{
			if (rules.neutralizeSeconds < 0 || rules.neutralizeSeconds > MAX_DURATION_SECONDS)
			{
				TBD_Log.Warn(CH, string.Format("objective '%1' rules.neutralizeSeconds=%2 is outside 0..%3 — using captureSeconds (%4 s)",
					subject, rules.neutralizeSeconds, MAX_DURATION_SECONDS, objective.m_fCaptureSeconds));
			}
			else
			{
				objective.m_fNeutralizeSeconds = rules.neutralizeSeconds;
			}
		}

		if (rules)
			objective.m_bContestable = rules.contestable;

		if (rules && !rules.onEmpty.IsEmpty())
		{
			if (rules.onEmpty == ON_EMPTY_DECAY)
			{
				objective.m_eOnEmpty = TBD_EObjectiveOnEmpty.DECAY;
			}
			else if (rules.onEmpty == ON_EMPTY_HOLD)
			{
				objective.m_eOnEmpty = TBD_EObjectiveOnEmpty.HOLD;
			}
			else
			{
				TBD_Log.Warn(CH, string.Format("objective '%1' rules.onEmpty='%2' is not one of hold|decay — using 'hold' (partial progress is kept)",
					subject, rules.onEmpty));
			}
		}

		if (rules && rules.decayRate != TBD_ObjectiveRulesStruct.ABSENT)
		{
			if (rules.decayRate <= 0)
			{
				TBD_Log.Warn(CH, string.Format("objective '%1' rules.decayRate=%2 must be > 0 — using %3",
					subject, rules.decayRate, DEFAULT_DECAY_RATE));
			}
			else
			{
				objective.m_fDecayRate = rules.decayRate;
			}
		}

		// An authored `faction` on a capture zone is a real restriction and must never be applied
		// silently — a side that cannot take an objective and is not told why will report it as a
		// bug in the capture logic.
		if (!objective.m_sFaction.IsEmpty())
			TBD_Log.Kv(CH, "note", string.Format("objective '%1' names faction '%2' — ONLY that side can own it; any other side can neutralise it but never take it",
				subject, objective.m_sFaction));
	}

	//------------------------------------------------------------------------------------------------
	//! `objective_hold_until`.
	//!
	//! ══ Why a missing rule here goes INERT rather than defaulting ════════════════════════════
	//! `holdSeconds` has no defensible default. `captureSeconds` does — every capture in the genre
	//! is tens of seconds to a couple of minutes and being wrong costs a slightly odd pace. A hold
	//! is the length of the ROUND: `last-stand-at-montfort.json` authors 2700 s inside a 3000 s time
	//! limit. Guessing it means guessing when the round ends and who won, and ending an event at the
	//! wrong moment is far worse than not ending it at all. The same goes for the holding faction:
	//! without it `hold_expired` has no winner to name.
	//!
	//! So this refuses, says exactly which key is missing, and the objective is excluded from the
	//! end-trigger authority. `ReportTriggerCoverage` then separately says that `hold_expired` can
	//! never fire, so the operator gets the cause and the consequence as two distinct lines.
	protected static void ResolveHoldRules(notnull TBD_Objective objective, TBD_ObjectiveRulesStruct rules, string subject)
	{
		if (objective.m_sFaction.IsEmpty())
		{
			objective.m_bUsable = false;
			objective.m_sInertReason = "objective_hold_until names no `faction`, so there is no way to know who is holding it or who wins when the clock runs out. Set `faction` to the side that must hold this ground.";
			return;
		}

		if (!rules || rules.holdSeconds == TBD_ObjectiveRulesStruct.ABSENT)
		{
			objective.m_bUsable = false;
			objective.m_sInertReason = "objective_hold_until has no readable rules.holdSeconds and there is no defensible default for how long a round should last. Author `rules.holdSeconds`.";
			return;
		}

		if (rules.holdSeconds <= 0 || rules.holdSeconds > MAX_DURATION_SECONDS)
		{
			objective.m_bUsable = false;
			objective.m_sInertReason = string.Format("rules.holdSeconds=%1 is outside 0..%2", rules.holdSeconds, MAX_DURATION_SECONDS);
			return;
		}

		objective.m_fHoldSeconds = rules.holdSeconds;
		objective.m_bPauseOnEnemy = rules.pauseOnEnemy;
		objective.m_bResetOnEnemy = rules.resetOnEnemy;
		objective.m_bRequireHolderPresent = rules.requireHolderPresent;
	}

	//------------------------------------------------------------------------------------------------
	//! `objective_destroy`.
	//!
	//! Only the rules are resolved here. Finding the actual target is deferred to the first LIVE
	//! evaluation (`ArmDestroyTargets`) because a target placed by any other subsystem may not exist
	//! while the world is still in LOBBY, and searching an empty world at load would report every
	//! destroy objective as targetless.
	protected static void ResolveDestroyRules(notnull TBD_Objective objective, TBD_ObjectiveRulesStruct rules, string subject)
	{
		if (!rules || rules.targetAlias.IsEmpty())
		{
			objective.m_bUsable = false;
			objective.m_sInertReason = "objective_destroy has no readable rules.targetAlias, so there is nothing to watch. Author `rules.targetAlias` with a registry alias (e.g. \"comp:ammo_cache\").";
			return;
		}

		objective.m_sTargetAlias = rules.targetAlias;

		if (rules.targetCount != TBD_ObjectiveRulesStruct.ABSENT_INT)
		{
			if (rules.targetCount < 0)
			{
				TBD_Log.Warn(CH, string.Format("objective '%1' rules.targetCount=%2 is negative — using 0 (destroy everything found)",
					subject, rules.targetCount));
			}
			else
			{
				objective.m_iTargetCount = rules.targetCount;
			}
		}

		if (objective.m_sFaction.IsEmpty())
			TBD_Log.Warn(CH, string.Format("objective '%1' (%2) names no `faction` — if it completes, '%3' will fire with no winning side named. Set `faction` to the side that must destroy it.",
				subject, TYPE_DESTROY, TRIGGER_DESTROYED));
	}

	//------------------------------------------------------------------------------------------------
	//! T-212 -- join the typed `objectives[]` row for this zone onto the prepared objective.
	//!
	//! No row is the NORMAL case, not a degraded one: every document authored before schemaVersion
	//! 1.3 has no `objectives[]` key at all, and one that does may still type only some of its
	//! zones. A missing row returns immediately and the objective runs exactly as it did before
	//! this slice.
	protected static void BindTypedEntity(notnull TBD_Objective objective, string subject)
	{
		TBD_ObjectiveEntityStruct row = TBD_ObjectiveEntityReader.ForZone(objective.m_sId);
		if (!row)
			return;

		TBD_ObjectiveEntityReader.MarkClaimed(objective.m_sId);

		objective.m_bTyped = true;
		objective.m_sEntityId = row.id;
		objective.m_sTaskType = row.type;
		objective.m_sSide = row.side;
		objective.m_bLocked = row.lock;
		objective.m_sAutoLoseFaction = row.autoLose;

		// An absent `type` leaves the framing on the zone's own kind rather than silently reading
		// as an attacker task: a hold zone is held by its faction whatever the row forgot to say.
		if (row.type.IsEmpty())
			objective.m_bSideDefends = objective.m_eKind == TBD_EObjectiveKind.HOLD_UNTIL;
		else
			objective.m_bSideDefends = TBD_ObjectiveEntityReader.IsDefenderFraming(row.type);

		ApplyFraming(objective, row.framing);
		ApplyTypedLabel(objective, row, subject);
		CheckTypedKind(objective, row, subject);
		CheckTypedSide(objective, row, subject);
		CheckAutoLose(objective, subject);
		SeedHolderFromSide(objective, subject);
	}

	//------------------------------------------------------------------------------------------------
	//! Copy `framing.attacker` / `framing.defender` onto the objective.
	//!
	//! The nested refs are ALWAYS non-null (`JsonLoadContext` allocates them whether or not the key
	//! was authored), so they are checked defensively and the STRINGS carry presence. A side framed
	//! with only a title, or only text, is legal -- both keys are optional in
	//! `#/$defs/objectiveFraming` -- and each half is copied independently for exactly that reason.
	protected static void ApplyFraming(notnull TBD_Objective objective, TBD_ObjectiveFramingStruct framing)
	{
		if (!framing)
			return;

		if (framing.attacker)
		{
			objective.m_sAttackerTitle = framing.attacker.title;
			objective.m_sAttackerText = framing.attacker.text;
		}

		if (framing.defender)
		{
			objective.m_sDefenderTitle = framing.defender.title;
			objective.m_sDefenderText = framing.defender.text;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! `objectives[].label` is the OBJECTIVE's display name; `zones[].label` names the VOLUME.
	//!
	//! The objective row is the more specific record, so it wins when authored -- but never
	//! silently: an override of a DIFFERENT zone label is a note, because "the board says something
	//! other than what I typed on the zone" is exactly the kind of thing an author reports as a bug
	//! in the board.
	protected static void ApplyTypedLabel(notnull TBD_Objective objective, TBD_ObjectiveEntityStruct row, string subject)
	{
		if (!row || row.label.IsEmpty())
			return;

		if (objective.m_sLabel == row.label)
			return;

		if (!objective.m_sLabel.IsEmpty())
			TBD_Log.Kv(CH, "note", string.Format("objective '%1' takes its display name from objectives[].label ('%2'); the zone's own label ('%3') names the VOLUME and is not what a player is shown",
				subject, row.label, objective.m_sLabel));

		objective.m_sLabel = row.label;
	}

	//------------------------------------------------------------------------------------------------
	//! Does the row's TASK type agree with the zone's VOLUME type?
	//!
	//! THE ZONE WINS in every disagreement, because the zone is what the runtime enforces -- the
	//! capture bar, the hold clock and the destroy query all read `m_eKind`. Reporting the
	//! disagreement instead of resolving it silently is the point: a player given a "destroy" task
	//! on ground that is actually being captured has been lied to by the document, and that is a
	//! defect an author can only fix if somebody tells them it exists.
	//!
	//! `defend` on an `objective_capture` zone is NOT a disagreement. It is the far side of the
	//! same capture, which is the whole reason the task vocabulary carries a fourth value the zone
	//! vocabulary does not.
	protected static void CheckTypedKind(notnull TBD_Objective objective, TBD_ObjectiveEntityStruct row, string subject)
	{
		if (!row || row.type.IsEmpty())
		{
			TBD_Log.Warn(CH, string.Format("objectives[] row for zone '%1' names no `type`, which #/$defs/objective requires. The zone's own type decides what runs; the per-side framing falls back to that kind.",
				subject));
			return;
		}

		TBD_EObjectiveKind declared = TBD_ObjectiveEntityReader.KindOfTaskType(row.type);

		if (declared == TBD_EObjectiveKind.NONE)
		{
			TBD_Log.Warn(CH, string.Format("objectives[] row for zone '%1' has type='%2', which is not one of capture|destroy|hold|defend. The zone's own type still decides what runs.",
				subject, row.type));
			return;
		}

		if (declared == objective.m_eKind)
			return;

		TBD_Log.Warn(CH, string.Format("objectives[] row for zone '%1' declares type='%2' but the zone is type='%3'. THE ZONE WINS -- it is what the runtime enforces. Fix one of the two, or a player is given a task that does not match the rules being applied to them.",
			subject, row.type, objective.m_Zone.m_sType));
	}

	//------------------------------------------------------------------------------------------------
	//! Validate `objectives[].side` and say what it does NOT mean.
	//!
	//! `side` and `zones[].faction` are different claims and both are kept. The faction is the
	//! OWNERSHIP restriction the capture rules enforce; the side is who the task is FOR. An
	//! objective can legitimately be for a side that is not allowed to own the zone -- "deny it to
	//! them" is a real task -- so a difference is a note, not a warning.
	protected static void CheckTypedSide(notnull TBD_Objective objective, TBD_ObjectiveEntityStruct row, string subject)
	{
		if (!row || row.side.IsEmpty())
			return;

		if (!TBD_ObjectiveEntityReader.FactionExists(row.side))
		{
			objective.m_bInvalidSide = true;
			TBD_Log.Warn(CH, string.Format("objective '%1' names side='%2', which is no factions[].key. Nobody matches it, so the per-side framing reads NEUTRAL to every player and the objective falls back to its label.",
				subject, row.side));
			return;
		}

		if (objective.m_sFaction.IsEmpty() || objective.m_sFaction == row.side)
			return;

		TBD_Log.Kv(CH, "note", string.Format("objective '%1' names side='%2' while its zone names faction='%3'. Both are kept and they are not the same claim: the zone's faction is the ownership restriction, the objective's side is who the task is for.",
			subject, row.side, objective.m_sFaction));
	}

	//------------------------------------------------------------------------------------------------
	//! `objectives[].autoLose` -- validated, reported, and NOT acted on. See the T-212 block in the
	//! file header for why enacting an INFERRED round-loss from here would be two mistakes at once.
	//!
	//! A key naming no faction is BLANKED rather than carried, so the slice that eventually adds
	//! the trigger cannot inherit a dangling reference and act on it.
	protected static void CheckAutoLose(notnull TBD_Objective objective, string subject)
	{
		if (objective.m_sAutoLoseFaction.IsEmpty())
			return;

		if (!TBD_ObjectiveEntityReader.FactionExists(objective.m_sAutoLoseFaction))
		{
			TBD_Log.Warn(CH, string.Format("objective '%1' declares autoLose='%2', which is no factions[].key. Dropping it rather than carrying a dangling side.",
				subject, objective.m_sAutoLoseFaction));
			objective.m_sAutoLoseFaction = string.Empty;
			return;
		}

		TBD_Log.Kv(CH, "note", string.Format("objective '%1' declares autoLose='%2'. CARRIED AND REPORTED ONLY -- nothing in this build ends a round on it. WOG's `_AutoLose` semantics are INFERRED and the round-end authority is TBD_FrameworkManager, not this file.",
			subject, objective.m_sAutoLoseFaction));
	}

	//------------------------------------------------------------------------------------------------
	//! An authored `side` supplies the holder an `objective_hold_until` zone did not name.
	//!
	//! Narrow on purpose, and CAPTURE is deliberately excluded. On a hold zone an empty `faction`
	//! means INERT and `hold_expired` can never fire, so there is nothing to lose and a document
	//! that plainly says who is holding the ground should be believed. On a CAPTURE zone an empty
	//! `faction` means "anyone may own this", and seeding it from `side` would invent an ownership
	//! restriction the author never wrote -- a silent behaviour change in the opposite direction.
	protected static void SeedHolderFromSide(notnull TBD_Objective objective, string subject)
	{
		if (objective.m_eKind != TBD_EObjectiveKind.HOLD_UNTIL)
			return;

		if (!objective.m_sFaction.IsEmpty() || objective.m_sSide.IsEmpty())
			return;

		if (!TBD_ObjectiveEntityReader.FactionExists(objective.m_sSide))
			return;

		objective.m_sFaction = objective.m_sSide;

		TBD_Log.Kv(CH, "note", string.Format("objective_hold_until '%1' names no zones[].faction; objectives[].side='%2' supplies the holder. Without it this objective would be INERT and 'hold_expired' could never fire.",
			subject, objective.m_sSide));
	}

	//------------------------------------------------------------------------------------------------
	//! What the typed pass produced, always, even at zero -- and every row that bound to nothing.
	//!
	//! `locked` is reported and NOT enforced; the line says so, because an operator reading a boot
	//! log must not have to come here to find out that a field the document authored does nothing.
	protected static void ReportTypedCoverage()
	{
		int bound = 0;
		int framed = 0;
		int locked = 0;
		int losers = 0;

		if (s_aObjectives)
		{
			foreach (TBD_Objective objective : s_aObjectives)
			{
				if (!objective || !objective.m_bTyped)
					continue;

				bound++;

				if (objective.HasFraming())
					framed++;

				if (objective.m_bLocked)
					locked++;

				if (!objective.m_sAutoLoseFaction.IsEmpty())
					losers++;
			}
		}

		TBD_Log.Kv(CH, "typed", string.Format("rows=%1 bound=%2 framed=%3 locked=%4 autoLose=%5",
			TBD_ObjectiveEntityReader.Count(), bound, framed, locked, losers));

		if (TBD_ObjectiveEntityReader.GatedOutCount() > 0)
			TBD_Log.Kv(CH, "note", string.Format("%1 objectives[] row(s) were excluded by the active variant set. A row excluded because its variantId names no variants[] entry cannot be told apart from one that is simply deselected here -- that walk belongs to TBD_MissionLoader.",
				TBD_ObjectiveEntityReader.GatedOutCount()));

		if (locked > 0)
			TBD_Log.Warn(CH, string.Format("%1 objective(s) author `lock: true`. THIS BUILD DOES NOT IMPLEMENT LOCKING -- the field is parsed, counted and reported, never enforced, and the round runs as if every objective were live. WOG's `_Lock` semantics are INFERRED and the parameter says 'at round start', which cannot be honoured without an unlock the corpus gives no evidence for.",
				locked));

		TBD_ObjectiveEntityReader.ReportUnclaimed();
	}

	// ════════════════════════════════════════════════════════════════════════════════════════════
	//  DESTROY: finding the target, and counting what is left of it
	// ════════════════════════════════════════════════════════════════════════════════════════════

	//------------------------------------------------------------------------------------------------
	//! Find this objective's destroy targets. Runs ONCE, on the first LIVE evaluation.
	//!
	//! ══ THE HONEST STATE OF THIS FEATURE — read before trusting it ═══════════════════════════
	//! The DESTRUCTION SIGNAL is real and proven: `DamageManagerComponent.GetState()` returns
	//! `EDamageState`, and `EDamageState.DESTROYED` is what vanilla itself tests for — see
	//! `SCR_SpectateTargetComponent.IsAlive()` and `SCR_InventoryStorageManagerComponent`, both of
	//! which read exactly this. Compile-proven against this engine build with a failing negative
	//! control (`DamageManagerComponent.ZZ_GetStateThatDoesNotExist` -> `Undefined function`).
	//!
	//! **Targets come from the world query, not from inventing props.** T-254:
	//! `TBD_MissionDocumentStruct` models `entities[]` and `TBD_MissionLoader.SpawnMissionEntities`
	//! places every resolvable row after parse. `ArmDestroyTargets` then AABB-queries the zone for
	//! the prefab resolved from `rules.targetAlias`. When that query returns zero, the inert
	//! reason names the real cause — unresolved registry alias, no matching `entities[]` row,
	//! authored position outside the zone, or spawn/query miss — never a "build does not spawn
	//! entities[]" lie.
	//!
	//! Terrain-placed prefabs still work when `targetAlias` matches something already in the zone.
	static void ArmDestroyTargets(notnull TBD_Objective objective)
	{
		objective.m_bArmed = true;

		bool resolved = false;
		ResourceName resource = TBD_Registry.Resolve(objective.m_sTargetAlias, resolved);
		if (!resolved)
		{
			objective.m_bUsable = false;
			objective.m_sInertReason = string.Format("rules.targetAlias '%1' is not in the registry, so there is no prefab to look for", objective.m_sTargetAlias);
			TBD_Log.Warn(CH, string.Format("objective '%1' INERT: %2", objective.m_sId, objective.m_sInertReason));
			RecountUsable();
			return;
		}

		objective.m_TargetResource = resource;
		objective.m_iTargetsFound = CountLiveTargets(objective, true);
		objective.m_iTargetsDestroyed = 0;

		if (objective.m_iTargetsFound == 0)
		{
			objective.m_bUsable = false;
			objective.m_sInertReason = DiagnoseEmptyDestroyTargets(objective);
			TBD_Log.Warn(CH, string.Format("objective '%1' INERT: %2", objective.m_sId, objective.m_sInertReason));
			RecountUsable();
			return;
		}

		TBD_Log.Kv(CH, "armed", string.Format("id=%1 alias='%2' targets=%3 required=%4",
			objective.m_sId, objective.m_sTargetAlias, objective.m_iTargetsFound, objective.RequiredKills()));
	}

	//------------------------------------------------------------------------------------------------
	//! Why the zone AABB query found zero matches for an already-resolved `targetAlias`.
	//! Distinguishes missing/skipped spawn vs out-of-zone authorship (T-437). Never claims the
	//! build refuses to spawn `entities[]` — that path shipped at T-254.
	protected static string DiagnoseEmptyDestroyTargets(notnull TBD_Objective objective)
	{
		string alias = objective.m_sTargetAlias;
		array<ref TBD_MissionEntityStruct> entities = TBD_MissionLoader.GetEntities();

		int authoredMatching = 0;
		int authoredInsideZone = 0;
		if (entities)
		{
			foreach (TBD_MissionEntityStruct ent : entities)
			{
				if (!ent || ent.alias != alias)
					continue;

				authoredMatching++;
				if (objective.m_Zone && objective.m_Zone.Contains(ent.x, ent.z))
					authoredInsideZone++;
			}
		}

		if (authoredMatching == 0)
		{
			return string.Format("no entity matching alias '%1' was found inside the zone at LIVE. No `entities[]` row with that alias was authored (and no terrain prefab matched) — SpawnMissionEntities only places authored rows whose alias resolves in the registry.", alias);
		}

		if (authoredInsideZone == 0)
		{
			return string.Format("no entity matching alias '%1' was found inside the zone at LIVE. %2 `entities[]` row(s) with that alias were authored, but none sit inside this objective's zone (out-of-zone placement).", alias, authoredMatching);
		}

		return string.Format("no entity matching alias '%1' was found inside the zone at LIVE. %2 `entities[]` row(s) with that alias are authored inside the zone, so spawn likely skipped or failed for this alias — check `[TBD][Entities]` warnings (unknown registry alias / Resource.Load / SpawnEntityPrefab).", alias, authoredInsideZone);
	}

	//------------------------------------------------------------------------------------------------
	//! How many matching, NOT-destroyed entities are inside this objective's zone right now.
	//!
	//! Deliberately re-queried rather than caching entity handles. Two reasons, both about being
	//! wrong safely:
	//!   * a destroyed entity may be DELETED rather than left as rubble, and a cached `IEntity`
	//!     handle to a deleted entity is a dangling pointer this code would have to defend against
	//!     on every single tick;
	//!   * re-querying makes "gone" and "present but DESTROYED" produce the same answer, which is
	//!     the answer an objective wants in both cases.
	//! The cost is one AABB query per destroy objective per evaluation — at most a handful, at
	//! 1 Hz, over a box a few tens of metres across.
	//!
	//! KNOWN LIMIT: a target that MOVES out of the zone reads as destroyed. Acceptable for the
	//! static compositions this is for, and stated rather than discovered.
	protected static int CountLiveTargets(notnull TBD_Objective objective, bool countAll)
	{
		s_QueryResource = objective.m_TargetResource;
		s_QueryZone = objective.m_Zone;
		s_iQueryAlive = 0;
		s_iQueryMatched = 0;

		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return 0;

		vector mins = Vector(objective.m_Zone.m_fMinX, -QUERY_Y_EXTENT_M, objective.m_Zone.m_fMinZ);
		vector maxs = Vector(objective.m_Zone.m_fMaxX, QUERY_Y_EXTENT_M, objective.m_Zone.m_fMaxZ);
		world.QueryEntitiesByAABB(mins, maxs, OnQueryEntity);

		s_QueryZone = null;
		s_QueryResource = string.Empty;

		if (countAll)
			return s_iQueryMatched;

		return s_iQueryAlive;
	}

	//------------------------------------------------------------------------------------------------
	//! World-query callback. Static because the query API takes a plain function; the scratch it
	//! writes into is documented on `s_QueryResource`.
	protected static bool OnQueryEntity(IEntity entity)
	{
		if (!entity || !s_QueryZone)
			return true;

		EntityPrefabData prefabData = entity.GetPrefabData();
		if (!prefabData)
			return true;

		ResourceName prefab = prefabData.GetPrefabName();
		if (prefab != s_QueryResource)
			return true;

		// The AABB is a box; the zone may be a polygon or a circle. Ask the zone itself so a target
		// in the box but outside the actual shape is not counted.
		vector origin = entity.GetOrigin();
		if (!TBD_ZoneVolume.ContainsOrigin(s_QueryZone, origin))
			return true;

		s_iQueryMatched++;

		DamageManagerComponent damage = DamageManagerComponent.Cast(entity.FindComponent(DamageManagerComponent));
		if (!damage)
		{
			// No damage manager means nothing can ever destroy it. Counted as alive so the objective
			// reports honestly as incomplete rather than pretending it was already done.
			s_iQueryAlive++;
			return true;
		}

		if (damage.GetState() != EDamageState.DESTROYED)
			s_iQueryAlive++;

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Re-derive the per-kind usable counts. Needed because a destroy objective can go inert AFTER
	//! the build, when arming discovers there is nothing to destroy.
	protected static void RecountUsable()
	{
		s_iCaptureCount = 0;
		s_iDestroyCount = 0;
		s_iHoldCount = 0;

		if (!s_aObjectives)
			return;

		foreach (TBD_Objective objective : s_aObjectives)
		{
			if (!objective || !objective.m_bUsable)
				continue;

			if (objective.m_eKind == TBD_EObjectiveKind.CAPTURE)
				s_iCaptureCount++;
			else if (objective.m_eKind == TBD_EObjectiveKind.DESTROY)
				s_iDestroyCount++;
			else
				s_iHoldCount++;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Count this objective's destroyed targets and complete it if enough have died.
	//! Returns true on the tick it completes, so the caller can announce exactly once.
	static bool EvaluateDestroy(notnull TBD_Objective objective)
	{
		if (objective.m_bComplete || !objective.m_bUsable || !objective.m_bArmed)
			return false;

		int alive = CountLiveTargets(objective, false);
		int destroyed = objective.m_iTargetsFound - alive;
		if (destroyed < 0)
			destroyed = 0;

		objective.m_iTargetsDestroyed = destroyed;

		if (destroyed < objective.RequiredKills())
			return false;

		objective.m_bComplete = true;
		return true;
	}

	// ════════════════════════════════════════════════════════════════════════════════════════════
	//  THE END-TRIGGER AUTHORITY
	//
	//  This is the seam `TBD_FrameworkManager.TickWinConditions` consumes. It is the only place in
	//  the mod that answers "have the objectives ended the round", and it deliberately does NOT end
	//  the round itself: the stage machine has one owner and adding a second component that can
	//  call SetStage(END) would split that authority across two files.
	// ════════════════════════════════════════════════════════════════════════════════════════════

	//------------------------------------------------------------------------------------------------
	//! `all_objectives_captured` — every usable capture objective is owned by the SAME faction.
	//!
	//! Three guards, each of which matters:
	//!   * at least one usable capture objective must exist, so a mission with none never "wins by
	//!     capturing nothing";
	//!   * every one must have a non-empty owner, so a fresh round (all neutral) cannot fire it;
	//!   * all owners must agree, so a two-objective split is a stalemate rather than a win.
	//! Inert objectives are excluded entirely — they can neither fire this nor block it, which is
	//! the only reading that lets a mission with one broken objective still be winnable.
	static bool AreAllObjectivesCaptured(out string winnerFaction)
	{
		winnerFaction = string.Empty;

		if (!s_aObjectives)
			return false;

		string owner;
		int considered = 0;

		foreach (TBD_Objective objective : s_aObjectives)
		{
			if (!objective || !objective.m_bUsable || objective.m_eKind != TBD_EObjectiveKind.CAPTURE)
				continue;

			considered++;

			if (objective.m_sOwner.IsEmpty())
				return false;

			if (owner.IsEmpty())
			{
				owner = objective.m_sOwner;
				continue;
			}

			if (objective.m_sOwner != owner)
				return false;
		}

		if (considered == 0)
			return false;

		winnerFaction = owner;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! `objective_destroyed` — any usable destroy objective has been completed.
	//! The winner is the zone's `faction`, i.e. the side that was told to destroy it. Empty when the
	//! zone named none, which is reported at load.
	static bool HasObjectiveBeenDestroyed(out string winnerFaction)
	{
		winnerFaction = string.Empty;

		if (!s_aObjectives)
			return false;

		foreach (TBD_Objective objective : s_aObjectives)
		{
			if (!objective || !objective.m_bUsable || objective.m_eKind != TBD_EObjectiveKind.DESTROY)
				continue;

			if (!objective.m_bComplete)
				continue;

			winnerFaction = objective.m_sFaction;
			return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! `hold_expired` — any usable hold objective ran its clock out.
	//! The winner is the HOLDER: the side that survived the timer is the side that wins by holding.
	static bool HasHoldExpired(out string winnerFaction)
	{
		winnerFaction = string.Empty;

		if (!s_aObjectives)
			return false;

		foreach (TBD_Objective objective : s_aObjectives)
		{
			if (!objective || !objective.m_bUsable || objective.m_eKind != TBD_EObjectiveKind.HOLD_UNTIL)
				continue;

			if (!objective.m_bComplete)
				continue;

			winnerFaction = objective.m_sFaction;
			return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! ONE call for `TBD_FrameworkManager.TickWinConditions`.
	//!
	//! Returns the name of the `winConditions.endOn` trigger that has fired, or an empty string when
	//! none has. `winnerFaction` carries the side that won, which may legitimately be empty for a
	//! destroy objective whose zone named no faction.
	//!
	//! Every branch is gated on `TBD_MissionLoader.HasEndTrigger`, so a mission that did not declare
	//! a trigger can never end on it no matter what its objectives do. That gate lives here rather
	//! than at the call site precisely so it cannot be forgotten by whoever wires this up.
	//!
	//! Evaluation order is declaration order in the schema enum and is only observable in the
	//! vanishingly unlikely case of two triggers firing on the same tick; it is fixed rather than
	//! arbitrary so the outcome is reproducible.
	static string EvaluateEndTriggers(out string winnerFaction)
	{
		winnerFaction = string.Empty;

		if (!s_bBuilt)
			return string.Empty;

		if (TBD_MissionLoader.HasEndTrigger(TRIGGER_ALL_CAPTURED) && AreAllObjectivesCaptured(winnerFaction))
			return TRIGGER_ALL_CAPTURED;

		if (TBD_MissionLoader.HasEndTrigger(TRIGGER_DESTROYED) && HasObjectiveBeenDestroyed(winnerFaction))
			return TRIGGER_DESTROYED;

		if (TBD_MissionLoader.HasEndTrigger(TRIGGER_HOLD_EXPIRED) && HasHoldExpired(winnerFaction))
			return TRIGGER_HOLD_EXPIRED;

		winnerFaction = string.Empty;
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! The objective board, one line per objective, from `factionKey`'s point of view.
	//!
	//! ── This is the server-fed seam for anything a player must see ──────────────────────────
	//! Clients hold no mission document, so a client cannot compute any of this. `factionKey` is
	//! resolved from SERVER-OWNED state (the caller reads the player's assigned slot) and is never
	//! taken from anything a client sends — the same discipline `TBD_MarkerService.BuildForPlayer`
	//! uses, and the reason its request RPC takes no arguments.
	//!
	//! Today's consumer is the chat feed in `TBD_ObjectivesComponent`. A HUD cannot be the consumer
	//! yet because a new `.layout` is INVISIBLE to the engine until Workbench rewrites
	//! `resourceDatabase.rdb` (recorded landmine).
	static array<string> BuildBoardForFaction(string factionKey)
	{
		array<string> lines = new array<string>();

		if (!s_aObjectives || s_aObjectives.IsEmpty())
		{
			lines.Insert("TBD: this mission has no objectives.");
			return lines;
		}

		foreach (TBD_Objective objective : s_aObjectives)
		{
			if (!objective)
				continue;

			lines.Insert(objective.BoardLine(factionKey));

			// T-212 -- the per-side task BODY, when this side was framed, as its own continuation
			// line. The board is a list of lines and its consumer renders them in order, so an
			// objective that reads differently to attacker and defender says so HERE rather than
			// only in the load log. Unframed objectives add nothing and the board is unchanged.
			string task = objective.TaskTextFor(factionKey);
			if (task.IsEmpty())
				continue;

			string detail = "    ";
			detail += task;
			lines.Insert(detail);
		}

		return lines;
	}
}
