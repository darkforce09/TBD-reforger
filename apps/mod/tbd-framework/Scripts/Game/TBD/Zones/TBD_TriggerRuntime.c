//! T-676 - editor-authored TRIGGERS: activation (condition / owner side / repeat / timeout) and
//! the effects they fire. The Enfusion half of `mission.schema.json#/properties/editorTriggers`.
//!
//! == What was missing, and what this is ======================================================
//! T-079 shipped the geometry palette and T-706 put `editorTriggers[]` on the wire. Nothing read
//! it: a word-boundary grep for `editorTriggers` over `apps/mod` returned ZERO hits on every
//! shipped build, so an author could draw a trigger, compile it into the document, and the round
//! would run as if it were not there. This file is the reader and the runtime.
//!
//! == Server-side only ========================================================================
//! Clients hold NO mission document - `TBD_FrameworkManager.OnPostInit` returns early for
//! `RplMode.Client` before `BeginLoad()` - so a client that built this registry would build an
//! empty one and conclude that no trigger exists. Everything here except the two `RplRcver.Owner`
//! RPC bodies runs on the authority, and the heartbeat below refuses to start on a client.
//!
//! == Why the heartbeat is a `modded class SCR_BaseGameMode` and not a game-mode component =====
//! Every other TBD subsystem ticks from an `SCR_BaseGameModeComponent` listed in
//! `Prefabs/Systems/TBD_GameMode.et`. That is the nicer shape and it is NOT what this uses, for
//! one reason: a component this slice cannot add to the prefab would never be instantiated, and a
//! runtime that can never fire is worse than an absent one - it reads as shipped in every grep and
//! is dead in every round. `modded class SCR_BaseGameMode` self-wires, is the established idiom in
//! this addon (ten `modded class` blocks already), and costs one guard: the heartbeat is fenced
//! behind `TBD_FrameworkManager.IsFrameworkWorld()`, the published test every vanilla-touching
//! modded block in this tree already asks, so a vanilla scenario running this mod schedules
//! nothing at all. If a later slice moves the tick onto a real component, the seam is
//! `TBD_TriggerRuntime.Tick()` and nothing else changes.
//!
//! == Why the wire is parsed a SECOND time ====================================================
//! `TBD_MissionDocumentStruct` (Backend/TBD_MissionLoader.c) declares no `editorTriggers` field,
//! and Enfusion's `JsonLoadContext` maps JSON keys onto NAMED class fields only - a key no class
//! declares is invisible at runtime, not rejected, not logged, simply absent. Rather than grow the
//! shared document struct from another lane, this runs its own `JsonLoadContext` pass over
//! `TBD_MissionLoader.GetRawJson()` with a root that declares `editorTriggers` and nothing else.
//! That is the pattern `Objectives/TBD_ObjectiveRules.c` established for the objective half of
//! `zoneRules`, for the same two reasons: the vocabulary stays next to the code that interprets
//! it, and the cost is one extra parse of a document that is at most 8 MB and is parsed exactly
//! ONCE per world.
//!
//! == `zoneRules` keeps today's semantics =====================================================
//! The sixteen `#/$defs/zoneRules` keys are read where they always were - the play-area three in
//! `TBD_MissionZoneRulesStruct`, the objective thirteen in `TBD_ObjectiveRulesStruct`. This file
//! declares NONE of them and changes NONE of them. It consumes the zone the registry already
//! prepared (`TBD_ZoneRegistry.GetAll()`), so a trigger's area is the same volume the play-area
//! and objective subsystems see, resolved once, with one owner.
//!
//! == Presence, and the nested-ref landmine ===================================================
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref <class>` field even when the JSON key is
//! ABSENT (measured - see the landmine on `TBD_MissionShapeStruct`). So `if (trigger.activation)`
//! is ALWAYS TRUE and is not a presence test. Every numeric and string field below therefore
//! carries an ABSENT sentinel, and every container is tested by COUNT. Bools cannot carry a
//! sentinel - there is no third value - so for `repeat` "absent" and "authored false" are
//! indistinguishable and always will be; `repeat = false` is the schema's own default, so nothing
//! is lost behaviourally and the limitation is stated rather than papered over.
//!
//! == What this file CANNOT prove =============================================================
//! The gate is `cargo xtask mod compile` - a headless compile against the native dedicated server.
//! It proves every symbol here exists and every signature matches. It cannot run a round. Whether
//! a trigger authored in the editor actually fires with players in the zone is a human checklist
//! item and is written up as one; nothing in this header claims otherwise.
//! @contract mission.schema.json#/$defs/editorTrigger

//------------------------------------------------------------------------------------------------
//! The `effects[].params` bag, as much of it as a TYPED parser can see.
//!
//! `#/$defs/editorTrigger` declares `params` as an OPEN object - "Effect-specific parameters,
//! interpreted by the T-676 reader. Open by design". This class IS that interpretation: it is the
//! params vocabulary, and a key not declared here is invisible to the runtime exactly as an
//! undeclared `zoneRules` key is invisible to the loader. The difference from `zoneRules` matters
//! and is stated once: `zoneRules` is `additionalProperties: false`, so schema validation catches a
//! misspelled key there. `params` is open, so **schema validation catches nothing here** and a
//! misspelled param is silent at both ends. That is the price of the open bag, it is the schema's
//! deliberate choice, and the mitigation is this class: every effect reports, by trigger id and
//! effect index, which params it actually resolved - so an author who typed `aliass` sees the
//! effect go INERT with the reason naming the param it wanted, instead of a trigger that fires and
//! does nothing.
//!
//! -- The vocabulary, by effect `type` --------------------------------------------------------
//! `spawn`         `alias` (required, registry alias) * `x` `z` (world metres; default = the
//!                 trigger zone's centre) * `headingDeg` (default 0) * `count` (default 1)
//! `delete`        `alias` (required) - every matching entity inside the trigger's zone
//! `end_mission`   `winner` (faction key, optional - carried into the `[TBD][Win]` line)
//! `set_objective` `objectiveId` (required, a `zones[].id`) * `state` ("complete"|"incomplete")
//!                 * `owner` (faction key)
//! `hint`          `text` (required) * `audience`
//! `play_sound`    `sound` (required, an `SCR_SoundEvent` name) * `audience`
//! `set_variant`   `variantId` (required)
//!
//! `audience` is "all" (or absent) * "owner" (the activation's `ownerSide`) * "enemy" (every side
//! that is not `ownerSide`) * or any faction key.
class TBD_TriggerParamsStruct
{
	//! Sentinel for "key absent from JSON". Same device and same reasoning as
	//! `TBD_MissionZoneRulesStruct.ABSENT` and `TBD_ObjectiveRulesStruct.ABSENT`: `JsonLoadContext`
	//! leaves a missing key at its field initializer and standard JSON cannot carry NaN, so an
	//! initializer no sane author would type doubles as the presence flag. It is what lets a
	//! coordinate authored at exactly 0 be told apart from a coordinate nobody authored - which
	//! matters here, because 0,0 is a real place on every Reforger terrain.
	static const float ABSENT = -1000000;

	//! Same idea for the one integer. Kept separate because `count: 0` is a value an author can
	//! type, so the sentinel must not be 0.
	static const int ABSENT_INT = -1;

	string alias;        //!< `spawn` / `delete`. Empty = absent.
	string text;         //!< `hint`. Empty = absent.
	string sound;        //!< `play_sound`. Empty = absent.
	string audience;     //!< `hint` / `play_sound`. Empty = absent (means "all").
	string objectiveId;  //!< `set_objective`. Empty = absent.
	string state;        //!< `set_objective`. Empty = absent (means "complete").
	string owner;        //!< `set_objective`. Empty = absent (leave the owner alone).
	string winner;       //!< `end_mission`. Empty = absent (no winner named).
	string variantId;    //!< `set_variant`. Empty = absent.

	float x = ABSENT;
	float z = ABSENT;
	float headingDeg = ABSENT;

	int count = ABSENT_INT;
}

//------------------------------------------------------------------------------------------------
//! One `effects[]` record on the wire.
//! @contract mission.schema.json#/$defs/editorTrigger/properties/effects/items
class TBD_TriggerEffectStruct
{
	string type;   //!< Schema enum: spawn|delete|end_mission|set_objective|hint|play_sound|set_variant.
	//! ALWAYS non-null after a parse even when the JSON key is absent - read its FIELDS against
	//! `TBD_TriggerParamsStruct.ABSENT` / empty, never this reference against null.
	ref TBD_TriggerParamsStruct params;
}

//------------------------------------------------------------------------------------------------
//! The `activation` object on the wire. Schema-required on a trigger, but `JsonLoadContext`
//! allocates it either way, so its presence is decided by `condition` being non-empty.
//! @contract mission.schema.json#/$defs/editorTrigger/properties/activation
class TBD_TriggerActivationStruct
{
	string condition;     //!< present|not_present|detected_by|seized_by|timer|objective_complete.
	string ownerSide;     //!< `factionKey`. Empty = absent; meaning is per condition.
	bool repeat;          //!< Schema default false. Absent and authored-false are the same value.
	float timeoutSeconds = TBD_TriggerParamsStruct.ABSENT;
}

//------------------------------------------------------------------------------------------------
//! One `editorTriggers[]` entry on the wire.
//! @contract mission.schema.json#/$defs/editorTrigger
class TBD_EditorTriggerStruct
{
	string id;
	//! The `zones[].id` whose volume arms this trigger. OPTIONAL: "a trigger without a zone is a
	//! document-wide condition" (schema). Empty = absent.
	string zoneId;
	ref TBD_TriggerActivationStruct activation;
	//! OPTIONAL. Null or empty means the trigger activates and does nothing, which is reported at
	//! load rather than ticked forever.
	ref array<ref TBD_TriggerEffectStruct> effects;
	string variantId;   //!< T-654. Empty = ungated. See TBD_TriggerRuntime's variant note.
}

//------------------------------------------------------------------------------------------------
//! The document root for the TRIGGER pass: declares `editorTriggers` and nothing else, so this
//! reader stays structurally blind to every other top-level key. That is not a claim about the
//! primary loader - `TBD_MissionDocumentStruct` models meta/zones/slots/entities/settings and more;
//! this struct exists only to reach `editorTriggers[]`, which that struct does not declare.
class TBD_TriggerDocStruct
{
	ref array<ref TBD_EditorTriggerStruct> editorTriggers;
}

