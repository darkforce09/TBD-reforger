/**
 * @file TBD_ObjectiveRegistry.c
 * @brief Builds the prepared objectives of the loaded mission once per world and answers its
 * objective end triggers.
 *
 * Role: turns every prepared objective zone into a `TBD_Objective` with its resolved rules and
 * typed row, and is the one authority on whether an objective kind's `winConditions.endOn`
 * trigger has fired.  Position: `TBD_ObjectivesComponent` builds it on the server; reads
 * `TBD_ZoneRegistry`, `TBD_ObjectiveRulesReader`, `TBD_ObjectiveEntityReader`, `TBD_ZoneVolume`
 * and the `TBD_ObjectiveKindBehaviour` of each zone; read by `TBD_FactionElimination` and
 * `TBD_EndBanner` (end triggers), `TBD_TriggerRuntime` and the objective runtime.
 * State: the prepared objectives and per-kind usable counts of the current world; statics
 * outlive a world, so `TBD_ObjectivesComponent.OnDelete` calls `Clear`.  Invariants: never builds
 * on a client (no mission document) and never caches an empty build; containment is never tested
 * here; the zone wins over a disagreeing typed row; an end trigger fires only when
 * `winConditions.endOn` declares it, checked in the lookup's kind order; `lock` and `autoLose`
 * are reported and never enforced.
 */

//! Prepared objectives of the loaded mission and the objective end-trigger authority.
class TBD_ObjectiveRegistry
{
	static const string CH = "Obj"; //!< log channel `[TBD][Obj]`

	protected static ref array<ref TBD_Objective> s_aObjectives; //!< every prepared objective, inert ones included; null until `Build`
	protected static bool s_bBuilt; //!< `Build` completed since the last `Clear`
	protected static ref map<int, int> s_mUsableCounts; //!< usable objectives per `TBD_EObjectiveKind`; a kind with none has no entry; null until `Build`

	//! Whether `Build` has completed since the last `Clear`.
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//! How many objectives of one kind are usable.
	//! @param kind the objective kind
	//! @return the usable count of a real kind; 0 for NONE, an unknown kind and before `Build`
	static int UsableCountOf(TBD_EObjectiveKind kind)
	{
		if (!s_mUsableCounts)
			return 0;

		int count = 0;
		s_mUsableCounts.Find(kind, count);
		return count;
	}

	//! The usable count of every real kind, in the lookup's kind order CAPTURE, DESTROY, HOLD_UNTIL,
	//! as `<CountLabel>=<count>` pairs joined by single spaces, for the load summary lines.
	//! @return the pairs, e.g. `capture=2 destroy=0 hold=1`
	static string UsableCountsSummary()
	{
		string summary = string.Empty;

		for (int i = 0; i < TBD_ObjectiveKindBehaviour.Count(); i++)
		{
			TBD_ObjectiveKindBehaviour behaviour = TBD_ObjectiveKindBehaviour.At(i);
			if (i > 0)
				summary += " ";

			summary += string.Format("%1=%2", behaviour.CountLabel(), UsableCountOf(behaviour.Kind()));
		}

		return summary;
	}

