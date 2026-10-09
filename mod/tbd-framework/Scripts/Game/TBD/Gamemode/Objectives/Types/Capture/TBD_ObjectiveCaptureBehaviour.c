/**
 * @file TBD_ObjectiveCaptureBehaviour.c
 * @brief The `objective_capture` behaviour: a zone a side owns by standing on it.
 *
 * Role: answers every hook of the capture kind: its zone type, end trigger and task types, its
 * rules, load log, starting owner, the `all_objectives_captured` condition, the per-tick capture,
 * its status text, HUD glyph and capture bar.  Position: found through
 * `TBD_ObjectiveKindBehaviour.For` and its sibling lookups; delegates rule validation to
 * `TBD_ObjectiveCaptureRules` and the tick to `TBD_ObjectiveCaptureProgress`; logs on the
 * registry's `Obj` channel and the zone volume's `ZoneVol` channel.
 * State: none; every objective's state lives on `TBD_Objective`.  Invariants: a capture objective
 * is never complete (ownership can flip all round); the end condition needs at least one usable
 * capture and every one owned by the same side; a fresh round (every capture neutral) fires
 * nothing; a split between two owners is a stalemate.
 */

//! Capture objective behaviour.
class TBD_ObjectiveCaptureBehaviour : TBD_ObjectiveKindBehaviour
{
	static const string ZONE_TYPE   = "objective_capture"; //!< `zones[].type` of a capture objective
	static const string END_TRIGGER = "all_objectives_captured"; //!< `winConditions.endOn`: every usable capture owned by one side
	static const string TASK_CAPTURE = "capture"; //!< `objectives[].type` value: the attacker side of a capture
	static const string TASK_DEFEND  = "defend"; //!< `objectives[].type` value: the defender side of a capture

	//! @return CAPTURE
	override TBD_EObjectiveKind Kind() { return TBD_EObjectiveKind.CAPTURE; }

	//! @return `objective_capture`
	override string ZoneType() { return ZONE_TYPE; }

	//! @return `all_objectives_captured`
	override string EndTrigger() { return END_TRIGGER; }

	//! `capture` and `defend` both belong on a capture zone; `defend` is its far side.
	//! @param taskType an `objectives[].type` value
	//! @return true for `capture` and `defend`
	override bool AcceptsTaskType(string taskType)
	{
		return taskType == TASK_CAPTURE || taskType == TASK_DEFEND;
	}

	//! @return `capture`
	override string CountLabel() { return "capture"; }

	//! The note that the capture objectives are tracked and end nothing.
	//! @param usableCount how many objectives of this kind are usable; above 0
	//! @return the note text
	override string UndeclaredTriggerNote(int usableCount)
	{
		return string.Format("%1 capture objective(s) but endOn does not declare '%2' -- they are tracked and announced, and will not end the round", usableCount, END_TRIGGER);
	}

	//! Validate the capture rules through `TBD_ObjectiveCaptureRules`.
	//! @param objective the objective being prepared
	//! @param rules the zone's rules from the rules pass; null runs on defaults
	//! @param subject the zone id, or `zones[<index>]`, for log lines
	override void ResolveRules(notnull TBD_Objective objective, TBD_ObjectiveRulesStruct rules, string subject)
	{
		TBD_ObjectiveCaptureRules.ResolveCaptureRules(objective, rules, subject);
	}

