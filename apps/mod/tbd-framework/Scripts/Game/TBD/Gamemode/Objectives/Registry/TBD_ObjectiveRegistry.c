/**
 * @file TBD_ObjectiveRegistry.c
 * @brief Builds the prepared objectives of the loaded mission once per world and answers its
 * objective end triggers.
 *
 * Role: turns every prepared `objective_*` zone into a `TBD_Objective` with its resolved rules
 * and typed row, and is the one authority on whether `all_objectives_captured`,
 * `objective_destroyed` or `hold_expired` has fired.  Position: `TBD_ObjectivesComponent` builds
 * it on the server; reads `TBD_ZoneRegistry`, `TBD_ObjectiveRulesReader`,
 * `TBD_ObjectiveEntityReader` and `TBD_ZoneVolume`; read by `TBD_FrameworkManager` (end
 * triggers), `TBD_MissionValidator` (vocabulary), `TBD_TriggerRuntime` and the objective runtime.
 * State: the prepared objectives and per-kind usable counts of the current world; statics
 * outlive a world, so `TBD_ObjectivesComponent.OnDelete` calls `Clear`.  Invariants: never builds
 * on a client (no mission document) and never caches an empty build; containment is never tested
 * here; the zone wins over a disagreeing typed row; an end trigger fires only when
 * `winConditions.endOn` declares it; `lock` and `autoLose` are reported and never enforced.
 */

//! Prepared objectives of the loaded mission and the objective end-trigger authority.
class TBD_ObjectiveRegistry
{
	static const string CH = "Obj"; //!< log channel `[TBD][Obj]`

	static const string TYPE_CAPTURE    = "objective_capture"; //!< `zones[].type` of a capture objective
	static const string TYPE_DESTROY    = "objective_destroy"; //!< `zones[].type` of a destroy objective
	static const string TYPE_HOLD_UNTIL = "objective_hold_until"; //!< `zones[].type` of a hold objective

	static const string TRIGGER_ALL_CAPTURED = "all_objectives_captured"; //!< `winConditions.endOn`: every usable capture owned by one side
	static const string TRIGGER_DESTROYED    = "objective_destroyed"; //!< `winConditions.endOn`: any usable destroy objective completed
	static const string TRIGGER_HOLD_EXPIRED = "hold_expired"; //!< `winConditions.endOn`: any usable hold objective ran its clock out

	protected static ref array<ref TBD_Objective> s_aObjectives; //!< every prepared objective, inert ones included; null until `Build`
	protected static bool s_bBuilt; //!< `Build` completed since the last `Clear`
	protected static int s_iCaptureCount; //!< usable capture objectives
	protected static int s_iDestroyCount; //!< usable destroy objectives
	protected static int s_iHoldCount; //!< usable hold objectives

	//! Whether `Build` has completed since the last `Clear`.
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//! How many capture objectives are usable.
	static int GetCaptureCount()
	{
		return s_iCaptureCount;
	}

	//! How many destroy objectives are usable.
	static int GetDestroyCount()
	{
		return s_iDestroyCount;
	}

	//! How many hold objectives are usable.
	static int GetHoldCount()
	{
		return s_iHoldCount;
	}

	//! Every prepared objective, inert ones included, so the count matches the document.
	//! @return the objectives, or null before `Build`
	static array<ref TBD_Objective> GetAll()
	{
		return s_aObjectives;
	}

	//! Drop every objective and the three document readers. Called on world teardown.
	static void Clear()
	{
		s_aObjectives = null;
		s_bBuilt = false;
		s_iCaptureCount = 0;
		s_iDestroyCount = 0;
		s_iHoldCount = 0;
		TBD_ObjectiveRulesReader.Clear();
		TBD_ZoneVolumeBounds.Clear();
		TBD_ObjectiveEntityReader.Clear();
	}

