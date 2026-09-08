//! T-181.39 - one mission objective, PREPARED for use at runtime, plus the state machine that
//! advances it.
//!
//! -- What this is, and what it deliberately is not -------------------------------------------
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
//! -- Server-authoritative --------------------------------------------------------------------
//! Every field below is server state. Clients hold NO mission document (recorded landmine), so a
//! client has no zones, no rules and nothing to advance. Nothing in this file is client-safe and
//! nothing in this file is called from a client path.
//!
//! -- Honest limits ---------------------------------------------------------------------------
//! Presence is sampled at 1 Hz against a player's CONTROLLED ENTITY ORIGIN. A player in a vehicle
//! passing through the zone counts as present for that second; a player in a helicopter over the
//! objective counts as present, because zones are footprints and Y is ignored throughout
//! `TBD_ZoneGeometry`. Both are consequences of the geometry layer's stated design, inherited on
//! purpose rather than re-litigated here.

//------------------------------------------------------------------------------------------------
//! Which of the schema's three objective zone types this is.
//!
//! `NONE` is not an error state in itself - it is what a non-objective zone (spawn, boundary,
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
//! == THE DECISION, IN THE PLACE IT IS MADE ===================================================
//! There is no prior art in this program for this, so it is chosen here and justified here.
//!
//! **The default is `HOLD`.** TBD events are ONE LIFE. A squad that spent lives to bank 90 seconds
//! on an objective and was then wiped should not watch that work evaporate to a timer they can no
//! longer influence - under one life they cannot simply come back and redo it. Decay also
//! systematically favours the side with more bodies to feed into a zone, which is the wrong bias
//! for the small, asymmetric, attacker-defender missions this framework exists to run.
//!
//! `DECAY` is fully implemented and one JSON key away (`rules.onEmpty: "decay"`, rate tunable with
//! `rules.decayRate`) for an operator who wants momentum to matter. Changing the DEFAULT is an
//! event-design call, not an engineering one.
//!
//! Decay applies ONLY while the objective is neutral. An objective that has actually been captured
//! never decays out of its owner's hands on its own - losing ground must require somebody to walk
//! onto it, or a side could lose an objective it took while nobody was near it.
enum TBD_EObjectiveOnEmpty
{
	HOLD,
	DECAY
}

//------------------------------------------------------------------------------------------------
//! T-212 -- which side of the SAME objective a viewer is on.
//!
//! FNF v4's insight is real: an objective reads differently to attacker and defender, and v4
//! generates genuinely different task text, titles and separate task trees from that. Its
//! IMPLEMENTATION is the part TBD does not copy. v4 ships TWO modules per objective, one per
//! side, and pays for it four documented ways (fnf_v4.md 14.4): identity is POSITIONAL, the two
//! halves are "the same objective" only because they share a sync target or a prefix string,
//! NOTHING validates that both halves exist, and the lobby tool divides the objective count by 2
//! so a forgotten second half yields a fractional count. `#/$defs/objective` forbids that shape
//! outright -- ONE ENTITY, TWO FRAMINGS, never two rows for one objective.
//!
//! So the role is DERIVED here, once, from one row. NEUTRAL is not an error state: it is what a
//! viewer with no resolved side gets, and what an objective naming no side at all reads as, which
//! is why the framing lookup can be a switch on this instead of a string compare at each call
//! site -- the same reason `TBD_EObjectiveKind.NONE` exists.
enum TBD_EObjectiveRole
{
	NEUTRAL,
	ATTACKER,
	DEFENDER
}

//------------------------------------------------------------------------------------------------
//! One prepared objective, with its resolved rules and its live state.
class TBD_Objective
{
	//! The prepared zone this objective lives on. Owns the shape, the bounds and the containment
	//! test. Strong reference - see the class header.
	ref TBD_Zone m_Zone;

	TBD_EObjectiveKind m_eKind;

	string m_sId;
	string m_sLabel;

	//! `zones[].faction`. The meaning is PER KIND and is not interchangeable:
	//!   * CAPTURE     - optional. When set, only that faction may ever OWN this objective; any
	//!                   other side can still neutralise it but never take it. Empty = anyone.
	//!   * HOLD_UNTIL  - REQUIRED. The side that is holding. Without it there is no way to know who
	//!                   wins when the clock runs out, so the objective is inert.
	//!   * DESTROY     - the side that must destroy the target, i.e. who `objective_destroyed`
	//!                   declares the winner. Optional, but a destroy objective with no faction
	//!                   ends the round for nobody in particular and says so at load.
	string m_sFaction;

	//! False when this objective cannot run: no usable shape, or a rule with no defensible default
	//! missing. An inert objective is EXCLUDED from every end-trigger authority - it can neither
	//! fire one nor block one - and `m_sInertReason` carries the operator-facing why.
	bool m_bUsable;
	string m_sInertReason;

	// -- Resolved rules. Never sentinels, never absent. See TBD_ObjectiveRegistry.ResolveRules. --
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

