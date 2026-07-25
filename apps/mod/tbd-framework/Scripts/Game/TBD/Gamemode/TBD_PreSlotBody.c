//! T-181.50 — THE PRE-SLOT GHOST: the inert entity a connected-but-unslotted player controls so
//! the engine is never asked to run a player who controls nothing.
//!
//! ── THE DEFECT THIS EXISTS FOR ──────────────────────────────────────────────────────────────
//! T-181.48 shipped `m_bAutoDeploy 0` on `Prefabs/Systems/TBD_GameMode.et`, which is correct — the
//! picker and the wave cannot both be on. But it was the first time this mod has ever run with
//! NOBODY seated at LOBBY, and it exposed something no previous session could: with vanilla
//! registration swallowed (`TBD_SCR_RespawnSystemComponent`) and the wave off, a joining player's
//! `GetControlledEntity()` is null and STAYS null until they pick a slot. Every session where the
//! picker was visible had auto-deploy ON, i.e. a body. The operator's black screen is that.
//!
//! ── WHAT THIS FIXES AND WHAT IT DOES NOT ────────────────────────────────────────────────────
//! Be precise, because the two halves land on different machines and only one of them is here:
//!
//!   * SERVER (this file). The engine and every vanilla system that reaches for
//!     `PlayerManager.GetPlayerControlledEntity(playerId)` now gets an entity instead of null.
//!   * CLIENT (`TBD_PreSlotCamera.c`). The actual black screen. A bare `GenericEntity` has no
//!     camera handler, so a ghost ALONE renders nothing — see the PlayableSelector note below.
//!
//! Anyone reading this expecting the ghost to be the whole fix will be wrong. It is the smaller
//! half.
//!
//! ── ORACLES ─────────────────────────────────────────────────────────────────────────────────
//! PlayableSelector (design-mirror only, NO LICENCE, never a line copied — it lives outside this
//! repo at ~/Projects/Archive/Reforger_Lobby/PlayableSelector-main and is not symlinked into a
//! slice worktree; T-181.52 is the ticket to fix that lane). Read for the SHAPE of the idea:
//!
//!   * `PS_GameModeCoop.c:721-742` — spawn a ghost per player and hand it over with
//!     `SetInitialMainEntity`. `:552` is a bare "TODO: remove CallLater" over a fixed 100 ms
//!     (500 ms under Workbench) delay after `OnPlayerConnected`, with no readiness test. We do not
//!     copy the delay: this is a RECONCILE (see `Tick`), so there is no moment to miss.
//!   * `PS_GameModeCoop.c:733` / `PS_PlayableControllerComponent.c:608` — the per-player cell,
//!     a lattice based at y = 100000. We take the ALTITUDE and the "one cell each" idea and write
//!     our own arithmetic; see `CellFor` for why our reason for separating them is much weaker than
//!     theirs.
//!   * `PS_PlayableManager.c:258-275` — the commit ordering, which is the load-bearing part:
//!     possession of the REAL body happens first and the ghost's deletion is QUEUED behind it, so
//!     the player is never controller-less. `NoteDeployRequested` below obeys the same rule.
//!   * `PS_GameModeCoop.c:568-583` + `:830-838` — their disconnect path deliberately does NOT
//!     delete the ghost, and nothing else does either. That is a leak we must not inherit; see
//!     `TBD_SpawnManager.OnPlayerDisconnected`.
//!   * Their ghost is a REAL `SCR_ChimeraCharacter` made invisible by material swap
//!     (`Prefabs/InitialPlayer_Version2.et:47-72`) with the damage manager, AI subtree, sound and
//!     vanilla VoN disabled component-by-component (`:4-6`, `:24-46`, `:76-96`). Because it is a
//!     character it comes with vanilla's character camera for free, and their picker simply draws
//!     an opaque background over it (`UI/Lobby/CoopLobby.layout:5`). WE DO NOT GET THAT, which is
//!     precisely why `TBD_PreSlotCamera` has to exist.
//!
//! CRF (Arma Public License — read for idiom, never copied): `CRF_PlayerHelper.c:30-39` is the
//! comment that settles HOW the body is assigned. Vanilla `SCR_SpawnHandlerComponent.AssignEntity_S`
//! (`vanilla_reference/Source/SCR_SpawnHandlerComponent.c:265-268`) returns false and abandons the
//! whole finalize sequence when the controller already controls the target entity, so CRF assigns
//! with `SetInitialMainEntity` directly and calls the spawn notification themselves. Both oracles
//! converged on that independently. This file never issues a spawn REQUEST for the ghost — the
//! ghost is not a spawn, it is an anchor.
//!
//! ── WHY THE GHOST IS `TBD_SpectatorHostEntity` AND NOT A NEW CLASS ──────────────────────────
//! T-181.24 already built and audited exactly this object for the spectator lane: a bare
//! `GenericEntity` with no mesh, no damage manager, no character controller, no physics and no
//! sound, and the header of `TBD_SpectatorHostEntity.c` carries the full ONE LIFE argument for why
//! it is not a character. Reusing it is not laziness, it is the safer choice, and the deciding
//! reason is a SECOND-ORDER one:
//!
//!   `TBD_SpectatorTargets.IsAlive` and `TBD_SpectatorTargets.Collect` already exclude anything
//!   `TBD_SpectatorHostEntity.IsHost()` answers true for. A brand-new ghost class would NOT be
//!   covered by those exclusions, so a lobby ghost could appear in a spectator's roster, be
//!   followed, and be counted as a living body — in a lane this slice does not own and must not
//!   silently regress.
//!
//! ── ONE LIFE (non-negotiable) ───────────────────────────────────────────────────────────────
//! Three claims, each enforced by a PROPERTY OF THE OBJECT rather than by a guard on a path,
//! because three rounds of one-life fixes in `TBD_SpawnManager` are what that lesson cost:
//!
//!   1. It cannot be mistaken for a life. It is not a `ChimeraCharacter`, so
//!      `TBD_SpawnManager.CensusAddEntity` does not count it, `TBD_SpawnManager.IsBodyDead` would
//!      call it dead, and it never enters `m_mSlotBodies` or `m_mPlayerSlot` at all — it holds no
//!      slot, so it contributes nothing to `CountAliveForFaction` / `CountClaimedForFaction`.
//!   2. It cannot be shot. It has no `DamageManagerComponent`, so there is nothing for damage to
//!      apply to, and `IsAcceptableGhost` REFUSES to hand a player anything that has one. It also
//!      sits ~100 km above the playable world (`CellFor`).
//!   3. It cannot spend a life. `TBD_SpawnManager.MarkLifeSpent` has exactly one caller,
//!      `OnPlayerKilled`, which is `SCR_BaseGameMode`'s dispatch off a CONTROLLABLE BEING
//!      DESTROYED (`vanilla_reference/Source/SCR_BaseGameMode.c:1085-1113`). An entity with no
//!      damage manager is never destroyed by damage, and deleting one programmatically is not a
//!      kill. Belt and braces on top: a ghost is never issued to a player whose life is already
//!      spent (`ShouldHoldGhost`), and it is retired the moment they hold anything else.
//!
//! @authority server — this whole class is server-side. A client never spawns or assigns anything.
class TBD_PreSlotBody
{
	//! One reconcile for the whole server. Same cadence and the same reasoning as
	//! `TBD_SpectatorHost.RECONCILE_MS`: a second of latency before a bodyless player gets an
	//! anchor is invisible, and a reconcile cannot be raced by ordering the way an event hook can.
	static const int RECONCILE_MS = 1000;

