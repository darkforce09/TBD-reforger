/**
 * @file TBD_Authority.c
 * @brief Answers which side of the replication session this machine is on.
 *
 * Role: the one place scripts ask "am I a client?" or "am I the authority?".  Position: read by
 * every framework script that gates work on where it runs; reads only `RplSession.Mode()`.
 * State: none.  Invariants: `IsClient()` and `IsServer()` are exact complements; a dedicated
 * server, a listen host and a single-player or Workbench session are all the authority.
 */

//! Replication-side predicates over `RplSession.Mode()`.
class TBD_Authority
{
	//! True when this machine is a remote client of a server (`RplMode.Client`). Callable on
	//! every machine; the answer is the client side of the question. Never fails.
	//! @authority client
	static bool IsClient()
	{
		return RplSession.Mode() == RplMode.Client;
	}

	//! True when this machine holds authority: a dedicated server, a listen host, or a session
	//! with no replication (single-player, Workbench play). The exact complement of `IsClient`;
	//! callable on every machine. Never fails.
	//! @authority server
	static bool IsServer()
	{
		return RplSession.Mode() != RplMode.Client;
	}
}
