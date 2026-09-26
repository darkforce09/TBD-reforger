/**
 * @file TBD_SpectatorHost.c
 * @brief Server registry of spectator streaming hosts: arming, the reconcile tick, camera moves and release.
 *
 * Role: keeps one streaming host record per dead player, arms and stops the one-second reconcile
 * tick, moves a host to the reported camera position inside the world box and the range leash,
 * and hands control back and deletes the host on release.
 * Position: TBD_SpectatorComponent calls Start and Shutdown; the modded SCR_PlayerController
 * transport calls MoveTo; TBD_DeployExecutor and TBD_SpawnDeparture call ReleaseFor;
 * TBD_SpectatorHostLifecycle runs each reconcile pass and TBD_SpectatorHostFactory builds hosts.
 * State: the static host map keyed by player id, the running flag, the range leash and the
 * missing-manager latch, all on the server; Shutdown clears them because statics outlive a world.
 * Invariants: a host belongs to a player whose life is spent, on the connection epoch it was opened
 * under; MoveTo applies nothing without the epoch and deadness checks; release un-possesses only
 * when the controlled entity is this host and deletes the host on the next frame; a framework world
 * without TBD_SpawnManager releases every host (fail closed); a plain vanilla world holds none.
 */

//! Server streaming host registry. Static because its owner, TBD_SpectatorComponent, is created and
//! destroyed with the world; Start and Shutdown tie it to that lifetime, and Shutdown releases every
//! host because statics outlive a world inside one process.
class TBD_SpectatorHost
{
	static const int RECONCILE_MS = 1000; //!< reconcile period in ms; one tick for the whole server, never one per player
	static const float MIN_MOVE_M = 1.5; //!< metres; a camera report closer than this to the applied position is not applied

	protected static ref map<int, ref TBD_SpectatorHostRecord> s_mHosts; //!< player id to host record; null while stopped
	protected static bool s_bRunning; //!< the reconcile tick is armed; default false
	protected static float s_fMaxRangeM; //!< metres a host may move from the death position; 0 or less = unlimited
	protected static bool s_bNoManagerLogged; //!< latch for the missing TBD_SpawnManager error; one line until a manager is seen

	//! Arm the registry on the authority: store the range leash, configure the factory, reset the
	//! latches and start the repeating reconcile tick. A client, or a second call while running,
	//! returns without effect.
	//! @param hostPrefab optional host prefab; empty selects the prefab-free host
	//! @param maxRangeM metres a host may move from the death position; 0 or less = unlimited
	//! @authority server
	static void Start(ResourceName hostPrefab, float maxRangeM)
	{
		if (TBD_Authority.IsClient())
			return;

		if (s_bRunning)
			return;

		s_mHosts = new map<int, ref TBD_SpectatorHostRecord>();
		TBD_SpectatorHostFactory.Configure(hostPrefab);
		s_fMaxRangeM = maxRangeM;
		s_bNoManagerLogged = false;
		TBD_SpectatorHostLifecycle.ResetLatches();
		s_bRunning = true;

		string mode = "prefab-free scripted host (no resourceDatabase.rdb dependency)";
		if (!hostPrefab.IsEmpty())
			mode = string.Format("prefab %1", hostPrefab);

		string line = string.Format("[TBD][spectator] streaming host ARMED -- %1", mode);
		line = line + string.Format(", range=%1 m (0 = unlimited)", s_fMaxRangeM);
		PrintFormat("%1", line);

		GetGame().GetCallqueue().CallLater(Tick, RECONCILE_MS, true);
	}

	//! Mission teardown: cancel the tick, release and delete every host, cancel the pending deferred
	//! deletes and clear every static. A host that survived into the next world would be an orphan
	//! still listed as somebody's controlled entity. ScriptCallQueue.Remove cancels by function, and
	//! there is exactly one Tick for the whole server. A no-op when nothing was started.
	//! @authority server
	static void Shutdown()
	{
		GetGame().GetCallqueue().Remove(Tick);

		ReleaseAll("mission teardown");

		// After ReleaseAll, which is what queues them: every pending delete belongs to a world that
		// is going away, so none may fire into the next one holding stale handles.
		GetGame().GetCallqueue().Remove(DeleteHostNextFrame);

		s_mHosts = null;
		TBD_SpectatorHostFactory.Reset();
		s_fMaxRangeM = 0;
		s_bNoManagerLogged = false;
		TBD_SpectatorHostLifecycle.ResetLatches();
		s_bRunning = false;
	}