	//! Metres above the world the ghost cells sit at. Taken from PlayableSelector's design
	//! (`PS_GameModeCoop.c:733` bases its lattice at 100000) for their stated reason: at that
	//! altitude the ghost cannot be seen, shot or collided with by anything on the ground.
	static const float CELL_ALTITUDE_M = 100000.0;

	//! Horizontal spacing between two players' cells. See `CellFor` — our reason for separating
	//! them at all is much weaker than PlayableSelector's, so this is a round number, not a tuned
	//! one.
	static const float CELL_PITCH_M = 1000.0;

	//! How many cells per row before the lattice steps in Z. Arbitrary; 16 keeps a 64-player
	//! server inside a 16 km x 4 km footprint of empty sky.
	static const int CELL_ROW = 16;

	//! How far a spawned ghost may land from the transform we asked for before `SelfTest` calls it a
	//! failure. Generous, because the question is "did the spawn honour the transform at all", not
	//! "to how many decimal places" — and it is the lever the self-test's negative control is pulled
	//! by (set it negative and every boot must go red). See `SelfTest`.
	static const float PLACE_TOLERANCE_M = 1.0;

	protected static ref map<int, ref TBD_PreSlotGhostRecord> s_mGhosts;
	protected static bool s_bRunning;

	//! One-shot latch for the "framework world with no spawn manager" fail-closed report. Without
	//! it the reconcile would write one ERROR per second for the rest of the event.
	protected static bool s_bNoManagerLogged;

	//! One-shot latch for a refusal to issue. Same shape and same reason as
	//! `TBD_SpectatorHost.s_bIssueRefusedLogged`: the reconcile retries every second, which is right
	//! for a transient cause and would bury the log for a persistent one. Cleared the moment a ghost
	//! is successfully issued, because that is the only evidence circumstances actually changed.
	protected static bool s_bIssueRefusedLogged;

	//! Reported once per world so a live run answers the question the compile lane cannot: whether a
	//! typename-spawned (therefore `RplComponent`-free, therefore server-only) ghost is enough. See
	//! the block on `SpawnGhostEntity`.
	protected static bool s_bReplicationReported;

	// ── Lifecycle ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! @authority server. Called by `TBD_PreSlotComponent.OnPostInit` on authority (dedicated AND
	//! listen host).
	static void Start()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (s_bRunning)
			return;

		s_mGhosts = new map<int, ref TBD_PreSlotGhostRecord>();
		s_bNoManagerLogged = false;
		s_bIssueRefusedLogged = false;
		s_bReplicationReported = false;
		s_bRunning = true;

		Print("[TBD][PreSlot] pre-slot ghost ARMED — a connected player with no body gets an inert anchor until they deploy");

