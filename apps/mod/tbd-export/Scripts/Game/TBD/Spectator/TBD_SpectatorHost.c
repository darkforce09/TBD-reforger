//! T-181.24 - SPECTATOR STREAMING HOST. Server authority for the dummy a dead player possesses
//! so the engine keeps streaming the world around their free camera.
//!
//! Three things live here, in the order they matter:
//!   1. `SCR_PlayerController` (modded) - ONE ask: "my camera is here". The transport.
//!   2. `TBD_SpectatorHostRecord` - what the authority remembers per spectator.
//!   3. `TBD_SpectatorHost` - SERVER: creates, moves, validates and retires the hosts.
//!
//! == THE PROBLEM =============================================================================
//! Streaming follows the CONTROLLED ENTITY. A dead player under ONE LIFE still controls their
//! corpse, so their replication origin is nailed to the spot where they fell; the free camera can
//! fly anywhere, but the machine was never sent the terrain, buildings or people it is pointed at.
//! `TBD_SpectatorTargets` already has to report a "not in view" count because of it.
//!
//! == THE FIX, AND ITS ONE MOVING PART ========================================================
//! The server spawns an inert `TBD_SpectatorHostEntity` and hands it to the dead player with
//! `SCR_PlayerController.SetPossessedEntity()`. The client tells the server where its camera is
//! (four times a second, unreliable, 12 bytes), the server teleports the host there, and the
//! engine's scoping follows the host.
//!
//! `SetPossessedEntity` is the primitive, read from real vanilla source
//! (`apps/mod/vanilla_reference/Source/SCR_PlayerController.c:301-390`), and it was chosen over
//! every alternative for one reason: **it is possession, not a spawn.** It remembers the previous
//! controlled entity in `m_MainEntity` and `SetPossessedEntity(null)` puts it back. It never
//! enters the vanilla spawn pipeline, never produces an `SCR_ESpawnResult`, never runs a finalize.
//! So it cannot be confused with - or mistaken for - a deploy.
//!
//! == ONE LIFE - WHY THIS IS NOT A SECOND DOOR ================================================
//! `TBD_SpawnManager.DeployPlayerEx` is the only door into the world and `AdminRespawn` is its
//! only sanctioned bypass. Nothing in this file goes near either. Point by point:
//!
//!   * **The host is not a life and does not restore one.** ONE LIFE is `m_mDeadPlayers`, keyed on
//!     the durable bind key. Nothing here reads it except to REQUIRE it (`IsPlayerDead` is a
//!     precondition for getting a host at all), and nothing here can clear it - `ClearLifeSpent`
//!     is `protected` inside `TBD_SpawnManager` and its only caller is `FinishAdminRespawn`.
//!     `IsPlayerDead` therefore keeps answering true for the whole time a host exists.
//!   * **It cannot be killed, damaged or revived into a playable body.** `IsAcceptableHost` refuses
//!     any candidate that is a `ChimeraCharacter`, or carries a `DamageManagerComponent`, or
//!     carries a `CharacterControllerComponent` - checked BEFORE possession, and the entity is
//!     deleted rather than used if it fails. A thing with no damage manager has no damage state to
//!     destroy, is not what `SCR_BaseGameMode.OnPlayerKilled` fires from, and has no controller to
//!     stand up, holster, shoot or be healed.
//!   * **It does not re-claim, hold or resurrect a slot.** No path here touches `m_mPlayerSlot`,
//!     `m_mSlotBodies`, `m_mBodyBoundTo`, `m_mDeployRequested` or `m_mDepartedSlots`. The dead
//!     player's seat is retained by ONE LIFE exactly as before, and their corpse stays where it
//!     fell (the host is a separate entity spawned AT the corpse, not the corpse moved).
//!   * **It does not launder the dead state.** `IsPlayerDead(playerId)` is `IsBindKeyDead(
//!     PlayerBindKey(playerId))` - a pure function of identity, with no reference to bodies,
//!     controllers or controlled entities. Possessing anything at all is invisible to it.
//!   * **Disconnect/reconnect cannot produce a live body.** `TBD_SpawnManager.OnPlayerDisconnected`
//!     releases the host FIRST, before any of its existing bookkeeping, so the rest of that method
//!     sees exactly the world it saw before this slice existed (the player controlling their
//!     corpse) and `ForgetBodyVanillaIsAboutToTake` keeps working unchanged. On the way back in,
//!     the join hook re-runs the ordinary one-life path: the seat is handed back, the life is still
//!     spent, `DeployPlayerInternal` answers DENIED, and the reconcile tick below notices a dead
//!     connected player with no host and gives them a new one. A host is never persisted, never
//!     reclaimed and never inherited.
//!
//! == THE playerId REUSE HAZARD (T-181.15 pattern, applied to STORED state) ===================
//! Numeric playerIds are recycled on a dedicated server. T-181.15 answered that for the CALLQUEUE
//! with connection epochs, because `ScriptCallQueue.Remove` cancels BY FUNCTION and cannot cancel
//! one player's pending callback. This file deliberately schedules **no per-player deferred
//! callback at all** - there is exactly one repeating reconcile tick for the whole server, which
//! `Remove(Tick)` cancels cleanly - but it does hold per-player STATE across time, which is the
//! same hazard wearing a different hat: a record left under a recycled number would hand a fresh
//! joiner a departed player's host. So every record carries the connection epoch it was opened
//! under and the reconcile tick retires any record whose epoch no longer matches. One mechanism,
//! the same definition of "still the same person", borrowed rather than reinvented.
//!
//! == SAFESTART - THE DELIBERATE DECISION =====================================================
//! `TBD_SafestartManager.CollectProtectables` sweeps `PlayerManager.GetPlayerControlledEntity`,
//! which IS the host once it is possessed, so the host WILL be swept into `m_mHeld`.
//!
//! **The decision: let it be swept, and make the sweep provably inert instead of hiding from it.**
//! `ApplyTo` only touches `SCR_CharacterDamageManagerComponent`, `CharacterControllerComponent` and
//! `EventHandlerManagerComponent`; `RestoreOne` early-returns `true` when there is no damage
//! manager. A host that is refused unless it has none of those is therefore a no-op on both ends -
//! it costs one map entry and nothing else, and it can never be the body that fails restore
//! verification and starts safestart's ERROR-spamming watchdog. Making the host structurally
//! unprotectable fixes that for good rather than per-path.
//! (Compile-verified reasoning over the real source of both files; not runtime-observed.)
//!
//! **T-181.30 - the ORIGINAL justification for this is now stale, and is corrected here rather
//! than quietly deleted.** This block used to say `Restore()` "does not restore each body's prior
//! value, it forces `EnableDamageHandling(true)` on everything it held", so a swept host would
//! come out of SAFE_START damageable. **T-181.33 fixed that**: safestart now records each body's
//! damage-handling value before its first mutation and hands back *that*, not `true`
//! (`TBD_SafestartHold.m_bDamageWasEnabled`, `RestoreOne`). The decision above survives intact
//! because it never actually rested on that bug - see `IsAcceptableHost` for the reason that does.
//!
//! == WHAT THIS COSTS - READ BEFORE SHIPPING AN EVENT =========================================
//! `TBD_SpectatorTargets` documents that its faction restriction is a DISCIPLINE measure and that
//! "the real limit is the engine's own replication range: an entity that was never streamed to you
//! does not exist on your machine". **This slice weakens that backstop by design** - steering the
//! streaming origin is the entire feature. An unmodified client still only sees its own side in the
//! roster; a MODIFIED client can now fly to the enemy and have the enemy streamed to it, where
//! before it would have flown into an empty world. `m_fHostMaxRangeM` on `TBD_SpectatorComponent`
//! is the lever (metres from the player's own death position, 0 = unlimited, which is the default
//! because seeing the AO is the point of a spectator camera). Flagged to the command centre rather
//! than buried.
modded class SCR_PlayerController
{
	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: "my spectator camera is here; put my streaming host there."
	//!
	//! The position is client-supplied and that is safe in the same way `TBD_RequestClaimSlot`'s
	//! slot key is safe: the authority does not trust it. `TBD_SpectatorHost.MoveTo` refuses anyone
	//! who is not dead, anyone with no host, anyone whose connection epoch has moved on, and clamps
	//! whatever survives to the world bounds and to the configured range. The worst a modified
	//! client achieves is choosing which part of the map is streamed to ITSELF - see the header.
	//!
	//! Unreliable on purpose: this is a position sample, and a dropped one is corrected 250 ms later
	//! by the next. A reliable channel would queue and replay stale camera positions after a stall.
	void TBD_ReportSpectatorCamera(vector position)
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_SpectatorHostAt, position);
			return;
		}

		// Listen host: the requester IS the authority, so short-circuit rather than RPC to
		// ourselves. Same shape as TBD_LobbyController, and it keeps both topologies on one path.
		TBD_SpectatorHost.MoveTo(GetPlayerId(), position);
	}

	//! @authority server
	//! @rpc Unreliable Server
	[RplRpc(RplChannel.Unreliable, RplRcver.Server)]
	protected void TBD_RpcAsk_SpectatorHostAt(vector position)
	{
		// GetPlayerId() is the authority's own answer for whoever owns this controller - the client
		// does not get to name a player, so this RPC cannot move somebody else's host.
		TBD_SpectatorHost.MoveTo(GetPlayerId(), position);
	}
}

