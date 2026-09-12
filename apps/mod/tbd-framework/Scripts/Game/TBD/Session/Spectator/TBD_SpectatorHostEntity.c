//! T-181.24 — the spectator STREAMING HOST entity: the dummy a dead player possesses so the
//! server keeps sending them a world to look at.
//!
//! ── WHY THIS EXISTS ─────────────────────────────────────────────────────────────────────────
//! Network streaming in Reforger is anchored to the player's CONTROLLED ENTITY, not to their
//! camera. Under ONE LIFE the dead player still controls their corpse, so their replication
//! origin is pinned to the spot where they fell — fly the free camera a kilometre away and the
//! world is empty, not because the camera is broken but because those entities were never sent
//! to that machine. `TBD_SpectatorController` has carried that landmine as a comment since
//! T-181.12; this is the fix it named.
//!
//! ── WHY IT IS *NOT* A CHARACTER, AND WHY THAT IS THE WHOLE SAFETY ARGUMENT ───────────────────
//! CRF solves the same problem with a real character prefab (`CRF_SpectatorCharacter`, a
//! `CRF_PlayerCharacter` with physics and damage switched off at `EOnInit`). Read as an oracle,
//! rejected as a design, for reasons that are all ONE LIFE reasons:
//!
//!   * A character can be KILLED. `SCR_BaseGameMode.OnPlayerKilled` fires off character death, so
//!     a character dummy is one stray bullet away from spending a life its owner already spent —
//!     or, worse, from being the thing a future "you died" path reacts to.
//!   * A character is re-armed by SAFESTART. `TBD_SafestartManager.CollectProtectables` sweeps
//!     `PlayerManager.GetPlayerControlledEntity` (which IS the dummy once it is possessed) and
//!     `Restore` then forces `EnableDamageHandling(true)` on everything it held — it does NOT
//!     restore each body's prior value. A character dummy therefore comes OUT of safestart
//!     damageable no matter what we set at spawn. A dummy with no damage manager cannot: vanilla
//!     has nothing to enable, and `RestoreOne` early-returns `true` on `!damage`.
//!   * A character reads as ALIVE to everything that asks. `TBD_SpectatorTargets.IsAlive`, the
//!     `[TBD][Audit] characters=` census in `TBD_SpawnManager.RunCensus`, and every future roster
//!     would all count a dead player's dummy as a living body.
//!   * A character is a body, and a body is a life. The invariant is that `AdminRespawn` is the
//!     only door back into the world; the safest dummy is the one that could not be walked
//!     through even if every guard were deleted.
//!
//! So the host is a bare `GenericEntity`: no damage manager, no character controller, no weapon,
//! no inventory, no mesh. `TBD_SpectatorHost.IsAcceptableHost` REFUSES to possess anything that
//! is a `ChimeraCharacter` or carries a `DamageManagerComponent`, which turns all of the above
//! from "we were careful" into "it cannot happen", and keeps an operator who points
//! `m_sHostPrefab` at a character prefab from quietly reintroducing a second door.
//!
//! ── MEASURED (probe /tmp/probe-t18124-a + negative control, 2026-07-25) ─────────────────────
//!   * `GetGame().SpawnEntity(TBD_SpectatorHostEntity, world, EntitySpawnParams)` compiles for a
//!     `GenericEntity` subclass — the same prefab-free, `resourceDatabase.rdb`-free route
//!     `TBD_SpectatorCamera` already uses. The negative control (`GetGame().SpawnStreamingHost`,
//!     `SCR_PlayerController.SetStreamingAnchorEntity`) failed with `Undefined function`, so the
//!     green result means something.
//!   * `SetOrigin` / `SetWorldTransform` on the spawned entity compile.
//!
//! MEASURED: this descriptor needs the trailing `;` — the same parser quirk
//! `TBD_SpectatorCameraClass` documents. Omit it and the NEXT class fails with a misleading
//! "Syntax error / Unexpected scope".
[EntityEditorProps(category: "TBD/Spectator", description: "TBD spectator streaming host — an inert, damage-free anchor a dead player possesses so the server keeps streaming the world around their camera.")]
class TBD_SpectatorHostEntityClass : GenericEntityClass {};

//! An inert anchor. It has no behaviour on purpose: every decision lives in `TBD_SpectatorHost`.
class TBD_SpectatorHostEntity : GenericEntity
{
	//------------------------------------------------------------------------------------------------
	void TBD_SpectatorHostEntity(IEntitySource src, IEntity parent)
	{
		// ACTIVE for the same reason TBD_SpectatorCamera sets it: an entity the engine considers
		// dormant is not something to be relying on as a streaming origin. No event mask — this
		// entity never ticks; the server teleports it and nothing else.
		SetFlags(EntityFlags.ACTIVE, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Is this entity a spectator streaming host?
	//!
	//! ONE Cast, used by every caller that must not mistake a host for a body — the client-side
	//! `TBD_SpectatorTargets.IsAlive` (so a host can never be followed, and so a spectator whose own
	//! controlled entity became the host is not mistaken for a player who came back to life) and
	//! `TBD_SpectatorTargets.Collect` (so a host never appears in anybody's roster).
	//!
	//! It is a class test rather than a component test so that it covers BOTH ways a host can come
	//! into being: spawned by typename with no prefab, or spawned from a prefab whose root class is
	//! this one (`m_sHostPrefab`).
	static bool IsHost(IEntity entity)
	{
		if (!entity)
			return false;

		TBD_SpectatorHostEntity host = TBD_SpectatorHostEntity.Cast(entity);
		if (host)
			return true;

		return false;
	}
}