		GetGame().GetCallqueue().CallLater(Tick, RECONCILE_MS, true);

		// Deferred one frame so the world is certainly up before anything is spawned into it — the
		// same reason `TBD_FrameworkManager.OnPostInit` defers its roll-call.
		GetGame().GetCallqueue().CallLater(SelfTest, 0, false);
	}

	// ── Self-test ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! T-181.50 — PROVE THE GHOST IS REAL, at every boot, on a harness that has no players.
	//!
	//! This exists because of an honest gap in the verification lane and not as decoration.
	//! `scripts/mod/compile.sh` proves the code compiles; `scripts/mod/world-boot.sh` boots the real
	//! scenario with ZERO PLAYERS, so it cannot exercise one single player-triggered path in this
	//! file — not `IssueGhost`, not `SetInitialMainEntity`, not the retirement. Without something
	//! like this, "green" would mean nothing more than "the class names resolve".
	//!
	//! So the two facts that CAN be established without a player are established, for real, in the
	//! live world:
	//!   * a `TBD_SpectatorHostEntity` genuinely spawns by typename at the transform we asked for
	//!     (the prefab-free route this whole design rests on), and
	//!   * `SCR_EntityHelper.DeleteEntityAndChildren` genuinely removes it, AND the handle we were
	//!     holding reads back null afterwards. That second half is not pedantry: it is the exact
	//!     engine behaviour PlayableSelector's ghost-respawn fallback silently depends on and never
	//!     checks (`PS_PlayableManager.c:246-253` re-spawns when the stored handle is null, but
	//!     nothing ever clears it). `RetireStaleGhosts` and `HasGhost` both make the same assumption,
	//!     so it is measured here rather than assumed.
	//!
	//! It also asserts, on the live object rather than from the prefab, the three ONE LIFE properties
	//! `IsAcceptableGhost` is built on. Those cannot fail for `TBD_SpectatorHostEntity` — it is a bare
	//! `GenericEntity` — which is precisely why an operator wants to know immediately if they ever do.
	//!
	//! ── NEGATIVE CONTROL (measured 2026-07-25) ──────────────────────────────────────────────────
	//! A self-test that cannot fail proves nothing, so this one was made to fail before it was
	//! believed: with `PLACE_TOLERANCE_M` temporarily set to a negative number the boot printed
	//! `[TBD][PreSlot] SELF-TEST FAILED: ghost spawned at <origin>, expected <cell>` and the run was
	//! visibly red in exactly the place claimed. Restored, it passes. Do not delete the tolerance
	//! constant to "simplify" — it is the handle the control is pulled by.
	//!
	//! Cost: one entity spawned and deleted once per world, ~100 km above the map. Deliberately not
	//! behind an attribute — a check an operator can turn off is a check that will be off on the
	//! night it mattered.
	//! @authority server
	protected static void SelfTest()
	{
		// A sentinel cell no player can be dealt: `CellFor` folds negative ids to positive, so this
		// negative lattice position is unreachable by construction.
		vector cell = Vector(-CELL_PITCH_M, CELL_ALTITUDE_M, -CELL_PITCH_M);

		IEntity probe = SpawnGhostEntity(cell);
		if (!probe)
		{
			Print("[TBD][PreSlot] SELF-TEST FAILED: a pre-slot ghost will not spawn at all — no waiting player will get an anchor this round.", LogLevel.ERROR);
			return;
		}

		int failures = 0;

		vector origin = probe.GetOrigin();
		if (vector.Distance(origin, cell) > PLACE_TOLERANCE_M)
		{
			Print(string.Format("[TBD][PreSlot] SELF-TEST FAILED: ghost spawned at %1, expected %2", origin.ToString(), cell.ToString()), LogLevel.ERROR);
			failures++;
		}

		// The ONE LIFE properties, read off the live object rather than trusted from the class
		// declaration. `IsAcceptableGhost` logs its own reason on refusal.
		if (!IsAcceptableGhost(probe, -1))
			failures++;

		SCR_EntityHelper.DeleteEntityAndChildren(probe);

		// NEXT FRAME. Asking whether the handle is null in the same frame as the delete would be
		// asking the wrong question — the engine tears down on the frame boundary, which is the very
		// reason `DeleteGhostNextFrame` exists.
		GetGame().GetCallqueue().CallLater(SelfTestVerifyGone, 0, false, probe, failures);
	}

	//------------------------------------------------------------------------------------------------
	//! The second half of the self-test: did the delete actually take, and does the handle we kept
	//! read back as nothing?
	//! @authority server
	protected static void SelfTestVerifyGone(IEntity probe, int failures)
	{
		int total = failures;

		if (probe)
		{
			Print("[TBD][PreSlot] SELF-TEST FAILED: the ghost survived DeleteEntityAndChildren, or a stale handle still reads live — retirement cannot be trusted this round.", LogLevel.ERROR);
			total++;
		}

		if (total > 0)
		{
			Print(string.Format("[TBD][PreSlot] SELF-TEST: %1 failure(s) — the pre-slot ghost is NOT trustworthy on this build. Treat a black lobby screen as expected until this is green.", total), LogLevel.ERROR);
			return;
		}

		Print("[TBD][PreSlot] SELF-TEST PASS — ghost spawns by typename at the requested transform, carries no character/damage/controller component, and is fully deleted (handle reads null next frame). Player-facing paths still need a live client.");
	}

	//------------------------------------------------------------------------------------------------
	//! Mission teardown. Statics outlive a world inside one process (measured landmine in this
	//! codebase, and `TBD_FrameworkManager.SelectMissionByNumber` restarts the scenario in-process),
	//! so this is not optional: a ghost that survived into the next world would be an orphan nobody
	//! owns, still listed as somebody's controlled entity.
	static void Shutdown()
	{
		GetGame().GetCallqueue().Remove(Tick);

		ReleaseAll("mission teardown");

		// AFTER ReleaseAll, because ReleaseAll is what queues them, and cancelling BY FUNCTION is
		// exactly right here: every pending delete belongs to a world that is going away and must
		// not fire into the next one holding a stale handle. Same call and same reasoning as
		// `TBD_SpectatorHost.Shutdown`.
		GetGame().GetCallqueue().Remove(DeleteGhostNextFrame);

		s_mGhosts = null;
		s_bRunning = false;
		s_bNoManagerLogged = false;
		s_bIssueRefusedLogged = false;
		s_bReplicationReported = false;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsRunning()
	{
		return s_bRunning;
	}

	//------------------------------------------------------------------------------------------------
	//! Does this player hold a pre-slot ghost right now? Public so the admin surfaces and a live
	//! probe can ask; `Tick` is the internal caller that makes it earn its place.
	static bool HasGhost(int playerId)
	{
		if (!s_mGhosts)
			return false;

		TBD_PreSlotGhostRecord record;
		if (!s_mGhosts.Find(playerId, record))
			return false;

		return record != null && record.ghost != null;
	}

	//------------------------------------------------------------------------------------------------
	//! The ghost entity this player holds, or null. The NEGATIVE-CONTROL surface: a probe that
	//! cannot see the ghost and cannot see it disappear proves nothing.
	static IEntity GhostFor(int playerId)
	{
		if (!s_mGhosts)
			return null;

		TBD_PreSlotGhostRecord record;
		if (!s_mGhosts.Find(playerId, record) || !record)
			return null;

		return record.ghost;
	}

	// ── The reconcile ───────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! One pass: retire what should no longer exist, then issue what should.
	//!
	//! A reconcile rather than a join hook, deliberately, and the reasoning is lifted wholesale from
	//! `TBD_SpectatorHost.Tick`: "does this player need an anchor" has one true answer (are they
	//! connected, bodyless, un-deployed and alive) and deriving it every second cannot drift, cannot
	//! miss an edge and cannot be raced by ordering. It is also what lets us skip PlayableSelector's
	//! fixed 100 ms post-connect delay and its own TODO about it (`PS_GameModeCoop.c:552`): there is
	//! no single moment to be early or late for.
	//! @authority server
	protected static void Tick()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!s_mGhosts)
			return;

		// A plain vanilla world must behave as if this file did not exist — the same guard every
		// other vanilla-suppressing class in this mod asks.
		if (!TBD_FrameworkManager.IsFrameworkWorld())
		{
			ReleaseAll("not a framework world");
			return;
		}

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
		{
			// FAIL CLOSED. Without the spawn manager we cannot tell a spent life from a live player
			// or a deployed one from a waiting one, and "cannot tell" must never resolve to "hand
			// them a possessed entity".
			if (!s_bNoManagerLogged)
			{
				s_bNoManagerLogged = true;
				Print("[TBD][PreSlot] pre-slot ghost STOOD DOWN — framework world with no TBD_SpawnManager (cannot tell a deployed player from a waiting one)", LogLevel.ERROR);
			}

			ReleaseAll("no TBD_SpawnManager");
			return;
		}

		s_bNoManagerLogged = false;

		RetireStaleGhosts(spawn);

		array<int> players = {};
		int count = GetGame().GetPlayerManager().GetPlayers(players);
		for (int i = 0; i < count; i++)
		{
			int playerId = players[i];

			if (HasGhost(playerId))
				continue;

			if (!ShouldHoldGhost(spawn, playerId))
				continue;

			IssueGhost(spawn, playerId);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! THE PRECONDITION, in one place so both halves of the reconcile agree by construction.
	//!
	//! Every clause is a refusal to interfere with something that already works:
	//!   * a player who CONTROLS SOMETHING is left completely alone. This is the one that matters —
	//!     a ghost must never displace a slot body, a corpse, a vehicle seat, a Game Master
	//!     possession or a spectator streaming host. It also makes `RetireStaleGhosts` correct for
	//!     free: the instant a deploy lands, the controlled entity stops being ours.
	//!   * a player who has DEPLOYED is on the vanilla possess pipeline and their body is arriving;
	//!     handing them an anchor mid-flight would be a second entity change inside the finalize.
	//!   * a player whose LIFE IS SPENT belongs to the spectator lane (`TBD_SpectatorHost`), which
	//!     owns anchors for the dead and has its own ONE LIFE reasoning. Two systems possessing on
	//!     one controller is exactly the fight `TBD_SpectatorHost.EnsureHost` refuses to have.
	//!   * END/DEBRIEF has no round left to wait for a slot in.
	//! @authority server
	protected static bool ShouldHoldGhost(notnull TBD_SpawnManager spawn, int playerId)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players || !players.GetPlayerController(playerId))
			return false;

		if (players.GetPlayerControlledEntity(playerId))
			return false;

		if (spawn.HasDeployRequested(playerId))
			return false;

		if (spawn.IsPlayerDead(playerId))
			return false;

		return IsStageWaitable();
	}

	//------------------------------------------------------------------------------------------------
	//! Is the round in a state where a player can still be waiting for a slot?
	//!
	//! LOADING is deliberately INCLUDED, unlike `TBD_SpectatorHost.IsStageHostable` which starts at
	//! SAFE_START. The two answer different questions: the spectator asks "is there a battle to
	//! watch", this asks "is there a player staring at nothing". A player who connects while the
	//! mission document is still loading is the earliest and most likely victim of the black screen,
	//! so refusing them an anchor would miss the case that reported the defect.
	protected static bool IsStageWaitable()
	{
		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (!framework)
			return false;

		return framework.GetStage() <= TBD_EGameStage.LIVE;
	}

	//------------------------------------------------------------------------------------------------
	//! Retire every ghost that has stopped being legitimate.
	//!
	//! The map is never mutated while it is walked: victims are collected first and released after —
	//! the discipline `TBD_SpectatorHost.RetireStaleHosts` and `TBD_SpawnManager.ReclaimDepartedSeat`
	//! both use.
	//! @authority server
	protected static void RetireStaleGhosts(notnull TBD_SpawnManager spawn)
	{
		array<int> victims = {};
		array<string> reasons = {};

		foreach (int playerId, TBD_PreSlotGhostRecord record : s_mGhosts)
		{
			if (!record || !record.ghost)
			{
				victims.Insert(playerId);
				reasons.Insert("ghost entity is gone");
				continue;
			}

			// The T-181.15 epoch test, applied to stored state. MUST come before the controller
			// test: a recycled numeric id HAS a controller, which is exactly how a fresh joiner
			// would inherit a departed player's ghost.
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

			// ORDERED BEFORE THE COMMIT DETECTOR BELOW, DELIBERATELY, and the reason is a narrow
			// race worth naming. A ghost-holder cannot normally become dead — a ghost cannot be
			// killed, which is the whole ONE LIFE argument — but their BIND KEY can change under
			// them: `TBD_SpawnManager.OnPlayerAuditSuccess` deliberately re-runs when the first
			// audit only produced a `player:<id>` lease, and the durable key it upgrades to may
			// already be in `m_mDeadPlayers` from earlier in the session. `IsPlayerDead` then flips
			// true while they still hold a ghost.
			//
			// When that happens the spectator lane wants this player (`TBD_SpectatorHost` serves
			// exactly the dead), and both reconciles run on a ~1 s beat, so whichever fires first
			// decides. Retiring on the DEATH reason first means the ghost is gone before the
			// spectator possesses anything, and the log names the real cause instead of reporting
			// "the player controls a real body now" about a spectator host.
			if (spawn.IsPlayerDead(playerId))
			{
				victims.Insert(playerId);
				reasons.Insert("life spent — the spectator lane owns this player's anchor now");
				continue;
			}

			// THE COMMIT DETECTOR, and the reason `NoteDeployRequested` can afford to be a courtesy
			// rather than the mechanism. The moment vanilla's possess finalize assigns the slot
			// body, the player's controlled entity stops being our ghost — whether that took one
			// frame or five, and whether or not anybody remembered to tell us.
			IEntity controlled = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
			if (controlled != record.ghost)
			{
				victims.Insert(playerId);

				// The two cases read very differently in a log: one is the picker working, the other
				// is something we did not initiate taking the controller over.
				if (record.deployRequested)
					reasons.Insert("deployed — the slot body landed");
				else
					reasons.Insert("the player controls something we did not give them");

				continue;
			}

			if (!IsStageWaitable())
			{
				victims.Insert(playerId);
				reasons.Insert("round is past the point of waiting for a slot");
				continue;
			}
		}

		for (int i = 0; i < victims.Count(); i++)
		{
			ReleaseFor(victims[i], reasons[i]);
		}
	}

	// ── Issue ───────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Give one bodyless player a ghost. Every failure path leaves the world exactly as it found it.
	//! @authority server
	protected static void IssueGhost(notnull TBD_SpawnManager spawn, int playerId)
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(
			GetGame().GetPlayerManager().GetPlayerController(playerId));
		if (!pc)
			return;

		// Somebody else has taken this controller over (Game Master, a future feature). Do not
		// fight over it — same refusal, same reasoning, as `TBD_SpectatorHost.EnsureHost`.
		if (pc.IsPossessing())
		{
			NoteIssueRefused(playerId, "the player controller is already possessing something else — not fighting over it");
			return;
		}

		vector cell = CellFor(playerId);

		IEntity ghost = SpawnGhostEntity(cell);
		if (!ghost)
		{
			NoteIssueRefused(playerId, "the ghost entity would not spawn");
			return;
		}

		// THE KEYSTONE. Checked BEFORE the player is given it, and the candidate is destroyed rather
		// than used if it fails — see the ONE LIFE block in the file header.
		if (!IsAcceptableGhost(ghost, playerId))
		{
			SCR_EntityHelper.DeleteEntityAndChildren(ghost);
			StandDownAfterImpossibleRefusal();
			return;
		}

		// DIRECT ASSIGNMENT, never a spawn request. Both oracles converged on this independently and
		// CRF wrote down why (`CRF_PlayerHelper.c:30-39`): vanilla's `AssignEntity_S`
		// (`vanilla_reference/Source/SCR_SpawnHandlerComponent.c:265-268`) abandons the entire
		// finalize when the controller already controls the target, so a request-shaped assignment is
		// silently unreliable. It would also be wrong on its own terms here — a spawn request is a
		// SPAWN, and `TBD_SCR_PossessSpawnHandlerComponent` would (correctly) refuse one this manager
		// never authorised. The ghost is an anchor, not a life.
		pc.SetInitialMainEntity(ghost);

		// Calling the setter is not evidence that it took. Read it back — the same discipline
		// `TBD_SpectatorHost.EnsureHost` and `TBD_SafestartManager.RestoreOne` both apply.
		IEntity controlled = GetGame().GetPlayerManager().GetPlayerControlledEntity(playerId);
		if (controlled != ghost)
		{
			NoteIssueRefused(playerId, "the engine did not transfer control to the ghost — rolled back, the player stays bodyless");
			GetGame().GetCallqueue().Call(DeleteGhostNextFrame, ghost);
			return;
		}

		// A ghost landed, so whatever was being suppressed is over. The next genuine problem gets
		// its own line instead of being swallowed by a latch set half an hour ago.
		s_bIssueRefusedLogged = false;

		TBD_PreSlotGhostRecord record = new TBD_PreSlotGhostRecord();
		record.epoch = spawn.ConnectionEpochFor(playerId);
		record.ghost = ghost;
		record.cell = cell;
		s_mGhosts.Set(playerId, record);

		// Built in two appended steps on purpose: a single long format chain is the measured
		// "Formula too complex" landmine in this codebase.
		string line = string.Format("[TBD][PreSlot] player=%1 ghost ISSUED at %2", playerId, ghost.GetOrigin().ToString());
		line = line + string.Format(" epoch=%1 replicated=%2 (no slot, no body, no life spent)", record.epoch, IsReplicated(ghost));
		PrintFormat("%1", line);

		if (!s_bReplicationReported)
		{
			s_bReplicationReported = true;
			ReportReplication(ghost);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Where this player's ghost stands.
	//!
	//! A lattice of one-cell-per-player, high above the world. PlayableSelector does the same thing
	//! (`PS_GameModeCoop.c:733`) and their reason is CONCRETE: their ghost is a real character
	//! carrying a live VoN component and two live radios, so co-located ghosts would bleed
	//! everyone's positional voice together. OURS CARRIES NONE OF THAT — no VoN, no radio, no sound
	//! component, no mesh, no physics, no collision — so honestly, the separation buys us very
	//! little on its own and is kept as belt and braces: two ghosts at one point would be
	//! indistinguishable to any future feature that ever queries by position, and that is a cheap
	//! thing to rule out.
	//!
	//! The ALTITUDE is the part that earns its place, and it is theirs: ~100 km up is somewhere
	//! nothing on the ground can see, shoot or collide with, which is a claim about the WORLD rather
	//! than about our component list. It is the second, independent reason the ONE LIFE argument in
	//! the header holds.
	//!
	//! Deliberately NOT clamped to the world bound box — `TBD_SpectatorHost.ClampToWorld` exists
	//! because a CLIENT asks for that position; nobody asks for this one, and clamping would drop
	//! every ghost back into the playable world, which is the one thing this must not do.
	protected static vector CellFor(int playerId)
	{
		int index = playerId;
		if (index < 0)
			index = -index;

		// MEASURED: the lattice indices must be resolved as INTs and only then widened. Written as
		// `float column = index % CELL_ROW;` the compiler types the whole expression from the
		// assignment target and rejects it with "Unknown operator '%'" — an int-only operator in
		// this dialect, and the diagnostic does not say so.
		int column = index % CELL_ROW;
		int row = index / CELL_ROW;

		float x = column * CELL_PITCH_M;
		float z = row * CELL_PITCH_M;

		return Vector(x, CELL_ALTITUDE_M, z);
	}

	//------------------------------------------------------------------------------------------------
	//! Build the ghost.
	//!
	//! BY TYPENAME with no prefab, which is what lets this work at all today: a new `.et` is
	//! invisible to the engine until Workbench rewrites `resourceDatabase.rdb`, and this mod ships a
	//! snapshot that no slice agent can regenerate. `TBD_SpectatorCamera` and `TBD_SpectatorHost`
	//! already take exactly this route and it is probed
	//! (`GetGame().SpawnEntity(<scripted class>, world, EntitySpawnParams)`).
	//!
	//! THE COST, stated plainly because a live test is the only thing that can settle it: a
	//! typename-spawned entity has NO `RplComponent`, so it exists on the SERVER only. That is
	//! sufficient for everything this file claims to fix (the server's view of
	//! `GetPlayerControlledEntity`), and it is NOT sufficient for anything that needs the CLIENT to
	//! see a controlled entity. This is why the black screen is fixed by `TBD_PreSlotCamera` on the
	//! client rather than by this ghost — the camera needs nothing replicated. `ReportReplication`
	//! puts the answer in the log on the first live run either way.
	//! @authority server
	protected static IEntity SpawnGhostEntity(vector position)
	{
		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return null;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = position;

		return GetGame().SpawnEntity(TBD_SpectatorHostEntity, world, params);
	}

	//------------------------------------------------------------------------------------------------
	//! ONE LIFE, enforced on the OBJECT rather than on the paths that reach it.
	//!
	//! Identical in shape and intent to `TBD_SpectatorHost.IsAcceptableHost`, and duplicated rather
	//! than shared on purpose: that function is `protected` inside a class this slice does not own,
	//! and a guard that another lane can weaken without noticing is not a guard. Whatever calls
	//! whatever, the thing a slot-less player ends up controlling has no way to be hurt, to die, or
	//! to be stood back up.
	//! @authority server
	protected static bool IsAcceptableGhost(notnull IEntity ghost, int playerId)
	{
		string refusal;

		if (ChimeraCharacter.Cast(ghost))
			refusal = "it is a ChimeraCharacter — a character can be killed, and a killed character spends a life";
		else if (ghost.FindComponent(DamageManagerComponent))
			refusal = "it carries a DamageManagerComponent — a waiting player would be controlling something that can be damaged, destroyed or healed, which is a death path under ONE LIFE";
		else if (ghost.FindComponent(CharacterControllerComponent))
			refusal = "it carries a CharacterControllerComponent — that is a playable body, not an anchor";

		if (refusal.IsEmpty())
			return true;

		Print(string.Format("[TBD][PreSlot] player=%1 ghost candidate REFUSED: %2", playerId, refusal), LogLevel.ERROR);
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! The built-in ghost cannot fail acceptance — `TBD_SpectatorHostEntity` is a bare
	//! `GenericEntity` with no components at all — so if it ever does, something is wrong at a level
	//! this file cannot reason about. Stand the feature down rather than spawning and deleting an
	//! entity once a second for the rest of the event, which would be a worse failure than the one
	//! being reported. (Removing a repeating call from inside the call itself is the idiom
	//! `TBD_SafestartManager.TickSweep` and `TBD_SpectatorHost.RecoverFromUnacceptableHost` already
	//! use.)
	//! @authority server
	protected static void StandDownAfterImpossibleRefusal()
	{
		Print("[TBD][PreSlot] the built-in pre-slot ghost was REFUSED, which should be impossible — standing the pre-slot ghost down for this round rather than churning entities", LogLevel.ERROR);

		GetGame().GetCallqueue().Remove(Tick);
		ReleaseAll("pre-slot ghost stood down after an impossible refusal");
		s_bRunning = false;
	}

	//------------------------------------------------------------------------------------------------
	//! Say once why a bodyless player is not getting a ghost, then be quiet until one is issued.
	//! The reconcile retries every second and a persistent cause would otherwise write one line per
	//! second for the whole event.
	//! @authority server
	protected static void NoteIssueRefused(int playerId, string reason)
	{
		if (s_bIssueRefusedLogged)
			return;

		s_bIssueRefusedLogged = true;

		string line = string.Format("[TBD][PreSlot] player=%1 has NO pre-slot ghost — %2.", playerId, reason);
		line = line + " They stay bodyless until they deploy. (Latched: one line per round until a ghost is issued.)";
		PrintFormat("%1", line, level: LogLevel.WARNING);
	}

	//------------------------------------------------------------------------------------------------
	//! Is this ghost visible to the network at all? Logged once per world so the first live run
	//! answers the one question the compile lane structurally cannot.
	protected static bool IsReplicated(notnull IEntity ghost)
	{
		RplComponent rpl = RplComponent.Cast(ghost.FindComponent(RplComponent));
		if (!rpl)
			return false;

		return rpl.Id().IsValid();
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected static void ReportReplication(notnull IEntity ghost)
	{
		if (IsReplicated(ghost))
		{
			Print("[TBD][PreSlot] the pre-slot ghost IS replicated — a client's GetControlledEntity() should be non-null while waiting for a slot.");
			return;
		}

		string line = "[TBD][PreSlot] the pre-slot ghost is SERVER-ONLY (typename-spawned, no RplComponent).";
		line = line + " The server's GetPlayerControlledEntity() is satisfied; a CLIENT's GetControlledEntity() is still null while waiting.";
		line = line + " That is why TBD_PreSlotCamera does not depend on it. If a client-visible body is ever required, the fix is a prefab whose root class is TBD_SpectatorHostEntity plus a Workbench pass on resourceDatabase.rdb.";
		PrintFormat("%1", line);
	}

	// ── Release ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Drop this player's ghost. Idempotent, safe for a player who has none, and safe to call while
	//! the engine is tearing that player down.
	//!
	//! Note what this deliberately does NOT do: it never touches `SetInitialMainEntity`. There is no
	//! vanilla call that un-sets a main entity (`SetInitialMainEntity` takes `notnull IEntity` — the
	//! negative-control probe confirmed `ClearInitialMainEntity` does not exist), and there is
	//! nothing to give back anyway: a player holding a ghost had no body before it. Deleting the
	//! entity is the whole release, and it happens NEXT FRAME for the reason
	//! `DeleteGhostNextFrame` states.
	//!
	//! Returns true when a ghost was actually released, so callers can say so in one line.
	//! @authority server
	static bool ReleaseFor(int playerId, string reason)
	{
		if (!s_mGhosts)
			return false;

		TBD_PreSlotGhostRecord record;
		if (!s_mGhosts.Find(playerId, record))
			return false;

		s_mGhosts.Remove(playerId);

		if (!record)
			return false;

		if (record.ghost)
		{
			IEntity ghost = record.ghost;
			record.ghost = null;
			GetGame().GetCallqueue().Call(DeleteGhostNextFrame, ghost);
		}

		Print(string.Format("[TBD][PreSlot] player=%1 ghost RELEASED — %2", playerId, reason));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.50 — the deploy courtesy, called from `TBD_SpawnManager.DeployPlayerInternal` at the
	//! point of no return.
	//!
	//! THE ORDERING IS THE POINT, and it is PlayableSelector's (`PS_PlayableManager.c:258-275`):
	//! possession of the REAL body happens first and synchronously, and the ghost's deletion is
	//! queued behind it, so the player is never controller-less. Reversing the two — releasing the
	//! ghost before the body lands — is exactly the failure this design avoids, and on our path it
	//! would be worse than on theirs, because our assignment goes through vanilla's POSSESS request
	//! and is therefore ASYNCHRONOUS: there is a real window between `RequestRespawn` returning true
	//! and the finalize assigning the body.
	//!
	//! So this does not delete anything by itself. It marks the record, and `RetireStaleGhosts`
	//! collects it on the next pass once it can SEE the player controlling something else. If the
	//! deploy silently never lands, the ghost correctly stays — a bodyless player with an anchor is
	//! the state this whole file exists to produce.
	//! @authority server
	static void NoteDeployRequested(int playerId)
	{
		if (!s_mGhosts)
			return;

		TBD_PreSlotGhostRecord record;
		if (!s_mGhosts.Find(playerId, record) || !record)
			return;

		record.deployRequested = true;
		Print(string.Format("[TBD][PreSlot] player=%1 deploy requested — ghost will be retired as soon as the real body lands", playerId));
	}

	//------------------------------------------------------------------------------------------------
	//! NEXT FRAME, not now.
	//!
	//! Deleting an entity the engine still lists as somebody's controlled entity is the hazard
	//! `TBD_SpectatorHost.DeleteHostNextFrame` documents, and here it is the NORMAL case rather than
	//! an edge: a released ghost is by definition the thing the player was controlling, and unlike
	//! the spectator host there is no previous body for the engine to fall back to. CRF answered the
	//! identical problem the identical way (`CRF_SpectatorCharacter.OnControlledByPlayer` defers its
	//! own delete a frame, with a comment about not upsetting the player controller) — read as an
	//! oracle, not copied.
	//!
	//! The argument is an ENTITY, not a playerId, so this deferred call carries none of the
	//! recycled-id hazard that made T-181.15 stamp epochs on everything else in the queue: there is
	//! no player to mistake, only an object to free.
	//! @authority server
	protected static void DeleteGhostNextFrame(IEntity ghost)
	{
		if (!ghost)
			return;

		SCR_EntityHelper.DeleteEntityAndChildren(ghost);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected static void ReleaseAll(string reason)
	{
		if (!s_mGhosts)
			return;

		array<int> holders = {};
		foreach (int playerId, TBD_PreSlotGhostRecord record : s_mGhosts)
		{
			holders.Insert(playerId);
		}

		for (int i = 0; i < holders.Count(); i++)
		{
			ReleaseFor(holders[i], reason);
		}
	}
}

//! One player's pre-slot ghost. Deliberately the same shape as `TBD_SpectatorHostRecord`: the two
//! solve the same problem at two different moments in a player's round.
class TBD_PreSlotGhostRecord
{
	//! The connection epoch this ghost was opened under (T-181.15). A record whose epoch has moved
	//! on belongs to somebody who has left, and their number may already have been handed to a
	//! stranger.
	int epoch;

	//! The ghost itself. A plain (non-`ref`) handle: the world owns entities, exactly as
	//! `TBD_SpawnManager.m_mSlotBodies` and `TBD_SpectatorHostRecord.host` do.
	IEntity ghost;

	//! Where it was put. Kept for the log and for a live probe to compare against.
	vector cell;

	//! Set by `NoteDeployRequested`. Diagnostic only — the retirement decision is made by observing
	//! the controlled entity, never by trusting this flag, because a deploy can be refused after it
	//! was requested.
	bool deployRequested;
}