//! What the authority remembers about one spectator's host. Server-side only; never replicated.
class TBD_SpectatorHostRecord
{
	//! The connection epoch this host was opened under. A record whose epoch no longer matches
	//! belongs to somebody who has left, and the number may already have been handed to a stranger.
	int epoch;

	//! The dummy itself. A plain (non-`ref`) handle: the world owns entities, exactly as
	//! `TBD_SpawnManager.m_mSlotBodies` does.
	IEntity host;

	//! Where the player died. The origin the range clamp measures from, and the fallback the host
	//! sits at until the first camera report arrives.
	vector anchor;

	//! Last position we actually applied, so a stationary camera costs zero teleports.
	vector applied;
}

//! SERVER - the whole lifecycle. Static for the same reason `TBD_SpectatorController` is: the
//! thing that owns it (`TBD_SpectatorComponent`) is created and destroyed with the world, and
//! `Start`/`Shutdown` are the two lines that tie this to that lifetime.
//!
//! **Statics outlive a world inside one process** (measured landmine in this codebase), which is
//! why `Shutdown` is not optional and why it releases every host rather than just dropping the map.
class TBD_SpectatorHost
{
	//! One tick for the whole server, not one per player. Cheap: a map walk plus one
	//! `GetPlayers()`. A second of latency between dying and getting a host is invisible - the
	//! camera is already up (`TBD_SpectatorController` polls at 250 ms) and it starts at the corpse,
	//! which is exactly where the streaming already is.
	static const int RECONCILE_MS = 1000;

