//! T-181.13.1 — THE single accessor for the `arma_id` this mod puts on the wire.
//!
//! ══ WHY THIS IS ITS OWN FILE ═══════════════════════════════════════════════════════════════
//! Two halves of one contract have to agree on a string, byte for byte, or the whole identity
//! system silently matches nobody, forever, with no error anywhere:
//!
//!   * `POST /api/v1/ingest/link-confirm` (`apps/website/api/src/handlers/me.rs:160-205`) is the
//!     GAME SERVER confirming a player's link code. It is the ONLY thing besides the dev seed that
//!     ever writes `users.arma_id`. **The mod does not implement it yet — that is T-181.35.**
//!   * `POST /api/v1/ingest/match-results` (`apps/website/api/src/handlers/telemetry.rs:215`)
//!     resolves each player with `SELECT discord_id FROM users WHERE arma_id = $1`
//!     (telemetry.rs:238). That is `TBD_ResultsReporter`, this slice.
//!
//! A join on a string is a join on a string: if T-181.35 sends the engine identity and this sends
//! anything else — a cached value, a lower-cased value, a `player:<id>` lease — the match returns
//! zero rows and the backend cheerfully reports success. Nothing logs. Nothing 500s. Attendance,
//! the user-stat recompute and the leaderboard refresh all just do nothing.
//!
//! So there is ONE function, `GetArmaId`, and both halves call it. **T-181.35 must not resolve the
//! identity itself.** If it needs a different shape (trimmed, prefixed, whatever), the change goes
//! HERE so both halves move together.
//!
//! ══ WHY NOT REUSE TBD_SpawnManager.PlayerBindKey ═══════════════════════════════════════════
//! `PlayerBindKey` (TBD_SpawnManager.c) answers a different question and has a deliberately
//! different failure mode. It must ALWAYS return a non-empty key because ONE LIFE is bookkept on
//! it, so when the engine issues no identity it falls back to a cached value and finally to
//! `player:<numeric id>` — a SEAT NUMBER, which dedicated servers recycle.
//!
//! That fallback is exactly right for one-life bookkeeping and exactly wrong for a backend
//! identity. Writing `player:7` into `users.arma_id` would bind a Discord account to whoever
//! occupies connection slot 7 next week. So this accessor returns EMPTY instead, and its callers
//! are required to drop the player from the payload rather than invent an identity for them.
//! It is also uncached on purpose: for the backend, "I cannot see who this is right now" must not
//! be answered with a stale guess.
//!
//! @authority server — identity is a server-side lookup; a client has no business resolving it.
class TBD_PlayerIdentity
{
	//! Vanilla stamps a SYNTHESIZED identity with this prefix (`SCR_PlayerIdentityUtils.c:33` —
	//! `string.Format("00bbbddd-%1-%2-%3-%4%5", …)` over a hash of the player's display NAME). It
	//! is the one reliable way to tell a real backend uuid from a name hash.
	protected static const string SYNTHETIC_PREFIX = "00bbbddd-";

	//------------------------------------------------------------------------------------------------
	//! The player's engine identity, formatted exactly as it goes on the wire, or EMPTY when this
	//! host issues none.
	//!
	//! Three cases, and only the first is acceptable for a real event:
	//!   1. BACKEND identity — a correctly configured dedicated server. Durable. Returned.
	//!   2. SYNTHESIZED `00bbbddd-…` name hash — listen / hosted / local host only; vanilla only
	//!      synthesizes when `RplSession.Mode() != RplMode.Dedicated`. Returned as-is so the two
	//!      halves of the contract can never disagree, but `IsDurable()` reports it false and the
	//!      caller is expected to say so out loud. A name change makes a new "person"; two players
	//!      sharing a name are one person.
	//!   3. NO identity — misconfigured dedicated server, or a player mid-teardown. Returns EMPTY.
	//!      Callers MUST drop the player rather than substitute anything.
	//!
	//! Proven, not assumed (T-181.13.1 compile probe; negative control
	//! `SCR_PlayerIdentityUtils.GetPlayerIdentityIdZZ` -> `Undefined function`):
	//! `GetPlayerIdentityId(int)` returns a `UUID`, `UUID.IsNull()` exists, and
	//! `string.Format("%1", uuid)` compiles. `IsNull()` is the correct emptiness test and
	//! `string.Format(...).IsEmpty()` is NOT: a null UUID formats to the same non-empty constant for
	//! everybody, which once collapsed every player onto one key (see TBD_SpawnManager.PlayerBindKey).
	static string GetArmaId(int playerId)
	{
		UUID identity = SCR_PlayerIdentityUtils.GetPlayerIdentityId(playerId);
		if (identity.IsNull())
			return string.Empty;

		return string.Format("%1", identity);
	}

	//------------------------------------------------------------------------------------------------
	//! True for a name-hash identity (case 2 above). Not an error — just not a person.
	static bool IsSynthetic(string armaId)
	{
		return armaId.StartsWith(SYNTHETIC_PREFIX);
	}

	//------------------------------------------------------------------------------------------------
	//! True only for an identity that will still mean the same human next session: a real backend
	//! uuid. This is the test to gate anything PERSISTED against.
	static bool IsDurable(string armaId)
	{
		return !armaId.IsEmpty() && !IsSynthetic(armaId);
	}
}
