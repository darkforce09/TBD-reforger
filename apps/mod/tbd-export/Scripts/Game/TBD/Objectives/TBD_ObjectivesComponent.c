[ComponentEditorProps(category: "TBD/Framework", description: "TBD objectives - capture progress, hold timers and destroy targets from mission JSON; drives the objective win conditions.")]
class TBD_ObjectivesComponentClass : SCR_BaseGameModeComponentClass {}

//! T-181.39 - the server-authoritative objective runner.
//!
//! == How it runs =============================================================================
//! ONE repeating server-side tick at `TICK_MS` walks the connected players once, samples who is
//! standing on which objective, advances every objective, and delivers whatever needs saying. It is
//! deliberately not a per-objective timer and emphatically not a per-frame check, for the same two
//! reasons `TBD_PlayAreaComponent` gives:
//!   * `ScriptCallQueue.Remove` cancels BY FUNCTION, not by arguments (recorded landmine), so a
//!     per-objective `CallLater` could not be cancelled individually - and a per-player deferred
//!     callback carrying a raw `playerId` survives that player's disconnect onto a RECYCLED id. A
//!     single tick that re-reads the live player list every time has neither problem.
//!   * Objective timings are authored in whole seconds and measured in minutes. 1 Hz is exact
//!     enough that the accumulators read in the same units the rules do.
//!
//! == Only while LIVE =========================================================================
//! Nothing advances outside `TBD_EGameStage.LIVE`. Not during SAFE_START - that is the phase where
//! damage is off and players are still walking to their start lines, and letting a side bank a
//! capture while the other side cannot shoot back would make safestart a land grab. Not during
//! LOBBY/BRIEFING/END either.
//!
//! Progress is NOT reset when the stage leaves LIVE. `TBD_PlayAreaComponent` clears its violations
//! on a stage change because a grace countdown is a transient; objective state is the ROUND'S
//! RECORD, and an admin bouncing LIVE -> SAFE_START -> LIVE to deal with an incident must not wipe
//! what both sides spent lives achieving.
//!
//! == What a player is told, and over what ====================================================
//! Clients hold NO mission document (recorded landmine), so every word below is composed on the
//! server from server-owned state and pushed out; a client computes nothing and is never asked to.
//! T-941.4: the board and capture bar go over Owner RPC to `TBD_ObjectiveHud`. Chat keeps ONLY the
//! objective-complete line (`CAPTURED` / `DESTROYED` / `HELD`). The old per-tick
//! `SCR_ChatComponent.SendPrivateMessage` pump is gone - it buried the log in progress spam.
//!
//! T-946.55: that HUD push is EVALUATED at 1 Hz but SENT only on a difference. `ReplicateHud` hashes
//! each owner's rendered snapshot and skips the RPC when it matches the last one that owner was
//! actually sent, so an idle round costs one board per player and nothing after it. Ungated it cost
//! 60 Reliable Owner RPCs per player per minute with nothing happening, which is the same wire bill
//! the chat pump was removed for.
//!
//! Everything is also logged server-side, so an operator can reconstruct the objective history of a
//! round even if delivery to a particular client failed.
//!
//! == What is NOT proven here =================================================================
//! Every API used is compile-proven against this engine build with a failing negative control.
//! NOTHING below has been observed running: `world-boot.sh` boots with zero players and never
//! leaves LOBBY, so no presence is ever sampled, no progress ever advances and no message is ever
//! delivered on this lane. What a green boot proves is that the component instantiates, that the
//! registry builds from a real mission document, and that the rules parsed to the values the JSON
//! authored. Behaviour needs a dedicated server with real clients (T-181.25).
class TBD_ObjectivesComponent : SCR_BaseGameModeComponent
{
	//! Evaluation cadence. 1 Hz - see the class header.
	static const int TICK_MS = 1000;

	//! `TICK_MS` as seconds, so the accumulators read in the units the rules are authored in.
	static const float TICK_SECONDS = 1.0;

	protected static TBD_ObjectivesComponent s_Instance;

	//! Latches for the once-per-world informational lines, so a 1 Hz tick cannot spam the log.
	protected bool m_bAnnouncedArmed;
	protected bool m_bAnnouncedNoObjectives;

	//! Was the round LIVE on the previous tick? Used to detect the LIVE edge without needing a hook
	//! into `TBD_FrameworkManager`, which this slice does not own.
	protected bool m_bLive;

	//! Latch: the "an end trigger is met but nothing is wired to act on it" banner fires ONCE.
	protected bool m_bEndTriggerAnnounced;

	//! This tick's COMPLETE lines only (captured / destroyed / held). Progress lives on the HUD.
	protected ref array<string> m_aBroadcasts;

