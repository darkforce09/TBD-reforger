/**
 * @file TBD_EAdminAction.c
 * @brief The admin powers the admin screen can ask the server to run.
 *
 * Role: names each menu power as one enum value.  Position: TBD_AdminScreen sends one through
 * TBD_AdminClient.Act; the player controller carries it as a plain int; TBD_AdminService.FromWire
 * turns the int back into a value and TBD_AdminService.Execute runs it.
 * State: none.  Invariants: NONE is 0, so an unset int is inert; a wire int outside the members
 * reads as NONE.
 */

//! The powers the admin surfaces expose. The values cross the wire as ints.
enum TBD_EAdminAction
{
	NONE, //!< 0: no action; Execute answers "unknown admin action"
	RESPAWN, //!< put a player who has spent their one life back in the world
	DEPLOY, //!< put a player who still has their life but has no body into the world
	STAGE_ADVANCE //!< force the round's stage machine one step forward
}
