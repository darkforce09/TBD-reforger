/**
 * @file TBD_IdentityLinkPending.c
 * @brief One queued `#tbd link <code>` confirm request.
 *
 * Role: carries everything needed to send the confirm and answer the player.  Position: built by
 * `TBD_IdentityLink.Submit`, queued and consumed by `TBD_IdentityLinkConfirm`.
 * State: plain data, owned by the confirm queue on the server.  Invariants: every field is
 * stamped at enqueue; none is re-derived after the player disconnects.
 */

//! One queued confirm request, captured at enqueue because nothing is re-derivable after the
//! player disconnects.
class TBD_IdentityLinkPending
{
	int playerId; //!< the typing player's id; also addresses the asynchronous replies
	string armaId; //!< JSON key `arma_id`; stamped at enqueue so a disconnect mid-flight keeps a valid link
	string armaCharacter; //!< JSON key `arma_character`; display name, may be empty
	string code; //!< JSON key `code`; the trimmed link code, never logged
}
