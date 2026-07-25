//! T-181.18 — per-player state for the play-area enforcer. One of these exists only while a
//! player is actually in violation; going back inside deletes it, which is what makes "am I being
//! warned right now" a null check rather than a flag that can get out of step.
class TBD_PlayAreaViolation
{
	float m_fSecondsOutside;      //!< Accumulated, in ticks, since the violation started.
	float m_fSecondsSinceWarned;  //!< Since the last message to the player.
	string m_sZoneKey;            //!< Which zone is being violated, so a change of zone re-announces.
	bool m_bPenaltyApplied;       //!< Latch: the penalty fires ONCE per violation, never per tick.
	//! Has this player actually been told anything? Needed as its own flag rather than inferred
	//! from `m_fSecondsSinceWarned > 0`: that counter is reset to 0 at the moment a warning is
	//! sent, so a player who stepped out and back inside within one tick would have been warned
	//! and then never told the clock had stopped.
	bool m_bWarned;
	//! The body this violation was observed against. A dedicated server RECYCLES numeric player
	//! ids, so a state row keyed only on the id could in principle outlive its owner by a tick and
	//! hand a fresh joiner a nearly-expired grace countdown. Comparing the controlled entity's id
	//! closes that: two players cannot share one body.
	EntityID m_LastBody;
}

[ComponentEditorProps(category: "TBD/Framework", description: "TBD play area — boundary / base-protection zones, out-of-bounds warning, grace and penalty.")]
class TBD_PlayAreaComponentClass : SCR_BaseGameModeComponentClass {}

//! T-181.18 — the AO. Server-authoritative out-of-bounds detection for `boundary` and
//! `base_protection` zones, with a warning, a grace countdown and a JSON-driven penalty.
//!
//! ══ THE ONE-LIFE DECISION — READ THIS BEFORE CHANGING THE DEFAULT ═══════════════════════════
//! TBD events are ONE LIFE. Death is terminal by design (TBD_MOD_DESIGN.md §2), recoverable only
//! by an admin `#tbd respawn`. "Kill the player for leaving the AO" is therefore not a slap on the
//! wrist — it is **permanent removal from the event**, for what is very often a navigation
//! mistake at a map edge nobody can see.
//!
//! This slice implements the terminal penalty in full and makes it **JSON-driven and off by
//! default**: `zones[].rules.penalty` is `"warn"` unless a mission says otherwise, so an authored
//! mission that is silent about penalties gets an AO that warns, counts down, keeps warning, and
//! never ends anybody's night. An operator who wants a hard AO writes `"penalty": "kill"` on that
//! zone and gets it, with the choice recorded in the log at load time. Changing the DEFAULT is an
//! operator decision, not an engineering one.
//!
//! ══ How it runs ═════════════════════════════════════════════════════════════════════════════
//! ONE repeating server-side tick at `TICK_MS` walks the connected players — deliberately not a
//! per-player timer and emphatically not a per-frame check. Two reasons beyond cost:
//!   * `ScriptCallQueue.Remove` cancels BY FUNCTION, not by arguments (recorded landmine), so a
//!     per-player `CallLater` could not be cancelled for one player without cancelling all of
//!     them — and a per-player deferred callback carrying a raw `playerId` survives that player's
//!     disconnect onto a RECYCLED id. A single tick that re-reads the live player list every time
//!     has neither problem and needs no connection epoch.
//!   * Enforcement only has to be as responsive as the grace period, which is measured in tens of
//!     seconds.
//!
//! ══ Honest failure ══════════════════════════════════════════════════════════════════════════
//! A mission with no `boundary` zone imposes NO play-area restriction. That is the deliberate
//! reading of an absent AO, it is stated once in the log, and the two verdicts "no boundary
//! applies to me" and "a boundary applies and I am outside it" are kept strictly apart in
//! `TBD_ZoneRegistry` so they can never collapse into confining everybody.
//!
//! ══ What is NOT proven here ═════════════════════════════════════════════════════════════════
//! Every API this uses is compile-proven against this engine build with a failing negative
//! control. Nothing on this lane can prove that `SCR_ChatComponent.SendPrivateMessage` reaches a
//! real client, or that `SCR_CharacterDamageManagerComponent.Kill` lands in
//! `SCR_BaseGameMode.OnPlayerKilled`. The reasoning for the latter is that `Kill(GetInstigator())`
//! is what vanilla's own `SCR_CharacterDamageManagerComponent.UpdateConsciousness` calls to end a
//! character, so it is the engine's terminal call and not a second death path invented here — but
//! that is an argument, not a measurement, and it wants a live server to settle.
class TBD_PlayAreaComponent : SCR_BaseGameModeComponent
{
	//! Enforcement cadence. 1 Hz: fine enough that a grace period measured in tens of seconds is
	//! accurate to within a second, coarse enough that walking every player costs nothing.
	static const int TICK_MS = 1000;