	//! Stop the reconcile tick and release every host for the rest of the round, leaving the host
	//! map in place. Called from inside a reconcile pass when the built-in host is refused.
	//! @param reason the release reason written on each host's RELEASED line
	//! @authority server
	static void StandDown(string reason)
	{
		// Removing a repeating call from inside that call is safe; the queue drops it after this run.
		GetGame().GetCallqueue().Remove(Tick);
		ReleaseAll(reason);
		s_bRunning = false;
	}

	//! @return true while the reconcile tick is armed
	static bool IsRunning()
	{
		return s_bRunning;
	}

	//! Does this player hold a streaming host right now? TBD_SpectatorHostLifecycle.Reconcile asks
	//! before issuing one.
	//! @param playerId the player to test
	//! @return true when a record exists and its host entity is alive
	static bool HasHost(int playerId)
	{
		if (!s_mHosts)
			return false;

		TBD_SpectatorHostRecord record;
		if (!s_mHosts.Find(playerId, record))
			return false;

		if (!record || !record.host)
			return false;

		return true;
	}


	//! The reconcile tick. Releases every host on a world that is not a framework world, fails
	//! closed (releases every host, logs once) when TBD_SpawnManager is missing, and otherwise runs
	//! one TBD_SpectatorHostLifecycle.Reconcile pass. A reconcile rather than a death hook: "has a
	//! spent life and a live connection" is derived afresh each second, so it cannot drift, miss an
	//! edge or race ordering, and it covers a player who reconnects onto a spent life.
	//! @authority server
	protected static void Tick()
	{
		if (TBD_Authority.IsClient())
			return;

		if (!s_mHosts)
			return;

		// A plain vanilla world holds no hosts; the same guard the rest of the mod uses.
		if (!TBD_FrameworkManager.IsFrameworkWorld())
		{
			ReleaseAll("not a framework world");
			return;
		}

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
		{
			// FAIL CLOSED. Without the spawn manager we cannot tell a spent life from a living
			// player, and "cannot tell" must never resolve to "hand them a possessed entity".
			if (!s_bNoManagerLogged)
			{
				s_bNoManagerLogged = true;
				Print("[TBD][spectator] streaming host STOOD DOWN -- framework world with no TBD_SpawnManager (cannot tell a dead player from a live one)", LogLevel.ERROR);
			}

			ReleaseAll("no TBD_SpawnManager");
			return;
		}

		s_bNoManagerLogged = false;

		TBD_SpectatorHostLifecycle.Reconcile(s_mHosts, spawn);
	}


	//! Move a player's host to their reported camera position. Refuses a player with no host, a
	//! record whose connection epoch has moved on, and a player who is not dead; clamps the rest to
	//! the world box and the range leash, and skips moves under MIN_MOVE_M. Never trusts its input.
	//! @param playerId the player the authority resolved from the calling controller
	//! @param position the client-supplied camera position, world metres
	//! @authority server
	static void MoveTo(int playerId, vector position)
	{
		if (TBD_Authority.IsClient())
			return;

		if (!s_mHosts)
			return;

		TBD_SpectatorHostRecord record;
		if (!s_mHosts.Find(playerId, record) || !record || !record.host)
			return;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return;

		// Fail closed on both halves of "is this still the same dead person": a recycled id can
		// belong to a different player who is also dead, so the epoch check is not redundant.
		if (!spawn.IsConnectionCurrent(playerId, record.epoch))
			return;

		if (!spawn.IsPlayerDead(playerId))
			return;

		vector wanted = ClampToWorld(position);
		wanted = ClampToRange(record.anchor, wanted);

		if (vector.Distance(record.applied, wanted) < MIN_MOVE_M)
			return;

		record.host.SetOrigin(wanted);
		record.applied = wanted;
	}

	//! Clamp a position into the world bounding box, the one bound that is always meaningful for a
	//! client-supplied position. A missing world or a degenerate box returns the input unchanged.
	//! @param wanted the requested position, world metres
	//! @return the clamped position
	protected static vector ClampToWorld(vector wanted)
	{
		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return wanted;

		vector mins;
		vector maxs;
		world.GetBoundBox(mins, maxs);

		// A degenerate box (an engine that answered nothing) must not collapse every host onto one
		// point, so it is left alone instead.
		if (mins[0] >= maxs[0] || mins[2] >= maxs[2])
			return wanted;

		vector clamped;
		clamped[0] = Math.Clamp(wanted[0], mins[0], maxs[0]);
		clamped[1] = Math.Clamp(wanted[1], mins[1], maxs[1]);
		clamped[2] = Math.Clamp(wanted[2], mins[2], maxs[2]);
		return clamped;
	}

