//! T-181.39 — one mission objective, PREPARED for use at runtime, plus the state machine that
//! advances it.
//!
//! ── What this is, and what it deliberately is not ───────────────────────────────────────────
//! This is the objective layer sitting ON TOP of T-181.18's zone work. It owns **progress,
//! ownership and completion**. It owns **no geometry at all**: containment is `m_Zone.Contains()`,
//! which is `TBD_ZoneGeometry`'s crossing-number polygon test and circle test, with the same
//! 1 m outward `EDGE_MARGIN_M` the play area uses. There is exactly one answer in this mod to
//! "is this player inside that shape", and it is not here.
//!
//! `m_Zone` is a `ref` on purpose. `TBD_ZoneRegistry.Clear()` is called from
//! `TBD_PlayAreaComponent.OnDelete`, and the ORDER in which two sibling components are deleted is
//! not something this file should depend on. A strong reference means an objective can never be
//! left pointing at a collected zone even if the zone registry is torn down first.
//!
//! ── Server-authoritative ────────────────────────────────────────────────────────────────────
//! Every field below is server state. Clients hold NO mission document (recorded landmine), so a
//! client has no zones, no rules and nothing to advance. Nothing in this file is client-safe and
//! nothing in this file is called from a client path.
//!
//! ── Honest limits ───────────────────────────────────────────────────────────────────────────
//! Presence is sampled at 1 Hz against a player's CONTROLLED ENTITY ORIGIN. A player in a vehicle
//! passing through the zone counts as present for that second; a player in a helicopter over the
//! objective counts as present, because zones are footprints and Y is ignored throughout
//! `TBD_ZoneGeometry`. Both are consequences of the geometry layer's stated design, inherited on
//! purpose rather than re-litigated here.

//------------------------------------------------------------------------------------------------
//! Which of the schema's three objective zone types this is.
//!
//! `NONE` is not an error state in itself — it is what a non-objective zone (spawn, boundary,
//! base_protection) resolves to, and it is why the registry can walk every prepared zone without a
//! string comparison at each call site.
enum TBD_EObjectiveKind
{
	NONE,
	CAPTURE,
	DESTROY,
	HOLD_UNTIL
}

//------------------------------------------------------------------------------------------------
//! What happens to PARTIAL capture progress when nobody at all is standing on the objective.
//!
//! ══ THE DECISION, IN THE PLACE IT IS MADE ═══════════════════════════════════════════════════
//! There is no prior art in this program for this, so it is chosen here and justified here.
//!
//! **The default is `HOLD`.** TBD events are ONE LIFE. A squad that spent lives to bank 90 seconds
//! on an objective and was then wiped should not watch that work evaporate to a timer they can no
//! longer influence — under one life they cannot simply come back and redo it. Decay also
//! systematically favours the side with more bodies to feed into a zone, which is the wrong bias
//! for the small, asymmetric, attacker-defender missions this framework exists to run.
//!
//! `DECAY` is fully implemented and one JSON key away (`rules.onEmpty: "decay"`, rate tunable with
//! `rules.decayRate`) for an operator who wants momentum to matter. Changing the DEFAULT is an
//! event-design call, not an engineering one.
//!
//! Decay applies ONLY while the objective is neutral. An objective that has actually been captured
//! never decays out of its owner's hands on its own — losing ground must require somebody to walk
//! onto it, or a side could lose an objective it took while nobody was near it.
enum TBD_EObjectiveOnEmpty
{
	HOLD,
	DECAY
}

//------------------------------------------------------------------------------------------------
//! One prepared objective, with its resolved rules and its live state.
class TBD_Objective
{
	//! The prepared zone this objective lives on. Owns the shape, the bounds and the containment
	//! test. Strong reference — see the class header.
	ref TBD_Zone m_Zone;

	TBD_EObjectiveKind m_eKind;

	string m_sId;
	string m_sLabel;

	//! `zones[].faction`. The meaning is PER KIND and is not interchangeable:
	//!   * CAPTURE     — optional. When set, only that faction may ever OWN this objective; any
	//!                   other side can still neutralise it but never take it. Empty = anyone.
	//!   * HOLD_UNTIL  — REQUIRED. The side that is holding. Without it there is no way to know who
	//!                   wins when the clock runs out, so the objective is inert.
	//!   * DESTROY     — the side that must destroy the target, i.e. who `objective_destroyed`
	//!                   declares the winner. Optional, but a destroy objective with no faction
	//!                   ends the round for nobody in particular and says so at load.
	string m_sFaction;

