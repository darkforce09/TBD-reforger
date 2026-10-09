/**
 * @file TBD_SafestartProtection.c
 * @brief The safe start shield on each body: damage off, weapon safety, projectiles deleted, restore.
 *
 * Role: sweeps every protectable body (players, AI agents, unpossessed slot bodies), records each
 * body's damage-handling value before the first change, turns damage off, sets weapon safety,
 * deletes every shot and thrown grenade, and on lift hands each body back the value it had,
 * verified by read-back.  Position: owned by TBD_SafestartManager, which arms, sweeps and restores
 * through it; TBD_SafestartWatchdog repeats Restore after a failed lift.
 * State: the held bodies with one TBD_SafestartHold each, the arm generation, the suppression and
 * found-disabled counters, and the 3 s sweep poll; server only.
 * Invariants: a body is recorded before its first mutation and its hold is carried, never re-read;
 * turning damage off never consults the getter; a body found invulnerable is left so and counted;
 * the projectile handlers do nothing unless the manager is armed.
 */

//! What safe start owes one body; one object, so the handler generation and the recorded damage
//! value cannot drift apart across restore passes.
class TBD_SafestartHold
{
	int m_iGeneration; //!< arm generation whose handlers are registered on the body; 0 = none (a failed restore removed them)
	bool m_bDamageWasEnabled = true; //!< damage handling as first found, before any change; default true, the fail-safe direction (every construction site also assigns it)
}

//! Per-body safe start shield of one framework world.
class TBD_SafestartProtection : Managed
{
	protected static const int SWEEP_MS = 3000; //!< re-sweep period (ms); the worst-case exposure of a new body

	protected TBD_SafestartManager m_Manager; //!< owning manager; its armed flag gates the handlers
	protected ref map<IEntity, ref TBD_SafestartHold> m_mHeld = new map<IEntity, ref TBD_SafestartHold>(); //!< every body owed a restore -> what it is owed
	protected ref map<int, bool> m_mNegligentDischarge = new map<int, bool>(); //!< players already named in the discharge log this arm
	protected int m_iArmGeneration; //!< increments per arm; starts at 1, 0 means no handlers registered
	protected int m_iSuppressedShots; //!< shots deleted this arm
	protected int m_iSuppressedThrows; //!< grenades deleted this arm
	protected int m_iUnrestored; //!< bodies the last restore could not verify; non-zero means players may be invulnerable
	protected int m_iFoundDisabled; //!< bodies this arm first touched with damage handling already off
	protected int m_iLeftDisabled; //!< bodies the last restore left with damage handling off, as found

	//! Bind the shield to its manager.
	//! @param manager the owning safe start manager
	void TBD_SafestartProtection(TBD_SafestartManager manager)
	{
		m_Manager = manager;
	}