	//! Log the capture lengths, `contestable`, `onEmpty`, `decayRate` and `points`.
	//! @param objective a usable prepared objective
	override void LogRules(notnull TBD_Objective objective)
	{
		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "objectiveRules", string.Format("id=%1 capture=%2s neutralize=%3s contestable=%4 onEmpty=%5 decayRate=%6 points=%7",
			objective.m_sId,
			objective.m_fCaptureSeconds,
			objective.m_fNeutralizeSeconds,
			objective.m_bContestable,
			typename.EnumToString(TBD_EObjectiveOnEmpty, objective.m_eOnEmpty),
			objective.m_fDecayRate,
			objective.m_fPoints));
	}

	//! Start the objective HELD (full progress) by `startingOwner` when that is a declared faction
	//! the objective may be owned by; otherwise log and leave it neutral.
	//! @param objective the objective being prepared
	//! @param startingOwner the zone's `rules.startingOwner`
	override void ApplyStartingOwner(notnull TBD_Objective objective, string startingOwner)
	{
		if (!TBD_DeclaredFactions.Exists(startingOwner))
		{
			TBD_Log.Warn(TBD_ZoneVolume.CH, string.Format("objective '%1' rules.startingOwner='%2' names no factions[].key -- leaving the objective NEUTRAL",
				objective.m_sId, startingOwner));
			return;
		}

		if (!objective.MayOwn(startingOwner))
		{
			TBD_Log.Warn(TBD_ZoneVolume.CH, string.Format("objective '%1' rules.startingOwner='%2' is excluded by zones[].faction='%3' -- leaving the objective NEUTRAL",
				objective.m_sId, startingOwner, objective.m_sFaction));
			return;
		}

		objective.m_sOwner = startingOwner;
		objective.m_sProgressFaction = startingOwner;
		objective.m_fProgress = objective.m_fCaptureSeconds;

		TBD_Log.Kv(TBD_ZoneVolume.CH, "startingOwner", string.Format("id=%1 owner=%2",
			objective.m_sId, startingOwner));
	}

	//! `all_objectives_captured`: at least one usable capture objective exists and every one is
	//! owned by the same side.
	//! @param prepared the prepared objectives
	//! @param winnerFaction set to that side, or empty when the condition is not met
	//! @return true when the condition is met
	//! @authority server
	override bool HasEnded(notnull array<ref TBD_Objective> prepared, out string winnerFaction)
	{
		winnerFaction = string.Empty;

		string owner;
		int considered = 0;

		foreach (TBD_Objective objective : prepared)
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

	//! Advance the capture by one tick through `TBD_ObjectiveCaptureProgress`.
	//! @param objective a usable capture objective
	//! @return the CAPTURED chat line on the tick a side takes it, else empty
	//! @authority server
	override string Advance(notnull TBD_Objective objective)
	{
		return TBD_ObjectiveCaptureProgress.AdvanceCapture(objective);
	}

	//! CAPTURE status: `neutral`, `OURS` or `held by <side>`, then ` -- CONTESTED` or the
	//! partial progress percentage and the side banking it.
	//! @param objective a usable capture objective
	//! @param viewerFaction the viewer's side from server-owned state; may be empty
	//! @return the status text
	override string StatusText(notnull TBD_Objective objective, string viewerFaction)
	{
		string text;

		if (objective.m_sOwner.IsEmpty())
		{
			text = "neutral";
		}
		else if (!viewerFaction.IsEmpty() && objective.m_sOwner == viewerFaction)
		{
			text = "OURS";
		}
		else
		{
			text = "held by ";
			text += objective.m_sOwner;
		}

		if (objective.m_bContested)
		{
			text += " -- CONTESTED";
			return text;
		}

		int percent = objective.ProgressPercent();
		if (percent > 0 && percent < 100)
		{
			text += " -- ";
			text += percent.ToString();
			text += "% ";
			text += objective.m_sProgressFaction;
		}

		return text;
	}

	//! `o` neutral, `+` owned by the viewer's side, `-` owned by another side.
	//! @param objective a usable capture objective, neither complete nor contested
	//! @param viewerFaction the viewer's side from server-owned state; may be empty
	//! @return the glyph
	override string HudIcon(notnull TBD_Objective objective, string viewerFaction)
	{
		if (objective.m_sOwner.IsEmpty())
			return "o";
		if (!viewerFaction.IsEmpty() && objective.m_sOwner == viewerFaction)
			return "+";
		return "-";
	}

	//! A player standing in a capture objective is shown its capture bar.
	//! @return true
	override bool ClaimsCaptureBar() { return true; }
}