	//! Count one more usable objective of a kind.
	//! @param kind the usable objective's kind
	protected static void CountUsable(TBD_EObjectiveKind kind)
	{
		int count = 0;
		s_mUsableCounts.Find(kind, count);
		s_mUsableCounts.Set(kind, count + 1);
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
		s_mUsableCounts = null;
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
		s_mUsableCounts = new map<int, int>();

		int usable = 0;

		foreach (int index, TBD_Zone zone : zones)
		{
			if (!zone)
				continue;

			TBD_ObjectiveKindBehaviour behaviour = TBD_ObjectiveKindBehaviour.ForZoneType(zone.m_sType);
			if (behaviour.Kind() == TBD_EObjectiveKind.NONE)
				continue;

			TBD_ObjectiveRulesStruct rules = TBD_ObjectiveRulesReader.ForZone(index, zone.m_sId);
			TBD_Objective objective = Prepare(zone, behaviour, rules, index);
			s_aObjectives.Insert(objective);

			if (objective.m_bUsable)
			{
				usable++;
				CountUsable(objective.m_eKind);
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
		TBD_Log.Kv(CH, "built", string.Format("objectives=%1 usable=%2 %3",
			s_aObjectives.Count(), usable, UsableCountsSummary()));

		ReportTriggerCoverage();
		TBD_ObjectiveTypedBinder.ReportCoverage(s_aObjectives);

		return true;
	}

	//! Cross-check the declared end triggers against the usable objectives: a declared trigger
	//! with no usable objective of its kind is a warning (it can never fire), and a kind with no
	//! declared trigger is a note (it is tracked and ends nothing). Every warning comes before
	//! every note, each in the lookup's kind order.
	protected static void ReportTriggerCoverage()
	{
		for (int i = 0; i < TBD_ObjectiveKindBehaviour.Count(); i++)
		{
			TBD_ObjectiveKindBehaviour behaviour = TBD_ObjectiveKindBehaviour.At(i);
			if (TBD_MissionLoader.HasEndTrigger(behaviour.EndTrigger()) && UsableCountOf(behaviour.Kind()) == 0)
				TBD_Log.Warn(CH, string.Format("winConditions.endOn declares '%1' but this mission has no usable %2 zone -- that trigger can NEVER fire",
					behaviour.EndTrigger(), behaviour.ZoneType()));
		}

		for (int k = 0; k < TBD_ObjectiveKindBehaviour.Count(); k++)
		{
			TBD_ObjectiveKindBehaviour undeclared = TBD_ObjectiveKindBehaviour.At(k);
			int usableOfKind = UsableCountOf(undeclared.Kind());
			if (usableOfKind > 0 && !TBD_MissionLoader.HasEndTrigger(undeclared.EndTrigger()))
				TBD_Log.Kv(CH, "note", undeclared.UndeclaredTriggerNote(usableOfKind));
		}
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

		TBD_ObjectiveTypedBinder.LogTyped(objective);
		TBD_ObjectiveKindBehaviour.For(objective.m_eKind).LogRules(objective);
	}

	//! Flatten one prepared zone and its rules into an objective, reporting every defect by zone
	//! id.
	//! @param behaviour the zone's kind behaviour; never the NONE behaviour
	//! @param index the zone's index, naming a zone without an id in the log
	//! @return the objective; inert when its shape or a required rule is unusable
	protected static TBD_Objective Prepare(notnull TBD_Zone zone, notnull TBD_ObjectiveKindBehaviour behaviour, TBD_ObjectiveRulesStruct rules, int index)
	{
		TBD_Objective objective = new TBD_Objective();
		objective.m_Zone = zone;
		objective.m_eKind = behaviour.Kind();
		objective.m_sId = zone.m_sId;
		objective.m_sLabel = zone.m_sLabel;
		objective.m_sFaction = zone.m_sFaction;
		objective.m_bUsable = true;

		string subject = zone.m_sId;
		if (subject.IsEmpty())
			subject = string.Format("zones[%1]", index);

		TBD_ObjectiveRuleResolver.ApplyDefaults(objective, behaviour);

		// Who the side defends before any typed row is read: the kind decides (a hold zone's
		// faction holds its ground; a capture or destroy zone's faction attacks it). A typed `type`
		// may override it.
		objective.m_bSideDefends = behaviour.SideDefendsByDefault();

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

		TBD_ObjectiveRuleResolver.Resolve(objective, behaviour, rules, subject);

		TBD_ZoneVolume.ApplyStartingOwner(objective);
		TBD_ZoneVolume.LogBound(objective);

		return objective;
	}

	//! Re-derive the per-kind usable counts, for a destroy objective that goes inert when arming
	//! finds nothing to destroy.
	static void RecountUsable()
	{
		s_mUsableCounts = new map<int, int>();

		if (!s_aObjectives)
			return;

		foreach (TBD_Objective objective : s_aObjectives)
		{
			if (!objective || !objective.m_bUsable)
				continue;

			CountUsable(objective.m_eKind);
		}
	}

	//! The `winConditions.endOn` trigger the objectives have fired, checked in the lookup's kind
	//! order CAPTURE, DESTROY, HOLD_UNTIL (the schema enum order). Each trigger is gated on
	//! `TBD_MissionLoader.HasEndTrigger`, so an undeclared trigger never fires. Called by
	//! `TBD_FactionElimination`, which owns ending the round.
	//! @param winnerFaction set to the winning side; may be empty for a destroy objective whose
	//! zone names no faction
	//! @return the trigger name, or empty when none has fired
	static string EvaluateEndTriggers(out string winnerFaction)
	{
		winnerFaction = string.Empty;

		if (!s_bBuilt)
			return string.Empty;

		for (int i = 0; i < TBD_ObjectiveKindBehaviour.Count(); i++)
		{
			TBD_ObjectiveKindBehaviour behaviour = TBD_ObjectiveKindBehaviour.At(i);
			string trigger = behaviour.EndTrigger();
			if (TBD_MissionLoader.HasEndTrigger(trigger) && behaviour.HasEnded(s_aObjectives, winnerFaction))
				return trigger;
		}

		winnerFaction = string.Empty;
		return string.Empty;
	}
}
