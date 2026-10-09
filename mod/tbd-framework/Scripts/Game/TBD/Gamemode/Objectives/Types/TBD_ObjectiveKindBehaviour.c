/**
 * @file TBD_ObjectiveKindBehaviour.c
 * @brief The objective kind behaviour contract, its NONE behaviour, and the lookup by kind.
 *
 * Role: declares every hook an objective kind answers (its vocabulary, load summary words, rule
 * defaults and resolution, typed-row seeding, load log, starting owner, end condition, LIVE edge,
 * per-tick advance, status text and HUD glyph) with a neutral default, and finds the one shared
 * behaviour of a kind, zone type, end trigger or task type.  Position: the engine under
 * `Gamemode/Objectives/Engine/`, `TBD_ZoneVolume` and the win-condition checks look a behaviour up
 * here and call its hooks; the subclasses live in the `Capture/`, `Destroy/` and `HoldUntil/`
 * folders beside this file.
 * State: one lazily built instance per real kind plus the shared NONE instance, held in statics
 * that outlive a world, so `TBD_ObjectivesComponent.OnDelete` calls `Clear`; the behaviours hold no
 * per-objective state (every objective's state lives on `TBD_Objective`).  Invariants: `Count()`
 * and `At()` list the real kinds in enum order CAPTURE, DESTROY, HOLD_UNTIL, which is also the
 * end-trigger evaluation order; NONE, an unknown kind and every lookup without a match answer the
 * NONE instance, whose hooks do nothing and return neutral values.
 */

//! One objective kind's behaviour; this base class is the NONE behaviour, every hook a no-op or
//! neutral default that a real kind overrides.
class TBD_ObjectiveKindBehaviour : Managed
{
	protected static ref array<ref TBD_ObjectiveKindBehaviour> s_aKinds; //!< the real kinds in enum order CAPTURE, DESTROY, HOLD_UNTIL; null until the first lookup and after `Clear`
	protected static ref TBD_ObjectiveKindBehaviour s_NoneBehaviour; //!< the shared NONE behaviour; null until the first lookup and after `Clear`

	//! Build the shared instances on first use: the NONE behaviour, then the real kinds in enum
	//! order.
	protected static void EnsureBuilt()
	{
		if (s_aKinds)
			return;

		s_NoneBehaviour = new TBD_ObjectiveKindBehaviour();

		s_aKinds = new array<ref TBD_ObjectiveKindBehaviour>();
		s_aKinds.Insert(new TBD_ObjectiveCaptureBehaviour());
		s_aKinds.Insert(new TBD_ObjectiveDestroyBehaviour());
		s_aKinds.Insert(new TBD_ObjectiveHoldUntilBehaviour());
	}

	//! The behaviour of an objective kind.
	//! @param kind the objective's kind
	//! @return the kind's behaviour, or the NONE behaviour for NONE and an unknown value
	static TBD_ObjectiveKindBehaviour For(TBD_EObjectiveKind kind)
	{
		EnsureBuilt();

		foreach (TBD_ObjectiveKindBehaviour behaviour : s_aKinds)
		{
			if (behaviour.Kind() == kind)
				return behaviour;
		}

		return s_NoneBehaviour;
	}

	//! The behaviour whose `zones[].type` is `zoneType`.
	//! @param zoneType a `zones[].type` value
	//! @return the matching behaviour, or the NONE behaviour for every non-objective zone type
	static TBD_ObjectiveKindBehaviour ForZoneType(string zoneType)
	{
		EnsureBuilt();

		foreach (TBD_ObjectiveKindBehaviour behaviour : s_aKinds)
		{
			if (behaviour.ZoneType() == zoneType)
				return behaviour;
		}

		return s_NoneBehaviour;
	}

	//! The behaviour whose `winConditions.endOn` trigger is `trigger`.
	//! @param trigger a `winConditions.endOn` value
	//! @return the matching behaviour, or the NONE behaviour for every trigger no objective kind fires
	static TBD_ObjectiveKindBehaviour ForEndTrigger(string trigger)
	{
		EnsureBuilt();

		foreach (TBD_ObjectiveKindBehaviour behaviour : s_aKinds)
		{
			if (behaviour.EndTrigger() == trigger)
				return behaviour;
		}

		return s_NoneBehaviour;
	}

	//! The behaviour an `objectives[].type` task type belongs on; `defend` is the far side of a
	//! capture.
	//! @param taskType an `objectives[].type` value
	//! @return the matching behaviour, or the NONE behaviour for an unknown task type
	static TBD_ObjectiveKindBehaviour ForTaskType(string taskType)
	{
		EnsureBuilt();

		foreach (TBD_ObjectiveKindBehaviour behaviour : s_aKinds)
		{
			if (behaviour.AcceptsTaskType(taskType))
				return behaviour;
		}

		return s_NoneBehaviour;
	}