//------------------------------------------------------------------------------------------------
//! What makes a trigger fire. `NONE` is not a condition - it is what an unrecognised or unauthored
//! `activation.condition` resolves to, and a trigger that resolves to it is INERT and says so.
enum TBD_ETriggerCondition
{
	NONE,
	PRESENT,             //!< At least one live player of `ownerSide` is inside the area.
	NOT_PRESENT,         //!< No live player of `ownerSide` is inside the area.
	DETECTED_BY,         //!< A side that is NOT `ownerSide` is inside and `ownerSide` is near it.
	SEIZED_BY,           //!< `ownerSide` is inside and no other side is.
	TIMER,               //!< Holds from the moment the runtime arms; `timeoutSeconds` is the whole rule.
	OBJECTIVE_COMPLETE   //!< The named objective (or every objective) is complete.
}

//------------------------------------------------------------------------------------------------
//! What an effect does. `NONE` is an unrecognised `effects[].type`; such an effect is dropped at
//! load with its index named, and never silently swallowed.
enum TBD_ETriggerEffect
{
	NONE,
	SPAWN,
	DELETE,
	END_MISSION,
	SET_OBJECTIVE,
	HINT,
	PLAY_SOUND,
	SET_VARIANT
}

//------------------------------------------------------------------------------------------------
//! One prepared effect: the wire record with its type resolved and its params flattened, validated
//! ONCE at load. `m_bUsable` false means this effect can never do anything and `m_sInertReason`
//! says why, by trigger id and effect index.
class TBD_TriggerEffect
{
	TBD_ETriggerEffect m_eKind;
	string m_sRawType;       //!< The authored string, kept for the diagnostic on an unknown type.

	string m_sAlias;
	string m_sText;
	string m_sSound;
	string m_sAudience;
	string m_sObjectiveId;
	string m_sState;
	string m_sOwner;
	string m_sWinner;
	string m_sVariantId;

	float m_fX;
	float m_fZ;
	float m_fHeadingDeg;
	int m_iCount;

	bool m_bUsable;
	string m_sInertReason;
}

//------------------------------------------------------------------------------------------------
//! Where a trigger is in its life. The state machine is deliberately four states and no flags:
//! "has it fired" and "is it counting down" are different questions and a bool pair would let them
//! disagree.
enum TBD_ETriggerState
{
	INERT,     //!< Could never fire. Reported at load; never ticked.
	ARMED,     //!< Waiting for the condition to hold.
	PENDING,   //!< The condition is holding; the timeout is counting.
	FIRED      //!< The effects have run. Terminal unless `repeat`.
}

//------------------------------------------------------------------------------------------------
//! One prepared trigger.
class TBD_Trigger
{
	string m_sId;
	string m_sZoneId;
	//! The prepared zone this trigger watches, or null for a document-wide condition. A REFERENCE
	//! into `TBD_ZoneRegistry`'s array, not a copy: the trigger's area and the play area's area are
	//! the same object, so they cannot drift.
	TBD_Zone m_Zone;

	TBD_ETriggerCondition m_eCondition;
	string m_sRawCondition;   //!< The authored string, for the unknown-condition diagnostic.
	string m_sOwnerSide;
	bool m_bRepeat;
	float m_fTimeoutSeconds;
	string m_sVariantId;

	ref array<ref TBD_TriggerEffect> m_aEffects;

	TBD_ETriggerState m_eState;
	//! Seconds the condition has held CONTINUOUSLY. Reset to 0 the moment it stops holding, which
	//! is what makes `timeoutSeconds` a dwell time rather than a stopwatch that survives a gap.
	float m_fHeldSeconds;
	int m_iFireCount;
	string m_sInertReason;

	//------------------------------------------------------------------------------------------------
	//! Stable identifier for logs. Built in steps, not one long `+` chain: a 9-term concatenation is
	//! a measured `Formula too complex` in this compiler, whose SECOND diagnostic is a misleading
	//! `Incompatible parameter`.
	string LogKey()
	{
		string key = "trigger:";
		key += m_sId;
		return key;
	}

	//------------------------------------------------------------------------------------------------
	//! How many prepared effects this trigger can actually run.
	int UsableEffectCount()
	{
		if (!m_aEffects)
			return 0;

		int usable = 0;
		foreach (TBD_TriggerEffect effect : m_aEffects)
		{
			if (effect && effect.m_bUsable)
				usable++;
		}

		return usable;
	}
}

//------------------------------------------------------------------------------------------------
//! The trigger runtime: reads `editorTriggers[]`, prepares them against the zone registry,
//! evaluates activation once a second while the round is LIVE, and fires the effects.
//!
//! -- Static, and therefore explicitly cleared ------------------------------------------------
//! A recorded landmine in this program is that statics OUTLIVE A WORLD inside one process
//! (`SelectMissionByNumber` restarts the scenario in-process via
//! `GameStateTransitions.RequestScenarioRestart()`). Mission A's triggers left standing would fire
//! into mission B. Two independent defences, because one of them is a promise and the other is a
//! measurement:
//!   * `Clear()` is called from `SCR_BaseGameMode.OnGameStart` - at the START of each world rather
//!     than the end of the last one, so it does not depend on a teardown hook firing;
//!   * `Tick()` refuses to run when the loaded mission id is not the id this registry was built
//!     for. That is the cheap version of the rule that an answer about the world must carry the
//!     world it was built for: a trigger array says "a player is inside zone z3", and z3 means
//!     nothing without the document it came from.
//!
//! -- Variants (T-654), and what `set_variant` is for -----------------------------------------
//! `editorTrigger.variantId` is a COMPILE-time gate in the schema: the compiler emits the subtree
//! only when the variant is selected. T-654 has not landed, so nothing strips them today and a
//! `variantId` arrives verbatim on the wire. This runtime therefore starts in NO-SELECTION mode,
//! where every trigger runs whatever its `variantId` - the compiler is the authority on inclusion
//! and refusing to run what it emitted would silently delete authored content. The FIRST
//! `set_variant` effect switches the runtime into SELECTION-IN-FORCE mode: from then on a trigger
//! with a non-empty `variantId` runs only while that variant is in the selected set, and a trigger
//! with no `variantId` always runs. Both mode switches are logged once. That gives `set_variant` a
//! live consumer in this build instead of a write nobody reads, and it is a no-op for every mission
//! that never authors one.
class TBD_TriggerRuntime
{
	//! Log channel. A literal rather than a `TBD_Log.CH_*` constant for the same reason
	//! `TBD_ZoneRegistry.CH` is one: `Core/TBD_Log.c` belongs to another lane, and keeping the
	//! string in a single place here preserves the greppable-tag property the constants exist for.
	static const string CH = "Trigger";

	//! Evaluation cadence. 1 Hz, matching `TBD_PlayAreaComponent.TICK_MS` and
	//! `TBD_ObjectivesComponent`: a dwell time authored in seconds is accurate to within a second,
	//! and walking every player costs nothing at this rate. Deliberately NOT per frame.
	static const int TICK_MS = 1000;

	//! `TICK_MS` as seconds, so the accumulator reads in the units `timeoutSeconds` is authored in.
	static const float TICK_SECONDS = 1.0;

	//! Schema `activation.condition` values (`#/$defs/editorTrigger`). A closed enum there, so a
	//! value outside this set cannot pass validation; a document that reaches us out of band with
	//! one is reported and the trigger goes INERT rather than guessing at an activation.
	static const string COND_PRESENT            = "present";
	static const string COND_NOT_PRESENT        = "not_present";
	static const string COND_DETECTED_BY        = "detected_by";
	static const string COND_SEIZED_BY          = "seized_by";
	static const string COND_TIMER              = "timer";
	static const string COND_OBJECTIVE_COMPLETE = "objective_complete";

	//! Schema `effects[].type` values (`#/$defs/editorTrigger`). Also a closed enum there.
	static const string FX_SPAWN         = "spawn";
	static const string FX_DELETE        = "delete";
	static const string FX_END_MISSION   = "end_mission";
	static const string FX_SET_OBJECTIVE = "set_objective";
	static const string FX_HINT          = "hint";
	static const string FX_PLAY_SOUND    = "play_sound";
	static const string FX_SET_VARIANT   = "set_variant";

	//! `params.audience` vocabulary. Anything outside it is treated as a faction key, which is why
	//! these three words are the ones an author must not use as a faction key.
	static const string AUD_ALL   = "all";
	static const string AUD_OWNER = "owner";
	static const string AUD_ENEMY = "enemy";

	//! `params.state` for `set_objective`.
	static const string STATE_COMPLETE   = "complete";
	static const string STATE_INCOMPLETE = "incomplete";

	//! How near a friendly must be to an intruder for `detected_by` to hold, in metres.
	//!
	//! DETECTION IS MODELLED AS PROXIMITY, NOT PERCEPTION, and that is a deliberate, stated
	//! limitation rather than an approximation nobody wrote down. Arma's "detected by" asks whether
	//! a side KNOWS about an enemy, which for AI is `knowsAbout` and for players is nothing at all -
	//! there is no per-player knowledge model in this engine to read. So this asks the question a
	//! server can actually answer: is somebody of `ownerSide` close enough to the intruder to have
	//! seen them. No line of sight is traced, so an enemy behind a ridge 200 m from a friendly
	//! counts as detected. 250 m is chosen as roughly the range at which a moving figure is
	//! reliably picked up by the naked eye in daylight; it is a constant here and not an authored
	//! key because `activation` is `additionalProperties: false` in the schema and this slice does
	//! not widen it (T-706 owns widening).
	static const float DETECT_RADIUS_M = 250.0;

	//! Ceiling on one `spawn` effect's `count`. A typo (`count: 10000`) inside a repeating trigger
	//! is a server the operator has to kill. Clamped and reported, never obeyed silently.
	static const int MAX_SPAWN_COUNT = 32;

	//! Vertical half-extent of the AABB a `delete` effect queries, in metres. Same value and same
	//! reasoning as `TBD_ObjectiveRegistry.QUERY_Y_EXTENT_M`: a zone is a 2D footprint, the box has
	//! to be tall enough to contain anything standing on any terrain inside it, and `TBD_Zone`
	//! decides the actual containment afterwards.
	static const float QUERY_Y_EXTENT_M = 5000.0;

	protected static ref array<ref TBD_Trigger> s_aTriggers;
	protected static bool s_bBuilt;
	protected static int s_iArmedCount;
	protected static int s_iInertCount;

	//! The mission this registry was built for. Compared against the live document every tick -
	//! see the class header on why a trigger array is meaningless without its document.
	protected static string s_sBuiltForMission;

