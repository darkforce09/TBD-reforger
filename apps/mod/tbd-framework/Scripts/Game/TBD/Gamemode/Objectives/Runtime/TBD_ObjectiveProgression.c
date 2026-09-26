/**
 * @file TBD_ObjectiveProgression.c
 * @brief Advances capture, hold and destroy objectives by one 1 Hz tick from the presence sample.
 *
 * Role: the objective rules in motion: capture build and teardown, empty-zone decay, the hold
 * clock with its pause and reset rules, and destroy completion; collects the tick's completion
 * lines for chat.  Position: owned by `TBD_ObjectivesComponent`, which samples presence, calls
 * `Advance` for each usable objective while the stage is LIVE, then delivers
 * `GetCompletionLines`; logs on the registry's `Obj` channel.
 * State: this tick's completion lines, owned by the component on the server.  Invariants: rates
 * never scale with headcount; taking an enemy-held objective is neutralise then capture; an owned
 * objective is lost only to a side standing on it, never to a timer; a contest freezes progress
 * in both directions; only CAPTURED, DESTROYED and HELD reach chat.
 */

//! Per-tick objective advancement for one objectives component.
class TBD_ObjectiveProgression : Managed
{
	protected ref array<string> m_aCompletionLines; //!< this tick's completion lines (captured, destroyed, held); progress goes to the HUD

	//! Allocate the completion line buffer.
	void TBD_ObjectiveProgression()
	{
		m_aCompletionLines = new array<string>();
	}

	//! Start a tick: drop the previous tick's completion lines.
	void BeginTick()
	{
		m_aCompletionLines.Clear();
	}

	//! This tick's completion lines, in the order the objectives completed.
	array<string> GetCompletionLines()
	{
		return m_aCompletionLines;
	}

	//! Advance one usable objective by one tick, by its kind.
	void Advance(notnull TBD_Objective objective)
	{
		if (objective.m_eKind == TBD_EObjectiveKind.CAPTURE)
			AdvanceCapture(objective);
		else if (objective.m_eKind == TBD_EObjectiveKind.HOLD_UNTIL)
			AdvanceHold(objective);
		else
			AdvanceDestroy(objective);
	}

	//! Advance one capture objective: freeze on a contest, apply the empty-zone rule when nobody
	//! is inside, else tear down an enemy's progress or ownership, else build toward ownership.
	//! The first stage costs `neutralizeSeconds` (default `captureSeconds`, 0 = instant).
	protected void AdvanceCapture(notnull TBD_Objective objective)
	{
		// Also sets `m_bContested` from the objective's presence sample and `contestable` rule.
		string acting = TBD_ZoneVolume.ResolveActingFaction(objective);

		if (objective.m_bContested != objective.m_bAnnouncedContested)
		{
			objective.m_bAnnouncedContested = objective.m_bContested;
			if (objective.m_bContested)
			{
				TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "contested", string.Format("id=%1 sides=%2 progress=%3s owner='%4'",
					objective.m_sId, objective.PresentFactionCount(), objective.m_fProgress, objective.m_sOwner));
			}
		}

		if (acting.IsEmpty())
		{
			// Frozen by a contest: nothing moves in either direction. That IS the design.
			if (objective.m_bContested)
				return;

			AdvanceCaptureEmpty(objective);
			return;
		}

		objective.m_fSinceAnnounce = objective.m_fSinceAnnounce + TBD_ObjectivesComponent.TICK_SECONDS;

		if (IsTearingDown(objective, acting))
		{
			TearDownCapture(objective, acting);
			return;
		}

