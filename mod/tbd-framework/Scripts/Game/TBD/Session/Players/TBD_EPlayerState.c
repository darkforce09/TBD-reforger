/**
 * @file TBD_EPlayerState.c
 * @brief Where a connected player stands: slotted, spectating or unslotted.
 *
 * Role: the state a TBD_PlayerInfo carries.
 * Position: TBD_PlayersMock fills it; TBD_PlayersCatalog and TBD_PlayersPanel group by it.
 * State: none.
 * Invariants: every player is in exactly one state.
 */

//! A connected player's state in the players catalog.
enum TBD_EPlayerState
{
	SLOTTED, //!< holds a slot on a faction
	SPECTATOR, //!< spectating
	UNSLOTTED //!< connected without a slot
}