	//! What each owner's HUD was LAST ACTUALLY SENT, keyed by playerId: the signature of the RENDERED
	//! per-player snapshot, NOT of the shared board. `FillHudSnapshot` resolves the viewer's side and
	//! carries that viewer's own capture bar, so two players reading the same board legitimately hold
	//! different frames. A single global dirty flag would be wrong twice over - it would miss a
	//! per-side text change, and it would suppress a bar that moves every tick for the one player
	//! standing in the zone.
	//!
	//! Bounded by CONCURRENT players, never by round length: a key is written only when a push
	//! actually went out, dropped by `PruneHudSignatures` on the first tick the owner is off the
	//! connected list, and dropped again by `OnPlayerDisconnected`.
	protected ref map<int, string> m_mHudSignatures;

	//------------------------------------------------------------------------------------------------
	static TBD_ObjectivesComponent GetInstance()
	{
		return s_Instance;
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server - objectives are enforced where the mission document lives. Clients hold no
	//! mission document at all (recorded landmine), so a client-side runner would have no zones, no
	//! rules and nothing to advance.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		s_Instance = this;
		m_aBroadcasts = new array<string>();

		// Built before the client bail-out so no path can reach a null map. A client never replicates
		// anything from here, but `OnDelete` runs on both sides and an empty map is cheaper than a
		// null check on every teardown.
		m_mHudSignatures = new map<int, string>();

		if (RplSession.Mode() == RplMode.Client)
			return;

		GetGame().GetCallqueue().CallLater(Tick, TICK_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Statics OUTLIVE A WORLD inside one process (recorded landmine - `SelectMissionByNumber`
	//! restarts the scenario in-process). Without this, mission B would inherit mission A's captured
	//! objectives and could satisfy `all_objectives_captured` at kickoff, and the tick would keep
	//! firing against a dead component. `ScriptCallQueue.Remove` cancels by function, which is
	//! exactly right: there is one instance of this tick per world.
	override void OnDelete(IEntity owner)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Tick);

		// Clears the rules reader too. Deliberately does NOT clear `TBD_ZoneRegistry` - that belongs
		// to `TBD_PlayAreaComponent`, and two components racing to tear down one static buys nothing.
		// `TBD_Objective.m_Zone` is a strong reference so the teardown order cannot matter.
		TBD_ObjectiveRegistry.Clear();

		if (m_aBroadcasts)
			m_aBroadcasts.Clear();

		HideAllHuds();

		// `HideAllHuds` already dropped the signature of every owner it could still reach; whatever
		// is left belonged to a player whose controller had gone before the world did.
		if (m_mHudSignatures)
			m_mHudSignatures.Clear();

		s_Instance = null;

		super.OnDelete(owner);
	}

	//------------------------------------------------------------------------------------------------
	//! `SCR_BaseGameMode.OnPlayerDisconnected` dispatches to every `SCR_BaseGameModeComponent`, so a
	//! leaver's last-sent HUD signature goes the instant they do rather than on the next tick.
	//! `PruneHudSignatures` would catch it a second later anyway; this is the belt to that pair of
	//! braces, and it earns its place on RECYCLED playerIds - the next holder of that number must
	//! never inherit the previous one's frame and be told their HUD is already up to date.
	//! @authority server
	override void OnPlayerDisconnected(int playerId, KickCauseCode cause, int timeout)
	{
		super.OnPlayerDisconnected(playerId, cause, timeout);

		if (m_mHudSignatures)
			m_mHudSignatures.Remove(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected void Tick()
	{
		// The registry cannot be built until the mission is loaded AND valid, which happens some
		// seconds after this component exists. Retry silently until it does; `Build()` refuses
		// rather than caching an empty registry.
		if (!TBD_ObjectiveRegistry.IsBuilt())
		{
			if (!TBD_ObjectiveRegistry.Build())
				return;

			AnnounceOnce();
		}

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm || fm.GetStage() != TBD_EGameStage.LIVE)
		{
			if (m_bLive)
				HideAllHuds();
			m_bLive = false;
			return;
		}

		if (!m_bLive)
		{
			m_bLive = true;
			OnEnterLive();
		}

		if (UsableCount() == 0)
			return;

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> connected = new array<int>();
		players.GetPlayers(connected);

		array<ref TBD_Objective> objectives = TBD_ObjectiveRegistry.GetAll();
		if (!objectives)
			return;

		m_aBroadcasts.Clear();

		SamplePresence(players, connected, objectives);

		foreach (TBD_Objective objective : objectives)
		{
			if (!objective || !objective.m_bUsable)
				continue;

			if (objective.m_eKind == TBD_EObjectiveKind.CAPTURE)
				AdvanceCapture(objective);
			else if (objective.m_eKind == TBD_EObjectiveKind.HOLD_UNTIL)
				AdvanceHold(objective);
			else
				AdvanceDestroy(objective);
		}

		Deliver(players, connected, objectives);
		CheckEndTriggers();
	}

	//------------------------------------------------------------------------------------------------
	//! How many objectives can actually run. Re-read every tick rather than cached, because a
	//! destroy objective can go inert at arming time when it discovers there is nothing to destroy.
	protected int UsableCount()
	{
		int total = TBD_ObjectiveRegistry.GetCaptureCount();
		total += TBD_ObjectiveRegistry.GetDestroyCount();
		total += TBD_ObjectiveRegistry.GetHoldCount();
		return total;
	}

	//------------------------------------------------------------------------------------------------
	//! One informational line per world, at the moment the registry becomes known-good.
	protected void AnnounceOnce()
	{
		if (UsableCount() == 0)
		{
			if (!m_bAnnouncedNoObjectives)
			{
				m_bAnnouncedNoObjectives = true;
				TBD_Log.Event(TBD_ObjectiveRegistry.CH,
					"no usable objective zone in this mission - objective win conditions cannot fire, and nothing here will run");
			}
			return;
		}

		if (m_bAnnouncedArmed)
			return;

		m_bAnnouncedArmed = true;
		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "armed", string.Format("capture=%1 destroy=%2 hold=%3 cadence=%4ms",
			TBD_ObjectiveRegistry.GetCaptureCount(),
			TBD_ObjectiveRegistry.GetDestroyCount(),
			TBD_ObjectiveRegistry.GetHoldCount(),
			TICK_MS));
	}

	//------------------------------------------------------------------------------------------------
	//! The round just went LIVE.
	//!
	//! Destroy targets are found HERE rather than at load because a target placed by any other
	//! subsystem may not exist while the world is still in LOBBY; searching an empty world at load
	//! would report every destroy objective as targetless for the wrong reason.
	//!
	//! Hold announcement ladders are seeded here too, so a short hold does not fire three "time
	//! remaining" messages in its first three seconds.
	protected void OnEnterLive()
	{
		array<ref TBD_Objective> objectives = TBD_ObjectiveRegistry.GetAll();
		if (!objectives)
			return;

		foreach (TBD_Objective objective : objectives)
		{
			if (!objective || !objective.m_bUsable)
				continue;

			if (objective.m_eKind == TBD_EObjectiveKind.DESTROY && !objective.m_bArmed)
			{
				TBD_ObjectiveRegistry.ArmDestroyTargets(objective);
				continue;
			}

			if (objective.m_eKind != TBD_EObjectiveKind.HOLD_UNTIL)
				continue;

			// Skip every announcement mark that is at or above the total hold length.
			while (NextHoldMark(objective.m_iHoldMarkIndex) > 0
				&& NextHoldMark(objective.m_iHoldMarkIndex) >= objective.m_fHoldSeconds)
			{
				objective.m_iHoldMarkIndex = objective.m_iHoldMarkIndex + 1;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! One walk of the player list: who is standing on what, right now.
	//!
	//! Exclusions, each for a stated reason:
	//!   * no controlled entity - in the lobby, spectating, or mid-deploy; not on the ground;
	//!   * a spent life (`TBD_SpawnManager.IsPlayerDead`) - already out of the event under ONE LIFE;
	//!   * a dead body - a corpse lying on an objective must not hold it;
	//!   * no resolved faction - not on a side, so they can neither capture nor contest. Standing on
	//!     an objective unassigned must not be a way to freeze it.
	protected void SamplePresence(notnull PlayerManager players, notnull array<int> connected, notnull array<ref TBD_Objective> objectives)
	{
		foreach (TBD_Objective objective : objectives)
		{
			if (objective)
				objective.BeginSample();
		}

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();

		foreach (int playerId : connected)
		{
			IEntity body = players.GetPlayerControlledEntity(playerId);
			if (!body)
				continue;

			if (spawn && spawn.IsPlayerDead(playerId))
				continue;

			if (IsBodyDead(body))
				continue;

			string factionKey = ResolveFaction(spawn, playerId);
			if (factionKey.IsEmpty())
				continue;

			vector origin = body.GetOrigin();
			float px = origin[0];
			float pz = origin[2];

			foreach (TBD_Objective objective : objectives)
			{
				if (!objective || !objective.m_bUsable)
					continue;

				// The ONE containment test in this mod - T-181.18's, verified against an independent
				// oracle, inclusive of the boundary within EDGE_MARGIN_M for circles and polygons
				// alike. There is deliberately no geometry in this slice.
				if (!objective.m_Zone.Contains(px, pz))
					continue;

				if (!TBD_ZoneVolume.ContainsAgl(objective.m_sId, origin))
					continue;

				objective.AddPresence(factionKey);
				objective.m_aPresentPlayers.Insert(playerId);
			}
		}
	}

	// ============================================================================================
	//  CAPTURE
	// ============================================================================================

	//------------------------------------------------------------------------------------------------
	//! Advance one capture objective by one tick.
	//!
	//! == THE STATE MACHINE, IN ONE SENTENCE ===================================================
	//! `m_fProgress` is the uninterrupted presence `m_sProgressFaction` has banked toward owning
	//! this objective; when it fills they own it, and an owned objective's progress belongs to its
	//! owner and must be torn back down to zero before anyone else can bank their own.
	//!
	//! So taking an enemy-held objective is a TWO-STAGE job - neutralise, then capture - which is
	//! what the whole genre does and what makes holding ground worth something. The cost of the
	//! first stage is `rules.neutralizeSeconds`, which defaults to `captureSeconds` (a symmetric
	//! 1:1 rate) and can be set to `0` for an operator who wants a single-stage capture instead.
	//! That is the knob; the default is the conventional reading.
	//!
	//! An owned objective is NEVER lost to a timer - only to somebody walking onto it. See
	//! `TBD_EObjectiveOnEmpty` for why decay is off by default and why it applies only while
	//! neutral.
	protected void AdvanceCapture(notnull TBD_Objective objective)
	{
		// Also computes m_bContested. See TBD_Objective.ResolveActingFaction for what
		// `rules.contestable` means and why.
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

		objective.m_fSinceAnnounce = objective.m_fSinceAnnounce + TICK_SECONDS;

		if (IsTearingDown(objective, acting))
		{
			TearDownCapture(objective, acting);
			return;
		}

		BuildCapture(objective, acting);
	}

	//------------------------------------------------------------------------------------------------
	//! Nobody is standing on it.
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

		objective.m_fProgress = objective.m_fProgress - (objective.m_fDecayRate * TICK_SECONDS);
		if (objective.m_fProgress > 0)
			return;

		objective.m_fProgress = 0;
		objective.m_sProgressFaction = string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Is `acting` tearing something down rather than building?
	//! Two cases: an objective OWNED by somebody else, or partial progress banked by somebody else.
	//! Both use the same rate, so there is one teardown concept rather than two.
	protected bool IsTearingDown(notnull TBD_Objective objective, string acting)
	{
		if (!objective.m_sOwner.IsEmpty() && objective.m_sOwner != acting)
			return true;

		if (objective.m_sOwner.IsEmpty() && !objective.m_sProgressFaction.IsEmpty() && objective.m_sProgressFaction != acting)
			return true;

		return false;
	}

	//------------------------------------------------------------------------------------------------
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
			step = objective.TeardownRate() * TICK_SECONDS;
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

	//------------------------------------------------------------------------------------------------
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
		objective.m_fProgress = objective.m_fProgress + TICK_SECONDS;

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
		m_aBroadcasts.Insert(msg);

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "captured", string.Format("id=%1 owner=%2 points=%3",
			objective.m_sId, acting, objective.m_fPoints));
	}

	//------------------------------------------------------------------------------------------------
	//! Server log of capture progress. The HUD bar is the player-facing channel (T-941.4).
	protected void AnnounceCaptureProgress(notnull TBD_Objective objective, string acting, string verb)
	{
		if (objective.m_fSinceAnnounce < objective.m_fAnnounceEverySeconds)
			return;

		objective.m_fSinceAnnounce = 0;

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "captureProgress", string.Format("id=%1 verb=%2 percent=%3 acting=%4",
			objective.m_sId, verb, objective.ProgressPercent(), acting));
	}

	// ============================================================================================
	//  HOLD UNTIL
	// ============================================================================================

	//------------------------------------------------------------------------------------------------
	//! Advance one hold objective by one tick.
	//!
	//! == WHY AN ENEMY INSIDE PAUSES THE CLOCK BY DEFAULT ======================================
	//! `hold_expired` reads as "the hold timer ran out", and `last-stand-at-montfort.json` confirms
	//! it: mode `defender_holds_or_attacker_destroys`, `holdSeconds: 2700` inside a 3000 s limit.
	//! The defenders win by surviving the clock.
	//!
	//! The naive implementation is a pure timer - but then the authored ZONE does nothing at all,
	//! and a mission author who drew a 70 m circle around Montfort Manor plainly meant the ground to
	//! matter. "Hold" means holding ground. So by default an enemy standing inside PAUSES the clock:
	//! attackers deny the hold by occupying the objective, which is the tactical shape the mission's
	//! own mode string describes.
	//!
	//! `rules.pauseOnEnemy: false` restores the pure timer for an author who wants one, and
	//! `rules.resetOnEnemy: true` makes an incursion punishing rather than merely delaying.
	//!
	//! `requireHolderPresent` defaults to FALSE on purpose. Under ONE LIFE a 45-minute hold that
	//! demands a continuously-manned zone becomes unwinnable the moment the defenders take
	//! casualties they cannot replace - the clock would stop for a reason they can no longer fix.
	//! It is one key away for an author who wants that pressure.
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

		objective.m_fHeldSeconds = objective.m_fHeldSeconds + TICK_SECONDS;

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
		m_aBroadcasts.Insert(msg);

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "holdExpired", string.Format("id=%1 holder=%2 held=%3s points=%4",
			objective.m_sId, objective.m_sFaction, objective.m_fHoldSeconds, objective.m_fPoints));
	}

	//------------------------------------------------------------------------------------------------
	//! Announce the hold clock at a fixed ladder of remaining times rather than on a fixed interval.
	//!
	//! A 2700 s hold on a 60 s interval is forty-five identical messages; a ladder is six, each at a
	//! moment that actually changes what a player should do. Both sides hear it - a hold is a race
	//! and the attackers need the clock as much as the defenders.
	protected void AnnounceHoldMark(notnull TBD_Objective objective)
	{
		float mark = NextHoldMark(objective.m_iHoldMarkIndex);
		if (mark <= 0)
			return;

		float remaining = objective.HoldRemaining();
		if (remaining > mark)
			return;

		objective.m_iHoldMarkIndex = objective.m_iHoldMarkIndex + 1;

		// Rounded into an int FIRST - see TBD_Objective.HoldStatusText for why.
		int whole = Math.Round(remaining);

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "holdMark", string.Format("id=%1 remain=%2s holder=%3",
			objective.m_sId, whole, objective.m_sFaction));
	}

	//------------------------------------------------------------------------------------------------
	//! The remaining-time ladder, in seconds. Written as a function rather than a static array
	//! because a `static const ref array<>` initialiser is not something this lane has proven, and a
	//! six-branch lookup is not worth a runtime experiment. Returns -1 past the end.
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

	// ============================================================================================
	//  DESTROY
	// ============================================================================================

	//------------------------------------------------------------------------------------------------
	//! Advance one destroy objective. The counting lives in the registry next to the world query it
	//! needs; this is only the announcement.
	//!
	//! Read `TBD_ObjectiveRegistry.ArmDestroyTargets` before trusting this: the destruction SIGNAL
	//! is proven, and T-254 spawns authored `entities[]` via `SpawnMissionEntities`. A destroy
	//! objective still goes inert when the alias is unresolved, nothing matching sits in the zone,
	//! or spawn/query missed - `m_sInertReason` names which (T-437).
	protected void AdvanceDestroy(notnull TBD_Objective objective)
	{
		if (!TBD_ObjectiveRegistry.EvaluateDestroy(objective))
			return;

		string msg = "TBD: ";
		msg += objective.DisplayName();
		msg += " has been DESTROYED.";
		m_aBroadcasts.Insert(msg);

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "destroyed", string.Format("id=%1 alias='%2' destroyed=%3/%4 by=%5 points=%6",
			objective.m_sId,
			objective.m_sTargetAlias,
			objective.m_iTargetsDestroyed,
			objective.RequiredKills(),
			objective.m_sFaction,
			objective.m_fPoints));
	}

	// ============================================================================================
	//  DELIVERY AND END TRIGGERS
	// ============================================================================================

	//------------------------------------------------------------------------------------------------
	//! One walk of the player list: complete-lines over chat, board + bar over HUD RPC.
	protected void Deliver(notnull PlayerManager players, notnull array<int> connected, notnull array<ref TBD_Objective> objectives)
	{
		foreach (string broadcast : m_aBroadcasts)
		{
			foreach (int playerId : connected)
			{
				Tell(players, playerId, broadcast);
			}
		}

		ReplicateHud(players, connected, objectives);
	}

	//------------------------------------------------------------------------------------------------
	//! Has an objective-driven end condition been met?
	//!
	//! == WHY THIS ONLY LOGS ===================================================================
	//! Ending the round means `TBD_FrameworkManager.SetStage(END)`, and the stage machine has ONE
	//! owner. A second component that could end the round would split that authority across two
	//! files and make "why did the round end" a question with two places to look. So this slice
	//! exposes `TBD_ObjectiveRegistry.EvaluateEndTriggers()` as the authority and stops there.
	//!
	//! Until `TickWinConditions` calls it, a met condition would otherwise be COMPLETELY SILENT -
	//! the objectives would be captured and the round would simply carry on. That is the worst
	//! possible failure mode for a wiring gap, so it is announced once, loudly, naming the exact
	//! call that is missing. If you are reading this line in a log, the seam is not wired yet.
	protected void CheckEndTriggers()
	{
		if (m_bEndTriggerAnnounced)
			return;

		string winner;
		string trigger = TBD_ObjectiveRegistry.EvaluateEndTriggers(winner);
		if (trigger.IsEmpty())
			return;

		m_bEndTriggerAnnounced = true;

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "endTriggerMet", string.Format("trigger=%1 winner='%2'", trigger, winner));
		TBD_Log.Banner(TBD_ObjectiveRegistry.CH,
			"OBJECTIVE END CONDITION MET but nothing acted on it - TBD_FrameworkManager.TickWinConditions must call TBD_ObjectiveRegistry.EvaluateEndTriggers(). The round will NOT end on its own.",
			false);
	}

	// ============================================================================================
	//  SERVER-SIDE QUERY SURFACE
	// ============================================================================================

	//------------------------------------------------------------------------------------------------
	//! The objective board as THIS player is allowed to see it.
	//!
	//! The viewer's side is resolved here, from the player's assigned slot, which is server-owned
	//! state. There is no faction parameter on this path, so there is nothing for a client to forge
	//! - the same discipline that makes `TBD_MarkerController`'s request RPC take no arguments.
	//!
	//! Server-owned board for one player. The HUD is the consumer (T-941.4); chat is complete-only.
	array<string> BuildBoardForPlayer(int playerId)
	{
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		string factionKey = ResolveFaction(spawn, playerId);
		return TBD_ObjectiveRegistry.BuildBoardForFaction(factionKey);
	}

	//------------------------------------------------------------------------------------------------
	//! Push the HUD snapshot to every connected player whose frame ACTUALLY MOVED. Still evaluated at
	//! the 1 Hz tick, but the wire cost is now proportional to CHANGE rather than to elapsed time.
	//!
	//! == WHY THIS IS GATED AT ALL =============================================================
	//! T-941.4 replaced a per-tick private-chat pump with this HUD, which is the right answer; sending
	//! it unconditionally was not. Ungated, this loop cost one Reliable Owner RPC of three string
	//! arrays PER PLAYER PER SECOND - 60 per player per minute on a round where nothing whatsoever
	//! happened, scaling with players x rows and never falling to zero. That is the very metric the
	//! chat pump was removed for, so T-941.4 could regress it (T-946.55).
	//!
	//! The comparison is per OWNER and against the RENDERED snapshot, because the rendered snapshot is
	//! what the RPC carries. An owner with no record is UNKNOWN, not clean, so a mid-round joiner -
	//! or anyone who has only just gained a player controller - is sent a full board on the very next
	//! tick rather than waiting for somebody else to move.
	protected void ReplicateHud(notnull PlayerManager players, notnull array<int> connected, notnull array<ref TBD_Objective> board)
	{
		PruneHudSignatures(connected);

		foreach (int playerId : connected)
		{
			array<string> icons = new array<string>();
			array<string> titles = new array<string>();
			array<string> details = new array<string>();
			string barLabel;
			int barPercent;
			int barVisible;

			FillHudSnapshot(playerId, board, icons, titles, details, barLabel, barPercent, barVisible);

			string signature = HudSignature(icons, titles, details, barLabel, barPercent, barVisible);
			if (!HudChanged(playerId, signature))
				continue;

			// Recorded ONLY when the RPC actually went out. A connected player with no controller yet
			// is skipped by the push, and recording here would leave them holding a frame they were
			// never sent and would never be offered again.
			if (PushHudSnapshot(players, playerId, icons, titles, details, barLabel, barPercent, barVisible, 1))
				m_mHudSignatures.Set(playerId, signature);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Does this owner's rendered snapshot differ from the last one they were actually sent?
	//!
	//! A MISS IS A SEND, never a skip, and that single line is the whole anti-starvation argument: the
	//! map holds only owners the server has provably pushed to, so everybody else - a new connection,
	//! a recycled id, a player whose HUD was hidden when the round left LIVE - reads as unknown and
	//! gets the entire board.
	protected bool HudChanged(int playerId, string signature)
	{
		string previous;
		if (!m_mHudSignatures.Find(playerId, previous))
			return true;

		return previous != signature;
	}

	//------------------------------------------------------------------------------------------------
	//! One string standing for exactly what the RPC would carry: every rendered row, plus the capture
	//! bar triple.
	//!
	//! LENGTH-PREFIXED, and that is not decoration. With plain separators two different snapshots can
	//! collide - ("A|B", "C") and ("A", "B|C") concatenate to the same bytes - and a HUD that MISSES a
	//! real change is a far worse failure than one that sends a redundant frame. Prefixing each field
	//! with its own length makes a collision impossible without needing a hash the engine does not
	//! expose to script. Row count is clamped to the shortest array so a future edit to
	//! `FillHudSnapshot` cannot turn this into an out-of-range read at 1 Hz.
	protected string HudSignature(notnull array<string> icons, notnull array<string> titles,
		notnull array<string> details, string barLabel, int barPercent, int barVisible)
	{
		string signature = string.Format("%1|%2|%3|%4", barVisible, barPercent, barLabel.Length(), barLabel);

		int rows = icons.Count();
		if (titles.Count() < rows)
			rows = titles.Count();
		if (details.Count() < rows)
			rows = details.Count();

		for (int i = 0; i < rows; i++)
		{
			string icon = icons.Get(i);
			string title = titles.Get(i);
			string detail = details.Get(i);
			signature += string.Format("|%1|%2|%3|%4|%5|%6",
				icon.Length(), icon, title.Length(), title, detail.Length(), detail);
		}

		return signature;
	}

	//------------------------------------------------------------------------------------------------
	//! Drop the last-sent record for owners who are no longer connected. Collected first and removed
	//! after: mutating a map while iterating it is not safe, and Enforce Script's `array.Remove` is by
	//! INDEX (recorded landmine), so the removal below is by KEY on the map - the same shape
	//! `TBD_PlayAreaComponent.PruneDeparted` uses for its violation rows.
	//!
	//! `Tick` re-reads the player list every second, so this ALONE bounds the map by concurrent
	//! players. It does not depend on the disconnect hook firing, which is why the hook is a second
	//! belt rather than the only one.
	protected void PruneHudSignatures(notnull array<int> connected)
	{
		if (m_mHudSignatures.Count() == 0)
			return;

		array<int> stale = new array<int>();
		foreach (int playerId, string signature : m_mHudSignatures)
		{
			if (connected.Find(playerId) == -1)
				stale.Insert(playerId);
		}

		foreach (int playerId : stale)
		{
			m_mHudSignatures.Remove(playerId);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Close the HUD on every connected client (left LIVE, or the component is tearing down).
	protected void HideAllHuds()
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> connected = new array<int>();
		players.GetPlayers(connected);
		array<ref TBD_Objective> board = new array<ref TBD_Objective>();
		foreach (int playerId : connected)
		{
			PushHudToPlayer(players, playerId, board, 0);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Public so a late-joining controller can ask without inventing a second snapshot builder.
	void PushHudTo(int playerId)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<ref TBD_Objective> board = TBD_ObjectiveRegistry.GetAll();
		if (!board)
			board = new array<ref TBD_Objective>();

		PushHudToPlayer(players, playerId, board, 1);
	}

	//------------------------------------------------------------------------------------------------
	//! The UNGATED push, and it stays ungated on purpose. Its two callers are the paths that must never
	//! be suppressed: `HideAllHuds` when the round leaves LIVE, and `PushHudTo`, the pull a client asks
	//! for. Both therefore RESET what this owner is known to hold - a hidden HUD is not a clean one,
	//! and re-entering LIVE on an unchanged board must still re-open the panel.
	//!
	//! MEASURED 2026-09-08, so the 1 Hz gate above cannot lean on it: the pull path is currently DEAD.
	//! `TBD_ObjectiveHud.TBD_RequestObjectiveHud` reaches `PushHudTo`, but nothing in `apps/mod` calls
	//! `TBD_RequestObjectiveHud` - its declaration is the only occurrence in either tree. So the sole
	//! thing standing between a joiner and an empty HUD is `HudChanged` treating an unknown owner as a
	//! send, which is why that rule is stated there rather than assumed here.
	protected void PushHudToPlayer(notnull PlayerManager players, int playerId, notnull array<ref TBD_Objective> board, int show)
	{
		array<string> icons = new array<string>();
		array<string> titles = new array<string>();
		array<string> details = new array<string>();
		string barLabel;
		int barPercent;
		int barVisible;

		FillHudSnapshot(playerId, board, icons, titles, details, barLabel, barPercent, barVisible);

		if (!PushHudSnapshot(players, playerId, icons, titles, details, barLabel, barPercent, barVisible, show))
			return;

		if (show == 0)
			m_mHudSignatures.Remove(playerId);
		else
			m_mHudSignatures.Set(playerId, HudSignature(icons, titles, details, barLabel, barPercent, barVisible));
	}

	//------------------------------------------------------------------------------------------------
	//! The wire itself: one Reliable Owner RPC, via `SCR_PlayerController.TBD_PushObjectiveHud` ->
	//! `TBD_RpcDo_ObjectiveHud`.
	//!
	//! Returns whether it actually went out. A connected player with no player controller is not an
	//! error - they are in the lobby, spectating or mid-deploy - but the caller must not record a
	//! frame that never left the server, or that player would be treated as up to date and the gate in
	//! `ReplicateHud` would starve them for the rest of the round.
	protected bool PushHudSnapshot(notnull PlayerManager players, int playerId, notnull array<string> icons,
		notnull array<string> titles, notnull array<string> details, string barLabel, int barPercent,
		int barVisible, int show)
	{
		SCR_PlayerController controller = SCR_PlayerController.Cast(players.GetPlayerController(playerId));
		if (!controller)
			return false;

		controller.TBD_PushObjectiveHud(icons, titles, details, barLabel, barPercent, barVisible, show);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected void FillHudSnapshot(int playerId, notnull array<ref TBD_Objective> board,
		notnull array<string> icons, notnull array<string> titles, notnull array<string> details,
		out string barLabel, out int barPercent, out int barVisible)
	{
		barLabel = string.Empty;
		barPercent = 0;
		barVisible = 0;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		string factionKey = ResolveFaction(spawn, playerId);

		foreach (TBD_Objective objective : board)
		{
			if (!objective)
				continue;

			icons.Insert(HudIcon(objective, factionKey));
			string title = objective.TitleFor(factionKey);
			titles.Insert(title);

			// T-212 framing must reach the rendered snapshot that T-946.55 signs and sends.
			// Keep status first; an unframed legacy row remains exactly its original status.
			string detail = objective.StatusText(factionKey);
			string taskText = objective.TaskTextFor(factionKey);
			if (!taskText.IsEmpty())
			{
				if (!detail.IsEmpty())
					detail += " | ";
				detail += taskText;
			}
			details.Insert(detail);

			if (objective.m_eKind != TBD_EObjectiveKind.CAPTURE)
				continue;
			if (objective.m_aPresentPlayers.Find(playerId) == -1)
				continue;

			bool take = barVisible == 0 || objective.m_bContested;
			if (!take)
				continue;

			barVisible = 1;
			barLabel = title;
			barPercent = objective.ProgressPercent();
		}
	}

	//------------------------------------------------------------------------------------------------
	//! One ASCII glyph per row: o neutral, + ours, - theirs, ! contested, v complete, # destroy, H hold.
	protected string HudIcon(notnull TBD_Objective objective, string factionKey)
	{
		if (!objective.m_bUsable)
			return ".";
		if (objective.m_bComplete)
			return "v";
		if (objective.m_bContested)
			return "!";
		if (objective.m_eKind == TBD_EObjectiveKind.DESTROY)
			return "#";
		if (objective.m_eKind == TBD_EObjectiveKind.HOLD_UNTIL)
			return "H";
		if (objective.m_sOwner.IsEmpty())
			return "o";
		if (!factionKey.IsEmpty() && objective.m_sOwner == factionKey)
			return "+";
		return "-";
	}

	//------------------------------------------------------------------------------------------------
	//! Server -> one client, over the channel this codebase already uses for per-player replies
	//! (`TBD_AdminCommands.Reply`, `TBD_PlayAreaComponent.Tell`). Complete-lines only (T-941.4).
	protected void Tell(notnull PlayerManager players, int playerId, string text)
	{
		PlayerController pc = players.GetPlayerController(playerId);
		if (!pc)
			return;

		SCR_ChatComponent chat = SCR_ChatComponent.Cast(pc.FindComponent(SCR_ChatComponent));
		if (!chat)
			return;

		chat.SendPrivateMessage(text, playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! The faction whose side this player is on, or empty when they have no assigned slot. Empty is
	//! a legitimate answer, not an error - but a player with no side is excluded from objective
	//! presence entirely, because they can neither capture nor contest.
	protected string ResolveFaction(TBD_SpawnManager spawn, int playerId)
	{
		if (!spawn)
			return string.Empty;

		TBD_MissionSlotStruct slot = spawn.GetAssignedSlot(playerId);
		if (!slot)
			return string.Empty;

		return slot.faction;
	}

	//------------------------------------------------------------------------------------------------
	//! Is this body a corpse? Asked separately from `TBD_SpawnManager.IsPlayerDead` because the two
	//! answer different questions - one is "has this identity spent its life", the other is "is the
	//! thing standing here alive right now" - and either being true means it does not hold ground.
	protected bool IsBodyDead(notnull IEntity body)
	{
		SCR_ChimeraCharacter character = SCR_ChimeraCharacter.Cast(body);
		if (!character)
			return false;

		CharacterControllerComponent controller = character.GetCharacterController();
		if (!controller)
			return false;

		return controller.IsDead();
	}
}
