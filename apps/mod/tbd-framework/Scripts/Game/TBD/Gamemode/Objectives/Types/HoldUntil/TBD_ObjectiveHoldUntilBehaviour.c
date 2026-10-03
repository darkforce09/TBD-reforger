/**
 * @file TBD_ObjectiveHoldUntilBehaviour.c
 * @brief The `objective_hold_until` behaviour: ground a holder must keep until a clock runs out.
 *
 * Role: answers every hook of the hold kind: its zone type, end trigger and task type, its
 * announcement default and rules, the holder a typed row's side supplies, load log, starting
 * owner, the `hold_expired` condition, the announcement ladder on the LIVE edge, the per-tick
 * hold clock, its status text and HUD glyph.  Position: found through
 * `TBD_ObjectiveKindBehaviour.For` and its sibling lookups; asks `TBD_ZoneVolume.EnemyContestsHold`
 * and `TBD_ZoneVolume.HolderPresent` about presence; logs on the registry's `Obj` channel and the
 * zone volume's `ZoneVol` channel.
 * State: none; every objective's state lives on `TBD_Objective`.  Invariants: a hold objective
 * without a holder or a length inside 0..`TBD_ObjectiveRuleResolver.MAX_DURATION_SECONDS` goes
 * inert (a guessed round length decides when the round ends); the zone's faction holds its ground,
 * so the side of an untyped hold defends; rates never scale with headcount; the holder wins when
 * the clock runs out.
 */

//! Hold-until objective behaviour.
class TBD_ObjectiveHoldUntilBehaviour : TBD_ObjectiveKindBehaviour
{
	static const string ZONE_TYPE   = "objective_hold_until"; //!< `zones[].type` of a hold objective
	static const string END_TRIGGER = "hold_expired"; //!< `winConditions.endOn`: any usable hold objective ran its clock out
	static const string TASK_HOLD   = "hold"; //!< `objectives[].type` value
	static const float DEFAULT_ANNOUNCE_SECONDS = 60.0; //!< seconds between hold progress log lines

	//! @return HOLD_UNTIL
	override TBD_EObjectiveKind Kind() { return TBD_EObjectiveKind.HOLD_UNTIL; }

	//! @return `objective_hold_until`
	override string ZoneType() { return ZONE_TYPE; }

	//! @return `hold_expired`
	override string EndTrigger() { return END_TRIGGER; }

	//! `hold` belongs on a hold zone.
	//! @param taskType an `objectives[].type` value
	//! @return true for `hold`
	override bool AcceptsTaskType(string taskType)
	{
		return taskType == TASK_HOLD;
	}

	//! @return `hold`
	override string CountLabel() { return "hold"; }

	//! The note that the hold objectives are tracked and end nothing.
	//! @param usableCount how many objectives of this kind are usable; above 0
	//! @return the note text
	override string UndeclaredTriggerNote(int usableCount)
	{
		return string.Format("%1 hold objective(s) but endOn does not declare '%2' -- tracked, will not end the round", usableCount, END_TRIGGER);
	}

	//! The hold announcement cadence default.
	//! @param objective the objective being prepared
	override void ApplyDefaults(notnull TBD_Objective objective)
	{
		objective.m_fAnnounceEverySeconds = DEFAULT_ANNOUNCE_SECONDS;
	}

	//! `objective_hold_until`: requires a holding `faction` and a `holdSeconds` inside
	//! 0..`TBD_ObjectiveRuleResolver.MAX_DURATION_SECONDS`, else the objective goes inert; then
	//! the pause, reset and presence rules.
	//! @param objective the objective being prepared
	//! @param rules the zone's rules from the rules pass; null makes the objective inert
	//! @param subject the zone id, or `zones[<index>]`, for log lines
	override void ResolveRules(notnull TBD_Objective objective, TBD_ObjectiveRulesStruct rules, string subject)
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

		if (rules.holdSeconds <= 0 || rules.holdSeconds > TBD_ObjectiveRuleResolver.MAX_DURATION_SECONDS)
		{
			objective.m_bUsable = false;
			objective.m_sInertReason = string.Format("rules.holdSeconds=%1 is outside 0..%2", rules.holdSeconds, TBD_ObjectiveRuleResolver.MAX_DURATION_SECONDS);
			return;
		}

