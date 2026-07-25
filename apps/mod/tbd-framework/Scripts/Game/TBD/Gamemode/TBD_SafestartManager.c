//! T-181.17 — SAFESTART. The warmup phase between BRIEFING and LIVE in which nobody can hurt
//! anybody, including themselves.
//!
//! ── Why this is not a nicety ────────────────────────────────────────────────────────────────
//! TBD events are ONE LIFE (TBD_MOD_DESIGN.md §2). Between deploy and "go" every player on the
//! server is standing in the open, shoulder to shoulder, checking kit. Without safestart a single
//! negligent discharge ends someone's entire event. There is no respawn to soften it — only the
//! admin glitch-death hatch, which is not meant to launder an ND.
//!
//! ── What it actually does ───────────────────────────────────────────────────────────────────
//! Three layers, strongest first:
//!
//!   1. DAMAGE OFF — `SCR_CharacterDamageManagerComponent.EnableDamageHandling(false)` on every
//!      protectable body. This is a BOOLEAN GATE, not a scale: damage is off, not reduced, and it
//!      is indifferent to who fired, so self-harm is covered by the same call. The API was proven
//!      by compile probe against a negative control that failed (slice report).
//!   2. PROJECTILES DELETED — `OnProjectileShot` / `OnGrenadeThrown` script handlers destroy the
//!      round the instant it exists. This is what makes damage-off cover bystanders whose damage
//!      manager we could not find, and what stops a grenade cooking through the lift.
//!   3. WEAPON SAFETY — `CharacterControllerComponent.SetSafety(true, true)`. A convenience, and
//!      honestly nothing more: the player can flick it back off. Layers 1 and 2 are the enforcement.
//!
//! Everything a safestart CANNOT stop is listed in `//! LIMITS` at the bottom of this header
//! rather than left for someone to discover during an event.
//!
//! ── The one thing that must never fail ──────────────────────────────────────────────────────
//! A safestart that fails to LIFT is worse than no safestart: a whole event of bullets that do
//! nothing, discovered at the first contact. So the lift is built to fail loudly and to keep
//! trying:
//!
//!   * `m_bArmed` is cleared FIRST, before any per-entity work. Every suppression path reads it,
//!     so a script handler we then fail to unregister is already inert. CRF's manager has no such
//!     guard and instead re-runs its removal pass at +1.5 s and +12.5 s hoping to catch it.
//!   * Restoration is VERIFIED, not assumed. `IsDamageHandlingEnabled()` is read back per entity
//!     after the setter — "we called the setter" is not evidence.
//!   * A watchdog keeps re-lifting every 5 s while any entity is unrestored, and keeps saying so
//!     at ERROR the whole time. It stops itself the moment the set is empty.
//!
//! ── Reference ───────────────────────────────────────────────────────────────────────────────
//! Shape learned from CRF's `CRF_SafestartManager` (Arma Public License — read, never copied):
//! the damage-off + safety + projectile-sink triple, and the periodic re-sweep for bodies that
//! appear mid-phase. TBD diverges on the parts that matter here: one life instead of waves, a
//! verified lift instead of a hopeful one, an armed-flag guard on the suppression handlers, and
//! unpossessed slot bodies included in the sweep.
//!
//! ── LIMITS (what this does NOT enforce) ─────────────────────────────────────────────────────
//!   * It cannot HOLSTER or lower a weapon. Nothing in the proven API surface forces a weapon
//!     away; `SetSafety` is the closest thing and the player owns that switch. Players will still
//!     see muzzles pointed at them — the round simply does not survive.
//!   * The shot still HAPPENS: report, muzzle flash, recoil, and a spent round. Only the
//!     projectile entity is destroyed, server-side, immediately after it spawns.
//!   * Melee, vehicle impacts, drowning and falls are not projectiles. They are covered only by
//!     layer 1, so they are covered exactly as well as the damage manager was found.
//!   * A body created between one sweep and the next is unprotected for up to SWEEP_MS.
//!   * Vehicles, static weapons and world objects keep their own damage managers; this manager
//!     touches characters only.
//!
//! @authority server — the phase, the countdown and every mutation run on the authority. The
//! only client-side code here is the countdown read-out.

[ComponentEditorProps(category: "TBD/Framework", description: "TBD safestart — damage-off warmup between BRIEFING and LIVE.")]
class TBD_SafestartManagerClass : SCR_BaseGameModeComponentClass {}

