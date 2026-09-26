/**
 * @file TBD_SpectatorHostRecord.c
 * @brief What the server remembers about one spectator's streaming host.
 *
 * Role: holds the connection epoch, the host entity, the anchor and the last applied position of
 * one dead player's host.
 * Position: TBD_SpectatorHostLifecycle.EnsureHost creates it; TBD_SpectatorHost stores it in its
 * host map, and MoveTo and ReleaseFor read and update it.
 * State: plain data, server only, never replicated.
 * Invariants: a record whose epoch differs from the player's connection belongs to somebody who
 * has left; the host handle is weak because the world owns entities.
 */

//! One spectator's streaming host, as the authority remembers it.
class TBD_SpectatorHostRecord
{
	int epoch; //!< connection epoch the host was opened under; a mismatch means the id was recycled
	IEntity host; //!< the possessed host entity; a weak handle, the world owns it
	vector anchor; //!< world metres; where the player died, the range leash origin and the first position
	vector applied; //!< world metres; last position applied, so a stationary camera costs no teleport
}