	//! Do not teleport for jitter. Below this the host stays put and the server does nothing.
	static const float MIN_MOVE_M = 1.5;

	protected static ref map<int, ref TBD_SpectatorHostRecord> s_mHosts;

	protected static bool s_bRunning;

	//! Operator configuration, copied from `TBD_SpectatorComponent` at Start so the rest of this
	//! class never has to reach back through the component graph.
	protected static ResourceName s_sHostPrefab;
	protected static float s_fMaxRangeM;

	//! One-shot latch: a configured prefab that will not load is reported ONCE and then the
	//! prefab-free route is used for the rest of the round. Without the latch a bad ResourceName
	//! would print an error every second for the whole event.
	protected static bool s_bPrefabFailureLogged;

	//! One-shot latch for the "no spawn manager on a framework world" fail-closed report.
	protected static bool s_bNoManagerLogged;

	//! -- LOG-FLOOD LATCH ---------------------------------------------------------------------
	//! The reconcile below RETRIES every second for every dead player who has no host, which is
	//! exactly right for a transient cause (the corpse has not appeared yet) and exactly wrong for a
	//! persistent one - an unservable player would otherwise put one line per second in the log for
	//! the rest of the event and bury everything else. So a refusal is reported once and then held
	//! quiet until a host is successfully issued, which is the only evidence that circumstances
	//! actually changed.
	protected static bool s_bIssueRefusedLogged;

	// -- Lifecycle ---------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! @authority server - a client has no business spawning or possessing anything.
	//! Called by `TBD_SpectatorComponent.OnPostInit` on authority (dedicated AND listen host).
	static void Start(ResourceName hostPrefab, float maxRangeM)
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (s_bRunning)
			return;

		s_mHosts = new map<int, ref TBD_SpectatorHostRecord>();
		s_sHostPrefab = hostPrefab;
		s_fMaxRangeM = maxRangeM;
		s_bPrefabFailureLogged = false;
		s_bNoManagerLogged = false;
		s_bIssueRefusedLogged = false;
		s_bRunning = true;

		string mode = "prefab-free scripted host (no resourceDatabase.rdb dependency)";
		if (!s_sHostPrefab.IsEmpty())
			mode = string.Format("prefab %1", s_sHostPrefab);

		string line = string.Format("[TBD][spectator] streaming host ARMED - %1", mode);
		line = line + string.Format(", range=%1 m (0 = unlimited)", s_fMaxRangeM);
		PrintFormat("%1", line);