	// -- Live state: CAPTURE -----------------------------------------------------------------
	//! Owning faction, or empty for neutral. Only ever changes when somebody stands on the zone.
	string m_sOwner;
	//! Whose banked progress `m_fProgress` is. Empty when progress is zero.
	string m_sProgressFaction;
	//! Seconds banked toward `m_fCaptureSeconds`, in [0, m_fCaptureSeconds].
	float m_fProgress;
	//! Two or more factions present and the rules say that freezes things.
	bool m_bContested;

	// -- Live state: HOLD_UNTIL --------------------------------------------------------------
	float m_fHeldSeconds;
	bool m_bHoldPaused;

	// -- Live state: DESTROY -----------------------------------------------------------------
	//! The target search has run for this round. Runs once, on the first LIVE evaluation, because
	//! a target spawned by another subsystem may not exist while the world is still in LOBBY.
	bool m_bArmed;
	//! Resolved prefab of `m_sTargetAlias`, empty when the alias did not resolve.
	ResourceName m_TargetResource;
	int m_iTargetsFound;
	int m_iTargetsDestroyed;

	// -- Terminal ----------------------------------------------------------------------------
	//! DESTROY / HOLD_UNTIL only. A CAPTURE objective is never "complete": ownership can flip for
	//! as long as the round runs, and treating a capture as done the first time it changes hands
	//! would freeze the map at whatever the first thirty seconds produced.
	bool m_bComplete;

	// -- Announcement bookkeeping ------------------------------------------------------------
	//! Seconds since the last progress message to the players standing on this objective.
	float m_fSinceAnnounce;

	//! The contested state the players were last TOLD about. Kept separate from `m_bContested` so a
	//! message is sent on the TRANSITION only - a 1 Hz tick that re-announced a standing state would
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
	//! count, plus the ids of the players who were inside. Reused rather than reallocated - this is
	//! walked once per objective per second for the whole round, and a fresh map per objective per
	//! tick is garbage for no benefit.
	ref array<string> m_aPresentFactions;
	ref array<int> m_aPresentCounts;
	//! Player ids sampled inside this objective this tick. Recorded during sampling rather than
	//! recomputed at delivery so containment is tested exactly once per player per objective.
	ref array<int> m_aPresentPlayers;

	// -- T-212: the typed `objectives[]` row bound to this objective, when one exists ------------
	//
	// Every field below is EMPTY on an untyped objective, and an untyped objective behaves exactly
	// as it did before T-212. Objectives-as-zones is the floor this sits on, not a legacy path
	// being migrated off: `zones[]` of `type: objective_*` remains the thing the runtime enforces,
	// and a typed row is an OVERLAY that adds identity, per-side framing and the two WOG scalars.

	//! True when a top-level `objectives[]` row named this objective's zone in `zoneId`.
	bool m_bTyped;

	//! `objectives[].id` -- the OBJECTIVE's own identity, which is not the zone's. Kept apart from
	//! `m_sId` (the zone id) rather than folded into it: they are separate namespaces in the schema,
	//! and collapsing them makes every log line ambiguous the first time an author lets them differ.
	string m_sEntityId;

	//! `objectives[].type` verbatim: capture | destroy | hold | defend. Empty when untyped.
	//!
	//! This records what the author SAID. `m_eKind`, resolved from `zones[].type`, remains what the
	//! runtime ENFORCES. They are kept apart on purpose so a disagreement between the two can be
	//! reported at load instead of silently resolved in favour of whichever was read last.
	string m_sTaskType;

	//! `objectives[].side` -- the faction this objective is FOR. Distinct from `m_sFaction`
	//! (`zones[].faction`), which is the OWNERSHIP restriction the capture rules enforce. An
	//! objective can be for a side that is not allowed to own the zone; those are different claims.
	string m_sSide;

	//! The explicitly authored side names no mission faction. Kept separate from an absent side:
	//! invalid means neutral framing, while absent retains the zone-faction fallback.
	bool m_bInvalidSide;

	//! Does `m_sSide` DEFEND this objective rather than attack it?
	//!
	//! `hold` and `defend` are the defender framings; `capture` and `destroy` are the attacker
	//! framings. An untyped objective falls back to its kind, which says the same thing: a
	//! HOLD_UNTIL zone's `faction` is the side holding the ground, and a capture or destroy zone's
	//! is the side told to take or break it.
	bool m_bSideDefends;

	//! `objectives[].lock`. SEMANTICS ARE INFERRED -- carried and reported, never enforced. See the
	//! T-212 block in `TBD_ObjectiveRegistry`'s header for why enacting it would be a misreading
	//! rather than a feature.
	bool m_bLocked;

	//! `objectives[].autoLose`. Also INFERRED, and also carried and reported only. Validated
	//! against `factions[].key` at load and blanked when it names no side, so a consumer that
	//! appears later never inherits a dangling key.
	string m_sAutoLoseFaction;