		objective.m_fHoldSeconds = rules.holdSeconds;
		objective.m_bPauseOnEnemy = rules.pauseOnEnemy;
		objective.m_bResetOnEnemy = rules.resetOnEnemy;
		objective.m_bRequireHolderPresent = rules.requireHolderPresent;
	}

	//! A hold zone's faction holds its ground, so the side of an untyped hold defends it.
	//! @return true
	override bool SideDefendsByDefault() { return true; }

	//! Let an authored `side` supply the holder of a hold zone that names no `faction`, which
	//! would otherwise be inert.
	//! @param objective the objective being bound
	//! @param subject the zone id, or `zones[<index>]`, for log lines
	override void SeedFromSide(notnull TBD_Objective objective, string subject)
	{
		if (!objective.m_sFaction.IsEmpty() || objective.m_sSide.IsEmpty())
			return;

		if (!TBD_DeclaredFactions.Exists(objective.m_sSide))
			return;

		objective.m_sFaction = objective.m_sSide;

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "note", string.Format("objective_hold_until '%1' names no zones[].faction; objectives[].side='%2' supplies the holder. Without it this objective would be INERT and 'hold_expired' could never fire.",
			subject, objective.m_sSide));
	}

	//! Log the hold length, the holder, the pause, reset and presence rules and `points`.
	//! @param objective a usable prepared objective
	override void LogRules(notnull TBD_Objective objective)
	{
		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "objectiveRules", string.Format("id=%1 hold=%2s holder='%3' pauseOnEnemy=%4 resetOnEnemy=%5 requireHolderPresent=%6 points=%7",
			objective.m_sId,
			objective.m_fHoldSeconds,
			objective.m_sFaction,
			objective.m_bPauseOnEnemy,
			objective.m_bResetOnEnemy,
			objective.m_bRequireHolderPresent,
			objective.m_fPoints));
	}

	//! A `startingOwner` other than the holder is logged and ignored; the holder is the zone's
	//! faction.
	//! @param objective the objective being prepared
	//! @param startingOwner the zone's `rules.startingOwner`
	override void ApplyStartingOwner(notnull TBD_Objective objective, string startingOwner)
	{
		if (startingOwner != objective.m_sFaction)
		{
			TBD_Log.Warn(TBD_ZoneVolume.CH, string.Format("objective '%1' rules.startingOwner='%2' is ignored on objective_hold_until (holder is zones[].faction='%3')",
				objective.m_sId, startingOwner, objective.m_sFaction));
		}
	}

	//! `hold_expired`: any usable hold objective ran its clock out. The winner is the holder.
	//! @param prepared the prepared objectives
	//! @param winnerFaction set to the holding side
	//! @return true when the condition is met
	//! @authority server
	override bool HasEnded(notnull array<ref TBD_Objective> prepared, out string winnerFaction)
	{
		winnerFaction = string.Empty;

		foreach (TBD_Objective objective : prepared)
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

	//! Skip every rung of the announcement ladder at or above the hold's length, so a short hold
	//! does not log them all at once.
	//! @param objective a usable hold objective
	//! @authority server
	override void OnEnterLive(notnull TBD_Objective objective)
	{
		// Skip every announcement mark that is at or above the total hold length.
		while (NextHoldMark(objective.m_iHoldMarkIndex) > 0
			&& NextHoldMark(objective.m_iHoldMarkIndex) >= objective.m_fHoldSeconds)
		{
			objective.m_iHoldMarkIndex = objective.m_iHoldMarkIndex + 1;
		}
	}

	//! Advance one hold objective. By default an enemy inside pauses the clock, so the drawn zone
	//! matters: `pauseOnEnemy: false` makes it a pure timer and `resetOnEnemy: true` restarts it.
	//! `requireHolderPresent` defaults to false because a one-life hold that needs a manned zone
	//! becomes unwinnable after casualties. When the clock runs out the holder wins and the HELD
	//! line is handed back.
	//! @param objective a usable hold objective
	//! @return the HELD chat line on the tick the clock runs out, else empty
	//! @authority server
	override string Advance(notnull TBD_Objective objective)
	{
		if (objective.m_bComplete)
			return "";

		bool enemyPresent = TBD_ZoneVolume.EnemyContestsHold(objective);
		bool holderPresent = TBD_ZoneVolume.HolderPresent(objective);

		objective.m_bContested = enemyPresent;

		bool paused = false;

		if (enemyPresent && objective.m_bResetOnEnemy)
		{
			paused = true;
			if (objective.m_fHeldSeconds > 0)
			{
				TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "holdReset", string.Format("id=%1 lost=%2s to an enemy incursion",
					objective.m_sId, objective.m_fHeldSeconds));
			}
			objective.m_fHeldSeconds = 0;
		}
		else if (enemyPresent && objective.m_bPauseOnEnemy)
		{
			paused = true;
		}
		else if (objective.m_bRequireHolderPresent && !holderPresent)
		{
			paused = true;
		}

		if (paused != objective.m_bHoldPaused)
		{
			objective.m_bHoldPaused = paused;

			TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "holdPaused", string.Format("id=%1 paused=%2 held=%3s enemyPresent=%4",
				objective.m_sId, paused, objective.m_fHeldSeconds, enemyPresent));
		}

		if (paused)
			return "";

		objective.m_fHeldSeconds = objective.m_fHeldSeconds + TBD_ObjectivesComponent.TICK_SECONDS;

		if (objective.m_fHeldSeconds < objective.m_fHoldSeconds)
		{
			AnnounceHoldMark(objective);
			return "";
		}

		objective.m_fHeldSeconds = objective.m_fHoldSeconds;
		objective.m_bComplete = true;

		string msg = "TBD: ";
		msg += objective.DisplayName();
		msg += " has been HELD to the clock by ";
		msg += objective.m_sFaction;
		msg += ".";

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "holdExpired", string.Format("id=%1 holder=%2 held=%3s points=%4",
			objective.m_sId, objective.m_sFaction, objective.m_fHoldSeconds, objective.m_fPoints));

		return msg;
	}

	//! Log the hold clock at the rungs of the remaining-time ladder (`NextHoldMark`) rather than
	//! on a fixed interval.
	protected void AnnounceHoldMark(notnull TBD_Objective objective)
	{
		float mark = NextHoldMark(objective.m_iHoldMarkIndex);
		if (mark <= 0)
			return;

		float remaining = objective.HoldRemaining();
		if (remaining > mark)
			return;

		objective.m_iHoldMarkIndex = objective.m_iHoldMarkIndex + 1;

		// Rounded into an int first, so the log shows no float precision.
		int whole = Math.Round(remaining);

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "holdMark", string.Format("id=%1 remain=%2s holder=%3",
			objective.m_sId, whole, objective.m_sFaction));
	}

	//! The remaining-time ladder, in seconds: 600, 300, 120, 60, 30, 10.
	//! @param index the rung
	//! @return the rung's seconds, or -1 past the end
	static float NextHoldMark(int index)
	{
		if (index == 0)
			return 600;
		if (index == 1)
			return 300;
		if (index == 2)
			return 120;
		if (index == 3)
			return 60;
		if (index == 4)
			return 30;
		if (index == 5)
			return 10;

		return -1;
	}

	//! HOLD_UNTIL status: `HELD`, or `hold <n>s left`, with ` (PAUSED)` while the clock stands.
	//! @param objective a usable hold objective
	//! @param viewerFaction the viewer's side; the hold status reads the same for every side
	//! @return the status text
	override string StatusText(notnull TBD_Objective objective, string viewerFaction)
	{
		if (objective.m_bComplete)
			return "HELD";

		// Rounded into an int first, so the text never shows float precision ("600.000000").
		int remaining = Math.Round(objective.HoldRemaining());

		string text = "hold ";
		text += remaining.ToString();
		text += "s left";
		if (objective.m_bHoldPaused)
			text += " (PAUSED)";

		return text;
	}

	//! `H` for a hold objective.
	//! @param objective a usable hold objective, neither complete nor contested
	//! @param viewerFaction the viewer's side; the glyph reads the same for every side
	//! @return the glyph
	override string HudIcon(notnull TBD_Objective objective, string viewerFaction)
	{
		return "H";
	}
}
