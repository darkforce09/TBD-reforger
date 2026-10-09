/**
 * @file TBD_SpectatorHostLifecycle.c
 * @brief One reconcile pass over the spectator streaming hosts: retire the stale, issue the missing.
 *
 * Role: retires every host that has stopped being legitimate and gives each dead, connected player
 * without one a host: resolve the anchor, spawn and vet the entity, possess it, read the
 * possession back and record it.
 * Position: TBD_SpectatorHost.Tick calls Reconcile once a second with its host map;
 * TBD_SpectatorHostFactory spawns and vets the entity; TBD_SpawnManager answers deadness, the
 * connection epoch and the assigned slot; releases go through TBD_SpectatorHost.ReleaseFor.
 * State: the issue-refused log latch (static, server); the host map belongs to TBD_SpectatorHost.
 * Invariants: only a player whose life is spent gets a host, and holding one never clears that
 * life, claims a slot or touches a body; a record is retired when its epoch moves on (checked
 * before the controller, because a recycled id has a controller), its controller is gone, the stage
 * drops below SAFE_START or the player is not dead; every failed issue leaves the world as it
 * found it; the map is never mutated while it is walked.
 */

//! Server reconcile pass for spectator streaming hosts. Static; no per-player deferred callback.
class TBD_SpectatorHostLifecycle
{
	protected static bool s_bIssueRefusedLogged; //!< latch: one "has NO streaming host" line until a host is issued; default false

	//! Clear the issue-refused latch. TBD_SpectatorHost.Start and Shutdown call it.
	static void ResetLatches()
	{
		s_bIssueRefusedLogged = false;
	}

	//! One pass: retire what should not exist, then issue a host to every connected player whose
	//! life is spent and who has none. Issuing waits for a stage at or past SAFE_START.
	//! @param hosts TBD_SpectatorHost's host map, mutated in place
	//! @param spawn the spawn manager that answers deadness and connection epochs
	//! @authority server
	static void Reconcile(notnull map<int, ref TBD_SpectatorHostRecord> hosts, notnull TBD_SpawnManager spawn)
	{
		bool stageOk = IsStageHostable();

		RetireStaleHosts(hosts, spawn, stageOk);

		if (!stageOk)
			return;

		array<int> players = {};
		int count = GetGame().GetPlayerManager().GetPlayers(players);
		for (int i = 0; i < count; i++)
		{
			int playerId = players[i];

			if (TBD_SpectatorHost.HasHost(playerId))
				continue;

			// The precondition: only a spent life gets a host. A living player's body already
			// anchors their streaming, and a host for a living player would be a route into the world.
			if (!spawn.IsPlayerDead(playerId))
				continue;

			EnsureHost(hosts, spawn, playerId);
		}
	}

	//! Retire every host that has stopped being legitimate: entity gone, connection epoch moved on,
	//! no controller, stage not spectatable, or the player not dead. Victims are collected
	//! first and released after, so the map is not mutated while it is walked.
	//! @param hosts the host map to walk
	//! @param spawn the spawn manager that answers deadness and connection epochs
	//! @param stageOk whether the round is at or past SAFE_START
	//! @authority server
	protected static void RetireStaleHosts(notnull map<int, ref TBD_SpectatorHostRecord> hosts, notnull TBD_SpawnManager spawn, bool stageOk)
	{
		array<int> victims = {};
		array<string> reasons = {};

		foreach (int playerId, TBD_SpectatorHostRecord record : hosts)
		{
			if (!record || !record.host)
			{
				victims.Insert(playerId);
				reasons.Insert("host entity is gone");
				continue;
			}

			// The connection-epoch test on stored state. It precedes the controller test: a recycled
			// id has a controller, which is how the wrong player would inherit this host.
			if (!spawn.IsConnectionCurrent(playerId, record.epoch))
			{
				victims.Insert(playerId);
				reasons.Insert("connection ended (epoch moved on)");
				continue;
			}

			if (!GetGame().GetPlayerManager().GetPlayerController(playerId))
			{
				victims.Insert(playerId);
				reasons.Insert("no player controller");
				continue;
			}

			if (!stageOk)
			{
				victims.Insert(playerId);
				reasons.Insert("round is no longer in a spectatable stage");
				continue;
			}

			// A player who is not dead must not be left possessing a host instead of their own
			// body. The deploy path releases the host itself before it deploys, so this fires only
			// if that is bypassed, and then in the safe direction.
			if (!spawn.IsPlayerDead(playerId))
			{
				victims.Insert(playerId);
				reasons.Insert("player is no longer dead (admin respawn?) -- handing control back");
				continue;
			}
		}

		for (int i = 0; i < victims.Count(); i++)
		{
			TBD_SpectatorHost.ReleaseFor(victims[i], reasons[i]);
		}
	}

	//! Hosting engages from SAFE_START onward, matching TBD_SpectatorController.IsStageSpectatable:
	//! a friendly-fire death during safe start spends a life like any other, and the client and
	//! server halves must agree about when they are live.
	//! @return true when TBD_FrameworkManager exists and its stage is SAFE_START or later
	protected static bool IsStageHostable()
	{
		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (!framework)
			return false;

		return framework.GetStage() >= TBD_EGameStage.SAFE_START;
	}


