/**
 * @file TBD_MissionWinConditionChecks.c
 * @brief Checks that every declared end trigger is a schema value and has, in this document, what
 * it needs to fire.
 *
 * Role: the win-condition checks of mission validation.  Position: called by
 * `TBD_MissionValidator.Run` with the slot census from `TBD_MissionSlotChecks`; asks
 * `TBD_ObjectiveRegistry` for the objective triggers, zone types and kinds, and
 * `TBD_MissionFlow.ResolveSeconds` for the flow duration rule; writes into a
 * `TBD_MissionValidationFindings`.
 * State: none.  Invariants: an ERROR means the mission cannot be played, a WARNING that it can be
 * played but may not end; so every reachability finding is a WARNING except `faction_eliminated`
 * with fewer than two sides holding slots, which is not a PvP event at all. This is a document
 * check: it proves the mission carries what each trigger watches, not that the piece resolves at
 * runtime (`TBD_ObjectiveDestroyTargets.ArmDestroyTargets` names that).
 */

//! Static end-trigger checks.
class TBD_MissionWinConditionChecks
{
	protected static const string TRIGGER_TIME_LIMIT         = "time_limit";         //!< `endOn` value `TBD_RoundClock` evaluates.
	protected static const string TRIGGER_FACTION_ELIMINATED = "faction_eliminated"; //!< `endOn` value `TBD_FactionElimination` evaluates.

	//! End triggers must be schema values, and each must be reachable: `faction_eliminated` needs
	//! two sides holding slots, `time_limit` needs `flow.timeLimitSeconds`, and the three objective
	//! triggers need a zone of their kind with a usable shape. The parse allocates `winConditions`
	//! even when absent, so both fields empty is the observable form of no block. All triggers
	//! unreachable is warned like an empty `endOn`.
	//! @param findings receives the findings
	//! @param mission the document
	//! @param slotsPerFaction the slot census
	static void CheckWinConditions(TBD_MissionValidationFindings findings, TBD_MissionDocumentStruct mission, map<string, int> slotsPerFaction)
	{
		TBD_MissionWinConditionsStruct conditions = mission.winConditions;

		bool hasMode = false;
		bool hasEndOn = false;
		if (conditions)
		{
			hasMode = !conditions.mode.IsEmpty();
			if (conditions.endOn)
				hasEndOn = !conditions.endOn.IsEmpty();
		}

		if (!hasMode && !hasEndOn)
		{
			findings.AddWarning("winConditions", "absent or empty -- the round has no end trigger and runs until an admin ends it");
			return;
		}

		if (!hasMode)
			findings.AddWarning("winConditions.mode", "empty -- the round has no declared mode label");

		if (!hasEndOn)
		{
			findings.AddWarning("winConditions.endOn", "empty -- the round has no end trigger and runs until an admin ends it");
			return;
		}

		// Computed once, before the loop, because the faction_eliminated check needs it and the
		// census is the same for every trigger.
		int sidesWithSlots = 0;
		foreach (string factionKey, int count : slotsPerFaction)
		{
			if (count > 0)
				sidesWithSlots++;
		}

		int declared = 0;
		int reachable = 0;

		foreach (int i, string trigger : conditions.endOn)
		{
			string subject = string.Format("winConditions.endOn[%1]", i);

			if (trigger.IsEmpty())
			{
				findings.AddError(subject, "empty end trigger");
				continue;
			}

			if (!IsKnownEndTrigger(trigger))
			{
				findings.AddError(subject, string.Format(
					"'%1' is not a mission.schema.json end trigger (time_limit, all_objectives_captured, faction_eliminated, objective_destroyed, hold_expired)",
					trigger));
				continue;
			}

			declared++;
			if (CheckTriggerReachable(findings, subject, trigger, mission, sidesWithSlots))
				reachable++;
		}

		// The roll-up, and the only finding that reads the mission as a whole. Each unreachable
		// trigger above is survivable on its own -- the round still ends on one of the others. All of
		// them unreachable means the round has NO way to end, which is the same state as an empty
		// `endOn` and is warned about in the same words, so an operator recognises it.
		if (declared > 0 && reachable == 0)
		{
			findings.AddWarning("winConditions.endOn", string.Format(
				"NONE of the %1 declared end trigger(s) can fire in this mission -- the round has no end trigger and runs until an admin ends it. Each one is reported above with the piece it is missing.",
				declared));
		}
	}