	//! `TICK_MS` as seconds, so the accumulators read in the same units the rules are authored in.
	static const float TICK_SECONDS = 1.0;

	protected static TBD_PlayAreaComponent s_Instance;

	protected ref map<int, ref TBD_PlayAreaViolation> m_mViolations;

	//! Latches for the once-per-world informational lines, so a 1 Hz tick cannot spam the log.
	protected bool m_bAnnouncedNoBoundary;
	protected bool m_bAnnouncedArmed;

	//------------------------------------------------------------------------------------------------
	static TBD_PlayAreaComponent GetInstance()
	{
		return s_Instance;
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — the play area is enforced where the mission document lives. Clients hold
	//! no mission document at all (recorded landmine), so a client-side check would have no zones
	//! to check against and would either do nothing or confine everyone.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		s_Instance = this;
		m_mViolations = new map<int, ref TBD_PlayAreaViolation>();

		if (RplSession.Mode() == RplMode.Client)
			return;

		GetGame().GetCallqueue().CallLater(Tick, TICK_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Statics OUTLIVE A WORLD inside one process (recorded landmine — `SelectMissionByNumber`
	//! restarts the scenario in-process). Without this, mission B's players would be confined to
	//! mission A's AO by a registry nobody rebuilt, and the tick would keep firing against a dead
	//! component. `ScriptCallQueue.Remove` cancels by function, which is exactly right: there is
	//! one instance of this tick per world.
	override void OnDelete(IEntity owner)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Tick);

		TBD_ZoneRegistry.Clear();
		if (m_mViolations)
			m_mViolations.Clear();

		s_Instance = null;

		super.OnDelete(owner);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected void Tick()
	{
		// The registry cannot be built until the mission is loaded AND valid, which happens some
		// seconds after this component exists. Retry silently until it does; `Build()` refuses
		// rather than caching an empty registry.
		if (!TBD_ZoneRegistry.IsBuilt())
		{
			if (!TBD_ZoneRegistry.Build())
				return;

			AnnounceOnce();
		}

		// Enforce only while the round is LIVE. Not during SAFE_START: safestart is the phase
		// where damage is off and nothing a player does can hurt anybody, and starting an
		// out-of-bounds countdown against somebody still walking to their start line would be a
		// trap. Not during LOBBY/BRIEFING/END either — nobody is meant to be manoeuvring.
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm || fm.GetStage() != TBD_EGameStage.LIVE)
		{
			// Leaving stale countdowns behind across a stage change would resume a player's grace
			// mid-way when LIVE returns. Drop them.
			if (m_mViolations.Count() > 0)
				m_mViolations.Clear();
			return;
		}

		// Nothing to enforce: no usable boundary and no usable base-protection zone in the mission.
		if (TBD_ZoneRegistry.GetBoundaryCount() == 0 && TBD_ZoneRegistry.GetBaseProtectionCount() == 0)
			return;

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> connected = new array<int>();
		players.GetPlayers(connected);

		// Prune first, on the live list, so a disconnected player's row can never be inherited by
		// whoever the server hands that number to next.
		PruneDeparted(connected);

		foreach (int playerId : connected)
		{
			EvaluatePlayer(players, playerId);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! One informational line per world, at the moment the registry becomes known-good.
	protected void AnnounceOnce()
	{
		if (TBD_ZoneRegistry.GetBoundaryCount() == 0)
		{
			if (!m_bAnnouncedNoBoundary)
			{
				m_bAnnouncedNoBoundary = true;
				TBD_Log.Event(TBD_ZoneRegistry.CH,
					"no usable boundary zone in this mission — NO play-area restriction is in force");
			}
			return;
		}

		if (m_bAnnouncedArmed)
			return;

		m_bAnnouncedArmed = true;
		TBD_Log.Kv(TBD_ZoneRegistry.CH, "armed", string.Format("boundary=%1 baseProtection=%2 cadence=%3ms",
			TBD_ZoneRegistry.GetBoundaryCount(), TBD_ZoneRegistry.GetBaseProtectionCount(), TICK_MS));
	}

	//------------------------------------------------------------------------------------------------
	//! Drop violation rows for players who are no longer connected. Collected first and removed
	//! after: mutating a map while iterating it is not safe, and Enforce Script's `array.Remove`
	//! is by INDEX (recorded landmine), so the removal below is by key on the map, not by index.
	protected void PruneDeparted(notnull array<int> connected)
	{
		if (m_mViolations.Count() == 0)
			return;

		array<int> stale = new array<int>();
		foreach (int playerId, TBD_PlayAreaViolation v : m_mViolations)
		{
			if (connected.Find(playerId) == -1)
				stale.Insert(playerId);
		}

		foreach (int playerId : stale)
		{
			m_mViolations.Remove(playerId);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected void EvaluatePlayer(notnull PlayerManager players, int playerId)
	{
		IEntity body = players.GetPlayerControlledEntity(playerId);

		// No body: in the lobby, spectating, or mid-deploy. Clearing rather than pausing is
		// deliberate — a freshly deployed player must always start clean, and it is the second
		// half of the recycled-id defence (see TBD_PlayAreaViolation.m_LastBody).
		if (!body)
		{
			m_mViolations.Remove(playerId);
			return;
		}

		// A spent life is already out of the event; hounding a corpse about the AO would be noise
		// at best and, with penalty=kill, an attempt to kill someone who is already dead.
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (spawn && spawn.IsPlayerDead(playerId))
		{
			m_mViolations.Remove(playerId);
			return;
		}

		if (IsBodyDead(body))
		{
			m_mViolations.Remove(playerId);
			return;
		}

		string factionKey = ResolveFaction(spawn, playerId);
		vector origin = body.GetOrigin();
		float px = origin[0];
		float pz = origin[2];

		TBD_Zone violated = FindViolation(factionKey, px, pz);
		if (!violated)
		{
			ClearViolation(players, playerId);
			return;
		}

		AccumulateViolation(players, playerId, body, violated);
	}

	//------------------------------------------------------------------------------------------------
	//! The zone this player is currently in violation of, or null when they are where they belong.
	//!
	//! Boundary first: being outside the AO is the more serious of the two and its message should
	//! win when a player manages both at once.
	protected TBD_Zone FindViolation(string factionKey, float px, float pz)
	{
		if (TBD_ZoneRegistry.HasBoundaryFor(factionKey) && !TBD_ZoneRegistry.IsInsideBoundary(factionKey, px, pz))
			return TBD_ZoneRegistry.GoverningBoundary(factionKey);

		return TBD_ZoneRegistry.FindViolatedProtection(factionKey, px, pz);
	}

	//------------------------------------------------------------------------------------------------
	//! Back inside. Say so once — a player who has been counting down deserves to know the clock
	//! stopped — then forget them.
	protected void ClearViolation(notnull PlayerManager players, int playerId)
	{
		TBD_PlayAreaViolation state = m_mViolations.Get(playerId);
		if (!state)
			return;

		m_mViolations.Remove(playerId);

		// Only tell them the clock stopped if they were ever told it was running. Nothing is ever
		// said to a player under penalty=NONE, so nothing is said now either.
		if (state.m_bWarned)
			Tell(players, playerId, "TBD: back inside the play area.");

		TBD_Log.Kv(TBD_ZoneRegistry.CH, "returned", string.Format("player=%1 zone=%2 outsideFor=%3s",
			playerId, state.m_sZoneKey, state.m_fSecondsOutside));
	}

	//------------------------------------------------------------------------------------------------
	//! In violation: count, warn on cadence, and fire the penalty once at expiry.
	protected void AccumulateViolation(notnull PlayerManager players, int playerId, notnull IEntity body, notnull TBD_Zone zone)
	{
		string zoneKey = zone.LogKey();
		EntityID bodyId = body.GetID();

		TBD_PlayAreaViolation state = m_mViolations.Get(playerId);

		// Restart the countdown when this is a new violation, a DIFFERENT zone, or a different
		// body under the same player id (the recycled-id case).
		if (!state || state.m_sZoneKey != zoneKey || state.m_LastBody != bodyId)
		{
			state = new TBD_PlayAreaViolation();
			state.m_sZoneKey = zoneKey;
			state.m_LastBody = bodyId;
			m_mViolations.Set(playerId, state);

			TBD_Log.Kv(TBD_ZoneRegistry.CH, "violation", string.Format("player=%1 zone=%2 grace=%3s penalty=%4",
				playerId, zoneKey, zone.m_fGraceSeconds, typename.EnumToString(TBD_EZonePenalty, zone.m_ePenalty)));

			WarnPlayer(players, playerId, state, zone, zone.m_fGraceSeconds);
			state.m_fSecondsSinceWarned = 0;
			return;
		}

		// The countdown starts on the tick AFTER first detection, so a player gets the full authored
		// grace plus up to one tick. Biased generous on purpose — see TBD_Zone.EDGE_MARGIN_M for
		// the same reasoning.
		state.m_fSecondsOutside += TICK_SECONDS;

		// The penalty is a LATCH, not a per-tick action: a player who is killed and somehow still
		// registers as outside next tick is not killed twice, and a WARN zone stops nagging once
		// the grace has expired rather than repeating forever.
		if (state.m_bPenaltyApplied)
			return;

		float remaining = zone.m_fGraceSeconds - state.m_fSecondsOutside;
		if (remaining <= 0)
		{
			state.m_bPenaltyApplied = true;
			ApplyPenalty(players, playerId, body, zone);
			return;
		}

		state.m_fSecondsSinceWarned += TICK_SECONDS;
		if (state.m_fSecondsSinceWarned >= zone.m_fWarnEverySeconds)
		{
			state.m_fSecondsSinceWarned = 0;
			WarnPlayer(players, playerId, state, zone, remaining);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Tell the player what is happening and how long they have. Built in steps rather than one
	//! long `+` chain — a 9-term concatenation is a measured `Formula too complex` in this
	//! compiler. `->` rather than `→`: the arrow glyph is not in the proven set for shipped
	//! screens and a tofu box in a countdown a player has seconds to read is not acceptable.
	protected void WarnPlayer(notnull PlayerManager players, int playerId, notnull TBD_PlayAreaViolation state, notnull TBD_Zone zone, float secondsRemaining)
	{
		if (zone.m_ePenalty == TBD_EZonePenalty.NONE)
			return;

		state.m_bWarned = true;

		int whole = Math.Round(secondsRemaining);
		if (whole < 0)
			whole = 0;

		string what = "outside the play area";
		if (zone.m_sType == TBD_ZoneRegistry.TYPE_BASE_PROTECTION)
			what = "inside a protected area";

		string msg = "TBD: you are ";
		msg += what;
		msg += " (";
		msg += zone.DisplayName();
		msg += ") -- return within ";
		msg += whole.ToString();
		msg += "s.";

		if (zone.m_ePenalty == TBD_EZonePenalty.KILL)
			msg += " ONE LIFE: you will be killed and cannot respawn.";

		Tell(players, playerId, msg);
	}

	//------------------------------------------------------------------------------------------------
	//! Grace expired.
	protected void ApplyPenalty(notnull PlayerManager players, int playerId, notnull IEntity body, notnull TBD_Zone zone)
	{
		if (zone.m_ePenalty == TBD_EZonePenalty.NONE)
		{
			TBD_Log.Kv(TBD_ZoneRegistry.CH, "expired", string.Format("player=%1 zone=%2 penalty=none — logged only",
				playerId, zone.LogKey()));
			return;
		}

		if (zone.m_ePenalty == TBD_EZonePenalty.WARN)
		{
			TBD_Log.Warn(TBD_ZoneRegistry.CH, string.Format("player=%1 zone=%2 grace expired — penalty=warn, no action taken",
				playerId, zone.LogKey()));
			Tell(players, playerId, "TBD: you are still out of the play area. Return to the AO.");
			return;
		}

		KillForViolation(players, playerId, body, zone);
	}

	//------------------------------------------------------------------------------------------------
	//! Terminal, under ONE LIFE.
	//!
	//! Deliberately NOT named `Kill`: this is a `SCR_BaseGameModeComponent` subclass and a bare
	//! `Kill` risks silently shadowing something in that hierarchy, which Enfusion would not
	//! necessarily complain about.
	//!
	//! Routed through the ENGINE's own kill — `SCR_CharacterDamageManagerComponent.Kill(Instigator)`
	//! is what vanilla's `UpdateConsciousness` calls to end a character — so this lands in
	//! `SCR_BaseGameMode.OnPlayerKilled` -> `TBD_SpawnManager.OnPlayerKilled` -> `MarkLifeSpent`
	//! exactly like a bullet would. No second way to end a life is invented here, nothing in
	//! `TBD_SpawnManager` is touched, and the admin `#tbd respawn` escape hatch keeps working
	//! because it always has.
	//!
	//! Self-instigated: the player killed themselves by leaving, and attributing it to anyone else
	//! would put a false kill in whatever reads the instigator.
	protected void KillForViolation(notnull PlayerManager players, int playerId, notnull IEntity body, notnull TBD_Zone zone)
	{
		SCR_CharacterDamageManagerComponent damage = SCR_CharacterDamageManagerComponent.Cast(
			body.FindComponent(SCR_CharacterDamageManagerComponent));

		if (!damage)
		{
			// Refuse loudly rather than reach for another way to kill them. A body with no damage
			// manager is not something this slice should be improvising against.
			TBD_Log.Error(TBD_ZoneRegistry.CH, string.Format("player=%1 zone=%2 penalty=kill but the body has no SCR_CharacterDamageManagerComponent — NOT killed",
				playerId, zone.LogKey()));
			return;
		}

		TBD_Log.Banner(TBD_ZoneRegistry.CH, string.Format(
			"ONE LIFE SPENT: player=%1 killed for leaving %2 (grace %3s expired) — admin '#tbd respawn %1' is the only way back",
			playerId, zone.LogKey(), zone.m_fGraceSeconds), true);

		Tell(players, playerId, "TBD: you left the play area. Your one life is spent -- contact an admin.");

		damage.Kill(Instigator.CreateInstigator(body));
	}

	//------------------------------------------------------------------------------------------------
	//! Server -> one client, over the channel this codebase already uses for per-player replies
	//! (`TBD_AdminCommands.Reply`). Chat rather than a HUD because a new `.layout` is INVISIBLE to
	//! the engine until Workbench rewrites `resourceDatabase.rdb` (recorded landmine) — a widget
	//! written on this lane could not open. Every message is also logged server-side, so an
	//! operator can reconstruct what a player was told even if delivery failed.
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
	//! The faction whose zones apply to this player, or empty when they have no assigned slot.
	//! Empty is a legitimate answer, not an error: it means the player is subject only to
	//! everyone-applies boundary zones, which is the conservative reading.
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
	//! answer different questions — one is "has this identity spent its life", the other is "is
	//! the thing standing here alive right now" — and either being true means leave them alone.
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