	//! `objectives[].framing.attacker` / `.defender` -- the per-side task title and body text.
	//! Empty means that side was not framed, and the neutral `DisplayName()` stands in for it.
	string m_sAttackerTitle;
	string m_sAttackerText;
	string m_sDefenderTitle;
	string m_sDefenderText;

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
	//! == WHAT `contestable` MEANS - chosen here, because the schema does not say ==============
	//! `rules.contestable` answers one question: **does the presence of another side stop you?**
	//!
	//!   * `true` (THE DEFAULT, and what all three shipped goldens author): two or more sides
	//!     inside FREEZES the objective. Nobody gains, nobody loses, and the bar sits exactly where
	//!     it was until one side clears the other off. This is the reading every capture game mode
	//!     in the genre uses and the one a mission author will expect from the word.
	//!
	//!   * `false`: enemy presence is not a veto. With two sides inside, the one with MORE living
	//!     bodies proceeds exactly as if it were alone; an exact tie freezes. This is the only
	//!     coherent meaning available for "not contestable while two sides are standing on it" -
	//!     the alternatives are either identical to `true` or let both sides bank progress at once,
	//!     which is incoherent. Documented as "weight of numbers decides".
	//!
	//! Note that the RATE never scales with headcount in either mode. Twenty players do not capture
	//! four times faster than five. That is deliberate: TBD events are small and often asymmetric,
	//! and a rate that scales with bodies turns every objective into a headcount contest and
	//! punishes the side that took casualties - under ONE LIFE, permanently. Headcount decides only
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
	//! T-212 -- which side of this objective `viewerFaction` is on.
	//!
	//! The side the objective is FOR is `objectives[].side` when authored and valid, and
	//! `zones[].faction` when absent. An invalid authored side stays NEUTRAL without that fallback.
	//! The absent-side fallback is a restatement of semantics this file already
	//! documents on `m_sFaction`, not a new guess: a capture zone's faction is the side allowed to
	//! take it, a destroy zone's is the side told to destroy it, and a hold zone's is the side
	//! holding the ground. `m_bSideDefends` carries which of the two readings applies.
	//!
	//! Everyone who is not that side gets the OPPOSITE role, and that is deliberate rather than a
	//! two-faction assumption: with three sides, two of them are attacking the same defender and
	//! both should read the attacker framing, which is exactly what this returns.
	//!
	//! The viewer's side is PASSED IN, never derived here, for the same reason `StatusText` takes
	//! it: it must come from server-owned state (the player's assigned slot) and there must be no
	//! faction parameter on this path that a client could phrase.
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

	//------------------------------------------------------------------------------------------------
	//! Was EITHER side framed? Read by the load log, so an operator can tell at a glance whether a
	//! mission actually authored per-side text or is running on the neutral label everywhere.
	bool HasFraming()
	{
		if (!m_sAttackerTitle.IsEmpty() || !m_sAttackerText.IsEmpty())
			return true;

		if (!m_sDefenderTitle.IsEmpty() || !m_sDefenderText.IsEmpty())
			return true;

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! The objective's title AS `viewerFaction` READS IT.
	//!
	//! Falls back to the neutral `DisplayName()` whenever that side was not framed, so a
	//! half-framed objective -- which the schema allows, both `framing` halves being optional --
	//! is still named to the side the author skipped rather than appearing as a blank row.
	string TitleFor(string viewerFaction)
	{
		TBD_EObjectiveRole role = RoleOf(viewerFaction);

		if (role == TBD_EObjectiveRole.ATTACKER && !m_sAttackerTitle.IsEmpty())
			return m_sAttackerTitle;

		if (role == TBD_EObjectiveRole.DEFENDER && !m_sDefenderTitle.IsEmpty())
			return m_sDefenderTitle;

		return DisplayName();
	}

	//------------------------------------------------------------------------------------------------
	//! The task BODY text for `viewerFaction`, or empty when that side was not framed.
	//!
	//! Empty rather than a fallback, and the asymmetry with `TitleFor` is on purpose: a title has
	//! something sensible to fall back to (the label), and a body does not. Repeating the label as
	//! a task description would be noise in a task list, so the caller is told there is no body and
	//! decides what to render.
	string TaskTextFor(string viewerFaction)
	{
		TBD_EObjectiveRole role = RoleOf(viewerFaction);

		if (role == TBD_EObjectiveRole.ATTACKER)
			return m_sAttackerText;

		if (role == TBD_EObjectiveRole.DEFENDER)
			return m_sDefenderText;

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! One line of the objective board, from `viewerFaction`'s point of view.
	//!
	//! The viewer's side is passed in rather than derived here because it must be resolved from
	//! SERVER-OWNED state (the player's assigned slot), never from anything a client sends. There is
	//! no faction parameter anywhere on this path that a client could phrase.
	//!
	//! `->` rather than the arrow glyph: `->` is not in the proven glyph set for shipped screens and
	//! a tofu box in a line a player reads mid-firefight is not acceptable.
	string BoardLine(string viewerFaction)
	{
		// T-212: the TITLE is per-side. `TitleFor` collapses to `DisplayName()` for an unframed
		// objective, so an untyped mission board is byte-identical to what it was before T-212.
		string line = TitleFor(viewerFaction);
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