	//! Runtime variant selection. See the class header. `s_bVariantSelectionInForce` is what
	//! distinguishes "nobody has ever called set_variant" from "set_variant selected nothing".
	protected static ref array<string> s_aSelectedVariants;
	protected static bool s_bVariantSelectionInForce;

	//! Per-tick player snapshot, rebuilt from the live player list each evaluation. Parallel arrays
	//! rather than a struct array: this is allocated once per tick and read once per trigger, and
	//! four flat arrays cost one allocation each instead of one per player.
	protected static ref array<int> s_aPlayerIds;
	protected static ref array<float> s_aPlayerX;
	protected static ref array<float> s_aPlayerZ;
	protected static ref array<string> s_aPlayerFaction;

	//! Scratch for the `delete` effect's world query. Static because `QueryEntitiesByAABB` takes a
	//! plain function reference and cannot carry state - the same device
	//! `TBD_ObjectiveRegistry.OnQueryEntity` uses, and for the same reason.
	protected static ResourceName s_QueryResource;
	protected static TBD_Zone s_QueryZone;
	protected static ref array<IEntity> s_aQueryHits;

	//! Latch so the "no triggers authored" line is said once per world and not once a second.
	protected static bool s_bAnnounced;

	//! T-181.30's latch shape for the stood-down line, which would otherwise write one ERROR per
	//! second for the rest of the round. Cleared the moment a snapshot succeeds again, so a genuine
	//! second occurrence still gets said.
	protected static bool s_bAnnouncedNoSnapshot;

	//------------------------------------------------------------------------------------------------
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//------------------------------------------------------------------------------------------------
	//! Triggers that can fire. Zero is a legitimate answer for a mission that authors none.
	static int GetArmedCount()
	{
		return s_iArmedCount;
	}

	//------------------------------------------------------------------------------------------------
	//! Triggers that were authored but can never fire. Every one of them was reported by id at load.
	static int GetInertCount()
	{
		return s_iInertCount;
	}

	//------------------------------------------------------------------------------------------------
	//! Every prepared trigger, INERT ones included so a count here matches the document. Null until
	//! `Build()` has run.
	static array<ref TBD_Trigger> GetAll()
	{
		return s_aTriggers;
	}

	//------------------------------------------------------------------------------------------------
	//! Drop everything. MUST run on every world start - see the class header.
	static void Clear()
	{
		s_aTriggers = null;
		s_bBuilt = false;
		s_iArmedCount = 0;
		s_iInertCount = 0;
		s_sBuiltForMission = string.Empty;
		s_aSelectedVariants = null;
		s_bVariantSelectionInForce = false;
		s_aPlayerIds = null;
		s_aPlayerX = null;
		s_aPlayerZ = null;
		s_aPlayerFaction = null;
		s_QueryResource = string.Empty;
		s_QueryZone = null;
		s_aQueryHits = null;
		s_bAnnounced = false;
		s_bAnnouncedNoSnapshot = false;
	}

	// ============================================================================================
	//  THE WIRE
	// ============================================================================================