	//! Prepare every objective zone of the loaded mission and log the result. Idempotent: only
	//! the first call after a `Clear` does work.
	//! @return false while no valid mission is loaded, so the caller keeps waiting
	//! @authority server
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		// The zone layer is the source of geometry. It is idempotent, so calling it here costs
		// nothing when `TBD_PlayAreaComponent` has already built it and makes objectives work when
		// that component is not on the prefab. Note this deliberately does NOT call
		// `TBD_ZoneRegistry.Clear()` -- that belongs to its owner, and two components racing to tear
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
		TBD_ZoneVolumeBounds.Read();


		// The typed `objectives[]` pass. A document without the key reads a clean false and every
		// objective runs untyped.
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
			TBD_Log.Warn(CH, "objective rules could not be re-read from the raw mission JSON -- every objective below is running on documented defaults. See TBD_ObjectiveRulesReader.");
		}
		else if (rulesOk && TBD_ObjectiveRulesReader.Count() != zones.Count())
		{
			// The two passes disagree about how many zones the document has. The per-zone join
			// verifies ids and falls back to a by-id search, so this is a warning rather than a
			// refusal -- but it means one of the two parses dropped something and an operator should
			// know before they wonder why an objective is on defaults.
			TBD_Log.Warn(CH, string.Format("zone count differs between the mission loader (%1) and the objective rules pass (%2) -- rules are joined by id where the index disagrees",
				zones.Count(), TBD_ObjectiveRulesReader.Count()));
		}

		// One greppable summary line, always, even at zero -- "this mission has no objectives" is
		// exactly as important to see in a boot log as "this mission has four".
		TBD_Log.Kv(CH, "built", string.Format("objectives=%1 usable=%2 capture=%3 destroy=%4 hold=%5",
			s_aObjectives.Count(), usable, s_iCaptureCount, s_iDestroyCount, s_iHoldCount));

		ReportTriggerCoverage();
		TBD_ObjectiveTypedBinder.ReportCoverage(s_aObjectives);

		return true;
	}

	//! Cross-check the declared end triggers against the usable objectives: a declared trigger
	//! with no usable objective of its kind is a warning (it can never fire), and a kind with no
	//! declared trigger is a note (it is tracked and ends nothing).
	protected static void ReportTriggerCoverage()
	{
		if (TBD_MissionLoader.HasEndTrigger(TRIGGER_ALL_CAPTURED) && s_iCaptureCount == 0)
			TBD_Log.Warn(CH, string.Format("winConditions.endOn declares '%1' but this mission has no usable %2 zone -- that trigger can NEVER fire",
				TRIGGER_ALL_CAPTURED, TYPE_CAPTURE));

		if (TBD_MissionLoader.HasEndTrigger(TRIGGER_DESTROYED) && s_iDestroyCount == 0)
			TBD_Log.Warn(CH, string.Format("winConditions.endOn declares '%1' but this mission has no usable %2 zone -- that trigger can NEVER fire",
				TRIGGER_DESTROYED, TYPE_DESTROY));

		if (TBD_MissionLoader.HasEndTrigger(TRIGGER_HOLD_EXPIRED) && s_iHoldCount == 0)
			TBD_Log.Warn(CH, string.Format("winConditions.endOn declares '%1' but this mission has no usable %2 zone -- that trigger can NEVER fire",
				TRIGGER_HOLD_EXPIRED, TYPE_HOLD_UNTIL));

		if (s_iCaptureCount > 0 && !TBD_MissionLoader.HasEndTrigger(TRIGGER_ALL_CAPTURED))
			TBD_Log.Kv(CH, "note", string.Format("%1 capture objective(s) but endOn does not declare '%2' -- they are tracked and announced, and will not end the round",
				s_iCaptureCount, TRIGGER_ALL_CAPTURED));

		if (s_iDestroyCount > 0 && !TBD_MissionLoader.HasEndTrigger(TRIGGER_DESTROYED))
			TBD_Log.Kv(CH, "note", string.Format("%1 destroy objective(s) but endOn does not declare '%2' -- tracked, will not end the round",
				s_iDestroyCount, TRIGGER_DESTROYED));

		if (s_iHoldCount > 0 && !TBD_MissionLoader.HasEndTrigger(TRIGGER_HOLD_EXPIRED))
			TBD_Log.Kv(CH, "note", string.Format("%1 hold objective(s) but endOn does not declare '%2' -- tracked, will not end the round",
				s_iHoldCount, TRIGGER_HOLD_EXPIRED));
	}

	//! The objective kind of a `zones[].type`.
	//! @return the kind, or NONE for every non-objective zone type
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

	//! Log one objective at load: an inert objective's reason, or its id, label, faction, bounds,
	//! typed row and resolved rules, so an operator checks them against the map before an event.
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
			TBD_ObjectiveTypedBinder.LogTyped(objective);
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
			TBD_ObjectiveTypedBinder.LogTyped(objective);
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

		TBD_ObjectiveTypedBinder.LogTyped(objective);

		TBD_Log.Kv(CH, "objectiveRules", string.Format("id=%1 targetAlias='%2' targetCount=%3 points=%4",
			objective.m_sId,
			objective.m_sTargetAlias,
			objective.m_iTargetCount,
			objective.m_fPoints));
	}

	//! Flatten one prepared zone and its rules into an objective, reporting every defect by zone
	//! id.
	//! @param index the zone's index, naming a zone without an id in the log
	//! @return the objective; inert when its shape or a required rule is unusable
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

		TBD_ObjectiveRuleResolver.ApplyDefaults(objective, kind);

		// Who the side defends before any typed row is read: a hold zone's faction holds its
		// ground; a capture or destroy zone's faction attacks it. A typed `type` may override it.
		objective.m_bSideDefends = kind == TBD_EObjectiveKind.HOLD_UNTIL;

		// The typed row binds before the kind rules and before the unusable-shape return: an
		// authored side supplies a hold objective's holder, and an inert objective keeps its
		// identity in the log that reports it.
		TBD_ObjectiveTypedBinder.Bind(objective, subject);

		// An objective with no usable shape can never contain anyone, so nothing can ever advance
		// it. `TBD_ZoneRegistry` has already reported the geometry defect; this reports the
		// CONSEQUENCE, which is the part an operator cares about.
		if (!zone.IsUsable())
		{
			objective.m_bUsable = false;
			objective.m_sInertReason = "the zone has no usable shape, so nobody can ever be inside it";
			return objective;
		}

		TBD_ObjectiveRuleResolver.Resolve(objective, kind, rules, subject);

		TBD_ZoneVolume.ApplyStartingOwner(objective);
		TBD_ZoneVolume.LogBound(objective);

		return objective;
	}

	//! Re-derive the per-kind usable counts, for a destroy objective that goes inert when arming
	//! finds nothing to destroy.
	static void RecountUsable()
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

	//! The `winConditions.endOn` trigger the objectives have fired, checked in schema enum order.
	//! Each trigger is gated on `TBD_MissionLoader.HasEndTrigger`, so an undeclared trigger never
	//! fires. Called by `TBD_FactionElimination`, which owns ending the round.
	//! @param winnerFaction set to the winning side; may be empty for a destroy objective whose
	//! zone names no faction
	//! @return the trigger name, or empty when none has fired
	static string EvaluateEndTriggers(out string winnerFaction)
	{
		winnerFaction = string.Empty;

		if (!s_bBuilt)
			return string.Empty;

		if (TBD_MissionLoader.HasEndTrigger(TRIGGER_ALL_CAPTURED) && TBD_ObjectiveEndConditions.AreAllObjectivesCaptured(s_aObjectives, winnerFaction))
			return TRIGGER_ALL_CAPTURED;

		if (TBD_MissionLoader.HasEndTrigger(TRIGGER_DESTROYED) && TBD_ObjectiveEndConditions.HasObjectiveBeenDestroyed(s_aObjectives, winnerFaction))
			return TRIGGER_DESTROYED;

		if (TBD_MissionLoader.HasEndTrigger(TRIGGER_HOLD_EXPIRED) && TBD_ObjectiveEndConditions.HasHoldExpired(s_aObjectives, winnerFaction))
			return TRIGGER_HOLD_EXPIRED;

		winnerFaction = string.Empty;
		return string.Empty;
	}
}