	//! Give one dead player a host: refuse when the controller is missing or already possessing,
	//! resolve the anchor, spawn and vet the entity, possess it, read the possession back and record
	//! it under the current connection epoch. Every failure path leaves the world as it found it.
	//! @param hosts the host map the new record is stored in
	//! @param spawn the spawn manager that answers the epoch and the assigned slot
	//! @param playerId a connected player whose life is spent
	//! @authority server
	protected static void EnsureHost(notnull map<int, ref TBD_SpectatorHostRecord> hosts, notnull TBD_SpawnManager spawn, int playerId)
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(
			GetGame().GetPlayerManager().GetPlayerController(playerId));
		if (!pc)
			return;

		// Somebody else (a Game Master) already possesses on this player's behalf. Corpse-anchored
		// streaming is worse but not broken; stealing possession would be.
		if (pc.IsPossessing())
		{
			NoteIssueRefused(playerId, "the player controller is already possessing something else -- not fighting over it");
			return;
		}

		vector anchor;
		if (!ResolveAnchor(spawn, playerId, anchor))
		{
			NoteIssueRefused(playerId, "no corpse and no assigned slot, so there is nowhere honest to put an anchor");
			return;
		}

		bool fromPrefab;
		IEntity host = TBD_SpectatorHostFactory.SpawnHostEntity(anchor, fromPrefab);
		if (!host)
		{
			NoteIssueRefused(playerId, "the host entity would not spawn");
			return;
		}

		// Checked before possession; a candidate that fails is destroyed rather than used.
		if (!TBD_SpectatorHostFactory.IsAcceptableHost(host, playerId))
		{
			SCR_EntityHelper.DeleteEntityAndChildren(host);
			RecoverFromUnacceptableHost(fromPrefab);
			return;
		}

		pc.SetPossessedEntity(host);

		// Calling the setter is not evidence that it took; read the controlled entity back.
		IEntity controlled = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
		if (controlled != host)
		{
			NoteIssueRefused(playerId, "the engine did not transfer control to the host -- rolled back, the spectator keeps corpse-anchored streaming");
			pc.SetPossessedEntity(null);
			SCR_EntityHelper.DeleteEntityAndChildren(host);
			return;
		}

		// A host landed, so the next refusal gets its own line.
		s_bIssueRefusedLogged = false;

		TBD_SpectatorHostRecord record = new TBD_SpectatorHostRecord();
		record.epoch = spawn.ConnectionEpochFor(playerId);
		record.host = host;
		record.anchor = anchor;
		record.applied = anchor;
		hosts.Set(playerId, record);

		string line = string.Format("[TBD][spectator] player=%1 streaming host ISSUED at %2", playerId, anchor.ToString());
		line = line + string.Format(" epoch=%1 replicated=%2 (life still spent, no slot, no body)",
			record.epoch, TBD_SpectatorHostFactory.IsReplicated(host));
		PrintFormat("%1", line);
	}

	//! Say once why a dead player is not getting a host, then stay quiet until one is issued: the
	//! reconcile retries every second, and a persistent cause would otherwise log every second.
	//! @param playerId the player refused
	//! @param reason why, written into the WARNING line
	//! @authority server
	protected static void NoteIssueRefused(int playerId, string reason)
	{
		if (s_bIssueRefusedLogged)
			return;

		s_bIssueRefusedLogged = true;

		string line = string.Format("[TBD][spectator] player=%1 has NO streaming host -- %2.", playerId, reason);
		line = line + " Their camera still works; it just stays anchored to their corpse. (Latched: one line per round until a host is issued.)";
		PrintFormat("%1", line, level: LogLevel.WARNING);
	}

	//! Recover from a candidate that failed the acceptance check without retrying every second,
	//! since a misconfiguration does not heal on its own. A refused prefab is dropped and the
	//! built-in host is used from then on; a refused built-in host (a bare GenericEntity, so this
	//! should be impossible) stands the whole feature down for the round.
	//! @param fromPrefab whether the refused candidate came from the configured prefab
	//! @authority server
	protected static void RecoverFromUnacceptableHost(bool fromPrefab)
	{
		if (fromPrefab)
		{
			TBD_SpectatorHostFactory.DropConfiguredPrefab();
			return;
		}

		Print("[TBD][spectator] the BUILT-IN streaming host was refused, which should be impossible -- standing the streaming host down for this round rather than churning entities", LogLevel.ERROR);
		TBD_SpectatorHost.StandDown("streaming host stood down after an impossible refusal");
	}

	//! Where the host starts: the controlled entity (the corpse) if there is one, else the ground
	//! under the player's assigned slot. Never the world origin, which would anchor streaming to
	//! the corner of the map.
	//! @param spawn the spawn manager that answers the assigned slot
	//! @param playerId the player to anchor
	//! @param anchor receives the anchor position, world metres
	//! @return false when there is neither a controlled entity nor an assigned slot
	//! @authority server
	protected static bool ResolveAnchor(notnull TBD_SpawnManager spawn, int playerId, out vector anchor)
	{
		IEntity corpse = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
		if (corpse)
		{
			anchor = corpse.GetOrigin();
			return true;
		}

		TBD_MissionSlotStruct slot = spawn.GetAssignedSlot(playerId);
		if (!slot)
			return false;

		float surfaceY = GetGame().GetWorld().GetSurfaceY(slot.x, slot.z);
		anchor = Vector(slot.x, surfaceY, slot.z);
		return true;
	}
}
