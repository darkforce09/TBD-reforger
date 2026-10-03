/**
 * @file TBD_ObjectiveRuleResolver.c
 * @brief Resolves one objective's authored `zones[].rules` onto its runtime fields, with defaults.
 *
 * Role: applies the documented defaults every kind shares, then the objective's kind behaviour
 * writes its own defaults; validates the rules every kind shares and hands the rest to the kind
 * behaviour, which takes each rule, falls back to its default with a warning, or makes the
 * objective inert with a reason.  Position: called by `TBD_ObjectiveRegistry.Prepare` with the
 * rules `TBD_ObjectiveRulesReader` read and the `TBD_ObjectiveKindBehaviour` of the zone; the
 * kind behaviours read the shared limits declared here.
 * State: none; writes only the objective it is given.  Invariants: every field holds a defined
 * value even on an inert path; the shared rules resolve before the kind's rules; every fallback
 * names its value in the log.
 */

//! Default and validated objective rules.
class TBD_ObjectiveRuleResolver
{
	static const float DEFAULT_CAPTURE_SECONDS = 120.0; //!< seconds; the capture length when none is readable
	static const float DEFAULT_DECAY_RATE = 1.0; //!< progress-seconds lost per second while decaying
	static const float DEFAULT_CAPTURE_ANNOUNCE_SECONDS = 15.0; //!< seconds between progress log lines unless the kind's defaults replace it
	static const float MAX_DURATION_SECONDS = 21600.0; //!< seconds; the schema `maximum` on capture, neutralize and hold lengths, enforced again for documents that bypass validation

	//! Write the documented default into every rule field, so no field holds a sentinel even on
	//! a path that goes inert, then let the kind behaviour write its own defaults over them.
	//! @param objective the objective being prepared
	//! @param behaviour the objective's kind behaviour
	static void ApplyDefaults(notnull TBD_Objective objective, notnull TBD_ObjectiveKindBehaviour behaviour)
	{
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
		behaviour.ApplyDefaults(objective);
	}

	//! Resolve the rules every kind shares, then the objective's own kind through its behaviour.
	//! @param objective the objective being prepared
	//! @param behaviour the objective's kind behaviour
	//! @param rules the zone's rules from the rules pass; null runs on defaults
	//! @param subject the zone id, or `zones[<index>]`, for log lines
	static void Resolve(notnull TBD_Objective objective, notnull TBD_ObjectiveKindBehaviour behaviour, TBD_ObjectiveRulesStruct rules, string subject)
	{
		ResolveCommonRules(objective, rules, subject);
		behaviour.ResolveRules(objective, rules, subject);
	}

	//! `points` and `announceEverySeconds`, which every kind shares.
	protected static void ResolveCommonRules(notnull TBD_Objective objective, TBD_ObjectiveRulesStruct rules, string subject)
	{
		if (!rules)
			return;

		if (rules.points != TBD_ObjectiveRulesStruct.ABSENT)
		{
			if (rules.points < 0)
			{
				TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' rules.points=%2 is negative -- using 0",
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
				TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' rules.announceEverySeconds=%2 must be > 0 -- using %3 s",
					subject, rules.announceEverySeconds, objective.m_fAnnounceEverySeconds));
			}
			else
			{
				objective.m_fAnnounceEverySeconds = rules.announceEverySeconds;
			}
		}
	}
}
