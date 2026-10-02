//! Classifying a `sqlx::Error` by the Postgres SQLSTATE it carries, so a handler can answer a
//! constraint violation with the 4xx it deserves instead of letting it fall through
//! [`crate::core::error_handling`]'s blanket `500`.

/// True if a sqlx error is a Postgres unique-violation (SQLSTATE 23505). Used for
/// semver-conflict 409s and link-code collisions.
pub fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505")
}

/// True if a sqlx error is a Postgres foreign-key violation (SQLSTATE 23503).
///
/// **In this schema 23503 means exactly one thing: the row being written names a parent that
/// does not exist.** The other direction — deleting a parent that children still reference —
/// does *not* arrive as 23503, which is what makes a 4xx that blames the request body safe here.
/// Measured on the migrated schema rather than assumed:
/// - `DELETE FROM modpacks` under `servers_required_modpack_id_fkey` raises **23001**
///   (`restrict_violation`), not 23503.
/// - `pg_constraint` over all 25 FKs: `confdeltype` = 17 `c` (CASCADE), 3 `n` (SET NULL),
///   5 `r` (RESTRICT), and **zero `a` (NO ACTION)** — NO ACTION on delete is the only delete rule
///   that raises 23503 from the parent side, and the schema has none.
/// - `confupdtype` is `a` for all 25, so a parent *key update* could raise 23503 from the parent
///   side. No handler updates one: every parent key is a `gen_random_uuid()` surrogate or
///   `users.discord_id`, a snowflake the platform does not mint (`0018:187-190` says so, and the
///   only `SET discord_id` writes in the crate are on the child `match_player_stats`).
///
/// Pair with [`violated_constraint`] rather than using this alone. A bare "any 23503 → 4xx" arm
/// would answer for constraints its message cannot possibly describe — the caller cannot name a
/// missing parent it did not identify, and a 400 that names the wrong one is worse than a 500.
pub fn is_foreign_key_violation(e: &sqlx::Error) -> bool {
    e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23503")
}

/// The name of the constraint a database error blames, when it blames one.
///
/// Reads Postgres' structured `CONSTRAINT NAME` error field, **not** the message text. The text
/// is localized by `lc_messages` and differs between the two 23503 shapes; the field is neither.
/// Measured: an unregistered-server heartbeat reports `CONSTRAINT NAME:
/// server_statuses_server_id_fkey` with `TABLE NAME: server_statuses` — the *child*, so `table()`
/// cannot tell a caller which parent is missing and the constraint name is the identifier to
/// branch on.
pub fn violated_constraint(e: &sqlx::Error) -> Option<&str> {
    e.as_database_error().and_then(|d| d.constraint())
}
