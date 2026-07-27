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

		TBD_Log.Kv(CH, "objectiveRules", string.Format("id=%1 targetAlias='%2' targetCount=%3 points=%4",
			objective.m_sId,
			objective.m_sTargetAlias,
			objective.m_iTargetCount,
			objective.m_fPoints));
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
		if (!s_QueryZone.Contains(origin[0], origin[2]))
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
		}

		return lines;
	}
}