	//! Whether this trigger can fire in this document, reporting the missing piece when it cannot.
	//! A trigger with no objective kind answers true, so a trigger added to the schema without a
	//! case here never drags the roll-up into claiming the round cannot end.
	//! @param findings receives the findings
	//! @param subject the trigger's finding subject
	//! @param trigger the trigger
	//! @param mission the document
	//! @param sidesWithSlots how many factions hold at least one slot
	//! @return true when the document carries everything the trigger needs
	protected static bool CheckTriggerReachable(TBD_MissionValidationFindings findings, string subject, string trigger,
		TBD_MissionDocumentStruct mission, int sidesWithSlots)
	{
		if (trigger == TRIGGER_FACTION_ELIMINATED)
			return CheckFactionEliminatedReachable(findings, subject, mission, sidesWithSlots);

		if (trigger == TRIGGER_TIME_LIMIT)
			return CheckTimeLimitReachable(findings, subject, mission);

		string zoneType;
		TBD_EObjectiveKind kind = ObjectiveKindFor(trigger, zoneType);

		// IsKnownEndTrigger admits five values and all five are mapped, so this answers only for a
		// trigger added to the schema without a case here; it is not evidence the round cannot end.
		if (kind == TBD_EObjectiveKind.NONE)
			return true;

		return CheckObjectiveTriggerReachable(findings, subject, trigger, zoneType, kind, mission);
	}

	//! `faction_eliminated` needs two sides holding slots; `TBD_FactionElimination`
	//! refuses to end a round with fewer than two contesting factions, so a one-sided mission would
	//! never end. The one end-trigger ERROR.
	//! @param findings receives the findings
	//! @param subject the trigger's finding subject
	//! @param mission the document
	//! @param sidesWithSlots how many factions hold at least one slot
	//! @return true when two or more sides hold slots; false (silently) when there are no slots
	protected static bool CheckFactionEliminatedReachable(TBD_MissionValidationFindings findings, string subject,
		TBD_MissionDocumentStruct mission, int sidesWithSlots)
	{
		// A document with no slots has no sides to eliminate; CheckSlots already said so, and
		// repeating it here would bury that finding under a second one saying the same thing. Not
		// reachable either, so the roll-up above is told the truth.
		if (!mission.slots || mission.slots.IsEmpty())
			return false;

		if (sidesWithSlots < 2)
		{
			findings.AddError(subject, string.Format(
				"declares faction_eliminated but only %1 faction(s) actually have slots -- no second side can ever be eliminated, so the round can never resolve",
				sidesWithSlots));
			return false;
		}

		return true;
	}

	//! `time_limit` needs a duration; `TBD_RoundClock.Arm` refuses to guess one.
	//! The absent, negative and zero rules come from `TBD_MissionFlow.ResolveSeconds`, never a copy.
	//! A duration authored without the trigger is reported by `TBD_RoundClock.Arm`, not here.
	//! @param findings receives the findings
	//! @param subject the trigger's finding subject
	//! @param mission the document
	//! @return true when a positive duration is authored
	protected static bool CheckTimeLimitReachable(TBD_MissionValidationFindings findings, string subject, TBD_MissionDocumentStruct mission)
	{
		// The parse always allocates `flow`; the guard only prevents a null dereference, and ABSENT
		// carries "not authored" into ResolveSeconds.
		int raw = TBD_MissionFlowStruct.ABSENT;
		if (mission.flow)
			raw = mission.flow.timeLimitSeconds;

		string source;
		int seconds = TBD_MissionFlow.ResolveSeconds(raw, source);

		if (source == TBD_MissionFlow.SRC_DEFAULT)
		{
			findings.AddWarning(subject, "declares 'time_limit' but flow.timeLimitSeconds is not authored -- TBD_RoundClock.Arm has no duration to arm, so this round CANNOT end on time. Author flow.timeLimitSeconds, or drop the trigger.");
			return false;
		}

		if (source == TBD_MissionFlow.SRC_INVALID)
		{
			findings.AddWarning(subject, string.Format(
				"declares 'time_limit' but flow.timeLimitSeconds=%1 is negative (mission.schema.json requires >= 0) -- the clock is not armed, so this round CANNOT end on time.",
				raw));
			return false;
		}

		if (seconds == 0)
		{
			findings.AddWarning(subject, "declares 'time_limit' but flow.timeLimitSeconds=0, which is an explicit NO LIMIT -- the clock is deliberately not armed, so this trigger will never end the round. Author a real duration, or drop the trigger.");
			return false;
		}

		return true;
	}