		BuildCapture(objective, acting);
	}

	//! Nobody is inside: under DECAY, drain a neutral objective's partial progress; an owned
	//! objective never decays.
	protected void AdvanceCaptureEmpty(notnull TBD_Objective objective)
	{
		if (objective.m_eOnEmpty != TBD_EObjectiveOnEmpty.DECAY)
			return;

		// A captured objective never decays out of its owner's hands: losing ground must require
		// somebody to walk onto it, or a side could lose an objective while nobody was near it.
		if (!objective.m_sOwner.IsEmpty())
			return;

		if (objective.m_fProgress <= 0)
			return;

		objective.m_fProgress = objective.m_fProgress - (objective.m_fDecayRate * TBD_ObjectivesComponent.TICK_SECONDS);
		if (objective.m_fProgress > 0)
			return;

		objective.m_fProgress = 0;
		objective.m_sProgressFaction = string.Empty;
	}

	//! Whether `acting` tears down rather than builds: the objective is owned by another side, or
	//! another side banked the partial progress. Both use the one teardown rate.
	protected bool IsTearingDown(notnull TBD_Objective objective, string acting)
	{
		if (!objective.m_sOwner.IsEmpty() && objective.m_sOwner != acting)
			return true;

		if (objective.m_sOwner.IsEmpty() && !objective.m_sProgressFaction.IsEmpty() && objective.m_sProgressFaction != acting)
			return true;

		return false;
	}

	//! Tear down one tick of another side's progress; at zero the objective returns to neutral and
	//! a former owner is logged.
	protected void TearDownCapture(notnull TBD_Objective objective, string acting)
	{
		// `neutralizeSeconds: 0` means instant. Handled here rather than through `TeardownRate()` so
		// no arithmetic is ever done on the sentinel that function returns for the zero case.
		float step;
		if (objective.m_fNeutralizeSeconds <= 0)
		{
			step = objective.m_fProgress;
		}
		else
		{
			step = objective.TeardownRate() * TBD_ObjectivesComponent.TICK_SECONDS;
		}

		objective.m_fProgress = objective.m_fProgress - step;

		if (objective.m_fProgress > 0)
		{
			AnnounceCaptureProgress(objective, acting, "neutralising");
			return;
		}

		objective.m_fProgress = 0;

		string previousOwner = objective.m_sOwner;
		objective.m_sOwner = string.Empty;
		objective.m_sProgressFaction = string.Empty;
		objective.m_fSinceAnnounce = 0;

		if (previousOwner.IsEmpty())
			return;

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "neutralised", string.Format("id=%1 by=%2 previousOwner=%3",
			objective.m_sId, acting, previousOwner));
	}

	//! Build one tick of `acting`'s progress. A side the zone's `faction` excludes never builds;
	//! a full bar captures the objective and queues the CAPTURED line, unless it was the owner's
	//! own bar being restored.
	protected void BuildCapture(notnull TBD_Objective objective, string acting)
	{
		// A side the zone's `faction` excludes can tear down but never bank. It has already done
		// whatever tearing down there was to do; there is nothing further for it here.
		if (!objective.MayOwn(acting))
			return;

		// Already theirs and full.
		if (objective.m_sOwner == acting && objective.m_fProgress >= objective.m_fCaptureSeconds)
			return;

		objective.m_sProgressFaction = acting;
		objective.m_fProgress = objective.m_fProgress + TBD_ObjectivesComponent.TICK_SECONDS;

		if (objective.m_fProgress < objective.m_fCaptureSeconds)
		{
			AnnounceCaptureProgress(objective, acting, "capturing");
			return;
		}

		objective.m_fProgress = objective.m_fCaptureSeconds;
		objective.m_fSinceAnnounce = 0;

		// Rebuilt after a partial teardown by an enemy who then left. The objective never changed
		// hands, so this is not a capture and must not be announced as one.
		if (objective.m_sOwner == acting)
		{
			TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "restored", string.Format("id=%1 owner=%2", objective.m_sId, acting));
			return;
		}

		objective.m_sOwner = acting;

		string msg = "TBD: ";
		msg += objective.DisplayName();
		msg += " has been CAPTURED by ";
		msg += acting;
		msg += ".";
		m_aCompletionLines.Insert(msg);

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "captured", string.Format("id=%1 owner=%2 points=%3",
			objective.m_sId, acting, objective.m_fPoints));
	}

	//! Log capture progress every `m_fAnnounceEverySeconds`; the HUD bar is the player channel.
	//! @param verb `capturing` or `neutralising`
	protected void AnnounceCaptureProgress(notnull TBD_Objective objective, string acting, string verb)
	{
		if (objective.m_fSinceAnnounce < objective.m_fAnnounceEverySeconds)
			return;

		objective.m_fSinceAnnounce = 0;

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "captureProgress", string.Format("id=%1 verb=%2 percent=%3 acting=%4",
			objective.m_sId, verb, objective.ProgressPercent(), acting));
	}

	//! Advance one hold objective. By default an enemy inside pauses the clock, so the drawn zone
	//! matters: `pauseOnEnemy: false` makes it a pure timer and `resetOnEnemy: true` restarts it.
	//! `requireHolderPresent` defaults to false because a one-life hold that needs a manned zone
	//! becomes unwinnable after casualties. When the clock runs out the holder wins and the HELD
	//! line is queued.
	protected void AdvanceHold(notnull TBD_Objective objective)
	{
		if (objective.m_bComplete)
			return;

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
			return;

		objective.m_fHeldSeconds = objective.m_fHeldSeconds + TBD_ObjectivesComponent.TICK_SECONDS;

		if (objective.m_fHeldSeconds < objective.m_fHoldSeconds)
		{
			AnnounceHoldMark(objective);
			return;
		}

		objective.m_fHeldSeconds = objective.m_fHoldSeconds;
		objective.m_bComplete = true;

		string msg = "TBD: ";
		msg += objective.DisplayName();
		msg += " has been HELD to the clock by ";
		msg += objective.m_sFaction;
		msg += ".";
		m_aCompletionLines.Insert(msg);

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "holdExpired", string.Format("id=%1 holder=%2 held=%3s points=%4",
			objective.m_sId, objective.m_sFaction, objective.m_fHoldSeconds, objective.m_fPoints));
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

	//! Advance one destroy objective and queue the DESTROYED line on the tick it completes. The
	//! counting lives in `TBD_ObjectiveDestroyTargets.EvaluateDestroy`.
	protected void AdvanceDestroy(notnull TBD_Objective objective)
	{
		if (!TBD_ObjectiveDestroyTargets.EvaluateDestroy(objective))
			return;

		string msg = "TBD: ";
		msg += objective.DisplayName();
		msg += " has been DESTROYED.";
		m_aCompletionLines.Insert(msg);

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "destroyed", string.Format("id=%1 alias='%2' destroyed=%3/%4 by=%5 points=%6",
			objective.m_sId,
			objective.m_sTargetAlias,
			objective.m_iTargetsDestroyed,
			objective.RequiredKills(),
			objective.m_sFaction,
			objective.m_fPoints));
	}
}
