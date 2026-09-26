/**
 * @file TBD_PlayerIdentity.c
 * @brief The single accessor for the `arma_id` this mod puts on every backend wire.
 *
 * Role: resolves a player's engine identity, formatted exactly as it goes on the wire, and says
 * whether it is durable.  Position: called by `TBD_IdentityLink` (`#tbd link <code>`, which
 * POSTs `/api/v1/ingest/link-confirm`, the only writer of `users.arma_id` besides the dev seed;
 * link-confirm SHIPS), by `TBD_ResultsReporter` (`/api/v1/ingest/match-results`, which joins on
 * `users.arma_id`), and by deployment authorisation and relays.
 * State: none; uncached, so "cannot see who this is" is never answered with a stale guess.
 * Invariants: both backend halves call `GetArmaId` and never resolve the identity themselves, so
 * the join compares identical bytes; a changed shape changes here for both. No identity returns
 * EMPTY, never a `player:<id>` seat lease (that fallback belongs to `TBD_SpawnIdentityKeys.PlayerBindKey`,
 * which bookkeeps one life on a key that must never be empty); callers drop such a player. An
 * ENGINE-resolved identity is still not a LINKED one: a player who never runs `#tbd link <code>`
 * has no `users.arma_id`, so match-results can return 200 while their rows match nobody.
 */

//! Engine identity accessor shared by every backend payload.
//! @authority server
class TBD_PlayerIdentity
{
	protected static const string SYNTHETIC_PREFIX = "00bbbddd-"; //!< prefix `SCR_PlayerIdentityUtils` stamps on an identity synthesized from the display name

	//! The player's engine identity, formatted exactly as it goes on the wire, or EMPTY when this
	//! host issues none.
	//!
	//! Three cases, and only the first is acceptable for a real event:
	//!   1. BACKEND identity -- a correctly configured dedicated server. Durable. Returned.
	//!   2. SYNTHESIZED `00bbbddd-...` name hash -- listen / hosted / local host only; vanilla only
	//!      synthesizes when `RplSession.Mode() != RplMode.Dedicated`. Returned as-is so the two
	//!      halves of the contract can never disagree, but `IsDurable()` reports it false and the
	//!      caller is expected to say so out loud. A name change makes a new "person"; two players
	//!      sharing a name are one person.
	//!   3. NO identity -- misconfigured dedicated server, or a player mid-teardown. Returns EMPTY.
	//!      Callers MUST drop the player rather than substitute anything.
	//!
	//! `UUID.IsNull()` is the emptiness test: a null UUID formats to the same non-empty constant
	//! for everybody, so `string.Format(...).IsEmpty()` would collapse every player onto one key.
	//! @return the identity, or empty
	//! @authority server
	static string GetArmaId(int playerId)
	{
		UUID identity = SCR_PlayerIdentityUtils.GetPlayerIdentityId(playerId);
		if (identity.IsNull())
			return string.Empty;

		return string.Format("%1", identity);
	}

	//! Whether `armaId` is a name-hash identity (case 2 of `GetArmaId`); not an error, just not a person.
	//! @return true for a synthesized identity
	static bool IsSynthetic(string armaId)
	{
		return armaId.StartsWith(SYNTHETIC_PREFIX);
	}

	//! Whether `armaId` still means the same human next session: a real backend uuid. The test to
	//! gate anything persisted against.
	//! @return true for a non-empty, non-synthesized identity
	static bool IsDurable(string armaId)
	{
		return !armaId.IsEmpty() && !IsSynthetic(armaId);
	}
}
