/**
 * @file TBD_Objective.c
 * @brief One prepared mission objective: its resolved rules, live progress and per-side framing.
 *
 * Role: the runtime record of one objective zone, and the presence, ownership and role queries
 * the objective runtime asks of it.  Position: built by `TBD_ObjectiveRegistry` and its binders;
 * advanced by `TBD_ObjectiveProgression`; read by the HUD publisher, `TBD_ObjectiveText`,
 * `TBD_ZoneVolume` and `TBD_TriggerRuntime`.
 * State: every field is server state for the world that built it.  Invariants: containment is
 * never tested here (`m_Zone` owns the shape); an untyped objective leaves every typed field
 * empty; presence is sampled at 1 Hz against each body's origin, footprint only (Y ignored).
 */

//! One prepared objective with its resolved rules and live state. `m_Zone` is a strong
//! reference, so the objective never points at a collected zone whichever component tears its
//! registry down first.
class TBD_Objective
{
	ref TBD_Zone m_Zone; //!< prepared zone: shape, bounds and containment
	TBD_EObjectiveKind m_eKind; //!< kind resolved from `zones[].type`
	string m_sId; //!< `zones[].id`
	string m_sLabel; //!< display label: `objectives[].label` when typed, else `zones[].label`
	string m_sFaction; //!< `zones[].faction`: CAPTURE the only side that may own it (empty = any); HOLD_UNTIL the holder (required); DESTROY the side told to destroy it
	bool m_bUsable; //!< false when the objective cannot run; an inert objective neither fires nor blocks an end trigger
	string m_sInertReason; //!< operator-facing reason the objective is inert; empty while usable

	float m_fCaptureSeconds; //!< `rules.captureSeconds`, seconds; default 120
	float m_fNeutralizeSeconds; //!< `rules.neutralizeSeconds`, seconds; default `m_fCaptureSeconds`, 0 = instant
	bool m_bContestable; //!< `rules.contestable`; default true
	TBD_EObjectiveOnEmpty m_eOnEmpty; //!< `rules.onEmpty`; default HOLD
	float m_fDecayRate; //!< `rules.decayRate`, progress-seconds lost per second; default 1
	float m_fHoldSeconds; //!< `rules.holdSeconds`, seconds; required on HOLD_UNTIL
	bool m_bPauseOnEnemy; //!< `rules.pauseOnEnemy`; default true
	bool m_bResetOnEnemy; //!< `rules.resetOnEnemy`; default false
	bool m_bRequireHolderPresent; //!< `rules.requireHolderPresent`; default false
	string m_sTargetAlias; //!< `rules.targetAlias`, a registry alias; required on DESTROY
	int m_iTargetCount; //!< `rules.targetCount`; 0 = every target found at LIVE
	float m_fPoints; //!< `rules.points`; carried and logged, read by no end trigger
	float m_fAnnounceEverySeconds; //!< `rules.announceEverySeconds`, seconds; default 15 (capture) or 60 (hold)

	string m_sOwner; //!< Owning faction, or empty for neutral. Only ever changes when somebody stands on the zone.
	string m_sProgressFaction; //!< Whose banked progress `m_fProgress` is. Empty when progress is zero.
	float m_fProgress; //!< Seconds banked toward `m_fCaptureSeconds`, in [0, m_fCaptureSeconds].
	bool m_bContested; //!< Two or more factions present and the rules say that freezes things.

	float m_fHeldSeconds; //!< seconds the holder has held, in [0, m_fHoldSeconds]
	bool m_bHoldPaused; //!< the hold clock stood still on the last tick

	bool m_bArmed; //!< the destroy-target search ran; it runs once, on the first LIVE tick
	ResourceName m_TargetResource; //!< resolved prefab of `m_sTargetAlias`; empty when the alias did not resolve
	int m_iTargetsFound; //!< targets found inside the zone at LIVE
	int m_iTargetsDestroyed; //!< targets found at LIVE that are now destroyed or gone
	bool m_bComplete; //!< DESTROY or HOLD_UNTIL finished; a CAPTURE objective is never complete, ownership can flip all round
	float m_fSinceAnnounce; //!< Seconds since the last progress message to the players standing on this objective.
	bool m_bAnnouncedContested; //!< contested state last logged, so only a transition is logged
	int m_iHoldMarkIndex; //!< next rung of the hold announcement ladder (`TBD_ObjectiveProgression.NextHoldMark`)
	string m_sPendingInsideMessage; //!< unused message slot, cleared by every `BeginSample`

	ref array<string> m_aPresentFactions; //!< this tick's present faction keys, parallel to `m_aPresentCounts`; reused every tick
	ref array<int> m_aPresentCounts; //!< this tick's living-body count per present faction
	ref array<int> m_aPresentPlayers; //!< player ids sampled inside this tick, so containment is tested once per player