	//! False when this objective cannot run: no usable shape, or a rule with no defensible default
	//! missing. An inert objective is EXCLUDED from every end-trigger authority — it can neither
	//! fire one nor block one — and `m_sInertReason` carries the operator-facing why.
	bool m_bUsable;
	string m_sInertReason;

	// ── Resolved rules. Never sentinels, never absent. See TBD_ObjectiveRegistry.ResolveRules. ──
	float m_fCaptureSeconds;
	float m_fNeutralizeSeconds;
	bool m_bContestable;
	TBD_EObjectiveOnEmpty m_eOnEmpty;
	float m_fDecayRate;
	float m_fHoldSeconds;
	bool m_bPauseOnEnemy;
	bool m_bResetOnEnemy;
	bool m_bRequireHolderPresent;
	string m_sTargetAlias;
	int m_iTargetCount;
	float m_fPoints;
	float m_fAnnounceEverySeconds;

	// ── Live state: CAPTURE ─────────────────────────────────────────────────────────────────
	//! Owning faction, or empty for neutral. Only ever changes when somebody stands on the zone.
	string m_sOwner;
	//! Whose banked progress `m_fProgress` is. Empty when progress is zero.
	string m_sProgressFaction;
	//! Seconds banked toward `m_fCaptureSeconds`, in [0, m_fCaptureSeconds].
	float m_fProgress;
	//! Two or more factions present and the rules say that freezes things.
	bool m_bContested;

	// ── Live state: HOLD_UNTIL ──────────────────────────────────────────────────────────────
	float m_fHeldSeconds;
	bool m_bHoldPaused;

	// ── Live state: DESTROY ─────────────────────────────────────────────────────────────────
	//! The target search has run for this round. Runs once, on the first LIVE evaluation, because
	//! a target spawned by another subsystem may not exist while the world is still in LOBBY.
	bool m_bArmed;
	//! Resolved prefab of `m_sTargetAlias`, empty when the alias did not resolve.
	ResourceName m_TargetResource;
	int m_iTargetsFound;
	int m_iTargetsDestroyed;

	// ── Terminal ────────────────────────────────────────────────────────────────────────────
	//! DESTROY / HOLD_UNTIL only. A CAPTURE objective is never "complete": ownership can flip for
	//! as long as the round runs, and treating a capture as done the first time it changes hands
	//! would freeze the map at whatever the first thirty seconds produced.
	bool m_bComplete;

	// ── Announcement bookkeeping ────────────────────────────────────────────────────────────
	//! Seconds since the last progress message to the players standing on this objective.
	float m_fSinceAnnounce;

	//! The contested state the players were last TOLD about. Kept separate from `m_bContested` so a
	//! message is sent on the TRANSITION only — a 1 Hz tick that re-announced a standing state would
	//! bury the log and the chat window in the same line sixty times a minute.
	bool m_bAnnouncedContested;

	//! HOLD_UNTIL: how far down the remaining-time announcement ladder this objective has got.
	//! See `TBD_ObjectivesComponent.NextHoldMark`.
	int m_iHoldMarkIndex;

	//! What the players standing ON this objective should be told at the end of this tick, or empty.
	//! Set during the advance pass and consumed by the single delivery walk, so one tick costs at
	//! most two passes over the player list however many objectives changed.
	string m_sPendingInsideMessage;

	//! Scratch for one tick's presence sample: parallel arrays of faction key and living body
	//! count, plus the ids of the players who were inside. Reused rather than reallocated — this is
	//! walked once per objective per second for the whole round, and a fresh map per objective per
	//! tick is garbage for no benefit.
	ref array<string> m_aPresentFactions;
	ref array<int> m_aPresentCounts;
	//! Player ids sampled inside this objective this tick. Recorded during sampling rather than
	//! recomputed at delivery so containment is tested exactly once per player per objective.
	ref array<int> m_aPresentPlayers;

	//------------------------------------------------------------------------------------------------
	void TBD_Objective()
	{
		m_aPresentFactions = new array<string>();
		m_aPresentCounts = new array<int>();
		m_aPresentPlayers = new array<int>();
	}

