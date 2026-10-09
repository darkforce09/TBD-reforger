/**
 * @file TBD_SpectatorHostEntity.c
 * @brief The inert, damage-free entity a dead player possesses so the server keeps streaming the world around their camera.
 *
 * Role: an anchor with no behaviour. Streaming follows the controlled entity, not the camera, so
 * a dead player who still controlled their corpse would see an empty world away from it; possessing
 * this entity, which the server moves to the camera, carries the streaming origin with the view.
 * Position: TBD_SpectatorHostFactory spawns it by type name (or from a prefab whose root class is
 * this one); TBD_SpectatorHost moves and deletes it; TBD_SpectatorTargets.IsAlive and Collect use
 * IsHost so a host is never read as a living body or listed in a roster.
 * State: none.
 * Invariants: a bare GenericEntity with no damage manager, character controller, weapon, inventory
 * or mesh, so it cannot be killed, healed, armed or stood up as a body, and the safestart hold
 * leaves it untouched; TBD_SpectatorHostFactory.IsAcceptableHost refuses any candidate that breaks
 * this; it is not a character, so the kill census and every roster never count it as alive.
 */

//! Editor descriptor of TBD_SpectatorHostEntity. The trailing semicolon is required: without it
//! the parser fails on the next class with "Syntax error / Unexpected scope".
[EntityEditorProps(category: "TBD/Spectator", description: "TBD spectator streaming host -- an inert, damage-free anchor a dead player possesses so the server keeps streaming the world around their camera.")]
class TBD_SpectatorHostEntityClass : GenericEntityClass {};

//! An inert anchor. It has no behaviour on purpose: every decision lives in `TBD_SpectatorHost`.
class TBD_SpectatorHostEntity : GenericEntity
{
	//! Mark the entity ACTIVE, since a dormant entity is not a reliable streaming origin. No event
	//! mask: the entity never ticks, the server moves it and nothing else.
	//! @param src the entity source, null when spawned by type name
	//! @param parent the parent entity, normally null
	void TBD_SpectatorHostEntity(IEntitySource src, IEntity parent)
	{
		SetFlags(EntityFlags.ACTIVE, true);
	}

	//! Is this entity a spectator streaming host? TBD_SpectatorTargets.IsAlive uses it so a host is
	//! never followed and a spectator whose controlled entity became a host is not read as alive
	//! again; TBD_SpectatorTargets.Collect uses it so a host never appears in a roster. A class test,
	//! so it covers a host spawned by type name and one spawned from a prefab of this class.
	//! @param entity the entity to test; null is not a host
	//! @return true when the entity is a TBD_SpectatorHostEntity
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