	bool m_bTyped; //!< True when a top-level `objectives[]` row named this objective's zone in `zoneId`.
	string m_sEntityId; //!< `objectives[].id`, the objective's own identity, kept apart from the zone id `m_sId`
	string m_sTaskType; //!< `objectives[].type` as authored (capture|destroy|hold|defend); `m_eKind` is what runs
	string m_sSide; //!< `objectives[].side`: who the task is for, a different claim from ownership `m_sFaction`
	bool m_bInvalidSide; //!< `objectives[].side` names no faction: framing reads NEUTRAL, with no zone-faction fallback
	bool m_bSideDefends; //!< the side the objective is for defends it (`hold`, `defend`, or an untyped HOLD_UNTIL)
	bool m_bLocked; //!< `objectives[].lock`; carried and reported, never enforced
	string m_sAutoLoseFaction; //!< `objectives[].autoLose`; validated against `factions[].key`, carried and reported, never enforced
	string m_sAttackerTitle; //!< `framing.attacker.title`; empty when unframed
	string m_sAttackerText; //!< `framing.attacker.text`; empty when unframed
	string m_sDefenderTitle; //!< `framing.defender.title`; empty when unframed
	string m_sDefenderText; //!< `framing.defender.text`; empty when unframed

	//! Allocate the reusable presence sample arrays.
	void TBD_Objective()
	{
		m_aPresentFactions = new array<string>();
		m_aPresentCounts = new array<int>();
		m_aPresentPlayers = new array<int>();
	}

	//! The name a player is shown.
	//! @return the label, else the zone id, else the kind name; never empty
	string DisplayName()
	{
		if (!m_sLabel.IsEmpty())
			return m_sLabel;
		if (!m_sId.IsEmpty())
			return m_sId;

		return typename.EnumToString(TBD_EObjectiveKind, m_eKind);
	}

	//! Start a fresh presence sample for this tick and clear the message slot. Runs for every
	//! objective, usable or not, so nothing lingers on one that went inert.
	void BeginSample()
	{
		m_aPresentFactions.Clear();
		m_aPresentCounts.Clear();
		m_aPresentPlayers.Clear();
		m_sPendingInsideMessage = string.Empty;
	}

	//! Count one living body of `factionKey` inside this objective. An empty key is dropped: a
	//! player on no side neither captures nor contests.
	void AddPresence(string factionKey)
	{
		if (factionKey.IsEmpty())
			return;

		int at = m_aPresentFactions.Find(factionKey);
		if (at == -1)
		{
			m_aPresentFactions.Insert(factionKey);
			m_aPresentCounts.Insert(1);
			return;
		}

		m_aPresentCounts[at] = m_aPresentCounts[at] + 1;
	}

	//! How many living bodies of `factionKey` the current sample holds.
	//! @return the count; 0 for an empty or absent key
	int PresenceOf(string factionKey)
	{
		if (factionKey.IsEmpty())
			return 0;

		int at = m_aPresentFactions.Find(factionKey);
		if (at == -1)
			return 0;

		return m_aPresentCounts[at];
	}

	//! How many distinct sides the current sample holds.
	int PresentFactionCount()
	{
		return m_aPresentFactions.Count();
	}

	//! Whether any side other than `factionKey` is inside in the current sample.
	bool HasEnemyPresent(string factionKey)
	{
		foreach (int index, string present : m_aPresentFactions)
		{
			if (present != factionKey && m_aPresentCounts[index] > 0)
				return true;
		}

		return false;
	}

	//! The side acting on this objective this tick, and sets `m_bContested`.
	//! Contestable (the default): two or more sides inside freezes the objective. Not
	//! contestable: the side with more living bodies acts and an exact tie freezes. Headcount
	//! never scales the rate.
	//! @return the acting faction key, or empty when nobody is inside or it is frozen
	string ResolveActingFaction()
	{
		int sides = m_aPresentFactions.Count();

		m_bContested = false;

		if (sides == 0)
			return string.Empty;

		if (sides == 1)
			return m_aPresentFactions[0];

		if (m_bContestable)
		{
			m_bContested = true;
			return string.Empty;
		}

		// Not contestable: weight of numbers, ties freeze.
		int best = -1;
		int bestCount = 0;
		bool tied = false;
		foreach (int index, int count : m_aPresentCounts)
		{
			if (count > bestCount)
			{
				bestCount = count;
				best = index;
				tied = false;
				continue;
			}

			if (count == bestCount)
				tied = true;
		}

		if (best == -1 || tied)
		{
			m_bContested = true;
			return string.Empty;
		}

		return m_aPresentFactions[best];
	}