		GetGame().GetCallqueue().CallLater(Tick, RECONCILE_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Mission teardown. Every host is released and deleted - a dummy that survived into the next
	//! world would be an orphan nobody owns, still listed as somebody's controlled entity.
	//!
	//! `ScriptCallQueue.Remove` cancels BY FUNCTION, which is exactly right here: there is one
	//! `Tick` for the whole server, so cancelling "every Tick" cancels precisely the one we own.
	static void Shutdown()
	{
		GetGame().GetCallqueue().Remove(Tick);

		ReleaseAll("mission teardown");

		// AFTER ReleaseAll, because ReleaseAll is what queues them. Cancelling BY FUNCTION is exactly
		// right here and for once the coarseness is the point: every pending delete belongs to a
		// world that is going away and will take its entities with it, so the deferred pass must not
		// be allowed to fire into the next one holding stale handles.
		GetGame().GetCallqueue().Remove(DeleteHostNextFrame);

		s_mHosts = null;
		s_sHostPrefab = string.Empty;
		s_fMaxRangeM = 0;
		s_bPrefabFailureLogged = false;
		s_bNoManagerLogged = false;
		s_bIssueRefusedLogged = false;
		s_bRunning = false;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsRunning()
	{
		return s_bRunning;
	}

	//------------------------------------------------------------------------------------------------
	//! Does this player hold a streaming host right now?
	//!
	//! T-181.30 - this used to claim it was public "so `TBD_SpawnManager` can say so in its own
	//! lines". `TBD_SpawnManager` has never called it. Kept because `Reconcile` genuinely needs it
	//! (below), not because anything outside this file does; if that internal caller ever goes away,
	//! so should this.
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

	// -- The reconcile -----------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! One pass: retire what should not exist, then create what should.
	//!
	//! A reconcile rather than an event hook, and that is a deliberate copy of the reasoning in
	//! `TBD_SpectatorController`: "am I in spectator" has one true answer (do I have a spent life
	//! and a live connection) and deriving it every second cannot drift, cannot miss an edge, and
	//! cannot be raced by ordering. It also covers the case a death hook structurally cannot - a
	//! player who reconnects onto an already-spent life never receives a death event at all.
	//! @authority server
	protected static void Tick()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!s_mHosts)
			return;

		// A plain vanilla world must behave as if this file did not exist - same guard the rest of
		// the mod uses.
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
				Print("[TBD][spectator] streaming host STOOD DOWN - framework world with no TBD_SpawnManager (cannot tell a dead player from a live one)", LogLevel.ERROR);
			}

