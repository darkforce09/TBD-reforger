//! The drill's scratch-database queries and the migration-file digest.
//!
//! **Role:** the `psql` scalar and row queries the drill runs inside the container, the forced drop
//! of the scratch database, and the SHA-384 of a migration file through `content_digest`.
//! **Position:** a child of [`crate::restore_drill`]; [`crate::restore_drill`]'s drop guard and its
//! `execution` child call these; every query goes through
//! [`crate::container_database::ct_capture`].
//! **Signals & state:** none; pure helpers over the container layer.
//! **Invariants:** a failed query reads as an empty answer, which the comparisons treat as a
//! mismatch, never a match; the drop names only the scratch database it was given.

use super::*;

/// The lowercase hex SHA-384 of the file at `path`, or empty when it cannot be read (the same
/// digest `sqlx` records for an applied migration).
pub(super) fn sha384_hex(path: &Path) -> String {
    content_digest::sha384_hex_of_file(path).unwrap_or_default()
}

pub(super) fn psql_scalar(db: &str, sql: &str) -> String {
    let user = db_user();
    match ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            db.to_string(),
            "-tAc".into(),
            sql.to_string(),
        ],
    ) {
        Ok((_rc, stdout, _stderr)) => stdout.trim().to_string(),
        Err(_) => String::new(),
    }
}

pub(super) fn psql_query(db: &str, sql: &str) -> String {
    // Same as scalar but preserve newlines (ORDER BY version rows).
    let user = db_user();
    match ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            db.to_string(),
            "-tAc".into(),
            sql.to_string(),
        ],
    ) {
        Ok((_rc, stdout, _stderr)) => stdout,
        Err(_) => String::new(),
    }
}

pub(super) fn drop_scratch_db(scratch: &str) -> Result<()> {
    let user = db_user();
    let sql = format!("DROP DATABASE IF EXISTS \"{scratch}\" WITH (FORCE);");
    let _ = ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            "postgres".into(),
            "-qc".into(),
            sql,
        ],
    )?;
    Ok(())
}