	//! Whether `factionKey` may ever own this objective (see `m_sFaction`).
	//! @return false for an empty key; true when the zone names no faction or names this one
	bool MayOwn(string factionKey)
	{
		if (factionKey.IsEmpty())
			return false;

		if (m_sFaction.IsEmpty())
			return true;

		return m_sFaction == factionKey;
	}

	//! Capture progress as a whole percentage of the bar.
	//! @return 0..100; 0 when the capture length is not positive
	int ProgressPercent()
	{
		if (m_fCaptureSeconds <= 0)
			return 0;

		float fraction = m_fProgress / m_fCaptureSeconds;
		return Math.Round(fraction * 100);
	}

	//! Seconds of hold still to run.
	//! @return the remaining seconds, never negative
	float HoldRemaining()
	{
		float remaining = m_fHoldSeconds - m_fHeldSeconds;
		if (remaining < 0)
			return 0;

		return remaining;
	}

	//! Teardown speed as a multiplier on the build rate: a full bar empties in exactly
	//! `m_fNeutralizeSeconds`.
	//! @return `float.MAX` for an instant teardown, 1 when the capture length is not positive
	float TeardownRate()
	{
		if (m_fNeutralizeSeconds <= 0)
			return float.MAX;

		if (m_fCaptureSeconds <= 0)
			return 1.0;

		return m_fCaptureSeconds / m_fNeutralizeSeconds;
	}

	//! Stable log identifier `<KIND>:<zone id>`, built in steps because a long `+` chain is too
	//! complex for this compiler.
	string LogKey()
	{
		string key = typename.EnumToString(TBD_EObjectiveKind, m_eKind);
		key += ":";
		key += m_sId;
		return key;
	}

	//! Which side of this objective `viewerFaction` is on. The objective is for
	//! `objectives[].side` when authored and valid, else `zones[].faction`; an invalid authored
	//! side reads NEUTRAL. Every viewer not on that side takes the opposite role, so with three
	//! sides both attackers read the attacker framing.
	//! @param viewerFaction the viewer's side, resolved from server-owned slot state
	//! @return the viewer's role; NEUTRAL for no viewer side or no objective side
	TBD_EObjectiveRole RoleOf(string viewerFaction)
	{
		if (viewerFaction.IsEmpty() || m_bInvalidSide)
			return TBD_EObjectiveRole.NEUTRAL;

		string owningSide = m_sSide;
		if (owningSide.IsEmpty())
			owningSide = m_sFaction;

		if (owningSide.IsEmpty())
			return TBD_EObjectiveRole.NEUTRAL;

		bool isOwningSide = viewerFaction == owningSide;

		if (m_bSideDefends)
		{
			if (isOwningSide)
				return TBD_EObjectiveRole.DEFENDER;

			return TBD_EObjectiveRole.ATTACKER;
		}

		if (isOwningSide)
			return TBD_EObjectiveRole.ATTACKER;

		return TBD_EObjectiveRole.DEFENDER;
	}

	//! Whether either side carries a framing title or text.
	bool HasFraming()
	{
		if (!m_sAttackerTitle.IsEmpty() || !m_sAttackerText.IsEmpty())
			return true;

		if (!m_sDefenderTitle.IsEmpty() || !m_sDefenderText.IsEmpty())
			return true;

		return false;
	}

	//! The title `viewerFaction` reads.
	//! @return that side's framing title, or `DisplayName()` when that side is unframed
	string TitleFor(string viewerFaction)
	{
		TBD_EObjectiveRole role = RoleOf(viewerFaction);

		if (role == TBD_EObjectiveRole.ATTACKER && !m_sAttackerTitle.IsEmpty())
			return m_sAttackerTitle;

		if (role == TBD_EObjectiveRole.DEFENDER && !m_sDefenderTitle.IsEmpty())
			return m_sDefenderTitle;

		return DisplayName();
	}

	//! The task body `viewerFaction` reads.
	//! @return that side's framing text, or empty when that side is unframed or NEUTRAL
	string TaskTextFor(string viewerFaction)
	{
		TBD_EObjectiveRole role = RoleOf(viewerFaction);

		if (role == TBD_EObjectiveRole.ATTACKER)
			return m_sAttackerText;

		if (role == TBD_EObjectiveRole.DEFENDER)
			return m_sDefenderText;

		return string.Empty;
	}

	//! How many targets must be destroyed to complete a DESTROY objective.
	//! @return `m_iTargetCount` when positive, else every target found at LIVE
	int RequiredKills()
	{
		if (m_iTargetCount > 0)
			return m_iTargetCount;

		return m_iTargetsFound;
	}
}
