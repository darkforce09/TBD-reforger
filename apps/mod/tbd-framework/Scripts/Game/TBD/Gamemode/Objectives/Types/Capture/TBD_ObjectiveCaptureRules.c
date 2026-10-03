/**
 * @file TBD_ObjectiveCaptureRules.c
 * @brief Resolves a capture objective's authored rules onto its runtime fields.
 *
 * Role: validates the capture and neutralize lengths, `contestable`, `onEmpty` and `decayRate`
 * of one `objective_capture` zone, taking each, or falling back to its default with a warning.
 * Position: called by `TBD_ObjectiveCaptureBehaviour.ResolveRules` after the rule resolver wrote
 * the shared defaults and the rules every kind shares; logs on the registry's `Obj` channel.
 * State: none; writes only the objective it is given.  Invariants: a capture objective without a
 * readable length defaults (keeping `all_objectives_captured` winnable) and never goes inert here;
 * teardown defaults to the capture length; every fallback names its value in the log.
 */

//! Capture rule validation.
class TBD_ObjectiveCaptureRules
{
	static const string ON_EMPTY_HOLD  = "hold"; //!< `rules.onEmpty` vocabulary.
	static const string ON_EMPTY_DECAY = "decay"; //!< `rules.onEmpty` value

	//! `objective_capture`: the capture and neutralize lengths, `contestable`, `onEmpty` and
	//! `decayRate`. An unreadable length defaults and warns; an authored `faction` is logged as
	//! the only side that may own it.
	//! @param objective the objective being prepared
	//! @param rules the zone's rules from the rules pass; null runs on defaults
	//! @param subject the zone id, or `zones[<index>]`, for log lines
	static void ResolveCaptureRules(notnull TBD_Objective objective, TBD_ObjectiveRulesStruct rules, string subject)
	{
		bool captureAuthored = false;

		if (rules && rules.captureSeconds != TBD_ObjectiveRulesStruct.ABSENT)
		{
			if (rules.captureSeconds <= 0 || rules.captureSeconds > TBD_ObjectiveRuleResolver.MAX_DURATION_SECONDS)
			{
				TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' rules.captureSeconds=%2 is outside 0..%3 -- using the default %4 s",
					subject, rules.captureSeconds, TBD_ObjectiveRuleResolver.MAX_DURATION_SECONDS, TBD_ObjectiveRuleResolver.DEFAULT_CAPTURE_SECONDS));
			}
			else
			{
				objective.m_fCaptureSeconds = rules.captureSeconds;
				captureAuthored = true;
			}
		}

		if (!captureAuthored)
			TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' (%2) has no readable rules.captureSeconds -- using the default %3 s. Either none was authored, or one was authored under a key this build does not declare and therefore cannot see; a typed JSON parser cannot tell those apart.",
				subject, TBD_ObjectiveCaptureBehaviour.ZONE_TYPE, TBD_ObjectiveRuleResolver.DEFAULT_CAPTURE_SECONDS));

		// Teardown defaults to a symmetric 1:1 rate with the build.
		objective.m_fNeutralizeSeconds = objective.m_fCaptureSeconds;

		if (rules && rules.neutralizeSeconds != TBD_ObjectiveRulesStruct.ABSENT)
		{
			if (rules.neutralizeSeconds < 0 || rules.neutralizeSeconds > TBD_ObjectiveRuleResolver.MAX_DURATION_SECONDS)
			{
				TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' rules.neutralizeSeconds=%2 is outside 0..%3 -- using captureSeconds (%4 s)",
					subject, rules.neutralizeSeconds, TBD_ObjectiveRuleResolver.MAX_DURATION_SECONDS, objective.m_fCaptureSeconds));
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
				TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' rules.onEmpty='%2' is not one of hold|decay -- using 'hold' (partial progress is kept)",
					subject, rules.onEmpty));
			}
		}

		if (rules && rules.decayRate != TBD_ObjectiveRulesStruct.ABSENT)
		{
			if (rules.decayRate <= 0)
			{
				TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' rules.decayRate=%2 must be > 0 -- using %3",
					subject, rules.decayRate, TBD_ObjectiveRuleResolver.DEFAULT_DECAY_RATE));
			}
			else
			{
				objective.m_fDecayRate = rules.decayRate;
			}
		}

		// An authored `faction` on a capture zone is a real restriction and must never be applied
		// silently -- a side that cannot take an objective and is not told why will report it as a
		// bug in the capture logic.
		if (!objective.m_sFaction.IsEmpty())
			TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "note", string.Format("objective '%1' names faction '%2' -- ONLY that side can own it; any other side can neutralise it but never take it",
				subject, objective.m_sFaction));
	}
}
