/**
 * @file TBD_PlayerFaction.c
 * @brief Resolves the mission faction key a player's assigned slot belongs to.
 *
 * Role: maps a player id to the `factions[].key` of their assigned slot.  Position: called by
 * the objective and play-area components; reads `TBD_SpawnManager.GetAssignedSlot`.
 * State: none.  Invariants: an unassigned player, or no spawn manager, yields the empty string,
 * which callers treat as "on no side" rather than as an error.
 */

//! Player-to-faction lookup over the slot assignment.
class TBD_PlayerFaction
{
	//! The faction key of `playerId`'s assigned slot.
	//! @param spawn the spawn manager holding slot assignments; may be null
	//! @param playerId the player to resolve
	//! @return the slot's `faction`, or empty when there is no manager or no assigned slot
	//! @authority server
	static string Of(TBD_SpawnManager spawn, int playerId)
	{
		if (!spawn)
			return string.Empty;

		TBD_MissionSlotStruct slot = spawn.GetAssignedSlot(playerId);
		if (!slot)
			return string.Empty;

		return slot.faction;
	}
}