	//------------------------------------------------------------------------------------------------
	//! What a human should be shown. The schema does not require `label`, so this falls back
	//! through id and then a type word rather than announcing a nameless objective to a player who
	//! then has no idea which one changed hands.
	string DisplayName()
	{
		if (!m_sLabel.IsEmpty())
			return m_sLabel;
		if (!m_sId.IsEmpty())
			return m_sId;

		return typename.EnumToString(TBD_EObjectiveKind, m_eKind);
	}

	//------------------------------------------------------------------------------------------------
	//! Start a fresh presence sample.
	//! Clears the pending message too, and deliberately HERE rather than in the advance pass: the
	//! advance pass skips objectives that are not usable, so a message left on an objective that
	//! went inert mid-round would be re-delivered every tick forever. `BeginSample` runs for every
	//! objective, usable or not, which makes it the only correct place for this.
	void BeginSample()
	{
		m_aPresentFactions.Clear();
		m_aPresentCounts.Clear();
		m_aPresentPlayers.Clear();
		m_sPendingInsideMessage = string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Record one living body of `factionKey` inside this objective. An empty faction key is
	//! DROPPED rather than counted as a side of its own: a player with no resolved slot is not on a
	//! side, so they can neither capture nor contest. Standing on an objective in an unassigned
	//! state must not be a way to freeze it.
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

	//------------------------------------------------------------------------------------------------
	//! How many living bodies of `factionKey` are inside, from the current sample.
	int PresenceOf(string factionKey)
	{
		if (factionKey.IsEmpty())
			return 0;

		int at = m_aPresentFactions.Find(factionKey);
		if (at == -1)
			return 0;

		return m_aPresentCounts[at];
	}

	//------------------------------------------------------------------------------------------------
	//! How many distinct sides are inside.
	int PresentFactionCount()
	{
		return m_aPresentFactions.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! Is any side other than `factionKey` inside?
	bool HasEnemyPresent(string factionKey)
	{
		foreach (int index, string present : m_aPresentFactions)
		{
			if (present != factionKey && m_aPresentCounts[index] > 0)
				return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Which side is ACTING on this objective this tick, or empty when nobody is.
	//!
	//! ══ WHAT `contestable` MEANS — chosen here, because the schema does not say ══════════════
	//! `rules.contestable` answers one question: **does the presence of another side stop you?**
	//!
	//!   * `true` (THE DEFAULT, and what all three shipped goldens author): two or more sides
	//!     inside FREEZES the objective. Nobody gains, nobody loses, and the bar sits exactly where
	//!     it was until one side clears the other off. This is the reading every capture game mode
	//!     in the genre uses and the one a mission author will expect from the word.
	//!
	//!   * `false`: enemy presence is not a veto. With two sides inside, the one with MORE living
	//!     bodies proceeds exactly as if it were alone; an exact tie freezes. This is the only
	//!     coherent meaning available for "not contestable while two sides are standing on it" —
	//!     the alternatives are either identical to `true` or let both sides bank progress at once,
	//!     which is incoherent. Documented as "weight of numbers decides".
	//!
	//! Note that the RATE never scales with headcount in either mode. Twenty players do not capture
	//! four times faster than five. That is deliberate: TBD events are small and often asymmetric,
	//! and a rate that scales with bodies turns every objective into a headcount contest and
	//! punishes the side that took casualties — under ONE LIFE, permanently. Headcount decides only
	//! the `contestable: false` tiebreak, where something has to.
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

	//------------------------------------------------------------------------------------------------
	//! May `factionKey` ever OWN this objective? See `m_sFaction`.
	bool MayOwn(string factionKey)
	{
		if (factionKey.IsEmpty())
			return false;

		if (m_sFaction.IsEmpty())
			return true;

		return m_sFaction == factionKey;
	}

	//------------------------------------------------------------------------------------------------
	//! Progress as a percentage of the capture bar, for logs and player-facing text.
	int ProgressPercent()
	{
		if (m_fCaptureSeconds <= 0)
			return 0;

		float fraction = m_fProgress / m_fCaptureSeconds;
		return Math.Round(fraction * 100);
	}

	//------------------------------------------------------------------------------------------------
	//! Seconds of hold still to run, never negative.
	float HoldRemaining()
	{
		float remaining = m_fHoldSeconds - m_fHeldSeconds;
		if (remaining < 0)
			return 0;

		return remaining;
	}

	//------------------------------------------------------------------------------------------------
	//! How fast a teardown runs, as a multiplier on the build rate.
	//!
	//! A full bar empties in exactly `m_fNeutralizeSeconds`, so the default (`= m_fCaptureSeconds`)
	//! is a symmetric 1:1 rate and `neutralizeSeconds: 0` is an instant, single-stage capture.
	//! Returns a large number for the instant case rather than dividing by zero; the caller clamps.
	float TeardownRate()
	{
		if (m_fNeutralizeSeconds <= 0)
			return float.MAX;

		if (m_fCaptureSeconds <= 0)
			return 1.0;

		return m_fCaptureSeconds / m_fNeutralizeSeconds;
	}

	//------------------------------------------------------------------------------------------------
	//! Stable identifier for logs. Built in steps, not one long `+` chain: a 9-term concatenation
	//! is a measured `Formula too complex` in this compiler, whose SECOND diagnostic is a
	//! misleading `Incompatible parameter`.
	string LogKey()
	{
		string key = typename.EnumToString(TBD_EObjectiveKind, m_eKind);
		key += ":";
		key += m_sId;
		return key;
	}

	//------------------------------------------------------------------------------------------------
	//! One line of the objective board, from `viewerFaction`'s point of view.
	//!
	//! The viewer's side is passed in rather than derived here because it must be resolved from
	//! SERVER-OWNED state (the player's assigned slot), never from anything a client sends. There is
	//! no faction parameter anywhere on this path that a client could phrase.
	//!
	//! `->` rather than the arrow glyph: `→` is not in the proven glyph set for shipped screens and
	//! a tofu box in a line a player reads mid-firefight is not acceptable.
	string BoardLine(string viewerFaction)
	{
		string line = DisplayName();
		line += " [";
		line += StatusText(viewerFaction);
		line += "]";
		return line;
	}

	//------------------------------------------------------------------------------------------------
	//! The status half of a board line.
	string StatusText(string viewerFaction)
	{
		if (!m_bUsable)
			return "inactive";

		if (m_eKind == TBD_EObjectiveKind.DESTROY)
			return DestroyStatusText();

		if (m_eKind == TBD_EObjectiveKind.HOLD_UNTIL)
			return HoldStatusText();

		return CaptureStatusText(viewerFaction);
	}

	//------------------------------------------------------------------------------------------------
	protected string DestroyStatusText()
	{
		if (m_bComplete)
			return "DESTROYED";

		string text = "intact ";
		text += m_iTargetsDestroyed.ToString();
		text += "/";
		text += RequiredKills().ToString();
		return text;
	}

	//------------------------------------------------------------------------------------------------
	protected string HoldStatusText()
	{
		if (m_bComplete)
			return "HELD";

		// Rounded into an int FIRST, then stringified. `Math.Round(...).ToString()` would render a
		// float's full precision ("600.000000") in a line a player reads at a glance; the same
		// two-step is what `TBD_PlayAreaComponent.WarnPlayer` does with its countdown.
		int remaining = Math.Round(HoldRemaining());

		string text = "hold ";
		text += remaining.ToString();
		text += "s left";
		if (m_bHoldPaused)
			text += " (PAUSED)";

		return text;
	}

	//------------------------------------------------------------------------------------------------
	protected string CaptureStatusText(string viewerFaction)
	{
		string text;

		if (m_sOwner.IsEmpty())
		{
			text = "neutral";
		}
		else if (!viewerFaction.IsEmpty() && m_sOwner == viewerFaction)
		{
			text = "OURS";
		}
		else
		{
			text = "held by ";
			text += m_sOwner;
		}

		if (m_bContested)
		{
			text += " -- CONTESTED";
			return text;
		}

		int percent = ProgressPercent();
		if (percent > 0 && percent < 100)
		{
			text += " -- ";
			text += percent.ToString();
			text += "% ";
			text += m_sProgressFaction;
		}

		return text;
	}

	//------------------------------------------------------------------------------------------------
	//! How many targets must die for a DESTROY objective to complete. `targetCount: 0` (or absent)
	//! means "every one that was there when the round went live", which is what an author writing
	//! `targetAlias` and nothing else obviously means.
	int RequiredKills()
	{
		if (m_iTargetCount > 0)
			return m_iTargetCount;

		return m_iTargetsFound;
	}
}
