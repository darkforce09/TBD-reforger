/**
 * @file TBD_ObjectiveCaptureProgress.c
 * @brief Advances one capture objective by one 1 Hz tick from its presence sample.
 *
 * Role: capture build and teardown, empty-zone decay, contest logging and progress logging of
 * one `objective_capture` objective; hands back the CAPTURED chat line on the tick a side takes
 * it.  Position: called by `TBD_ObjectiveCaptureBehaviour.Advance` once per tick while the stage
 * is LIVE; asks `TBD_ZoneVolume.ResolveActingFaction` which side acts; logs on the registry's
 * `Obj` channel.
 * State: none; writes only the objective it is given.  Invariants: rates never scale with
 * headcount; taking an enemy-held objective is neutralise then capture; an owned objective is
 * lost only to a side standing on it, never to a timer; a contest freezes progress in both
 * directions; restoring the owner's own bar is never announced as a capture.
 */

//! Per-tick capture advancement.
class TBD_ObjectiveCaptureProgress
{
	//! Advance one capture objective: freeze on a contest, apply the empty-zone rule when nobody
	//! is inside, else tear down an enemy's progress or ownership, else build toward ownership.
	//! The first stage costs `neutralizeSeconds` (default `captureSeconds`, 0 = instant).
	//! @param objective a usable capture objective
	//! @return the CAPTURED chat line on the tick a side takes it, else empty
	//! @authority server
	static string AdvanceCapture(notnull TBD_Objective objective)
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
				return "";

			AdvanceCaptureEmpty(objective);
			return "";
		}

		objective.m_fSinceAnnounce = objective.m_fSinceAnnounce + TBD_ObjectivesComponent.TICK_SECONDS;

		if (IsTearingDown(objective, acting))
		{
			TearDownCapture(objective, acting);
			return "";
		}

		return BuildCapture(objective, acting);
	}

	//! Nobody is inside: under DECAY, drain a neutral objective's partial progress; an owned
	//! objective never decays.
	protected static void AdvanceCaptureEmpty(notnull TBD_Objective objective)
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
	protected static bool IsTearingDown(notnull TBD_Objective objective, string acting)
	{
		if (!objective.m_sOwner.IsEmpty() && objective.m_sOwner != acting)
			return true;

		if (objective.m_sOwner.IsEmpty() && !objective.m_sProgressFaction.IsEmpty() && objective.m_sProgressFaction != acting)
			return true;

		return false;
	}

	//! Tear down one tick of another side's progress; at zero the objective returns to neutral and
	//! a former owner is logged.
	protected static void TearDownCapture(notnull TBD_Objective objective, string acting)
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
	//! a full bar captures the objective and hands back the CAPTURED line, unless it was the
	//! owner's own bar being restored.
	//! @return the CAPTURED chat line, or empty
	protected static string BuildCapture(notnull TBD_Objective objective, string acting)
	{
		// A side the zone's `faction` excludes can tear down but never bank. It has already done
		// whatever tearing down there was to do; there is nothing further for it here.
		if (!objective.MayOwn(acting))
			return "";

		// Already theirs and full.
		if (objective.m_sOwner == acting && objective.m_fProgress >= objective.m_fCaptureSeconds)
			return "";

		objective.m_sProgressFaction = acting;
		objective.m_fProgress = objective.m_fProgress + TBD_ObjectivesComponent.TICK_SECONDS;

		if (objective.m_fProgress < objective.m_fCaptureSeconds)
		{
			AnnounceCaptureProgress(objective, acting, "capturing");
			return "";
		}

		objective.m_fProgress = objective.m_fCaptureSeconds;
		objective.m_fSinceAnnounce = 0;

		// Rebuilt after a partial teardown by an enemy who then left. The objective never changed
		// hands, so this is not a capture and must not be announced as one.
		if (objective.m_sOwner == acting)
		{
			TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "restored", string.Format("id=%1 owner=%2", objective.m_sId, acting));
			return "";
		}

		objective.m_sOwner = acting;

		string msg = "TBD: ";
		msg += objective.DisplayName();
		msg += " has been CAPTURED by ";
		msg += acting;
		msg += ".";

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "captured", string.Format("id=%1 owner=%2 points=%3",
			objective.m_sId, acting, objective.m_fPoints));

		return msg;
	}

	//! Log capture progress every `m_fAnnounceEverySeconds`; the HUD bar is the player channel.
	//! @param verb `capturing` or `neutralising`
	protected static void AnnounceCaptureProgress(notnull TBD_Objective objective, string acting, string verb)
	{
		if (objective.m_fSinceAnnounce < objective.m_fAnnounceEverySeconds)
			return;

		objective.m_fSinceAnnounce = 0;

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "captureProgress", string.Format("id=%1 verb=%2 percent=%3 acting=%4",
			objective.m_sId, verb, objective.ProgressPercent(), acting));
	}
}
