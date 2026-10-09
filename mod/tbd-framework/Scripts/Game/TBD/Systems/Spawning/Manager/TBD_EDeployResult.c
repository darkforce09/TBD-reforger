/**
 * @file TBD_EDeployResult.c
 * @brief The answer of one deploy attempt through TBD_SpawnManager.
 *
 * Role: names every outcome a deploy can have.  Position: returned by TBD_DeployExecutor and the
 * TBD_SpawnManager deploy entry points; read by the lobby, admin, spectator and vanilla bridge callers.
 * State: none.  Invariants: only NOT_MINE lets the vanilla spawn path run; every other value means
 * the framework owns this player and vanilla stands down.
 */

//! Result of a deploy attempt. Only NOT_MINE may reach the vanilla spawn path.
enum TBD_EDeployResult
{
	DEPLOYED, //!< bound to the slot body in this call
	ALREADY, //!< the player is already bound
	RETRY, //!< a transient precondition (bodies, roster, controller) is missing; retry shortly
	FAILED, //!< a permanent failure (kit resolve, body spawn, LOBBY stage); logged, no vanilla body
	NOT_MINE, //!< client side, or no framework mission; vanilla may handle it
	DENIED, //!< refused by policy: the one life is spent or a slot body is unplayable; never retried
	AUTHORIZING, //!< the platform decides this event seat; its decision continues or refuses the deploy
	UNAUTHORIZED, //!< the event seat cannot be authorized now; the player was told why and keeps the seat
}