class TBD_SafestartManager : SCR_BaseGameModeComponent
{
	//! Countdown length when the mission document does not say otherwise. Five minutes is the
	//! usual TBD warmup: long enough to fix a wrong kit, short enough that nobody wanders off.
	static const int DEFAULT_COUNTDOWN_SECONDS = 300;
	//! Floor: below this the countdown is theatre — players cannot read it and react.
	static const int MIN_COUNTDOWN_SECONDS = 5;
	//! Ceiling: an hour of safestart is an admin typo, not an intention.
	static const int MAX_COUNTDOWN_SECONDS = 3600;
	//! Replicated sentinel for "safestart is not running". Deliberately negative so a client that
	//! has never received the property (default 0) cannot read as "0 seconds left, about to go".
	static const int NOT_RUNNING = -1;

	//! Re-sweep cadence. CRF re-sweeps every 10 s; SAFE_START is exactly when bodies are still
	//! landing, so this is faster. It is also the worst-case exposure window for a fresh body.
	protected static const int SWEEP_MS = 3000;
	//! Post-lift verification cadence. Runs only while something is still unrestored.
	protected static const int WATCHDOG_MS = 5000;
	//! Watchdog passes that get an ERROR line each; after this it drops to one line per this many
	//! passes. Repair attempts are NOT throttled — only the noise is.
	protected static const int LOUD_WATCHDOG_PASSES = 12;

	//! @replicated m_iSecondsRemaining — server-owned countdown, NOT_RUNNING when off.
	//! Clients react in OnCountdownReplicated (onRplName hook); the authority calls the same
	//! helper from its own setters because onRplName never fires on authority (design §5).
	[RplProp(onRplName: "OnCountdownReplicated")]
	protected int m_iSecondsRemaining = -1;

	// ── Server state ────────────────────────────────────────────────────────────────────────

	//! THE suppression flag. Read by every handler before it destroys anything, so clearing it
	//! disarms the whole system in one assignment even if per-entity teardown then fails.
	protected bool m_bArmed;
	protected int m_iConfiguredSeconds = 300;

	//! Every body we have touched -> the arm generation whose script handlers are live on it
	//! (0 = none). Membership alone means "we owe this body a restore", and a body is inserted
	//! BEFORE its first mutation, so there is no window in which we have altered an entity the
	//! lift would not visit.
	//!
	//! The value is a generation rather than a bool because a body whose restore could not be
	//! VERIFIED stays in this map with its handlers already removed. A bool would read as "still
	//! suppressed" on the next arm and quietly skip re-registering them.
	protected ref map<IEntity, int> m_mHeld = new map<IEntity, int>();

	//! Increments on every Arm(). Starts at 1 so it can never collide with the 0 that means
	//! "no handlers registered".
	protected int m_iArmGeneration;

	//! playerId -> "we have already named this player in the ND log". One line per player per
	//! safestart; the counters carry the volume (ENF-1: nothing here is per-frame).
	protected ref map<int, bool> m_mNegligentDischarge = new map<int, bool>();

	protected int m_iSuppressedShots;
	protected int m_iSuppressedThrows;
	//! Entities the last restore pass could not verify. Non-zero means players may be invulnerable.
	protected int m_iUnrestored;
	protected bool m_bWatchdogRunning;
	protected bool m_bLiftFailureAnnounced;
	protected int m_iWatchdogPasses;

	// ── Client state ────────────────────────────────────────────────────────────────────────

	//! Last countdown value this machine's UI acted on, so a redundant replication callback
	//! carrying an unchanged value cannot re-pop the banner.
	protected int m_iLocalLastSeen = -1;

	//------------------------------------------------------------------------------------------------
	//! Resolved off the LIVE game mode, not cached in a static like its sibling managers.
	//!
	//! That divergence is deliberate and load-bearing. `TBD_FrameworkManager.SetStage` asks this
	//! function exactly one question — "does this world have safestart enforcement on it?" — and
	//! refuses SAFE_START when the answer is no. A static set in a constructor OUTLIVES its world
	//! inside one Workbench process (the measured landmine `IsFrameworkWorld()` exists to dodge),
	//! so a stale instance from a previous world would answer "yes" for a world that has no
	//! safestart at all. That is the one wrong answer this function must never give.
	//!
	//! The cost is a FindComponent on a handful of admin/stage-transition calls. Nothing on a
	//! tick path calls this — the ticks all run on `this`.
	static TBD_SafestartManager GetInstance()
	{
		SCR_BaseGameMode gameMode = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
		if (!gameMode)
			return null;

		return TBD_SafestartManager.Cast(gameMode.FindComponent(TBD_SafestartManager));
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// Authority only — the countdown, the sweeps and every damage mutation are server-owned;
		// a client running them would desync and would still not protect anybody.
		if (RplSession.Mode() == RplMode.Client)
			return;

		m_iConfiguredSeconds = DEFAULT_COUNTDOWN_SECONDS;
	}