	//! How many real objective kinds exist.
	//! @return the count of behaviours `At` lists
	static int Count()
	{
		EnsureBuilt();
		return s_aKinds.Count();
	}

	//! One real objective kind, in enum order CAPTURE, DESTROY, HOLD_UNTIL.
	//! @param index 0 to `Count()` - 1
	//! @return the behaviour, or the NONE behaviour for an index out of range
	static TBD_ObjectiveKindBehaviour At(int index)
	{
		EnsureBuilt();

		if (index < 0 || index >= s_aKinds.Count())
			return s_NoneBehaviour;

		return s_aKinds[index];
	}

	//! Drop every shared instance; the next lookup builds them again. Called on world teardown.
	static void Clear()
	{
		s_aKinds = null;
		s_NoneBehaviour = null;
	}

	//! @return the kind this behaviour runs; NONE here
	TBD_EObjectiveKind Kind() { return TBD_EObjectiveKind.NONE; }

	//! @return the `zones[].type` of this kind; empty here
	string ZoneType() { return ""; }

	//! @return the `winConditions.endOn` trigger this kind fires; empty here
	string EndTrigger() { return ""; }

	//! Whether an `objectives[].type` task type belongs on this kind.
	//! @param taskType an `objectives[].type` value
	//! @return false here
	bool AcceptsTaskType(string taskType) { return false; }

	//! @return the word the load summary lines print before this kind's usable count; empty here
	string CountLabel() { return ""; }

	//! The load note for this kind's usable objectives when `winConditions.endOn` does not declare
	//! its end trigger.
	//! @param usableCount how many objectives of this kind are usable; above 0
	//! @return the note text; empty here
	string UndeclaredTriggerNote(int usableCount) { return ""; }

	//! Write this kind's rule defaults over the shared defaults the rule resolver wrote.
	//! @param objective the objective being prepared
	void ApplyDefaults(notnull TBD_Objective objective) {}

	//! Validate and apply this kind's authored rules: take each, fall back to its default with a
	//! warning, or make the objective inert with a reason.
	//! @param objective the objective being prepared
	//! @param rules the zone's rules from the rules pass; null runs on defaults
	//! @param subject the zone id, or `zones[<index>]`, for log lines
	void ResolveRules(notnull TBD_Objective objective, TBD_ObjectiveRulesStruct rules, string subject) {}

	//! Whether the side an untyped objective is for defends it.
	//! @return false here
	bool SideDefendsByDefault() { return false; }

	//! Let the typed row's `side` supply what the zone left empty.
	//! @param objective the objective being bound
	//! @param subject the zone id, or `zones[<index>]`, for log lines
	void SeedFromSide(notnull TBD_Objective objective, string subject) {}

	//! Log this kind's resolved rules on the registry's channel, after the typed row's line.
	//! @param objective a usable prepared objective
	void LogRules(notnull TBD_Objective objective) {}

	//! Apply the zone's `startingOwner`, which the zone volume found non-empty.
	//! @param objective the objective being prepared
	//! @param startingOwner the zone's `rules.startingOwner`
	void ApplyStartingOwner(notnull TBD_Objective objective, string startingOwner) {}

	//! Whether this kind's end condition is met over the prepared objectives.
	//! @param prepared the prepared objectives
	//! @param winnerFaction set to the winning side, or empty when the condition is not met
	//! @return false here
	//! @authority server
	bool HasEnded(notnull array<ref TBD_Objective> prepared, out string winnerFaction)
	{
		winnerFaction = string.Empty;
		return false;
	}

	//! The round just went LIVE: prepare one usable objective of this kind.
	//! @param objective a usable objective
	//! @authority server
	void OnEnterLive(notnull TBD_Objective objective) {}

	//! Advance one usable objective of this kind by one 1 Hz tick.
	//! @param objective a usable objective
	//! @return this tick's chat completion line, or empty; empty here
	//! @authority server
	string Advance(notnull TBD_Objective objective) { return ""; }

	//! The status half of a usable objective's board line.
	//! @param objective a usable objective
	//! @param viewerFaction the viewer's side from server-owned state; may be empty
	//! @return the status text; empty here
	string StatusText(notnull TBD_Objective objective, string viewerFaction) { return ""; }

	//! The HUD row glyph of a usable objective that is neither complete nor contested.
	//! @param objective a usable objective
	//! @param viewerFaction the viewer's side from server-owned state; may be empty
	//! @return one ASCII glyph; empty here
	string HudIcon(notnull TBD_Objective objective, string viewerFaction) { return ""; }

	//! Whether a player standing in an objective of this kind is shown its capture bar.
	//! @return false here
	bool ClaimsCaptureBar() { return false; }
}