	//! Hold a position within s_fMaxRangeM of the anchor, the lever that trades spectator reach
	//! against how much of the map a modified client can have streamed to it. A leash of 0 or less
	//! returns the input unchanged.
	//! @param anchor the death position the leash is measured from
	//! @param wanted the requested position, world metres
	//! @return the position pulled back onto the leash sphere when it lies beyond it
	protected static vector ClampToRange(vector anchor, vector wanted)
	{
		if (s_fMaxRangeM <= 0)
			return wanted;

		vector offset = wanted - anchor;
		float distance = offset.Length();
		if (distance <= s_fMaxRangeM)
			return wanted;

		if (distance <= 0)
			return anchor;

		return anchor + offset * (s_fMaxRangeM / distance);
	}


	//! Hand control back and delete the host. Idempotent, safe for a player who has none, and safe
	//! while the engine is tearing that player down. Control is switched away before the delete,
	//! the rule TBD_SpectatorController.Leave follows for the camera: SetPossessedEntity(null)
	//! restores the entity the player controlled when the host was issued (vanilla's m_MainEntity),
	//! so the player is back on their corpse, the state TBD_SpawnManager is written against.
	//! @param playerId the player whose host to release
	//! @param reason written on the RELEASED log line
	//! @return true when a host record was released
	//! @authority server
	static bool ReleaseFor(int playerId, string reason)
	{
		if (!s_mHosts)
			return false;

		TBD_SpectatorHostRecord record;
		if (!s_mHosts.Find(playerId, record))
			return false;

		s_mHosts.Remove(playerId);

		if (!record)
			return false;

		// The controller can already be gone (disconnect teardown); deleting the entity is still
		// necessary. The view is given back only when the player holds this host:
		// SetPossessedEntity(null) ends whatever possession is in progress, so calling it blind would
		// cancel a Game Master that took the controller over. A record whose host is already
		// destroyed has nothing to compare against, and un-possessing is then the only way not to
		// strand the player with IsPossessing() true against a dead entity.
		SCR_PlayerController pc = SCR_PlayerController.Cast(
			GetGame().GetPlayerManager().GetPlayerController(playerId));
		if (pc && pc.IsPossessing())
		{
			IEntity controlled = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
			if (!record.host || controlled == record.host)
				pc.SetPossessedEntity(null);
			else
				Print(string.Format("[TBD][spectator] player=%1 is possessing something that is not their streaming host -- leaving that alone and only deleting the host", playerId), LogLevel.WARNING);
		}

		if (record.host)
		{
			IEntity host = record.host;
			record.host = null;
			GetGame().GetCallqueue().Call(DeleteHostNextFrame, host);
		}

		Print(string.Format("[TBD][spectator] player=%1 streaming host RELEASED -- %2", playerId, reason));
		return true;
	}

	//! Delete a released host on the next frame, not now. A player who reconnected onto a spent life
	//! had no body when the host was issued, so SetPossessedEntity(null) has nothing to give back
	//! and the host is still their controlled entity when it is let go; deleting it in the same
	//! frame breaks the player controller (CRF_SpectatorCharacter.OnControlledByPlayer defers for
	//! the same reason). The argument is an entity, not a player id, so a recycled id cannot be
	//! mistaken. A null or already deleted host is ignored.
	//! @param host the released host entity
	//! @authority server
	protected static void DeleteHostNextFrame(IEntity host)
	{
		if (!host)
			return;

		SCR_EntityHelper.DeleteEntityAndChildren(host);
	}

	//! Release every host in the map. The holders are collected first, so the map is not mutated
	//! while it is walked.
	//! @param reason written on each RELEASED log line
	//! @authority server
	protected static void ReleaseAll(string reason)
	{
		if (!s_mHosts)
			return;

		array<int> holders = {};
		foreach (int playerId, TBD_SpectatorHostRecord record : s_mHosts)
		{
			holders.Insert(playerId);
		}

		for (int i = 0; i < holders.Count(); i++)
		{
			ReleaseFor(holders[i], reason);
		}
	}
}
