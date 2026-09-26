/**
 * @file TBD_CharacterUtil.c
 * @brief Character body predicates shared by the zone, objective and spawn systems.
 *
 * Role: answers whether a body in the world is a corpse.  Position: called by the objective,
 * play-area, trigger and spawn systems on bodies they already hold; reads the engine's
 * `CharacterControllerComponent`.
 * State: none.  Invariants: never throws on a null or non-character entity; the caller chooses
 * whether an unreadable body counts as dead.
 */

//! Stateless character-body queries.
class TBD_CharacterUtil
{
	//! Whether `body` is a dead character. A body that cannot be read -- null, not a
	//! `ChimeraCharacter`, or carrying no character controller -- returns `missingCountsAsDead`:
	//! false for presence checks (an unreadable body neither holds nor leaves ground), true for the
	//! spawn path (an unreadable body is rebuilt rather than handed on).
	//! @param body the entity to test; may be null
	//! @param missingCountsAsDead the answer for a body that cannot be read
	//! @return the controller's `IsDead()`, or `missingCountsAsDead`
	static bool IsDead(IEntity body, bool missingCountsAsDead = false)
	{
		if (!body)
			return missingCountsAsDead;

		ChimeraCharacter character = ChimeraCharacter.Cast(body);
		if (!character)
			return missingCountsAsDead;

		CharacterControllerComponent controller = character.GetCharacterController();
		if (!controller)
			return missingCountsAsDead;

		return controller.IsDead();
	}
}