	// ── Public state ────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! True while damage is suppressed. The one question every other system should ask.
	bool IsArmed()
	{
		return m_bArmed;
	}

	//------------------------------------------------------------------------------------------------
	int GetSecondsRemaining()
	{
		return m_iSecondsRemaining;
	}

	//------------------------------------------------------------------------------------------------
	//! Entities still owed a restore. Non-zero after a lift is the catastrophic case.
	int GetUnrestoredCount()
	{
		return m_iUnrestored;
	}

	// ── Phase machine ───────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Driven by TBD_FrameworkManager.SetStage for EVERY transition, not just the two that look
	//! relevant. That is deliberate: SAFE_START arms, and literally anything else lifts. An admin
	//! who jumps SAFE_START -> END, or restarts the round back to LOBBY, must not leave a server
	//! full of invulnerable players behind.
	//! @authority server
	void OnStageChanged(TBD_EGameStage stage)
	{
		// Authority only — the phase machine is server-owned (TBD_FrameworkManager.SetStage).
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (stage == TBD_EGameStage.SAFE_START)
		{
			Arm();
			return;
		}

		Lift(typename.EnumToString(TBD_EGameStage, stage));
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected void Arm()
	{
		if (m_bArmed)
			return;

		m_bArmed = true;
		m_iArmGeneration++;
		m_iSuppressedShots = 0;
		m_iSuppressedThrows = 0;
		m_iUnrestored = 0;
		m_bLiftFailureAnnounced = false;
		m_mNegligentDischarge = new map<int, bool>();

		int covered = SweepApply();

		SetCountdown(m_iConfiguredSeconds);

		TBD_Log.Kv(TBD_Log.CH_SAFESTART, "armed",
			string.Format("seconds=%1 bodies=%2", m_iSecondsRemaining, covered));

		string msg = "[TBD] SAFESTART — damage OFF, weapons cold. Live in ";
		msg += FormatClock(m_iSecondsRemaining);
		msg += ".";
		Broadcast(msg);

		GetGame().GetCallqueue().Remove(TickCountdown);
		GetGame().GetCallqueue().Remove(TickSweep);
		GetGame().GetCallqueue().CallLater(TickCountdown, 1000, true);
		GetGame().GetCallqueue().CallLater(TickSweep, SWEEP_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	//! End safestart and give everybody their damage back.
	//!
	//! Ordering here is the whole safety argument, so do not reorder it:
	//!   1. `m_bArmed = false` — one assignment disarms every suppression path. From this line on,
	//!      a script handler we fail to unregister is a no-op instead of a silent bullet-eater.
	//!   2. Timers off, countdown cleared, clients told.
	//!   3. Per-entity restore, VERIFIED by read-back.
	//!   4. Watchdog armed if anything did not verify.
	//! @authority server
	void Lift(string reason)
	{
		// Authority only — restoring damage is a server-side mutation of server-owned bodies.
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!m_bArmed && m_mHeld.Count() == 0)
			return;

		m_bArmed = false;

		GetGame().GetCallqueue().Remove(TickCountdown);
		GetGame().GetCallqueue().Remove(TickSweep);

		SetCountdown(NOT_RUNNING);

		int owed = m_mHeld.Count();
		m_iUnrestored = Restore();

		string kv = string.Format("reason=%1 bodies=%2 unrestored=%3", reason, owed, m_iUnrestored);
		kv += string.Format(" suppressedShots=%1 suppressedThrows=%2", m_iSuppressedShots, m_iSuppressedThrows);
		TBD_Log.Kv(TBD_Log.CH_SAFESTART, "lift", kv);

		if (m_iUnrestored == 0)
		{
			Broadcast("[TBD] SAFESTART OVER — WEAPONS LIVE. Damage is ON, and you have ONE life.");
			return;
		}

		AnnounceLiftFailure();
		StartWatchdog();
	}

	//------------------------------------------------------------------------------------------------
	//! Countdown reached zero, or an admin said go. Routes through the stage machine so LIVE is
	//! entered exactly once, by the one component that owns it — Lift then arrives back here
	//! through OnStageChanged like any other transition.
	//! @authority server
	void GoLive(string reason)
	{
		// Authority only — advancing the round is server-owned.
		if (RplSession.Mode() == RplMode.Client)
			return;

		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (framework && framework.GetStage() == TBD_EGameStage.SAFE_START)
		{
			framework.SetStage(TBD_EGameStage.LIVE);
			return;
		}

		// The stage machine is gone or has already moved on. Damage still has to come back, so
		// lift on our own authority rather than waiting for a transition that will never arrive.
		TBD_Log.Warn(TBD_Log.CH_SAFESTART,
			"go-live could not drive the stage machine — lifting directly (reason=" + reason + ")");
		Lift(reason);
	}

	// ── Countdown ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected void TickCountdown()
	{
		if (!m_bArmed)
		{
			GetGame().GetCallqueue().Remove(TickCountdown);
			return;
		}

		// Defence in depth: if the stage moved off SAFE_START without OnStageChanged reaching us,
		// the countdown is the last thing still running that can notice. Lift immediately.
		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (framework && framework.GetStage() != TBD_EGameStage.SAFE_START)
		{
			TBD_Log.Warn(TBD_Log.CH_SAFESTART, "stage left SAFE_START without notifying safestart — lifting now");
			Lift("stage-drift");
			return;
		}

		int next = m_iSecondsRemaining - 1;
		if (next < 0)
			next = 0;

		SetCountdown(next);

		if (next <= 0)
		{
			GoLive("countdown expired");
			return;
		}

		if (IsChatMilestone(next))
		{
			string msg = "[TBD] SAFESTART — live in ";
			msg += FormatClock(next);
			msg += ". Weapons cold, damage off.";
			Broadcast(msg);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! The one place `m_iSecondsRemaining` is written. Replicates, then drives the local UI —
	//! both, every time, because either alone is wrong on one of the two topologies.
	//! @authority server
	protected void SetCountdown(int seconds)
	{
		m_iSecondsRemaining = seconds;
		Replication.BumpMe();
		NotifyLocalSafestartUI();
	}

	// ── Sweep / apply ───────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected void TickSweep()
	{
		if (!m_bArmed)
		{
			GetGame().GetCallqueue().Remove(TickSweep);
			return;
		}

		SweepApply();
	}

	//------------------------------------------------------------------------------------------------
	//! Apply protection to everything currently protectable. Idempotent, and re-applies
	//! damage-off even to bodies already held: a heal, a re-equip or a fresh materialisation can
	//! flip damage handling back on underneath us, and the sweep is what catches that.
	//! Returns the number of bodies covered for the FIRST time by this pass.
	//! @authority server
	protected int SweepApply()
	{
		array<IEntity> targets = {};
		CollectProtectables(targets);

		int fresh = 0;
		foreach (IEntity ent : targets)
		{
			if (ApplyTo(ent))
				fresh++;
		}

		return fresh;
	}

	//------------------------------------------------------------------------------------------------
	//! Returns true when this body's suppression handlers were attached by THIS pass.
	//! @authority server
	protected bool ApplyTo(IEntity ent)
	{
		if (!ent)
			return false;

		// Recorded BEFORE anything is mutated. If the mutation below only half-lands, the entity
		// is still on the list the lift walks — an unrecorded half-disabled body is precisely the
		// thing that would survive the lift and eat bullets all round.
		// Contains-then-Get rather than a bare Get: what an Enforce `map.Get()` does with an
		// absent key is not something the compile lane can prove, and this runs on every body on
		// every sweep — not a place to find out it logs (ENF-1).
		int suppressedGeneration = 0;
		if (m_mHeld.Contains(ent))
			suppressedGeneration = m_mHeld.Get(ent);

		m_mHeld.Set(ent, suppressedGeneration);

		// LAYER 1 — damage off. A boolean gate, re-asserted on every sweep because a heal, a
		// re-equip or a fresh materialisation can turn it back on underneath us.
		SCR_CharacterDamageManagerComponent damage = SCR_CharacterDamageManagerComponent.Cast(
			ent.FindComponent(SCR_CharacterDamageManagerComponent));
		if (damage)
			damage.EnableDamageHandling(false);

		if (suppressedGeneration == m_iArmGeneration)
			return false;

		// LAYER 3 — weapon safety. Cosmetic-grade: the player can switch it back.
		CharacterControllerComponent controller = CharacterControllerComponent.Cast(
			ent.FindComponent(CharacterControllerComponent));
		if (controller)
			controller.SetSafety(true, true);

		// LAYER 2 — the projectile sink. Registered once per body per arm; registering twice
		// would leave a second handler behind at lift time.
		EventHandlerManagerComponent events = EventHandlerManagerComponent.Cast(
			ent.FindComponent(EventHandlerManagerComponent));
		if (events)
		{
			events.RegisterScriptHandler("OnProjectileShot", this, OnSafestartProjectile);
			events.RegisterScriptHandler("OnGrenadeThrown", this, OnSafestartGrenade);
		}

		m_mHeld.Set(ent, m_iArmGeneration);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Everything a safestart is responsible for keeping alive.
	//!
	//! The third source is the one CRF does not have and TBD needs: slot bodies that
	//! `TBD_SpawnManager` has materialised but nobody has possessed yet. They belong to the
	//! player who is about to inherit them, so a body killed during safestart is a player killed
	//! during safestart — one stage late, and with a spent life. Neither the player sweep nor the
	//! AI sweep can see them, because they are neither.
	//! @authority server
	protected void CollectProtectables(notnull array<IEntity> outEntities)
	{
		map<IEntity, bool> seen = new map<IEntity, bool>();

		PlayerManager players = GetGame().GetPlayerManager();
		if (players)
		{
			array<int> ids = {};
			int count = players.GetPlayers(ids);
			for (int i = 0; i < count; i++)
			{
				IEntity ent = players.GetPlayerControlledEntity(ids[i]);
				if (!ent || seen.Contains(ent))
					continue;
				seen.Set(ent, true);
				outEntities.Insert(ent);
			}
		}

		SCR_AIWorld aiWorld = SCR_AIWorld.Cast(GetGame().GetAIWorld());
		if (aiWorld)
		{
			array<AIAgent> agents = {};
			aiWorld.GetAIAgents(agents);
			foreach (AIAgent agent : agents)
			{
				if (!agent)
					continue;

				IEntity ent = agent.GetControlledEntity();
				if (!ent || seen.Contains(ent))
					continue;
				seen.Set(ent, true);
				outEntities.Insert(ent);
			}
		}

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!spawn || !slots)
			return;

		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (!slot)
				continue;

			IEntity ent = spawn.GetSlotBody(slot.Key());
			if (!ent || seen.Contains(ent))
				continue;
			seen.Set(ent, true);
			outEntities.Insert(ent);
		}
	}

	// ── Restore ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Walk everything we owe a restore to and give it back. Returns how many bodies could NOT be
	//! verified as restored — the number that decides whether this was a lift or a disaster.
	//!
	//! Scope is exactly `m_mHeld`, never "every body in the world". Turning damage handling ON for
	//! an entity we never turned it off for would trample whatever else disabled it (a spectator
	//! dummy, a cutscene prop) — a safestart must not have that blast radius.
	//! @authority server
	protected int Restore()
	{
		array<IEntity> owed = {};
		foreach (IEntity held, int generation : m_mHeld)
			owed.Insert(held);

		// Rebuilt rather than mutated in place: an entity that has been deleted reads back as
		// null, and a null key cannot be removed from the map it is still sitting in. Rebuilding
		// is also what keeps a deleted body from counting as a lift failure forever.
		m_mHeld = new map<IEntity, int>();

		int failures = 0;
		int gone = 0;

		foreach (IEntity ent : owed)
		{
			// The body was deleted (disconnect, cleanup). It cannot hurt or be hurt, and it is
			// not a lift failure — dropping it is how the watchdog eventually goes quiet.
			if (!ent)
			{
				gone++;
				continue;
			}

			if (RestoreOne(ent))
				continue;

			// Still owed, and its handlers are gone (RestoreOne removes them unconditionally) —
			// so generation 0, not the arm generation.
			m_mHeld.Set(ent, 0);
			failures++;
		}

		if (gone > 0)
			TBD_Log.Kv(TBD_Log.CH_SAFESTART, "restore", string.Format("droppedDeletedBodies=%1", gone));

		return failures;
	}

	//------------------------------------------------------------------------------------------------
	//! Restore one body. True only when damage handling is READ BACK as enabled — calling the
	//! setter is not evidence that it took, and under ONE LIFE the difference is the whole event.
	//! @authority server
	protected bool RestoreOne(IEntity ent)
	{
		// Suppression teardown first, and unconditionally: these are best-effort by construction,
		// and `m_bArmed` has already made both inert, so a failure here is cosmetic.
		CharacterControllerComponent controller = CharacterControllerComponent.Cast(
			ent.FindComponent(CharacterControllerComponent));
		if (controller)
			controller.SetSafety(false, false);

		EventHandlerManagerComponent events = EventHandlerManagerComponent.Cast(
			ent.FindComponent(EventHandlerManagerComponent));
		if (events)
		{
			events.RemoveScriptHandler("OnProjectileShot", this, OnSafestartProjectile);
			events.RemoveScriptHandler("OnGrenadeThrown", this, OnSafestartGrenade);
		}

		SCR_CharacterDamageManagerComponent damage = SCR_CharacterDamageManagerComponent.Cast(
			ent.FindComponent(SCR_CharacterDamageManagerComponent));
		if (!damage)
		{
			// No damage manager now means none to re-enable. If one existed when we armed, this
			// body has been rebuilt underneath us and the new one was never disabled.
			return true;
		}

		damage.EnableDamageHandling(true);
		return damage.IsDamageHandlingEnabled();
	}

	// ── The loud failure ────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected void StartWatchdog()
	{
		if (m_bWatchdogRunning)
			return;

		m_bWatchdogRunning = true;
		m_iWatchdogPasses = 0;
		GetGame().GetCallqueue().CallLater(TickWatchdog, WATCHDOG_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Keeps trying to finish a lift that did not finish, and keeps saying so until it does.
	//!
	//! Self-terminating on purpose: it stops when nothing is owed, and it stands down entirely
	//! while safestart is legitimately armed again (an admin can re-enter SAFE_START).
	//! @authority server
	protected void TickWatchdog()
	{
		if (m_bArmed)
		{
			// Legitimately re-armed. Arm() owns the state again; this pass would fight it.
			StopWatchdog();
			return;
		}

		m_iWatchdogPasses++;
		m_iUnrestored = Restore();

		if (m_iUnrestored == 0)
		{
			StopWatchdog();
			TBD_Log.Banner(TBD_Log.CH_SAFESTART, "SAFESTART LIFT RECOVERED — every body verified damage-ON", false);
			Broadcast("[TBD] Safestart lift recovered — damage is ON for everyone.");
			return;
		}

		AnnounceLiftFailure();
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected void StopWatchdog()
	{
		m_bWatchdogRunning = false;
		GetGame().GetCallqueue().Remove(TickWatchdog);
	}

	//------------------------------------------------------------------------------------------------
	//! The thing an operator must not be able to scroll past, and the thing players must not be
	//! left to discover at first contact.
	//!
	//! Volume is bounded on purpose. `IsDamageHandlingEnabled()` is proven to EXIST (compile probe
	//! + failing negative control) but its exact semantics are a runtime property, so a read-back
	//! that never agrees is a possible failure mode of the CHECK, not only of the lift. An
	//! unbounded ERROR stream would then bury the rest of the event log for an hour. So: banner
	//! once, ERROR every pass for the first minute, then once a minute — while the repair itself
	//! keeps retrying every pass regardless.
	//! @authority server
	protected void AnnounceLiftFailure()
	{
		string detail = string.Format("bodies=%1 — THESE PLAYERS MAY BE INVULNERABLE", m_iUnrestored);

		if (!m_bLiftFailureAnnounced)
		{
			m_bLiftFailureAnnounced = true;
			TBD_Log.Banner(TBD_Log.CH_SAFESTART, "SAFESTART FAILED TO LIFT — " + detail, true);
			Broadcast("[TBD] !! SAFESTART FAILED TO LIFT for some players — damage may still be OFF. Tell an admin NOW.");
			return;
		}

		if (m_iWatchdogPasses <= LOUD_WATCHDOG_PASSES)
		{
			TBD_Log.Error(TBD_Log.CH_SAFESTART, "still not lifted — " + detail);
			return;
		}

		if (m_iWatchdogPasses % LOUD_WATCHDOG_PASSES != 0)
			return;

		string quieter = "still not lifted after ";
		quieter += m_iWatchdogPasses.ToString();
		quieter += " repair passes — " + detail;
		quieter += ". If this never clears, suspect the damage read-back itself and check a body by hand.";
		TBD_Log.Error(TBD_Log.CH_SAFESTART, quieter);
	}

	// ── Suppression handlers ────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Every round fired while safestart is armed dies here, the moment it exists.
	//!
	//! The `m_bArmed` guard is not belt-and-braces, it is the design: `RemoveScriptHandler` is the
	//! one teardown step whose failure we cannot detect, so this handler is written to be harmless
	//! if it leaks. CRF's equivalent deletes unconditionally and re-runs its removal pass three
	//! times hoping none leaked.
	//! @authority server
	protected void OnSafestartProjectile(int playerId, BaseWeaponComponent weapon, IEntity projectile)
	{
		if (!m_bArmed)
			return;

		if (!projectile)
			return;

		m_iSuppressedShots++;
		NoteNegligentDischarge(playerId, "fired a weapon");
		delete projectile;
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected void OnSafestartGrenade(int playerId, BaseWeaponComponent weapon, IEntity grenade)
	{
		if (!m_bArmed)
			return;

		if (!grenade)
			return;

		m_iSuppressedThrows++;
		NoteNegligentDischarge(playerId, "threw a grenade");
		delete grenade;
	}

	//------------------------------------------------------------------------------------------------
	//! Name the player once. Under ONE LIFE an ND during safestart is the incident that would have
	//! ended somebody's event, so it belongs in the log with a name on it — but once per player,
	//! not once per round of a burst (ENF-1).
	//! @authority server
	protected void NoteNegligentDischarge(int playerId, string what)
	{
		if (playerId <= 0 || m_mNegligentDischarge.Contains(playerId))
			return;

		m_mNegligentDischarge.Set(playerId, true);

		string who = "player";
		PlayerManager players = GetGame().GetPlayerManager();
		if (players)
			who = players.GetPlayerName(playerId);

		TBD_Log.Warn(TBD_Log.CH_SAFESTART,
			string.Format("ND during safestart: %1(%2) %3 — round suppressed", who, playerId, what));
	}

	// ── Admin surface ───────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! One line an admin can read at a glance. Safe to call in any stage.
	string StatusLine()
	{
		if (!m_bArmed)
		{
			string idle = string.Format("TBD safestart: OFF (next arm = %1). ", FormatClock(m_iConfiguredSeconds));
			if (m_iUnrestored > 0)
				return idle + string.Format("!! %1 body(s) NOT restored — damage may be off for them.", m_iUnrestored);
			return idle + "Damage is live.";
		}

		string armed = string.Format("TBD safestart: ON, live in %1. ", FormatClock(m_iSecondsRemaining));
		armed += string.Format("bodies=%1 suppressed shots=%2 grenades=%3",
			m_mHeld.Count(), m_iSuppressedShots, m_iSuppressedThrows);
		return armed;
	}

	//------------------------------------------------------------------------------------------------
	//! Set the countdown length. Applies immediately when armed, otherwise to the next arm.
	//! @authority server
	string AdminSetSeconds(int seconds, out bool ok)
	{
		ok = false;

		// Authority only — the countdown replicates outward from here.
		if (RplSession.Mode() == RplMode.Client)
			return "TBD: safestart is server-side only.";

		if (seconds < MIN_COUNTDOWN_SECONDS || seconds > MAX_COUNTDOWN_SECONDS)
		{
			return string.Format("TBD: safestart length must be %1-%2 seconds.",
				MIN_COUNTDOWN_SECONDS, MAX_COUNTDOWN_SECONDS);
		}

		m_iConfiguredSeconds = seconds;
		ok = true;

		if (!m_bArmed)
			return string.Format("TBD: safestart length set to %1 (applies when SAFE_START is entered).", FormatClock(seconds));

		SetCountdown(seconds);
		string msg = "[TBD] SAFESTART extended — live in ";
		msg += FormatClock(seconds);
		msg += ".";
		Broadcast(msg);
		return string.Format("TBD: safestart now ends in %1.", FormatClock(seconds));
	}

	// ── Client-side read-out ────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! @authority client — onRpl hook for m_iSecondsRemaining; runs on proxies on replication.
	void OnCountdownReplicated()
	{
		NotifyLocalSafestartUI();
	}

	//------------------------------------------------------------------------------------------------
	//! Show the countdown on THIS machine, if it has a screen.
	//!
	//! Called from BOTH the replication callback (proxy) and `SetCountdown` (authority), through
	//! one guarded helper — the listen-host landmine from TBD_MOD_DESIGN.md §5: on a listen host
	//! the authority IS the local player and never receives its own onRplName, so wiring only the
	//! callback would silently leave the host with no countdown at all.
	//!
	//! Deliberately `SCR_PopUpNotification` and NOT a TBD screen: modded menu presets do not
	//! resolve until Workbench regenerates `resourceDatabase.rdb`, and a countdown nobody can see
	//! is not a countdown. This is a vanilla HUD element and works today. The durable channel is
	//! the chat broadcast on the server side; this is the glanceable one.
	protected void NotifyLocalSafestartUI()
	{
		// No workspace = dedicated server. It has no screen and must never try to drive one.
		if (!GetGame().GetWorkspace())
			return;

		int now = m_iSecondsRemaining;
		int last = m_iLocalLastSeen;
		if (now == last)
			return;

		m_iLocalLastSeen = now;

		SCR_PopUpNotification popup = SCR_PopUpNotification.GetInstance();
		if (!popup)
			return;

		// Lifted: was running, now is not.
		if (now < 0)
		{
			if (last >= 0)
				popup.PopupMsg("SAFESTART OVER — WEAPONS LIVE", 8, "Damage is ON. You have ONE life.");
			return;
		}

		// First value this machine has seen — either safestart just armed, or we joined into it.
		if (last < 0)
		{
			popup.PopupMsg("SAFESTART — WEAPONS COLD", 8,
				"No damage, rounds are suppressed. Live in " + FormatClock(now) + ".");
			return;
		}

		if (IsPopupMilestone(now))
			popup.PopupMsg("SAFESTART — LIVE IN " + FormatClock(now), 3, "Weapons cold, damage off");
	}

	// ── Helpers ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Server -> every connected player, in their own chat feed.
	//!
	//! The durable half of the signal. A pop-up is missed by anyone alt-tabbed or looking at their
	//! map; a chat line is still there when they look back. It also needs no client-side script
	//! and no menu preset, so it works on a dedicated server today, rdb blocker or not.
	//! @authority server
	protected void Broadcast(string text)
	{
		// Authority only — the server is the only machine that should be telling everyone anything.
		if (RplSession.Mode() == RplMode.Client)
			return;

		Print("[TBD][Safestart] broadcast: " + text, LogLevel.NORMAL);

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> ids = {};
		int count = players.GetPlayers(ids);
		for (int i = 0; i < count; i++)
		{
			PlayerController controller = players.GetPlayerController(ids[i]);
			if (!controller)
				continue;

			SCR_ChatComponent chat = SCR_ChatComponent.Cast(controller.FindComponent(SCR_ChatComponent));
			if (!chat)
				continue;

			chat.SendPrivateMessage(text, ids[i]);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! `4:30`, `0:09`. Minutes are unpadded, seconds are always two digits.
	static string FormatClock(int seconds)
	{
		if (seconds <= 0)
			return "0:00";

		int minutes = seconds / 60;
		int rest = seconds % 60;
		return string.Format("%1:%2", minutes, rest.ToString(2));
	}

	//------------------------------------------------------------------------------------------------
	//! Sparse. Chat is durable, so it is also the channel that becomes noise fastest.
	static bool IsChatMilestone(int seconds)
	{
		if (seconds == 600)
			return true;
		if (seconds == 300)
			return true;
		if (seconds == 120)
			return true;
		if (seconds == 60)
			return true;
		if (seconds == 30)
			return true;
		if (seconds == 10)
			return true;
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Denser than chat: a pop-up is transient, and the last few seconds are the ones people
	//! actually need to feel.
	static bool IsPopupMilestone(int seconds)
	{
		if (IsChatMilestone(seconds))
			return true;
		if (seconds == 240)
			return true;
		if (seconds == 180)
			return true;
		if (seconds == 15)
			return true;
		if (seconds > 0 && seconds <= 5)
			return true;
		return false;
	}
}