			ReleaseAll("no TBD_SpawnManager");
			return;
		}

		s_bNoManagerLogged = false;

		bool stageOk = IsStageHostable();

		RetireStaleHosts(spawn, stageOk);

		if (!stageOk)
			return;

		array<int> players = {};
		int count = GetGame().GetPlayerManager().GetPlayers(players);
		for (int i = 0; i < count; i++)
		{
			int playerId = players[i];

			if (HasHost(playerId))
				continue;

			// THE PRECONDITION. Only a spent life gets a host - a living player already has a body
			// anchoring their streaming, and handing one to somebody who is not dead is the only way
			// this feature could ever become a route into the world.
			if (!spawn.IsPlayerDead(playerId))
				continue;

			EnsureHost(spawn, playerId);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Retire every host that has stopped being legitimate.
	//!
	//! The map is never mutated while it is being walked: the victims are collected first and
	//! released after, the same discipline `TBD_SpawnManager.ReclaimDepartedSeat` uses.
	//! @authority server
	protected static void RetireStaleHosts(notnull TBD_SpawnManager spawn, bool stageOk)
	{
		array<int> victims = {};
		array<string> reasons = {};

		foreach (int playerId, TBD_SpectatorHostRecord record : s_mHosts)
		{
			if (!record || !record.host)
			{
				victims.Insert(playerId);
				reasons.Insert("host entity is gone");
				continue;
			}

			// The T-181.15 epoch test, applied to stored state. MUST come before the controller
			// test: a recycled id HAS a controller, which is exactly how the wrong player would
			// inherit this host.
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

			// Belt and braces against the one outcome that would matter: a player who is somehow no
			// longer dead must not be left possessing a dummy instead of their own body. In practice
			// `TBD_SpawnManager` releases the host itself on the admin-respawn path before it
			// deploys, so this should never fire - and if it ever does, it fires in the safe
			// direction and says so.
			if (!spawn.IsPlayerDead(playerId))
			{
				victims.Insert(playerId);
				reasons.Insert("player is no longer dead (admin respawn?) - handing control back");
				continue;
			}
		}

		for (int i = 0; i < victims.Count(); i++)
		{
			ReleaseFor(victims[i], reasons[i]);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Spectator hosting engages from SAFE_START onward, matching
	//! `TBD_SpectatorController.IsStageSpectatable` exactly - a friendly-fire death during safe
	//! start spends a life like any other, and the two halves of the feature must not disagree about
	//! when they are live.
	protected static bool IsStageHostable()
	{
		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (!framework)
			return false;

		return framework.GetStage() >= TBD_EGameStage.SAFE_START;
	}

	// -- Create ------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Give one dead player a host. Every failure path leaves the world exactly as it found it.
	//! @authority server
	protected static void EnsureHost(notnull TBD_SpawnManager spawn, int playerId)
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(
			GetGame().GetPlayerManager().GetPlayerController(playerId));
		if (!pc)
			return;

		// Somebody else is already possessing on this player's behalf (Game Master, a future
		// feature). Do not fight over the controller - the corpse-anchored streaming they have today
		// is worse, but it is not broken, and stealing possession would be.
		if (pc.IsPossessing())
		{
			NoteIssueRefused(playerId, "the player controller is already possessing something else - not fighting over it");
			return;
		}

		vector anchor;
		if (!ResolveAnchor(spawn, playerId, anchor))
		{
			NoteIssueRefused(playerId, "no corpse and no assigned slot, so there is nowhere honest to put an anchor");
			return;
		}

		bool fromPrefab;
		IEntity host = SpawnHostEntity(anchor, fromPrefab);
		if (!host)
		{
			NoteIssueRefused(playerId, "the host entity would not spawn");
			return;
		}

		// THE KEYSTONE. Checked before possession, and the candidate is destroyed rather than used
		// if it fails - see the ONE LIFE block in the file header.
		if (!IsAcceptableHost(host, playerId))
		{
			SCR_EntityHelper.DeleteEntityAndChildren(host);
			RecoverFromUnacceptableHost(fromPrefab);
			return;
		}

		pc.SetPossessedEntity(host);

		// Calling the setter is not evidence that it took - the same discipline
		// `TBD_SafestartManager.RestoreOne` applies to damage handling. Read it back.
		IEntity controlled = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
		if (controlled != host)
		{
			NoteIssueRefused(playerId, "the engine did not transfer control to the host - rolled back, the spectator keeps corpse-anchored streaming");
			pc.SetPossessedEntity(null);
			SCR_EntityHelper.DeleteEntityAndChildren(host);
			return;
		}

		// A host landed, so whatever was being suppressed is over. The next genuine problem gets its
		// own line instead of being swallowed by a latch set half an hour ago.
		s_bIssueRefusedLogged = false;

		TBD_SpectatorHostRecord record = new TBD_SpectatorHostRecord();
		record.epoch = spawn.ConnectionEpochFor(playerId);
		record.host = host;
		record.anchor = anchor;
		record.applied = anchor;
		s_mHosts.Set(playerId, record);

		string line = string.Format("[TBD][spectator] player=%1 streaming host ISSUED at %2", playerId, anchor.ToString());
		line = line + string.Format(" epoch=%1 replicated=%2 (life still spent, no slot, no body)",
			record.epoch, IsReplicated(host));
		PrintFormat("%1", line);
	}

	//------------------------------------------------------------------------------------------------
	//! Say once why a dead player is not getting a host, then be quiet until one is issued.
	//! See the s_bIssueRefusedLogged latch - the reconcile retries every second and a persistent
	//! cause would otherwise write one line per second for the whole event.
	//! @authority server
	protected static void NoteIssueRefused(int playerId, string reason)
	{
		if (s_bIssueRefusedLogged)
			return;

		s_bIssueRefusedLogged = true;

		string line = string.Format("[TBD][spectator] player=%1 has NO streaming host - %2.", playerId, reason);
		line = line + " Their camera still works; it just stays anchored to their corpse. (Latched: one line per round until a host is issued.)";
		PrintFormat("%1", line, level: LogLevel.WARNING);
	}

	//------------------------------------------------------------------------------------------------
	//! A candidate host failed the ONE LIFE acceptance check. Do not simply try again next second -
	//! nothing about a misconfiguration heals on its own, and respawning and deleting an entity once
	//! a second for the rest of the event is a worse failure than the one being reported.
	//!
	//!   * A bad PREFAB is recoverable: drop `m_sHostPrefab` and use the built-in prefab-free host
	//!     from here on. The operator gets their spectator streaming and a loud line saying why
	//!     their prefab was ignored.
	//!   * A bad BUILT-IN host cannot happen - `TBD_SpectatorHostEntity` is a bare `GenericEntity`
	//!     with no components at all - so if it ever does, something is wrong at a level this file
	//!     cannot reason about and the whole feature stands down rather than guessing.
	//! @authority server
	protected static void RecoverFromUnacceptableHost(bool fromPrefab)
	{
		if (fromPrefab)
		{
			Print(string.Format("[TBD][spectator] ignoring m_sHostPrefab %1 for the rest of this round - falling back to the built-in prefab-free host", s_sHostPrefab), LogLevel.ERROR);
			s_sHostPrefab = string.Empty;
			return;
		}

		Print("[TBD][spectator] the BUILT-IN streaming host was refused, which should be impossible - standing the streaming host down for this round rather than churning entities", LogLevel.ERROR);

		// Removing a repeating call from inside the call itself is the idiom
		// `TBD_SafestartManager.TickSweep` already uses.
		GetGame().GetCallqueue().Remove(Tick);
		ReleaseAll("streaming host stood down after an impossible refusal");
		s_bRunning = false;
	}

	//------------------------------------------------------------------------------------------------
	//! Where the host starts: the corpse if it is still there, else the player's own slot transform.
	//!
	//! Never the world origin. CRF learned that one loudly enough to put it in capitals
	//! (`CRF_EntityHelper.ZERO_SPAWN_VECTOR`), and a host at 0,0,0 would anchor streaming to the
	//! corner of the map - the exact failure this slice exists to remove.
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

	//------------------------------------------------------------------------------------------------
	//! Build the dummy.
	//!
	//! Default route is BY TYPENAME with no prefab, which is what makes the streaming host work
	//! today: new `.et`/`.conf`/`.layout` resources are invisible to the engine until Workbench
	//! rewrites `resourceDatabase.rdb`, and the mod's rdb is a stale snapshot that already does not
	//! list the spectator menu preset. A scripted entity spawned by typename sidesteps that
	//! entirely - the same trick `TBD_SpectatorCamera` uses.
	//!
	//! `m_sHostPrefab` is the escape hatch, and it exists for one specific reason worth stating: a
	//! typename-spawned entity has NO `RplComponent`, so it exists on the server only. That is
	//! sufficient if the engine scopes replication on the SERVER's view of the controlled entity
	//! (which is the hypothesis this slice is built on and which only a live dedicated-server test
	//! can settle). If it turns out a replicated host is required, an operator points this attribute
	//! at a prefab whose root class is `TBD_SpectatorHostEntity` and nothing else changes - the
	//! acceptance check below still refuses to let that prefab be a character.
	//!
	//! `fromPrefab` tells the caller WHICH route produced the entity, so a candidate that fails the
	//! acceptance check can be recovered from correctly (drop the prefab) instead of standing the
	//! whole feature down.
	//! @authority server
	protected static IEntity SpawnHostEntity(vector position, out bool fromPrefab)
	{
		fromPrefab = false;

		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return null;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = position;

		if (!s_sHostPrefab.IsEmpty())
		{
			IEntity configured = SpawnHostPrefab(params, world);
			if (configured)
			{
				fromPrefab = true;
				return configured;
			}
		}

		IEntity spawned = GetGame().SpawnEntity(TBD_SpectatorHostEntity, world, params);
		if (!spawned)
			return null;

		NeutralisePhysics(spawned);
		return spawned;
	}

	//------------------------------------------------------------------------------------------------
	//! The configured-prefab route. Returns null (having said why, once) so the caller falls back to
	//! the prefab-free host rather than leaving the spectator with nothing.
	//! @authority server
	protected static IEntity SpawnHostPrefab(notnull EntitySpawnParams params, notnull BaseWorld world)
	{
		Resource resource = Resource.Load(s_sHostPrefab);
		if (!resource || !resource.IsValid())
		{
			if (!s_bPrefabFailureLogged)
			{
				s_bPrefabFailureLogged = true;
				Print(string.Format("[TBD][spectator] streaming host prefab %1 will not load (missing, or not in resourceDatabase.rdb) - falling back to the prefab-free host", s_sHostPrefab), LogLevel.WARNING);
			}

			return null;
		}

		IEntity spawned = GetGame().SpawnEntityPrefab(resource, world, params);
		if (!spawned)
		{
			if (!s_bPrefabFailureLogged)
			{
				s_bPrefabFailureLogged = true;
				Print(string.Format("[TBD][spectator] streaming host prefab %1 loaded but failed to spawn - falling back to the prefab-free host", s_sHostPrefab), LogLevel.WARNING);
			}

			return null;
		}

		NeutralisePhysics(spawned);
		return spawned;
	}

	//------------------------------------------------------------------------------------------------
	//! ONE LIFE, enforced on the entity itself rather than on the paths that reach it.
	//!
	//! A guard on a code path only holds while nobody adds another path - three rounds of one-life
	//! fixes in `TBD_SpawnManager` are what that lesson cost. This one is a property of the object:
	//! whatever calls whatever, the thing a dead player ends up possessing has no way to be hurt, to
	//! die, or to be stood back up.
	//!
	//! It also stops an operator from turning `m_sHostPrefab` into a second door by pointing it at a
	//! character prefab. That would otherwise hand every dead player a fresh, damageable body, which
	//! is precisely a respawn.
	//!
	//! T-181.30 - WHY THE DAMAGE-MANAGER REFUSAL IS STILL RIGHT, on a rationale that is actually
	//! true. It used to cite safestart: `Restore()` force-enabled damage handling on everything it
	//! swept, so a host with a damage manager "would come out of SAFE_START damageable". T-181.33
	//! made safestart save and restore the prior value, so that specific sentence is now false. The
	//! guard is unchanged because the real reason never needed it:
	//!
	//!   * A dead player POSSESSES this entity. Anything carrying a damage manager can be damaged
	//!     and destroyed, and under ONE LIFE an entity a dead player controls that can be killed is
	//!     a second death path - while one that can be healed back up is respawn-shaped. Refusing
	//!     the component makes that structurally impossible instead of path-dependent, which is the
	//!     entire point of this function.
	//!   * It also keeps the host a provable no-op through safestart's whole arm/lift cycle: with no
	//!     damage manager, `RestoreOne` takes its `!damage` early return, so the host can never be
	//!     the entity whose restore fails verification and pins the watchdog at ERROR.
	//!
	//! Note the refusal is deliberately BROADER than the sweep: this tests `DamageManagerComponent`,
	//! while safestart only ever touches `SCR_CharacterDamageManagerComponent`. That was true before
	//! T-181.33 as well - another sign the safestart framing was never what carried this guard.
	//! @authority server
	protected static bool IsAcceptableHost(notnull IEntity host, int playerId)
	{
		string refusal;

		if (ChimeraCharacter.Cast(host))
			refusal = "it is a ChimeraCharacter - a character can be killed, and a killed character spends a life";
		else if (host.FindComponent(DamageManagerComponent))
			refusal = "it carries a DamageManagerComponent - a dead player would be possessing something that can be damaged, destroyed or healed, which is a second death path under ONE LIFE";
		else if (host.FindComponent(CharacterControllerComponent))
			refusal = "it carries a CharacterControllerComponent - that is a playable body, not an anchor";

		if (refusal.IsEmpty())
			return true;

		Print(string.Format("[TBD][spectator] player=%1 streaming host candidate REFUSED: %2", playerId, refusal), LogLevel.ERROR);
		Print("[TBD][spectator] the spectator streaming host must be an inert entity. Check m_sHostPrefab on TBD_SpectatorComponent; leave it EMPTY for the built-in prefab-free host.", LogLevel.ERROR);
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Best effort, and genuinely a no-op for the prefab-free host: a typename-spawned
	//! `GenericEntity` has no `Physics` at all, so `GetPhysics()` returns null and this returns
	//! immediately. It earns its place only on the `m_sHostPrefab` route, where whatever the
	//! operator pointed at must not fall, drift or push anybody.
	//! (Call shape mirrors CRF_SpectatorCharacter.DisablePhysicsAndDamage - read as an oracle, not
	//! copied: the damage half is deliberately absent because a host with a damage manager is
	//! refused outright rather than pacified.)
	//! @authority server
	protected static void NeutralisePhysics(notnull IEntity host)
	{
		Physics physics = host.GetPhysics();
		if (!physics)
			return;

		physics.EnableGravity(false);
		physics.SetMass(0);
		physics.ChangeSimulationState(SimulationState.NONE);
		physics.SetInteractionLayer(EPhysicsLayerDefs.CharNoCollide);
	}

	//------------------------------------------------------------------------------------------------
	//! Is this host visible to the network at all? Logged once per issue so the first live run
	//! answers the one question the compile lane cannot: whether the prefab-free (server-only) host
	//! is enough, or whether the `m_sHostPrefab` route is required.
	protected static bool IsReplicated(notnull IEntity host)
	{
		RplComponent rpl = RplComponent.Cast(host.FindComponent(RplComponent));
		if (!rpl)
			return false;

		return rpl.Id().IsValid();
	}

	// -- Move --------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! The camera moved; move the anchor. Called from the transport above - never trusts its input.
	//! @authority server
	static void MoveTo(int playerId, vector position)
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!s_mHosts)
			return;

		TBD_SpectatorHostRecord record;
		if (!s_mHosts.Find(playerId, record) || !record || !record.host)
			return;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return;

		// Fail closed on both halves of "is this still the same dead person". The epoch check is not
		// redundant with the deadness check: a recycled id could belong to a DIFFERENT player who is
		// also dead, and that player must not be able to drive this record's host.
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

	//------------------------------------------------------------------------------------------------
	//! Keep the host inside the world. A client can send anything; the world box is the one bound
	//! that is always meaningful.
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

	//------------------------------------------------------------------------------------------------
	//! The operator's leash, off by default. See the "what this costs" block in the file header -
	//! this is the lever that trades spectator reach against how much of the map a modified client
	//! can pull down.
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

	// -- Release -----------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Hand control back and delete the dummy. Idempotent, safe to call for a player who has none,
	//! and safe to call while the engine is tearing that player down.
	//!
	//! Order matters and is the same rule `TBD_SpectatorController.Leave` states for the camera:
	//! switch away BEFORE deleting, never after. `SetPossessedEntity(null)` restores the entity the
	//! player controlled when the host was issued (vanilla remembers it as `m_MainEntity`), so a
	//! release puts them back on their corpse - which is where a dead player was before this slice
	//! existed, and is exactly what every downstream assumption in `TBD_SpawnManager` is written
	//! against.
	//!
	//! Returns true when a host was actually released, so callers can say so in one line.
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

		// The controller can already be gone (disconnect teardown). Deleting the entity is still
		// correct and still necessary - vanilla would otherwise delete it for us on some paths and
		// leak it on others.
		// Give the view back - but only if what the player is holding is actually OUR host.
		//
		// `SetPossessedEntity(null)` ends whatever possession is in progress, not specifically ours,
		// so calling it blind would cancel a Game Master (or any future feature) that had taken the
		// controller over on top of us. The controlled-entity comparison is what makes this release
		// mind its own business. A record whose host has already been destroyed is the one case
		// where there is nothing to compare against and un-possessing is unambiguously right -
		// leaving `IsPossessing()` true against a dead entity would strand the player.
		SCR_PlayerController pc = SCR_PlayerController.Cast(
			GetGame().GetPlayerManager().GetPlayerController(playerId));
		if (pc && pc.IsPossessing())
		{
			IEntity controlled = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
			if (!record.host || controlled == record.host)
				pc.SetPossessedEntity(null);
			else
				Print(string.Format("[TBD][spectator] player=%1 is possessing something that is not their streaming host - leaving that alone and only deleting the host", playerId), LogLevel.WARNING);
		}

		if (record.host)
		{
			IEntity host = record.host;
			record.host = null;
			GetGame().GetCallqueue().Call(DeleteHostNextFrame, host);
		}

		Print(string.Format("[TBD][spectator] player=%1 streaming host RELEASED - %2", playerId, reason));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! NEXT FRAME, not now.
	//!
	//! Deleting an entity the engine still lists as somebody's controlled entity is the exact thing
	//! `TBD_SpectatorController.Leave` warns about for the camera, and here it is not hypothetical.
	//! `SetPossessedEntity(null)` gives control back to whatever the player controlled when the host
	//! was issued (vanilla's `m_MainEntity`) - but a player who RECONNECTED onto an already-spent
	//! life had no body at that moment, so there is nothing to give back and the host is still the
	//! controlled entity at the instant we let go of it. CRF hit the same wall and answered it the
	//! same way: `CRF_SpectatorCharacter.OnControlledByPlayer` defers its own delete with the comment
	//! "Need to call on next frame so we dont mess up the player controller".
	//!
	//! The argument is an ENTITY, not a playerId, so this deferred call carries none of the
	//! recycled-id hazard that made T-181.15 stamp epochs on everything else in the queue - there is
	//! no player to mistake, only an object to free.
	//! @authority server
	protected static void DeleteHostNextFrame(IEntity host)
	{
		if (!host)
			return;

		SCR_EntityHelper.DeleteEntityAndChildren(host);
	}

	//------------------------------------------------------------------------------------------------
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