	//------------------------------------------------------------------------------------------------
	//! The mission id this registry belongs to, or empty when none is loaded.
	protected static string CurrentMissionId()
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !doc.meta)
			return string.Empty;

		return doc.meta.id;
	}

	//------------------------------------------------------------------------------------------------
	//! Second typed pass over the raw mission JSON for `editorTriggers[]`.
	//!
	//! Returns null on every honest "nothing to read": no raw document (a client, or a mission that
	//! has not loaded yet), unparseable JSON, or a document that authors no `editorTriggers` key.
	//! The caller distinguishes those from a parse that produced an empty array.
	protected static array<ref TBD_EditorTriggerStruct> ReadWire()
	{
		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return null;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
		{
			TBD_Log.Error(CH, "the mission document did not parse as JSON on the trigger pass - no trigger is armed this round");
			return null;
		}

		TBD_TriggerDocStruct doc = new TBD_TriggerDocStruct();
		if (!ctx.ReadValue("", doc))
		{
			TBD_Log.Error(CH, "the mission document parsed but its root would not read on the trigger pass - no trigger is armed this round");
			return null;
		}

		return doc.editorTriggers;
	}

	// ============================================================================================
	//  BUILD
	// ============================================================================================

	//------------------------------------------------------------------------------------------------
	//! Prepare every authored trigger. Safe to call repeatedly; only the first call after a
	//! `Clear()` does work.
	//!
	//! Returns false when there is nothing to build from yet - no valid mission, or a zone registry
	//! that has not been built - so the caller keeps waiting instead of caching an empty registry as
	//! if it were the answer. That is the same contract `TBD_ZoneRegistry.Build()` has, and it is
	//! why the heartbeat can call this every second without a guard of its own.
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		// The zone registry is this runtime's geometry. Without it a `zoneId` cannot be resolved,
		// and a trigger whose area silently became "the whole world" would be far worse than one
		// that waited a second longer.
		if (!TBD_ZoneRegistry.IsBuilt())
			return false;

		string missionId = CurrentMissionId();
		array<ref TBD_EditorTriggerStruct> raw = ReadWire();
		if (!raw)
		{
			// A mission that authors no `editorTriggers` key is the overwhelmingly common case and
			// is NOT an error. Mark the registry built so the wire is not re-parsed once a second
			// for the rest of the round.
			if (!TBD_MissionLoader.GetRawJson().IsEmpty())
			{
				s_aTriggers = new array<ref TBD_Trigger>();
				s_bBuilt = true;
				s_sBuiltForMission = missionId;
				return true;
			}

			return false;
		}

		s_aTriggers = new array<ref TBD_Trigger>();
		s_aSelectedVariants = new array<string>();
		s_iArmedCount = 0;
		s_iInertCount = 0;

		foreach (int index, TBD_EditorTriggerStruct rawTrigger : raw)
		{
			if (!rawTrigger)
			{
				TBD_Log.Warn(CH, string.Format("editorTriggers[%1] is null - skipped", index));
				continue;
			}

			TBD_Trigger trigger = Prepare(rawTrigger, index);
			s_aTriggers.Insert(trigger);

			if (trigger.m_eState == TBD_ETriggerState.INERT)
			{
				s_iInertCount++;
				TBD_Log.Warn(CH, string.Format("%1 INERT: %2", trigger.LogKey(), trigger.m_sInertReason));
			}
			else
			{
				s_iArmedCount++;
				LogPrepared(trigger);
			}
		}

		s_bBuilt = true;
		s_sBuiltForMission = missionId;

		// One greppable summary line, in the shape `TBD_ZoneRegistry.built` established.
		TBD_Log.Kv(CH, "built", string.Format("mission='%1' triggers=%2 armed=%3 inert=%4 cadence=%5ms",
			missionId, raw.Count(), s_iArmedCount, s_iInertCount, TICK_MS));

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! One line per armed trigger, at load. Same two jobs the zone registry's equivalent has: an
	//! operator can read what will fire straight out of the boot log and check it against the
	//! mission before an event, and the line is computed from the parsed fields, so a run that
	//! prints a plausible condition and a resolved zone has DEMONSTRATED that the second JSON pass
	//! really populated `editorTriggers[]` - not merely that a non-null array arrived.
	protected static void LogPrepared(notnull TBD_Trigger trigger)
	{
		string zoneLabel = "<document-wide>";
		if (trigger.m_Zone)
			zoneLabel = trigger.m_Zone.LogKey();

		TBD_Log.Kv(CH, "trigger", string.Format("id=%1 condition=%2 owner='%3' zone=%4 repeat=%5 timeout=%6s effects=%7 variant='%8'",
			trigger.m_sId,
			typename.EnumToString(TBD_ETriggerCondition, trigger.m_eCondition),
			trigger.m_sOwnerSide,
			zoneLabel,
			trigger.m_bRepeat,
			trigger.m_fTimeoutSeconds,
			trigger.UsableEffectCount(),
			trigger.m_sVariantId));
	}

	//------------------------------------------------------------------------------------------------
	//! Flatten one wire trigger into its runtime form, reporting every defect by trigger id.
	protected static TBD_Trigger Prepare(notnull TBD_EditorTriggerStruct rawTrigger, int index)
	{
		TBD_Trigger trigger = new TBD_Trigger();
		trigger.m_sId = rawTrigger.id;
		if (trigger.m_sId.IsEmpty())
			trigger.m_sId = string.Format("editorTriggers[%1]", index);

		trigger.m_sZoneId = rawTrigger.zoneId;
		trigger.m_sVariantId = rawTrigger.variantId;
		trigger.m_eState = TBD_ETriggerState.ARMED;
		trigger.m_aEffects = new array<ref TBD_TriggerEffect>();

		ResolveActivation(trigger, rawTrigger.activation);
		ResolveZone(trigger);
		ResolveEffects(trigger, rawTrigger.effects);

		if (trigger.m_eState == TBD_ETriggerState.INERT)
			return trigger;

		if (trigger.UsableEffectCount() == 0)
		{
			trigger.m_eState = TBD_ETriggerState.INERT;
			trigger.m_sInertReason = "it has no effect this build can run, so activating it could not change anything. Author at least one `effects[]` record whose `type` and `params` resolve.";
		}

		return trigger;
	}

	//------------------------------------------------------------------------------------------------
	//! `activation` -> condition enum, owner side, repeat and dwell time.
	//!
	//! `TIMEOUT IS A DWELL, NOT A STOPWATCH.` The schema calls `timeoutSeconds` the "delay between
	//! the condition holding and the effects firing", which is Arma's trigger timeout: the condition
	//! must hold CONTINUOUSLY for that long. A gap resets it. That reading is the one an author gets
	//! from Eden and it is the only one that makes `not_present` usable - "nobody has been in this
	//! zone for two minutes" is a sentence; "nobody was in this zone at some point two minutes ago"
	//! is not.
	protected static void ResolveActivation(notnull TBD_Trigger trigger, TBD_TriggerActivationStruct activation)
	{
		trigger.m_eCondition = TBD_ETriggerCondition.NONE;
		trigger.m_fTimeoutSeconds = 0;

		if (!activation)
		{
			// Cannot happen through JsonLoadContext, which allocates the nested ref regardless.
			// Handled anyway rather than dereferenced on faith.
			trigger.m_eState = TBD_ETriggerState.INERT;
			trigger.m_sInertReason = "it has no `activation` block, so there is nothing to make it fire";
			return;
		}

		trigger.m_sRawCondition = activation.condition;
		trigger.m_sOwnerSide = activation.ownerSide;
		trigger.m_bRepeat = activation.repeat;

		if (activation.timeoutSeconds != TBD_TriggerParamsStruct.ABSENT)
		{
			if (activation.timeoutSeconds < 0)
			{
				TBD_Log.Error(CH, string.Format("trigger '%1' authors activation.timeoutSeconds=%2, which is negative - using 0s (fire as soon as the condition holds)",
					trigger.m_sId, activation.timeoutSeconds));
			}
			else
			{
				trigger.m_fTimeoutSeconds = activation.timeoutSeconds;
			}
		}

		trigger.m_eCondition = ConditionFromString(activation.condition);
		if (trigger.m_eCondition == TBD_ETriggerCondition.NONE)
		{
			trigger.m_eState = TBD_ETriggerState.INERT;
			if (activation.condition.IsEmpty())
			{
				trigger.m_sInertReason = "its `activation.condition` is absent. The schema's vocabulary is present | not_present | detected_by | seized_by | timer | objective_complete.";
			}
			else
			{
				trigger.m_sInertReason = string.Format("its `activation.condition` is '%1', which this build does not recognise. The schema's vocabulary is present | not_present | detected_by | seized_by | timer | objective_complete.",
					activation.condition);
			}
			return;
		}

		// Two conditions are questions ABOUT a side and cannot be asked without one. Refusing here
		// rather than defaulting to "anybody" is the conservative direction: a `seized_by` with no
		// owner that quietly meant "somebody is here alone" would fire on the wrong side's advance.
		if (trigger.m_sOwnerSide.IsEmpty())
		{
			if (trigger.m_eCondition == TBD_ETriggerCondition.SEIZED_BY || trigger.m_eCondition == TBD_ETriggerCondition.DETECTED_BY)
			{
				trigger.m_eState = TBD_ETriggerState.INERT;
				trigger.m_sInertReason = string.Format("condition '%1' asks which SIDE seized or detected, and `activation.ownerSide` is absent. Set it to a `factions[].key`.",
					activation.condition);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Schema enum string -> condition. Returns `NONE` for absent and for anything outside the
	//! closed vocabulary; the caller turns that into an INERT trigger with the authored value named.
	protected static TBD_ETriggerCondition ConditionFromString(string raw)
	{
		if (raw == COND_PRESENT)
			return TBD_ETriggerCondition.PRESENT;
		if (raw == COND_NOT_PRESENT)
			return TBD_ETriggerCondition.NOT_PRESENT;
		if (raw == COND_DETECTED_BY)
			return TBD_ETriggerCondition.DETECTED_BY;
		if (raw == COND_SEIZED_BY)
			return TBD_ETriggerCondition.SEIZED_BY;
		if (raw == COND_TIMER)
			return TBD_ETriggerCondition.TIMER;
		if (raw == COND_OBJECTIVE_COMPLETE)
			return TBD_ETriggerCondition.OBJECTIVE_COMPLETE;

		return TBD_ETriggerCondition.NONE;
	}

	//------------------------------------------------------------------------------------------------
	//! Bind `zoneId` to the zone the registry already prepared.
	//!
	//! An absent `zoneId` is LEGAL and means "document-wide" (the schema says so). An authored
	//! `zoneId` that names nothing, or names a zone with no usable shape, is a defect: the author
	//! drew an area and the trigger would silently become world-wide. That is refused.
	protected static void ResolveZone(notnull TBD_Trigger trigger)
	{
		if (trigger.m_eState == TBD_ETriggerState.INERT)
			return;

		if (trigger.m_sZoneId.IsEmpty())
		{
			// `detected_by` needs an area to be intruded upon; without one there is no intrusion to
			// notice, only "somebody is near somebody", which is not what the author asked for.
			if (trigger.m_eCondition == TBD_ETriggerCondition.DETECTED_BY)
			{
				trigger.m_eState = TBD_ETriggerState.INERT;
				trigger.m_sInertReason = "condition 'detected_by' needs an area to be entered, and no `zoneId` is authored. Point it at a `zones[].id`.";
			}
			return;
		}

		array<ref TBD_Zone> zones = TBD_ZoneRegistry.GetAll();
		if (!zones)
		{
			trigger.m_eState = TBD_ETriggerState.INERT;
			trigger.m_sInertReason = "the zone registry produced no zones, so its `zoneId` cannot be resolved";
			return;
		}

		foreach (TBD_Zone zone : zones)
		{
			if (zone && zone.m_sId == trigger.m_sZoneId)
			{
				trigger.m_Zone = zone;
				break;
			}
		}

		if (!trigger.m_Zone)
		{
			trigger.m_eState = TBD_ETriggerState.INERT;
			trigger.m_sInertReason = string.Format("its `zoneId` is '%1' and no `zones[]` row has that id", trigger.m_sZoneId);
			return;
		}

		if (!trigger.m_Zone.IsUsable())
		{
			trigger.m_Zone = null;
			trigger.m_eState = TBD_ETriggerState.INERT;
			trigger.m_sInertReason = string.Format("zone '%1' has no usable shape (no circle, or a polygon with fewer than 3 vertices), so it can never contain anybody",
				trigger.m_sZoneId);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! `effects[]` -> prepared effects, each validated once, each defect reported by index.
	protected static void ResolveEffects(notnull TBD_Trigger trigger, array<ref TBD_TriggerEffectStruct> effects)
	{
		if (trigger.m_eState == TBD_ETriggerState.INERT)
			return;

		if (!effects || effects.Count() == 0)
		{
			trigger.m_eState = TBD_ETriggerState.INERT;
			trigger.m_sInertReason = "it authors no `effects[]`, so firing it would do nothing";
			return;
		}

		foreach (int index, TBD_TriggerEffectStruct rawEffect : effects)
		{
			if (!rawEffect)
			{
				TBD_Log.Warn(CH, string.Format("trigger '%1' effects[%2] is null - skipped", trigger.m_sId, index));
				continue;
			}

			TBD_TriggerEffect effect = PrepareEffect(trigger, rawEffect);
			trigger.m_aEffects.Insert(effect);

			if (!effect.m_bUsable)
			{
				TBD_Log.Warn(CH, string.Format("trigger '%1' effects[%2] (type '%3') is INERT: %4",
					trigger.m_sId, index, effect.m_sRawType, effect.m_sInertReason));
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Flatten one effect record and decide, once, whether it can ever run.
	protected static TBD_TriggerEffect PrepareEffect(notnull TBD_Trigger trigger, notnull TBD_TriggerEffectStruct rawEffect)
	{
		TBD_TriggerEffect effect = new TBD_TriggerEffect();
		effect.m_sRawType = rawEffect.type;
		effect.m_eKind = EffectFromString(rawEffect.type);
		effect.m_bUsable = true;

		effect.m_fX = TBD_TriggerParamsStruct.ABSENT;
		effect.m_fZ = TBD_TriggerParamsStruct.ABSENT;
		effect.m_fHeadingDeg = 0;
		effect.m_iCount = 1;

		if (rawEffect.params)
		{
			effect.m_sAlias = rawEffect.params.alias;
			effect.m_sText = rawEffect.params.text;
			effect.m_sSound = rawEffect.params.sound;
			effect.m_sAudience = rawEffect.params.audience;
			effect.m_sObjectiveId = rawEffect.params.objectiveId;
			effect.m_sState = rawEffect.params.state;
			effect.m_sOwner = rawEffect.params.owner;
			effect.m_sWinner = rawEffect.params.winner;
			effect.m_sVariantId = rawEffect.params.variantId;

			if (rawEffect.params.x != TBD_TriggerParamsStruct.ABSENT)
				effect.m_fX = rawEffect.params.x;
			if (rawEffect.params.z != TBD_TriggerParamsStruct.ABSENT)
				effect.m_fZ = rawEffect.params.z;
			if (rawEffect.params.headingDeg != TBD_TriggerParamsStruct.ABSENT)
				effect.m_fHeadingDeg = rawEffect.params.headingDeg;
			if (rawEffect.params.count != TBD_TriggerParamsStruct.ABSENT_INT)
				effect.m_iCount = rawEffect.params.count;
		}

		ValidateEffect(trigger, effect);
		return effect;
	}

	//------------------------------------------------------------------------------------------------
	//! Schema enum string -> effect kind. `NONE` for absent and for anything outside the closed
	//! `effects[].type` vocabulary.
	protected static TBD_ETriggerEffect EffectFromString(string raw)
	{
		if (raw == FX_SPAWN)
			return TBD_ETriggerEffect.SPAWN;
		if (raw == FX_DELETE)
			return TBD_ETriggerEffect.DELETE;
		if (raw == FX_END_MISSION)
			return TBD_ETriggerEffect.END_MISSION;
		if (raw == FX_SET_OBJECTIVE)
			return TBD_ETriggerEffect.SET_OBJECTIVE;
		if (raw == FX_HINT)
			return TBD_ETriggerEffect.HINT;
		if (raw == FX_PLAY_SOUND)
			return TBD_ETriggerEffect.PLAY_SOUND;
		if (raw == FX_SET_VARIANT)
			return TBD_ETriggerEffect.SET_VARIANT;

		return TBD_ETriggerEffect.NONE;
	}

	//------------------------------------------------------------------------------------------------
	//! Everything that can be decided about an effect BEFORE the round starts is decided here, so a
	//! defect is a line in the boot log an operator reads before an event rather than a trigger that
	//! fires into nothing an hour in.
	//!
	//! `params` is an OPEN object in the schema, so a misspelled param key reaches this build as an
	//! absent one and validation cannot tell the two apart. Each message therefore NAMES the param
	//! it wanted - that name is the only thing that turns "alias is absent" back into "you wrote
	//! `aliass`".
	protected static void ValidateEffect(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		if (effect.m_eKind == TBD_ETriggerEffect.NONE)
		{
			effect.m_bUsable = false;
			if (effect.m_sRawType.IsEmpty())
			{
				effect.m_sInertReason = "its `type` is absent. The schema's vocabulary is spawn | delete | end_mission | set_objective | hint | play_sound | set_variant.";
			}
			else
			{
				effect.m_sInertReason = "its `type` is outside the schema vocabulary spawn | delete | end_mission | set_objective | hint | play_sound | set_variant.";
			}
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.SPAWN)
		{
			if (effect.m_sAlias.IsEmpty())
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "`params.alias` is absent, so there is no prefab to spawn";
				return;
			}

			// No zone AND no coordinates means nowhere to put it. With a zone, the zone's centre is
			// the default and is a place an author can point at on the map.
			if (!trigger.m_Zone && (effect.m_fX == TBD_TriggerParamsStruct.ABSENT || effect.m_fZ == TBD_TriggerParamsStruct.ABSENT))
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "the trigger has no `zoneId` to take a position from and `params.x`/`params.z` are absent, so there is nowhere to spawn";
				return;
			}

			if (effect.m_iCount < 1)
			{
				TBD_Log.Error(CH, string.Format("trigger '%1' spawn effect authors params.count=%2 - using 1",
					trigger.m_sId, effect.m_iCount));
				effect.m_iCount = 1;
			}
			else if (effect.m_iCount > MAX_SPAWN_COUNT)
			{
				TBD_Log.Error(CH, string.Format("trigger '%1' spawn effect authors params.count=%2, above the %3 ceiling - clamped",
					trigger.m_sId, effect.m_iCount, MAX_SPAWN_COUNT));
				effect.m_iCount = MAX_SPAWN_COUNT;
			}

			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.DELETE)
		{
			if (effect.m_sAlias.IsEmpty())
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "`params.alias` is absent, so there is nothing to identify for deletion";
				return;
			}

			// Deliberately refused rather than widened to the world. A `delete` with no area is an
			// instruction to remove every matching object on the terrain, which is never what an
			// author drawing a trigger meant and is not recoverable mid-round.
			if (!trigger.m_Zone)
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "the trigger has no `zoneId`, and a world-wide delete is refused - give the trigger a zone so the deletion has a boundary";
			}

			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.SET_OBJECTIVE)
		{
			if (effect.m_sObjectiveId.IsEmpty())
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "`params.objectiveId` is absent, so there is no objective to set";
				return;
			}

			if (!effect.m_sState.IsEmpty() && effect.m_sState != STATE_COMPLETE && effect.m_sState != STATE_INCOMPLETE)
			{
				TBD_Log.Error(CH, string.Format("trigger '%1' set_objective authors params.state='%2' - the vocabulary is complete | incomplete; using complete",
					trigger.m_sId, effect.m_sState));
				effect.m_sState = STATE_COMPLETE;
			}

			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.HINT)
		{
			if (effect.m_sText.IsEmpty())
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "`params.text` is absent, so the hint would be an empty message";
			}

			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.PLAY_SOUND)
		{
			if (effect.m_sSound.IsEmpty())
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "`params.sound` is absent, so there is no sound event to raise";
			}

			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.SET_VARIANT)
		{
			if (effect.m_sVariantId.IsEmpty())
			{
				effect.m_bUsable = false;
				effect.m_sInertReason = "`params.variantId` is absent, so there is no variant to select";
			}
		}

		// END_MISSION needs nothing: `params.winner` is optional and an unwinnable end is still an
		// end.
	}

	// ============================================================================================
	//  THE TICK
	// ============================================================================================

	//------------------------------------------------------------------------------------------------
	//! One evaluation pass. Called at `TICK_MS` by the heartbeat below.
	//!
	//! @authority server - the mission document, the zone registry and every effect here are
	//! server-owned. The heartbeat never schedules this on a client.
	static void Tick()
	{
		if (!s_bBuilt)
		{
			if (!Build())
				return;

			AnnounceOnce();
		}

		// THE WORLD THIS WAS BUILT FOR. A trigger array is a set of statements about zone ids in one
		// document; the same ids in a different mission are different places. If the loaded mission
		// is not the one this registry was prepared against, the registry is stale - rebuild rather
		// than answer confidently about somewhere else.
		string missionId = CurrentMissionId();
		if (missionId != s_sBuiltForMission)
		{
			TBD_Log.Warn(CH, string.Format("the loaded mission is '%1' but the trigger registry was built for '%2' - rebuilding rather than firing another mission's triggers",
				missionId, s_sBuiltForMission));
			Clear();
			return;
		}

		if (!s_aTriggers || s_aTriggers.Count() == 0)
			return;

		// Evaluate only while the round is LIVE, for the reason `TBD_PlayAreaComponent` gives on the
		// identical guard: SAFE_START is the phase where nothing anyone does can hurt anybody, and a
		// trigger that spawned an ambush or ended the mission while players were still walking to
		// their start line would be a trap. A stage change also drops every countdown, so a dwell
		// cannot resume half-way when LIVE returns.
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm || fm.GetStage() != TBD_EGameStage.LIVE)
		{
			ResetDwell();
			return;
		}

		if (s_iArmedCount == 0)
			return;

		// STOOD DOWN rather than evaluated from no evidence - see SnapshotPlayers on which
		// direction "closed" is and why an empty snapshot is not the same as an empty zone.
		if (!SnapshotPlayers())
		{
			if (!s_bAnnouncedNoSnapshot)
			{
				s_bAnnouncedNoSnapshot = true;
				TBD_Log.Error(CH, "trigger evaluation STOOD DOWN - no PlayerManager or no TBD_SpawnManager on this world, so presence cannot be read. Treating that as 'nobody is anywhere' would make every not_present trigger fire.");
			}

			ResetDwell();
			return;
		}

		s_bAnnouncedNoSnapshot = false;

		foreach (TBD_Trigger trigger : s_aTriggers)
		{
			if (trigger)
				Evaluate(trigger);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! One informational line per world, the moment the registry becomes known-good.
	protected static void AnnounceOnce()
	{
		if (s_bAnnounced)
			return;

		s_bAnnounced = true;

		if (s_iArmedCount == 0)
		{
			TBD_Log.Event(CH, "this mission authors no trigger this build can fire - editorTriggers[] is absent, empty, or every entry is inert");
			return;
		}

		TBD_Log.Kv(CH, "armed", string.Format("triggers=%1 inert=%2 cadence=%3ms detectRadius=%4m",
			s_iArmedCount, s_iInertCount, TICK_MS, DETECT_RADIUS_M));
	}

	//------------------------------------------------------------------------------------------------
	//! Drop every in-progress dwell. Called whenever the round is not LIVE - see `Tick`.
	protected static void ResetDwell()
	{
		if (!s_aTriggers)
			return;

		foreach (TBD_Trigger trigger : s_aTriggers)
		{
			if (!trigger || trigger.m_eState != TBD_ETriggerState.PENDING)
				continue;

			trigger.m_eState = TBD_ETriggerState.ARMED;
			trigger.m_fHeldSeconds = 0;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Rebuild the per-tick view of who is where. Returns false when the view could not be built at
	//! all, which is NOT the same as "nobody is anywhere".
	//!
	//! A player is in the snapshot only when they have a body, that body is alive, and their life is
	//! not already spent. The exclusions are the same three `TBD_PlayAreaComponent.EvaluatePlayer`
	//! applies and they matter for the same reason: a dead player's controlled entity is their
	//! SPECTATOR STREAMING HOST, which is somewhere over the map and is emphatically not a soldier
	//! standing in the zone. Counting it would let a corpse hold an objective area open.
	//!
	//! == WHY THIS FAILS CLOSED, AND WHICH DIRECTION "CLOSED" IS =================================
	//! Without a `TBD_SpawnManager` there is no way to tell a spent life from a living player, and
	//! this must NOT resolve to an empty snapshot: `not_present` reads an empty snapshot as TRUE.
	//! A mission whose `not_present` trigger fires `end_mission` would then end the round the moment
	//! the runtime lost sight of the players, and it would look exactly like the authored condition
	//! being met. So "cannot see the players" returns FALSE here and `Tick` stands the whole
	//! evaluation down, rather than answering a presence question from no evidence.
	//!
	//! This is the same asymmetry `TBD_PlayAreaComponent` was fixed for at T-181.30: the sibling
	//! failing closed while this one failed open was the bug, not the guard.
	protected static bool SnapshotPlayers()
	{
		s_aPlayerIds = new array<int>();
		s_aPlayerX = new array<float>();
		s_aPlayerZ = new array<float>();
		s_aPlayerFaction = new array<string>();

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return false;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return false;

		array<int> connected = new array<int>();
		players.GetPlayers(connected);

		foreach (int playerId : connected)
		{
			if (spawn.IsPlayerDead(playerId))
				continue;

			IEntity body = players.GetPlayerControlledEntity(playerId);
			if (!body || IsBodyDead(body))
				continue;

			string factionKey;
			TBD_MissionSlotStruct slot = spawn.GetAssignedSlot(playerId);
			if (slot)
				factionKey = slot.faction;

			vector origin = body.GetOrigin();
			s_aPlayerIds.Insert(playerId);
			s_aPlayerX.Insert(origin[0]);
			s_aPlayerZ.Insert(origin[2]);
			s_aPlayerFaction.Insert(factionKey);
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Is this body a corpse? Asked separately from `TBD_SpawnManager.IsPlayerDead` because the two
	//! answer different questions - one is "has this identity spent its life", the other is "is the
	//! thing standing here alive right now" - and either being true means it is not present.
	protected static bool IsBodyDead(notnull IEntity body)
	{
		SCR_ChimeraCharacter character = SCR_ChimeraCharacter.Cast(body);
		if (!character)
			return false;

		CharacterControllerComponent controller = character.GetCharacterController();
		if (!controller)
			return false;

		return controller.IsDead();
	}

	//------------------------------------------------------------------------------------------------
	//! Advance one trigger's state machine by one tick.
	//!
	//! -- Re-arming requires the condition to FALL first --------------------------------------
	//! A `repeat` trigger returns to ARMED only once its condition stops holding. Without that, a
	//! `present` trigger with `repeat: true` and no timeout would fire every single second for as
	//! long as anybody stood in the zone. Eden's repeatable flag behaves the same way, and it is the
	//! only reading under which "repeat" means "again", not "continuously".
	protected static void Evaluate(notnull TBD_Trigger trigger)
	{
		if (trigger.m_eState == TBD_ETriggerState.INERT)
			return;

		if (!IsVariantActive(trigger))
		{
			// A trigger whose variant is not selected is not merely skipped - its dwell is dropped,
			// so re-selecting the variant does not resume a countdown that ran while it was off.
			if (trigger.m_eState == TBD_ETriggerState.PENDING)
			{
				trigger.m_eState = TBD_ETriggerState.ARMED;
				trigger.m_fHeldSeconds = 0;
			}
			return;
		}

		// A one-shot that has fired is finished. Answered BEFORE the condition is evaluated, so a
		// spent trigger stops costing a zone scan (or an objective walk) once a second for the rest
		// of the round.
		if (trigger.m_eState == TBD_ETriggerState.FIRED && !trigger.m_bRepeat)
			return;

		bool holds = ConditionHolds(trigger);

		if (trigger.m_eState == TBD_ETriggerState.FIRED)
		{
			if (!holds)
			{
				trigger.m_eState = TBD_ETriggerState.ARMED;
				trigger.m_fHeldSeconds = 0;
				TBD_Log.Kv(CH, "rearmed", string.Format("id=%1 fires=%2", trigger.m_sId, trigger.m_iFireCount));
			}

			return;
		}

		if (!holds)
		{
			if (trigger.m_eState == TBD_ETriggerState.PENDING)
			{
				TBD_Log.Kv(CH, "lapsed", string.Format("id=%1 held=%2s of %3s",
					trigger.m_sId, trigger.m_fHeldSeconds, trigger.m_fTimeoutSeconds));
			}

			trigger.m_eState = TBD_ETriggerState.ARMED;
			trigger.m_fHeldSeconds = 0;
			return;
		}

		if (trigger.m_eState == TBD_ETriggerState.ARMED)
		{
			trigger.m_eState = TBD_ETriggerState.PENDING;
			trigger.m_fHeldSeconds = 0;

			TBD_Log.Kv(CH, "holding", string.Format("id=%1 condition=%2 owner='%3' timeout=%4s",
				trigger.m_sId,
				typename.EnumToString(TBD_ETriggerCondition, trigger.m_eCondition),
				trigger.m_sOwnerSide,
				trigger.m_fTimeoutSeconds));
		}
		else
		{
			trigger.m_fHeldSeconds += TICK_SECONDS;
		}

		if (trigger.m_fHeldSeconds < trigger.m_fTimeoutSeconds)
			return;

		Fire(trigger);
	}

	//------------------------------------------------------------------------------------------------
	//! Is this trigger's variant selected? See the class header for the two-mode contract.
	protected static bool IsVariantActive(notnull TBD_Trigger trigger)
	{
		if (trigger.m_sVariantId.IsEmpty())
			return true;

		if (!s_bVariantSelectionInForce)
			return true;

		if (!s_aSelectedVariants)
			return false;

		return s_aSelectedVariants.Find(trigger.m_sVariantId) != -1;
	}

	// ============================================================================================
	//  CONDITIONS
	// ============================================================================================

	//------------------------------------------------------------------------------------------------
	//! Does this trigger's condition hold RIGHT NOW, against the current tick's snapshot?
	protected static bool ConditionHolds(notnull TBD_Trigger trigger)
	{
		if (trigger.m_eCondition == TBD_ETriggerCondition.TIMER)
			return true;

		if (trigger.m_eCondition == TBD_ETriggerCondition.OBJECTIVE_COMPLETE)
			return ObjectiveComplete(trigger);

		if (trigger.m_eCondition == TBD_ETriggerCondition.PRESENT)
			return CountInside(trigger.m_Zone, trigger.m_sOwnerSide, true) > 0;

		if (trigger.m_eCondition == TBD_ETriggerCondition.NOT_PRESENT)
			return CountInside(trigger.m_Zone, trigger.m_sOwnerSide, true) == 0;

		if (trigger.m_eCondition == TBD_ETriggerCondition.SEIZED_BY)
		{
			if (CountInside(trigger.m_Zone, trigger.m_sOwnerSide, true) == 0)
				return false;

			return CountInside(trigger.m_Zone, trigger.m_sOwnerSide, false) == 0;
		}

		if (trigger.m_eCondition == TBD_ETriggerCondition.DETECTED_BY)
			return Detected(trigger);

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! How many snapshot players are inside `zone` and on the side selected by `factionKey`.
	//!
	//! `zone` null means the whole world - that is the schema's "document-wide condition", and it is
	//! what makes `not_present` able to express "this side has nobody left standing".
	//!
	//! `matching` true counts players whose faction EQUALS `factionKey` (an empty `factionKey`
	//! matches everybody, which is what `present` with no `ownerSide` should mean). `matching` false
	//! counts the players on some OTHER named side - a player with no resolved slot has no side and
	//! is counted by neither, so an unassigned body cannot contest a seizure. That is the same
	//! direction `TBD_ZoneRegistry.AppliesToFaction` takes and it is stated here because the
	//! alternative - treating "unknown" as hostile - would make `seized_by` unusable on any server
	//! with an admin walking around.
	protected static int CountInside(TBD_Zone zone, string factionKey, bool matching)
	{
		if (!s_aPlayerIds)
			return 0;

		int hits = 0;
		int count = s_aPlayerIds.Count();
		for (int i = 0; i < count; i++)
		{
			string playerFaction = s_aPlayerFaction[i];

			if (matching)
			{
				if (!factionKey.IsEmpty() && playerFaction != factionKey)
					continue;
			}
			else
			{
				if (playerFaction.IsEmpty() || playerFaction == factionKey)
					continue;
			}

			if (zone && !zone.Contains(s_aPlayerX[i], s_aPlayerZ[i]))
				continue;

			hits++;
		}

		return hits;
	}

	//------------------------------------------------------------------------------------------------
	//! `detected_by`: somebody who is not of `ownerSide` is inside the zone, and somebody of
	//! `ownerSide` is within `DETECT_RADIUS_M` of them.
	//!
	//! The friendly does NOT have to be inside the zone. That is the point of the radius: a section
	//! overwatching a road from 200 m away detects the convoy that drives into the area, and a
	//! condition that required them to be standing in it would only ever fire once contact was
	//! already made. Read `DETECT_RADIUS_M` for what this does not model.
	protected static bool Detected(notnull TBD_Trigger trigger)
	{
		if (!s_aPlayerIds || !trigger.m_Zone)
			return false;

		float radiusSq = DETECT_RADIUS_M * DETECT_RADIUS_M;
		int count = s_aPlayerIds.Count();

		for (int i = 0; i < count; i++)
		{
			string intruderFaction = s_aPlayerFaction[i];
			if (intruderFaction.IsEmpty() || intruderFaction == trigger.m_sOwnerSide)
				continue;

			if (!trigger.m_Zone.Contains(s_aPlayerX[i], s_aPlayerZ[i]))
				continue;

			for (int j = 0; j < count; j++)
			{
				if (s_aPlayerFaction[j] != trigger.m_sOwnerSide)
					continue;

				float distSq = TBD_ZoneGeometry.DistanceSqXZ(s_aPlayerX[j], s_aPlayerZ[j],
					s_aPlayerX[i], s_aPlayerZ[i]);

				if (distSq <= radiusSq)
					return true;
			}
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! `objective_complete`: with a `zoneId`, that one objective; without one, EVERY usable
	//! objective in the mission.
	//!
	//! The all-objectives form needs at least one usable objective, so a mission with none never
	//! satisfies "everything is done by doing nothing". Inert objectives are excluded from both
	//! sides of that test, exactly as `TBD_ObjectiveRegistry.AreAllObjectivesCaptured` excludes
	//! them: one broken objective must not make the mission unwinnable.
	protected static bool ObjectiveComplete(notnull TBD_Trigger trigger)
	{
		array<ref TBD_Objective> objectiveList = TBD_ObjectiveRegistry.GetAll();
		if (!objectiveList)
			return false;

		if (!trigger.m_sZoneId.IsEmpty())
		{
			foreach (TBD_Objective one : objectiveList)
			{
				if (one && one.m_sId == trigger.m_sZoneId)
					return one.m_bComplete;
			}

			return false;
		}

		int usable = 0;
		foreach (TBD_Objective objective : objectiveList)
		{
			if (!objective || !objective.m_bUsable)
				continue;

			usable++;
			if (!objective.m_bComplete)
				return false;
		}

		return usable > 0;
	}

	// ============================================================================================
	//  EFFECTS
	// ============================================================================================

	//------------------------------------------------------------------------------------------------
	//! Run every usable effect on this trigger, in authored order, then latch it FIRED.
	//!
	//! Order is the document's order and is honoured deliberately: an author who writes a `hint`
	//! before an `end_mission` wants the message delivered before the round stops.
	protected static void Fire(notnull TBD_Trigger trigger)
	{
		trigger.m_eState = TBD_ETriggerState.FIRED;
		trigger.m_iFireCount++;
		trigger.m_fHeldSeconds = 0;

		TBD_Log.Banner(CH, string.Format("TRIGGER FIRED: %1 (condition=%2 owner='%3' dwell=%4s fires=%5)",
			trigger.m_sId,
			typename.EnumToString(TBD_ETriggerCondition, trigger.m_eCondition),
			trigger.m_sOwnerSide,
			trigger.m_fTimeoutSeconds,
			trigger.m_iFireCount), false);

		if (!trigger.m_aEffects)
			return;

		foreach (int index, TBD_TriggerEffect effect : trigger.m_aEffects)
		{
			if (!effect || !effect.m_bUsable)
				continue;

			RunEffect(trigger, effect, index);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Dispatch one effect. Every branch logs what it did, by trigger id and effect index, because
	//! nothing else in the round records that a trigger acted.
	protected static void RunEffect(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect, int index)
	{
		if (effect.m_eKind == TBD_ETriggerEffect.HINT)
		{
			EffectHint(trigger, effect);
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.PLAY_SOUND)
		{
			EffectPlaySound(trigger, effect);
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.SPAWN)
		{
			EffectSpawn(trigger, effect);
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.DELETE)
		{
			EffectDelete(trigger, effect);
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.SET_OBJECTIVE)
		{
			EffectSetObjective(trigger, effect);
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.SET_VARIANT)
		{
			EffectSetVariant(trigger, effect);
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.END_MISSION)
		{
			EffectEndMission(trigger, effect);
			return;
		}

		// Unreachable: `ValidateEffect` marks an unknown kind unusable and `Fire` skips those.
		// Kept so a future enum entry that nobody wired shows up as a log line and not as silence.
		TBD_Log.Error(CH, string.Format("trigger '%1' effects[%2] has kind %3 with no runner - nothing happened",
			trigger.m_sId, index, typename.EnumToString(TBD_ETriggerEffect, effect.m_eKind)));
	}

	//------------------------------------------------------------------------------------------------
	//! `hint` - one private chat line per player in the audience.
	//!
	//! Chat rather than a HUD widget for the reason `TBD_PlayAreaComponent.Tell` records: a new
	//! `.layout` is INVISIBLE to the engine until Workbench rewrites `resourceDatabase.rdb`, so a
	//! widget written on this lane could not open. The text is also logged server-side, so an
	//! operator can reconstruct what players were told even if delivery failed.
	protected static void EffectHint(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> audience = ResolveAudience(trigger, effect.m_sAudience);
		string msg = "TBD: ";
		msg += effect.m_sText;

		int sent = 0;
		foreach (int playerId : audience)
		{
			if (Tell(players, playerId, msg))
				sent++;
		}

		TBD_Log.Kv(CH, "hint", string.Format("id=%1 audience='%2' players=%3 sent=%4 text='%5'",
			trigger.m_sId, effect.m_sAudience, audience.Count(), sent, effect.m_sText));
	}

	//------------------------------------------------------------------------------------------------
	//! Server -> one client, over the channel this codebase already uses for per-player replies
	//! (`TBD_AdminCommands.Reply`, `TBD_PlayAreaComponent.Tell`). Returns whether the message could
	//! be handed to a chat component at all, so the caller can report delivery honestly rather than
	//! claiming every player in the audience heard it.
	protected static bool Tell(notnull PlayerManager players, int playerId, string text)
	{
		PlayerController pc = players.GetPlayerController(playerId);
		if (!pc)
			return false;

		SCR_ChatComponent chat = SCR_ChatComponent.Cast(pc.FindComponent(SCR_ChatComponent));
		if (!chat)
			return false;

		chat.SendPrivateMessage(text, playerId);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! `play_sound` - raise a 2D UI sound event on each client in the audience.
	//!
	//! The server has no audio device, so this cannot be played where it is decided: the sound event
	//! name is pushed to each client's own `SCR_PlayerController` over an `RplRcver.Owner` RPC and
	//! raised there. That is the transport every per-player payload in this addon already uses
	//! (markers, briefing, radio, lobby) and it keeps the decision server-authoritative while the
	//! playback happens on the only machine that can hear it.
	//!
	//! `params.sound` is an `SCR_SoundEvent` name (for example `SOUND_HINT`). An unknown name is not
	//! a script error - `SCR_UISoundEntity.SoundEvent` simply raises nothing - so the authored value
	//! is logged here, where an operator can compare it against what players reported hearing.
	protected static void EffectPlaySound(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> audience = ResolveAudience(trigger, effect.m_sAudience);

		int sent = 0;
		foreach (int playerId : audience)
		{
			SCR_PlayerController controller = SCR_PlayerController.Cast(players.GetPlayerController(playerId));
			if (!controller)
				continue;

			controller.TBD_PushTriggerSound(effect.m_sSound);
			sent++;
		}

		TBD_Log.Kv(CH, "playSound", string.Format("id=%1 sound='%2' audience='%3' players=%4 sent=%5",
			trigger.m_sId, effect.m_sSound, effect.m_sAudience, audience.Count(), sent));
	}

	//------------------------------------------------------------------------------------------------
	//! Which connected players an audience string selects.
	//!
	//! Built from the LIVE player list rather than the tick snapshot: the snapshot deliberately drops
	//! dead players, and a dead player still has a chat window and should still be told the mission
	//! ended. Presence decides whether a trigger fires; it does not decide who is allowed to hear
	//! about it.
	protected static array<int> ResolveAudience(notnull TBD_Trigger trigger, string audience)
	{
		array<int> selected = new array<int>();

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return selected;

		array<int> connected = new array<int>();
		players.GetPlayers(connected);

		// "all" and absent both mean everybody, and everybody is the majority case - answer it
		// without resolving a single slot.
		if (audience.IsEmpty() || audience == AUD_ALL)
		{
			foreach (int everyone : connected)
			{
				selected.Insert(everyone);
			}

			return selected;
		}

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
		{
			TBD_Log.Warn(CH, string.Format("trigger '%1' wants audience '%2' but there is no TBD_SpawnManager to resolve sides - nobody is addressed",
				trigger.m_sId, audience));
			return selected;
		}

		// `owner` and `enemy` are relative to the activation's side, so they are only answerable
		// when one was authored. Refuse rather than fall back to everybody: a message meant for one
		// side that goes to both is a side-discipline failure, not a cosmetic one.
		string wanted = audience;
		bool invert = false;
		if (audience == AUD_OWNER || audience == AUD_ENEMY)
		{
			if (trigger.m_sOwnerSide.IsEmpty())
			{
				TBD_Log.Warn(CH, string.Format("trigger '%1' wants audience '%2' but authors no activation.ownerSide - nobody is addressed",
					trigger.m_sId, audience));
				return selected;
			}

			wanted = trigger.m_sOwnerSide;
			invert = audience == AUD_ENEMY;
		}

		foreach (int playerId : connected)
		{
			string factionKey;
			TBD_MissionSlotStruct slot = spawn.GetAssignedSlot(playerId);
			if (slot)
				factionKey = slot.faction;

			if (invert)
			{
				if (!factionKey.IsEmpty() && factionKey != wanted)
					selected.Insert(playerId);
			}
			else if (factionKey == wanted)
			{
				selected.Insert(playerId);
			}
		}

		return selected;
	}

	//------------------------------------------------------------------------------------------------
	//! `spawn` - place `count` copies of a registry alias.
	//!
	//! The alias resolution, `Resource.Load`, ground snap and transform are the shape
	//! `TBD_MissionLoader.SpawnMissionEntities` already uses for authored `entities[]` rows, so a
	//! trigger-spawned prefab and a mission-placed one arrive identically - which is what lets an
	//! `objective_destroy` target spawned by a trigger be found by the same AABB query that finds an
	//! authored one.
	//!
	//! Every copy lands on the SAME position. Scattering them would need a radius key that
	//! `activation` and this params vocabulary do not carry, and a stack of prefabs at one point is
	//! at least honest about what was asked for.
	protected static void EffectSpawn(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		bool ok;
		ResourceName prefab = TBD_Registry.Resolve(effect.m_sAlias, ok);
		if (!ok || prefab.IsEmpty())
		{
			TBD_Log.Error(CH, string.Format("trigger '%1' spawn alias='%2' is not in the registry - nothing spawned",
				trigger.m_sId, effect.m_sAlias));
			return;
		}

		Resource resource = Resource.Load(prefab);
		if (!resource || !resource.IsValid())
		{
			TBD_Log.Error(CH, string.Format("trigger '%1' spawn alias='%2' prefab=%3 failed Resource.Load - nothing spawned",
				trigger.m_sId, effect.m_sAlias, prefab));
			return;
		}

		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return;

		float px = effect.m_fX;
		float pz = effect.m_fZ;
		if (px == TBD_TriggerParamsStruct.ABSENT || pz == TBD_TriggerParamsStruct.ABSENT)
		{
			// Guaranteed by ValidateEffect: an effect with no coordinates and no zone is unusable and
			// never reaches here.
			px = ZoneCentreX(trigger.m_Zone);
			pz = ZoneCentreZ(trigger.m_Zone);
		}

		float surfaceY = world.GetSurfaceY(px, pz);
		vector pos = Vector(px, surfaceY, pz);

		float yawRad = effect.m_fHeadingDeg * Math.DEG2RAD;

		int spawned = 0;
		for (int i = 0; i < effect.m_iCount; i++)
		{
			EntitySpawnParams params = new EntitySpawnParams();
			params.TransformMode = ETransformMode.WORLD;
			Math3D.MatrixIdentity4(params.Transform);
			params.Transform[3] = pos;
			params.Transform[0] = Vector(Math.Cos(yawRad), 0, Math.Sin(yawRad));
			params.Transform[2] = Vector(-Math.Sin(yawRad), 0, Math.Cos(yawRad));

			IEntity body = GetGame().SpawnEntityPrefab(resource, world, params);
			if (body)
				spawned++;
		}

		if (spawned < effect.m_iCount)
		{
			TBD_Log.Error(CH, string.Format("trigger '%1' spawn alias='%2' produced %3 of %4 requested - SpawnEntityPrefab refused the rest",
				trigger.m_sId, effect.m_sAlias, spawned, effect.m_iCount));
		}

		TBD_Log.Kv(CH, "spawn", string.Format("id=%1 alias='%2' at=%3 heading=%4 count=%5 spawned=%6",
			trigger.m_sId, effect.m_sAlias, pos.ToString(), effect.m_fHeadingDeg, effect.m_iCount, spawned));
	}

	//------------------------------------------------------------------------------------------------
	//! `delete` - remove every entity of the aliased prefab that is inside the trigger's zone.
	//!
	//! Collected first and deleted after, deliberately: `QueryEntitiesByAABB` is walking the world's
	//! own structures while the callback runs, and deleting from inside it would mutate the thing
	//! being iterated. The same two-phase shape `TBD_PlayAreaComponent.PruneDeparted` uses on its
	//! map, for the same reason.
	//!
	//! Children go with the parent (`SCR_EntityHelper.DeleteEntityAndChildren`): a composition
	//! deleted without its slotted parts leaves the parts floating.
	protected static void EffectDelete(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		bool ok;
		ResourceName prefab = TBD_Registry.Resolve(effect.m_sAlias, ok);
		if (!ok || prefab.IsEmpty())
		{
			TBD_Log.Error(CH, string.Format("trigger '%1' delete alias='%2' is not in the registry - nothing deleted",
				trigger.m_sId, effect.m_sAlias));
			return;
		}

		BaseWorld world = GetGame().GetWorld();
		if (!world || !trigger.m_Zone)
			return;

		s_QueryResource = prefab;
		s_QueryZone = trigger.m_Zone;
		s_aQueryHits = new array<IEntity>();

		vector mins = Vector(trigger.m_Zone.m_fMinX, -QUERY_Y_EXTENT_M, trigger.m_Zone.m_fMinZ);
		vector maxs = Vector(trigger.m_Zone.m_fMaxX, QUERY_Y_EXTENT_M, trigger.m_Zone.m_fMaxZ);
		world.QueryEntitiesByAABB(mins, maxs, OnDeleteQueryEntity);

		array<IEntity> hits = s_aQueryHits;
		s_QueryZone = null;
		s_QueryResource = string.Empty;
		s_aQueryHits = null;

		int deleted = 0;
		foreach (IEntity hit : hits)
		{
			if (!hit)
				continue;

			SCR_EntityHelper.DeleteEntityAndChildren(hit);
			deleted++;
		}

		TBD_Log.Kv(CH, "delete", string.Format("id=%1 alias='%2' zone=%3 matched=%4 deleted=%5",
			trigger.m_sId, effect.m_sAlias, trigger.m_Zone.LogKey(), hits.Count(), deleted));
	}

	//------------------------------------------------------------------------------------------------
	//! World-query callback for `delete`. Static because the query API takes a plain function; the
	//! scratch it writes into is documented on `s_QueryResource`.
	//!
	//! The AABB is a box and the zone may be a polygon, so the zone is asked again per hit - a
	//! prefab inside the box but outside the drawn shape is NOT deleted.
	protected static bool OnDeleteQueryEntity(IEntity entity)
	{
		if (!entity || !s_QueryZone || !s_aQueryHits)
			return true;

		EntityPrefabData prefabData = entity.GetPrefabData();
		if (!prefabData)
			return true;

		if (prefabData.GetPrefabName() != s_QueryResource)
			return true;

		vector origin = entity.GetOrigin();
		if (!s_QueryZone.Contains(origin[0], origin[2]))
			return true;

		s_aQueryHits.Insert(entity);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! `set_objective` - force one objective's completion state, and optionally its owner.
	//!
	//! `m_bComplete = true` is exactly what `TBD_ObjectiveRegistry.EvaluateDestroy` and
	//! `TBD_ObjectivesComponent` do when an objective is finished the normal way, so a trigger-driven
	//! completion is indistinguishable downstream: `EvaluateEndTriggers` reads the same flag, and a
	//! mission that ends on `all_objectives_captured` ends on this too.
	//!
	//! `params.owner` writes the capture owner, because "this objective is complete" and "this side
	//! holds it" are separate facts and `AreAllObjectivesCaptured` needs the second one - completing
	//! a capture objective without an owner would leave the round unwinnable by that route.
	protected static void EffectSetObjective(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		array<ref TBD_Objective> objectiveList = TBD_ObjectiveRegistry.GetAll();
		if (!objectiveList)
		{
			TBD_Log.Warn(CH, string.Format("trigger '%1' set_objective '%2' but the objective registry has not been built - nothing changed",
				trigger.m_sId, effect.m_sObjectiveId));
			return;
		}

		TBD_Objective target;
		foreach (TBD_Objective objective : objectiveList)
		{
			if (objective && objective.m_sId == effect.m_sObjectiveId)
			{
				target = objective;
				break;
			}
		}

		if (!target)
		{
			TBD_Log.Warn(CH, string.Format("trigger '%1' set_objective '%2' names no objective in this mission - nothing changed",
				trigger.m_sId, effect.m_sObjectiveId));
			return;
		}

		bool complete = effect.m_sState != STATE_INCOMPLETE;
		target.m_bComplete = complete;

		if (!effect.m_sOwner.IsEmpty())
			target.m_sOwner = effect.m_sOwner;

		TBD_Log.Kv(CH, "setObjective", string.Format("id=%1 objective=%2 complete=%3 owner='%4'",
			trigger.m_sId, target.m_sId, complete, target.m_sOwner));
	}

	//------------------------------------------------------------------------------------------------
	//! `set_variant` - select a variant at runtime. See the class header for the two-mode contract.
	protected static void EffectSetVariant(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		if (!s_aSelectedVariants)
			s_aSelectedVariants = new array<string>();

		if (!s_bVariantSelectionInForce)
		{
			s_bVariantSelectionInForce = true;
			TBD_Log.Event(CH, "variant selection is now IN FORCE - from here a trigger with a `variantId` runs only while that variant is selected; triggers with no `variantId` are unaffected");
		}

		if (s_aSelectedVariants.Find(effect.m_sVariantId) == -1)
			s_aSelectedVariants.Insert(effect.m_sVariantId);

		TBD_Log.Kv(CH, "setVariant", string.Format("id=%1 variant='%2' selected=%3",
			trigger.m_sId, effect.m_sVariantId, s_aSelectedVariants.Count()));
	}

	//------------------------------------------------------------------------------------------------
	//! `end_mission` - take the round to END through the stage machine's own door.
	//!
	//! `TBD_FrameworkManager.SetStage` is the single authority on the stage, exactly as
	//! `TBD_ObjectiveRegistry`'s header says: the objective registry deliberately does not end the
	//! round itself, and neither does this. Routing through `SetStage` means a trigger-driven end
	//! lifts safestart, notifies the spawn manager and reaches the local stage UI identically to
	//! every other end.
	//!
	//! `params.winner` is REPORTED, not stored. Nothing in this build persists a winner - the
	//! attrition and objective paths both `Print` theirs into `[TBD][Win]` and stop there - so this
	//! writes the same line in the same shape rather than inventing a winner sink no reader consumes.
	protected static void EffectEndMission(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
		{
			TBD_Log.Error(CH, string.Format("trigger '%1' end_mission but there is no TBD_FrameworkManager on this world - the round was NOT ended",
				trigger.m_sId));
			return;
		}

		Print(string.Format("[TBD][Win] trigger:%1 - winner=%2", trigger.m_sId, effect.m_sWinner));

		fm.SetStage(TBD_EGameStage.END);

		string refusal = fm.GetLastStageRefusal();
		if (!refusal.IsEmpty())
		{
			TBD_Log.Error(CH, string.Format("trigger '%1' end_mission was REFUSED by the stage machine: %2",
				trigger.m_sId, refusal));
			return;
		}

		TBD_Log.Kv(CH, "endMission", string.Format("id=%1 winner='%2'", trigger.m_sId, effect.m_sWinner));
	}

	//------------------------------------------------------------------------------------------------
	//! A zone's centre in X. For a circle that is the authored centre; for a polygon it is the
	//! midpoint of the precomputed bounds, which is inside any convex ring and is at least always
	//! within the drawn extent. Returns 0 for no zone, which callers never reach - `ValidateEffect`
	//! refuses a zone-less spawn that authors no coordinates.
	protected static float ZoneCentreX(TBD_Zone zone)
	{
		if (!zone)
			return 0;

		if (zone.m_eShape == TBD_EZoneShapeKind.CIRCLE)
			return zone.m_fCx;

		return (zone.m_fMinX + zone.m_fMaxX) * 0.5;
	}

	//------------------------------------------------------------------------------------------------
	//! A zone's centre in Z. See `ZoneCentreX`.
	protected static float ZoneCentreZ(TBD_Zone zone)
	{
		if (!zone)
			return 0;

		if (zone.m_eShape == TBD_EZoneShapeKind.CIRCLE)
			return zone.m_fCz;

		return (zone.m_fMinZ + zone.m_fMaxZ) * 0.5;
	}
}

//------------------------------------------------------------------------------------------------
//! T-676 - the client half of the `play_sound` effect.
//!
//! This is the SEVENTH `modded class SCR_PlayerController` block in the addon (mission browser,
//! briefing, lobby, markers, radio, spectator host, and now triggers). It keeps the same minimal
//! exposure the sixth one documented: it overrides NO vanilla method, adds NO `modded enum
//! ChimeraMenuPreset` entry, and every symbol it introduces is `TBD_`-prefixed. What this lane
//! still cannot prove is runtime coexistence of the blocks - that is the open question already
//! filed as T-181.25 and this adds one more block to it, which is stated rather than hidden.
modded class SCR_PlayerController
{
	//------------------------------------------------------------------------------------------------
	//! @authority server - called from `TBD_TriggerRuntime.EffectPlaySound` on the machine that owns
	//! the mission document.
	//!
	//! On a listen host the authority IS the player, and an `RplRcver.Owner` RPC is not delivered to
	//! the machine that sent it, so the local case is played in place rather than RPCed to nowhere.
	//! That asymmetry is the recorded shape in `TBD_RadioController` and it is why this is not a
	//! bare `Rpc()`.
	void TBD_PushTriggerSound(string soundEvent)
	{
		if (soundEvent.IsEmpty())
			return;

		if (GetGame().GetPlayerController() == this)
		{
			SCR_UISoundEntity.SoundEvent(soundEvent);
			return;
		}

		Rpc(TBD_RpcDo_TriggerSound, soundEvent);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority owner - executes on the addressed client and no other (RplRcver.Owner).
	//!
	//! A 2D UI sound, not a positioned one: this is a cue attached to a mission event, not a noise
	//! made by an object in the world, and a positioned source would need an entity to hang it on
	//! that the trigger does not have.
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_TriggerSound(string soundEvent)
	{
		SCR_UISoundEntity.SoundEvent(soundEvent);
	}
}

//------------------------------------------------------------------------------------------------
//! T-676 - the trigger runtime's heartbeat. See `TBD_TriggerRuntime`'s header for why the tick
//! hangs off the game mode class rather than a game-mode component.
modded class SCR_BaseGameMode
{
	//------------------------------------------------------------------------------------------------
	//! @authority server - triggers are evaluated where the mission document lives.
	//!
	//! Statics outlive a world inside one process (`SelectMissionByNumber` restarts the scenario
	//! in-process), so the registry is cleared HERE, at the start of each world, rather than in a
	//! teardown hook this class does not have. Clearing on the way in is strictly stronger: it does
	//! not depend on the previous world having shut down tidily.
	protected override void OnGameStart()
	{
		super.OnGameStart();

		TBD_TriggerRuntime.Clear();

		// Clients hold no mission document, so a client-side evaluation would have no trigger to
		// evaluate and no authority to act on one.
		if (RplSession.Mode() == RplMode.Client)
			return;

		// The fence that keeps this out of vanilla scenarios that merely have the mod loaded. It is
		// the published test (`TBD_FrameworkManager.IsFrameworkWorld`) every vanilla-touching modded
		// block in this addon already asks, and it resolves off the live game mode's components,
		// which exist from construction - long before a game mode starts.
		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		GetGame().GetCallqueue().CallLater(TBD_TriggerTick, TBD_TriggerRuntime.TICK_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	//! One evaluation, then re-arm.
	//!
	//! -- Why one-shot and self-re-arming rather than a repeating CallLater -------------------
	//! `ScriptCallQueue.Remove` cancels BY FUNCTION, and this class has no teardown hook to call it
	//! from - `OnDelete(IEntity)` is a COMPONENT lifecycle method and a game mode is an entity. A
	//! repeating timer would therefore survive a world teardown and fire forever against a dead
	//! game mode. A one-shot that re-arms itself only while it is still the LIVE game mode's timer
	//! stops on its own the moment the world it belongs to is replaced: the stale callback runs
	//! exactly once more, sees that it is not the current game mode, and does not re-arm. No
	//! cancellation is needed and none can be forgotten.
	void TBD_TriggerTick()
	{
		if (GetGame().GetGameMode() != this)
			return;

		TBD_TriggerRuntime.Tick();

		GetGame().GetCallqueue().CallLater(TBD_TriggerTick, TBD_TriggerRuntime.TICK_MS, false);
	}
}
