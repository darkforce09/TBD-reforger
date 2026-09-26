/**
 * @file TBD_ObjectiveRuleResolver.c
 * @brief Resolves one objective's authored `zones[].rules` onto its runtime fields, with defaults.
 *
 * Role: applies the documented defaults, then validates each authored rule for the objective's
 * kind and either takes it, falls back to the default with a warning, or makes the objective
 * inert with a reason.  Position: called by `TBD_ObjectiveRegistry.Prepare` with the rules
 * `TBD_ObjectiveRulesReader` read.
 * State: none; writes only the objective it is given.  Invariants: every field holds a defined
 * value even on an inert path; a capture objective without a readable length defaults (keeping
 * `all_objectives_captured` winnable) while a hold objective without a length or holder goes
 * inert (a guessed round length decides when the round ends); every fallback names its value in
 * the log.
 */

//! Default and validated objective rules.
class TBD_ObjectiveRuleResolver
{
	static const string ON_EMPTY_HOLD  = "hold"; //!< `rules.onEmpty` vocabulary.
	static const string ON_EMPTY_DECAY = "decay"; //!< `rules.onEmpty` value
	static const float DEFAULT_CAPTURE_SECONDS = 120.0; //!< seconds; the capture length when none is readable
	static const float DEFAULT_DECAY_RATE = 1.0; //!< progress-seconds lost per second while decaying
	static const float DEFAULT_CAPTURE_ANNOUNCE_SECONDS = 15.0; //!< seconds between capture progress log lines
	static const float DEFAULT_HOLD_ANNOUNCE_SECONDS = 60.0; //!< seconds between hold progress log lines
	static const float MAX_DURATION_SECONDS = 21600.0; //!< seconds; the schema `maximum` on capture, neutralize and hold lengths, enforced again for documents that bypass validation

	//! Write the documented default into every rule field, so no field holds a sentinel even on
	//! a path that goes inert.
	//! @param kind the objective's kind, which picks the announcement cadence
	static void ApplyDefaults(notnull TBD_Objective objective, TBD_EObjectiveKind kind)
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
		if (kind == TBD_EObjectiveKind.HOLD_UNTIL)
			objective.m_fAnnounceEverySeconds = DEFAULT_HOLD_ANNOUNCE_SECONDS;
	}

	//! Resolve the rules every kind shares, then the objective's own kind.
	//! @param rules the zone's rules from the rules pass; null runs on defaults
	//! @param subject the zone id, or `zones[<index>]`, for log lines
	static void Resolve(notnull TBD_Objective objective, TBD_EObjectiveKind kind, TBD_ObjectiveRulesStruct rules, string subject)
	{
		ResolveCommonRules(objective, rules, subject);

		if (kind == TBD_EObjectiveKind.CAPTURE)
			ResolveCaptureRules(objective, rules, subject);
		else if (kind == TBD_EObjectiveKind.HOLD_UNTIL)
			ResolveHoldRules(objective, rules, subject);
		else
			ResolveDestroyRules(objective, rules, subject);
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

	//! `objective_capture`: the capture and neutralize lengths, `contestable`, `onEmpty` and
	//! `decayRate`. An unreadable length defaults and warns; an authored `faction` is logged as
	//! the only side that may own it.
	protected static void ResolveCaptureRules(notnull TBD_Objective objective, TBD_ObjectiveRulesStruct rules, string subject)
	{
		bool captureAuthored = false;

		if (rules && rules.captureSeconds != TBD_ObjectiveRulesStruct.ABSENT)
		{
			if (rules.captureSeconds <= 0 || rules.captureSeconds > MAX_DURATION_SECONDS)
			{
				TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' rules.captureSeconds=%2 is outside 0..%3 -- using the default %4 s",
					subject, rules.captureSeconds, MAX_DURATION_SECONDS, DEFAULT_CAPTURE_SECONDS));
			}
			else
			{
				objective.m_fCaptureSeconds = rules.captureSeconds;
				captureAuthored = true;
			}
		}

		if (!captureAuthored)
			TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' (%2) has no readable rules.captureSeconds -- using the default %3 s. Either none was authored, or one was authored under a key this build does not declare and therefore cannot see; a typed JSON parser cannot tell those apart.",
				subject, TBD_ObjectiveRegistry.TYPE_CAPTURE, DEFAULT_CAPTURE_SECONDS));

		// Teardown defaults to a symmetric 1:1 rate with the build.
		objective.m_fNeutralizeSeconds = objective.m_fCaptureSeconds;

		if (rules && rules.neutralizeSeconds != TBD_ObjectiveRulesStruct.ABSENT)
		{
			if (rules.neutralizeSeconds < 0 || rules.neutralizeSeconds > MAX_DURATION_SECONDS)
			{
				TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' rules.neutralizeSeconds=%2 is outside 0..%3 -- using captureSeconds (%4 s)",
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
				TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' rules.onEmpty='%2' is not one of hold|decay -- using 'hold' (partial progress is kept)",
					subject, rules.onEmpty));
			}
		}

		if (rules && rules.decayRate != TBD_ObjectiveRulesStruct.ABSENT)
		{
			if (rules.decayRate <= 0)
			{
				TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' rules.decayRate=%2 must be > 0 -- using %3",
					subject, rules.decayRate, DEFAULT_DECAY_RATE));
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

	//! `objective_hold_until`: requires a holding `faction` and a `holdSeconds` inside
	//! 0..`MAX_DURATION_SECONDS`, else the objective goes inert; then the pause, reset and
	//! presence rules.
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

	//! `objective_destroy`: requires `targetAlias`, else inert; takes `targetCount`. Finding the
	//! targets waits for LIVE (`TBD_ObjectiveDestroyTargets.ArmDestroyTargets`).
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
				TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' rules.targetCount=%2 is negative -- using 0 (destroy everything found)",
					subject, rules.targetCount));
			}
			else
			{
				objective.m_iTargetCount = rules.targetCount;
			}
		}

		if (objective.m_sFaction.IsEmpty())
			TBD_Log.Warn(TBD_ObjectiveRegistry.CH, string.Format("objective '%1' (%2) names no `faction` -- if it completes, '%3' will fire with no winning side named. Set `faction` to the side that must destroy it.",
				subject, TBD_ObjectiveRegistry.TYPE_DESTROY, TBD_ObjectiveRegistry.TRIGGER_DESTROYED));
	}
}