	//! Stop the sweep poll.
	void CancelCallbacks()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(TickSweep);
	}

	//! Start a new arm: bump the generation, reset the counters, sweep once and start the poll.
	//! Held bodies from a failed lift stay held with the value recorded for them.
	//! @return the number of bodies covered for the first time
	//! @authority server
	int BeginArm()
	{
		m_iArmGeneration++;
		m_iSuppressedShots = 0;
		m_iSuppressedThrows = 0;
		m_iUnrestored = 0;
		m_iFoundDisabled = 0;
		m_iLeftDisabled = 0;
		m_mNegligentDischarge = new map<int, bool>();

		int covered = SweepApply();

		GetGame().GetCallqueue().Remove(TickSweep);
		GetGame().GetCallqueue().CallLater(TickSweep, SWEEP_MS, true);
		return covered;
	}

	//! Bodies currently owed a restore.
	//! @return the held count
	int HeldCount()
	{
		return m_mHeld.Count();
	}

	//! Bodies the last restore could not verify.
	//! @return the unrestored count
	int UnrestoredCount()
	{
		return m_iUnrestored;
	}

	//! Bodies this arm found with damage handling already off.
	//! @return the count
	int FoundDisabledCount()
	{
		return m_iFoundDisabled;
	}

	//! Bodies the last restore left with damage handling off, as found.
	//! @return the count
	int LeftDisabledCount()
	{
		return m_iLeftDisabled;
	}

	//! Shots deleted this arm.
	//! @return the count
	int SuppressedShots()
	{
		return m_iSuppressedShots;
	}

	//! Grenades deleted this arm.
	//! @return the count
	int SuppressedThrows()
	{
		return m_iSuppressedThrows;
	}

	//! One sweep poll; stops itself once the manager is disarmed.
	//! @authority server
	protected void TickSweep()
	{
		if (!m_Manager.IsArmed())
		{
			GetGame().GetCallqueue().Remove(TickSweep);
			return;
		}

		SweepApply();
	}

	//! Apply the shield to every protectable body. Idempotent, and re-asserts damage off on held
	//! bodies, because a heal, re-equip or rematerialisation can turn it back on.
	//! @return the number of bodies whose handlers this pass attached
	//! @authority server
	int SweepApply()
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

	//! Record, then shield, one body.
	//! @param ent the body; null does nothing
	//! @return true when this pass attached the body's handlers
	//! @authority server
	protected bool ApplyTo(IEntity ent)
	{
		if (!ent)
			return false;

		// Resolved first: the hold records this component's value before safe start changes it.
		SCR_CharacterDamageManagerComponent damage = SCR_CharacterDamageManagerComponent.Cast(
			ent.FindComponent(SCR_CharacterDamageManagerComponent));

		// Recorded before any mutation, so a half-applied body is still on the list the lift walks.
		// Contains before Get: this runs on every body on every sweep.
		TBD_SafestartHold hold;
		if (m_mHeld.Contains(ent))
			hold = m_mHeld.Get(ent);

		if (!hold)
		{
			// The one read of the prior value, taken once per body and carried from then on; a body
			// without a damage manager keeps the fail-safe `true`.
			hold = new TBD_SafestartHold();
			hold.m_bDamageWasEnabled = true;
			if (damage)
				hold.m_bDamageWasEnabled = damage.IsDamageHandlingEnabled();

			m_mHeld.Set(ent, hold);

			if (!hold.m_bDamageWasEnabled)
				m_iFoundDisabled++;
		}

		// Layer 1, damage off: re-asserted every sweep and never gated on the recorded value, so a
		// wrong getter cannot stop arming.
		if (damage)
			damage.EnableDamageHandling(false);

		if (hold.m_iGeneration == m_iArmGeneration)
			return false;

		// Layer 3, weapon safety: a convenience, the player can switch it back.
		CharacterControllerComponent controller = CharacterControllerComponent.Cast(
			ent.FindComponent(CharacterControllerComponent));
		if (controller)
			controller.SetSafety(true, true);

		// Layer 2, the projectile sink, once per body per arm. The engine checks neither the event
		// name nor the callback signature, so only a live test shot (the suppressed-shot count in
		// StatusLine) proves the binding.
		EventHandlerManagerComponent events = EventHandlerManagerComponent.Cast(
			ent.FindComponent(EventHandlerManagerComponent));
		if (events)
		{
			events.RegisterScriptHandler("OnProjectileShot", this, OnSafestartProjectile);
			events.RegisterScriptHandler("OnGrenadeThrown", this, OnSafestartGrenade);
		}

		hold.m_iGeneration = m_iArmGeneration;
		return true;
	}

	//! Every body safe start keeps alive: player bodies, AI agents' bodies, and unpossessed slot
	//! bodies from TBD_SpawnManager, each once.
	//! @param outEntities receives the bodies
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


	//! Hand every held body back the damage value it had, verified by read-back; deleted bodies are
	//! dropped, unverified ones stay held with their original hold and generation 0. Logs dropped
	//! bodies and, at WARNING, bodies left invulnerable as found.
	//! @return the number of bodies not verified as restored (also kept for UnrestoredCount)
	//! @authority server
	int Restore()
	{
		// Rebuilt, not mutated: a deleted body reads back as a null key that cannot be removed.
		map<IEntity, ref TBD_SafestartHold> stillOwed = new map<IEntity, ref TBD_SafestartHold>();

		int failures = 0;
		int gone = 0;
		int leftDisabled = 0;

		foreach (IEntity ent, TBD_SafestartHold held : m_mHeld)
		{
			// A deleted body can neither hurt nor be hurt; dropping it lets the watchdog go quiet.
			if (!ent)
			{
				gone++;
				continue;
			}

			// A missing hold takes the fail-safe branch: assume damage was on and give it back.
			TBD_SafestartHold hold = held;
			if (!hold)
			{
				hold = new TBD_SafestartHold();
				hold.m_bDamageWasEnabled = true;
			}

			// Initialised here too: a garbage read would invent a false WARNING.
			bool wasLeftDisabled = false;
			bool settled = RestoreOne(ent, hold, wasLeftDisabled);
			if (wasLeftDisabled)
				leftDisabled++;

			if (settled)
				continue;

			// Still owed; RestoreOne removed its handlers, so generation 0.
			hold.m_iGeneration = 0;
			stillOwed.Set(ent, hold);
			failures++;
		}

		m_mHeld = stillOwed;
		m_iLeftDisabled = leftDisabled;
		m_iUnrestored = failures;

		if (gone > 0)
			TBD_Log.Kv(TBD_Log.CH_SAFESTART, "restore", string.Format("droppedDeletedBodies=%1", gone));

		// Loud although correct: 0 for a round of live players, near the body count means a wrong pre-read.
		if (leftDisabled > 0)
		{
			string kept = "left ";
			kept += leftDisabled.ToString();
			kept += " body(s) with damage handling OFF -- that is how safestart found them, so it never turned it off and has nothing to give back.";
			kept += " Expected 0 for a round of live players.";
			TBD_Log.Warn(TBD_Log.CH_SAFESTART, kept);
		}

		return failures;
	}

	//! Put one body back as safe start found it: safety off, handlers removed, damage handling on
	//! and read back, unless it was already off when found.
	//! @param ent the body
	//! @param hold what the body is owed
	//! @param leftDisabled set true when the body was found invulnerable and is left so
	//! @return true when settled (nothing more owed); false when the read-back disagrees
	//! @authority server
	protected bool RestoreOne(IEntity ent, notnull TBD_SafestartHold hold, out bool leftDisabled)
	{
		leftDisabled = false;

		// Suppression teardown first, unconditionally; the cleared armed flag already made it inert.
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
			// No damage manager: nothing to re-enable (a rebuilt body's new one was never disabled).
			return true;
		}

		if (!hold.m_bDamageWasEnabled)
		{
			// Something else disabled it before safe start; forcing it on would overrule that owner.
			leftDisabled = true;
			return true;
		}

		damage.EnableDamageHandling(true);
		return damage.IsDamageHandlingEnabled();
	}

	//! Shot handler: while armed, delete the projectile and note the shooter. Harmless if it leaks
	//! past a failed RemoveScriptHandler, because it checks the armed flag first.
	//! @param playerId the shooter
	//! @param weapon the firing weapon
	//! @param projectile the projectile, deleted
	//! @authority server
	protected void OnSafestartProjectile(int playerId, BaseWeaponComponent weapon, IEntity projectile)
	{
		if (!m_Manager.IsArmed())
			return;

		if (!projectile)
			return;

		m_iSuppressedShots++;
		NoteNegligentDischarge(playerId, "fired a weapon");
		delete projectile;
	}

	//! Grenade handler: while armed, delete the grenade and note the thrower.
	//! @param playerId the thrower
	//! @param weapon the throwing weapon
	//! @param grenade the grenade, deleted
	//! @authority server
	protected void OnSafestartGrenade(int playerId, BaseWeaponComponent weapon, IEntity grenade)
	{
		if (!m_Manager.IsArmed())
			return;

		if (!grenade)
			return;

		m_iSuppressedThrows++;
		NoteNegligentDischarge(playerId, "threw a grenade");
		delete grenade;
	}

	//! Log a negligent discharge once per player per arm, with the player's name.
	//! @param playerId the player; 0 or less is not logged
	//! @param what the action, for the log line
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
			string.Format("ND during safestart: %1(%2) %3 -- round suppressed", who, playerId, what));
	}
}