	//! `all_objectives_captured`, `objective_destroyed` and `hold_expired` each need at least one
	//! zone of their kind carrying geometry something can be inside.
	//! @param findings receives the findings
	//! @param subject the trigger's finding subject
	//! @param trigger the trigger
	//! @param zoneType the schema zone type to name in the finding
	//! @param kind the objective kind the trigger watches
	//! @param mission the document
	//! @return true when a zone of that kind has a usable shape
	protected static bool CheckObjectiveTriggerReachable(TBD_MissionValidationFindings findings, string subject, string trigger, string zoneType,
		TBD_EObjectiveKind kind, TBD_MissionDocumentStruct mission)
	{
		int placeable;
		int total = CountObjectiveZones(mission, kind, placeable);

		if (total == 0)
		{
			findings.AddWarning(subject, string.Format(
				"declares '%1' but this mission has no '%2' zone for it to watch -- that trigger can NEVER fire. Add one to zones[], or drop the trigger.",
				trigger, zoneType));
			return false;
		}

		if (placeable == 0)
		{
			// A schema-valid document cannot reach this (`$defs/circle` requires r > 0 and
			// `$defs/polygon` three vertices), so it fires only on a hand-edited file.
			findings.AddWarning(subject, string.Format(
				"declares '%1' and this mission has %2 '%3' zone(s), but none of them carries a usable shape (no circle with r > 0 and no polygon) -- nothing can ever be inside them, so that trigger can NEVER fire.",
				trigger, total, zoneType));
			return false;
		}

		return true;
	}

	//! How many zones of `kind` the document declares, and how many carry geometry. The kind comes
	//! from `TBD_ObjectiveRegistry.KindOf`, so no zone-type string is spelled here. A circle counts
	//! when `r > 0` and a polygon when it has vertices; the parse allocates `circle` regardless.
	//! @param mission the document
	//! @param kind the objective kind
	//! @param outPlaceable receives how many of those zones carry a usable shape
	//! @return the number of zones of that kind
	protected static int CountObjectiveZones(TBD_MissionDocumentStruct mission, TBD_EObjectiveKind kind,
		out int outPlaceable)
	{
		outPlaceable = 0;

		if (!mission.zones)
			return 0;

		int total = 0;

		foreach (TBD_MissionZoneStruct zone : mission.zones)
		{
			if (!zone)
				continue;

			if (TBD_ObjectiveRegistry.KindOf(zone.type) != kind)
				continue;

			total++;

			if (!zone.shape)
				continue;

			if (zone.shape.circle && zone.shape.circle.r > 0)
				outPlaceable++;
			else if (zone.shape.polygon && zone.shape.polygon.Count() > 0)
				outPlaceable++;
		}

		return total;
	}

	//! The objective kind a trigger needs at least one of, and the zone type to name in a finding.
	//! Trigger and zone-type names come from `TBD_ObjectiveRegistry`; the pairing is the one fact it
	//! exposes no accessor for, and it matches `TBD_ObjectiveRegistry.ReportTriggerCoverage`.
	//! @param trigger the trigger
	//! @param zoneType receives the schema zone type, or empty
	//! @return the kind, or `TBD_EObjectiveKind.NONE` when the trigger watches no objective
	protected static TBD_EObjectiveKind ObjectiveKindFor(string trigger, out string zoneType)
	{
		if (trigger == TBD_ObjectiveRegistry.TRIGGER_ALL_CAPTURED)
		{
			zoneType = TBD_ObjectiveRegistry.TYPE_CAPTURE;
			return TBD_EObjectiveKind.CAPTURE;
		}

		if (trigger == TBD_ObjectiveRegistry.TRIGGER_DESTROYED)
		{
			zoneType = TBD_ObjectiveRegistry.TYPE_DESTROY;
			return TBD_EObjectiveKind.DESTROY;
		}

		if (trigger == TBD_ObjectiveRegistry.TRIGGER_HOLD_EXPIRED)
		{
			zoneType = TBD_ObjectiveRegistry.TYPE_HOLD_UNTIL;
			return TBD_EObjectiveKind.HOLD_UNTIL;
		}

		zoneType = string.Empty;
		return TBD_EObjectiveKind.NONE;
	}

	//! Membership of the schema's `winConditions.endOn` enum; legality only.
	//! @param trigger the trigger
	//! @return true for the five schema values
	protected static bool IsKnownEndTrigger(string trigger)
	{
		return trigger == TRIGGER_TIME_LIMIT
			|| trigger == TRIGGER_FACTION_ELIMINATED
			|| trigger == TBD_ObjectiveRegistry.TRIGGER_ALL_CAPTURED
			|| trigger == TBD_ObjectiveRegistry.TRIGGER_DESTROYED
			|| trigger == TBD_ObjectiveRegistry.TRIGGER_HOLD_EXPIRED;
	}
}
