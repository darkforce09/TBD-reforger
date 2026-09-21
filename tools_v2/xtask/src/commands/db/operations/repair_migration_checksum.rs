//! `cargo xtask db repair-migration-checksum` — repoint a recorded migration checksum after an
//! edit that changed a migration file's comments but not its DDL.
//!
//! ── THE FAILURE THIS ANSWERS ────────────────────────────────────────────────────────────────
//!
//! `sqlx` stores `Sha384::digest(<the whole migration file>)` in `_sqlx_migrations.checksum` and
//! compares it on every boot. It hashes the file, not the statements, so rewriting a comment in an
//! applied migration is indistinguishable at boot from rewriting its DDL:
//!
//! ```text
//! Error: migration 21 was previously applied but has been modified
//! ```
//!
//! Every database that applied the old bytes refuses to start, including production, and the
//! schema is not wrong in the slightest.
//!
//! ── WHY THIS IS NOT A `--force` FLAG WITH A NICE NAME ───────────────────────────────────────
//!
//! Blindly repointing a checksum is exactly how a real schema divergence gets papered over: the
//! database keeps the old shape, the file claims a new one, and nothing ever compares them again.
//! So this refuses by default and proves the edit was harmless before it writes.
//!
//! The recorded checksum identifies the exact bytes the database applied, so those bytes can be
//! recovered: the migration's own git history is searched for the blob whose SHA-384 matches. With
//! the old and new content both in hand, the two are compared with comments stripped. Only when
//! the statements are identical is the row repointed.
//!
//! A checkout with no history to search (the deploy rsync excludes `.git/`, so a server has none)
//! cannot run that proof, and is refused unless `--force` says a human did it by hand.

use std::process::Command;

use anyhow::Result;
use developer_tools::content_digest::sha384_hex;

use super::super::operations::{WEB, echo};
use crate::commands::deploy::database_operations::{ct_capture, db_user};

/// How a migration file's current bytes relate to the bytes the database applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Same bytes. Nothing to repair.
    Identical,
    /// Different bytes, identical statements — a comment or whitespace edit.
    CommentsOnly,
    /// The statements themselves differ. Never repaired automatically.
    DdlChanged,
}

/// Strips `--` line comments and blank lines, leaving the statements.
///
/// Quote-aware: a `--` inside a string literal is content, not a comment, and treating it as one
/// could make two genuinely different statements normalise to the same text — which is the single
/// way this tool could do harm.
pub fn normalize_sql(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    for line in sql.lines() {
        let mut in_quote = false;
        let mut code_end = line.len();
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\'' => in_quote = !in_quote,
                b'-' if !in_quote && i + 1 < bytes.len() && bytes[i + 1] == b'-' => {
                    code_end = i;
                    break;
                }
                _ => {}
            }
            i += 1;
        }
        let code = line[..code_end].trim_end();
        if !code.trim().is_empty() {
            out.push_str(code);
            out.push('\n');
        }
    }
    out
}

pub fn classify(current: &str, applied: &str) -> Change {
    if current == applied {
        Change::Identical
    } else if normalize_sql(current) == normalize_sql(applied) {
        Change::CommentsOnly
    } else {
        Change::DdlChanged
    }
}

/// The leading version number of a migration filename (`0021_rate_limit_buckets.sql` → 21).
pub fn version_of(file_name: &str) -> Option<i64> {
    let digits: String = file_name
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

/// A unified-ish rendering of which lines differ, for the operator to read before trusting it.
pub fn comment_diff(current: &str, applied: &str) -> String {
    let applied_lines: Vec<&str> = applied.lines().collect();
    let current_lines: Vec<&str> = current.lines().collect();
    let mut out = String::new();
    for line in &applied_lines {
        if !current_lines.contains(line) {
            out.push_str(&format!("    - {line}\n"));
        }
    }
    for line in &current_lines {
        if !applied_lines.contains(line) {
            out.push_str(&format!("    + {line}\n"));
        }
    }
    out
}

// ── LIVE HALF ────────────────────────────────────────────────────────────────────────────────

/// `psql -tAc <sql>` inside the db container, output captured.
///
/// Goes through the backup lane's container exec rather than `compose exec`: this needs only a
/// running container, not a compose provider, and the two are not always both present on a host.
fn psql_capture(sql: &str) -> Result<(i32, String)> {
    let argv: Vec<String> = ["psql", "-U", &db_user(), "-d", &db_name(), "-tAc", sql]
        .iter()
        .map(|a| (*a).to_string())
        .collect();
    let (code, stdout, stderr) = ct_capture(false, &argv)?;
    if code != 0 && !stderr.trim().is_empty() {
        eprintln!("{}", stderr.trim_end());
    }
    Ok((code, stdout))
}

/// The dev database name, overridable for the same reason the container and user are.
fn db_name() -> String {
    std::env::var("TBD_DB_NAME").unwrap_or_else(|_| "tbd_reforger".to_string())
}

/// Every `(version, checksum-hex)` the database has recorded.
fn applied_checksums() -> Result<Vec<(i64, String)>> {
    let (code, stdout) = psql_capture(
        "SELECT version, encode(checksum,'hex') FROM _sqlx_migrations ORDER BY version;",
    )?;
    if code != 0 {
        anyhow::bail!(
            "could not read _sqlx_migrations (psql exit {code}). Is the database up? `cargo xtask db up`"
        );
    }
    let mut rows = Vec::new();
    for line in stdout.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some((v, c)) = line.split_once('|')
            && let Ok(version) = v.trim().parse::<i64>()
        {
            rows.push((version, c.trim().to_string()));
        }
    }
    Ok(rows)
}

/// Recovers the exact bytes the database applied, by hunting this file's git history for the blob
/// whose SHA-384 is the recorded checksum. `None` means the history does not contain them.
fn applied_content(repo_root: &std::path::Path, rel_path: &str, checksum: &str) -> Option<String> {
    let log = Command::new("git")
        .args([
            "log",
            "--all",
            "--follow",
            "--pretty=format:C%H",
            "--name-only",
            "--",
            rel_path,
        ])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !log.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&log.stdout).into_owned();

    // `--follow` prints each commit followed by the path AS IT WAS in that commit, so a rename in
    // the file's past is handled by pairing the two rather than assuming today's spelling.
    let mut commit = String::new();
    for line in text.lines() {
        if let Some(sha) = line.strip_prefix('C') {
            commit = sha.trim().to_string();
            continue;
        }
        let path = line.trim();
        if path.is_empty() || commit.is_empty() {
            continue;
        }
        let show = Command::new("git")
            .arg("show")
            .arg(format!("{commit}:{path}"))
            .current_dir(repo_root)
            .output()
            .ok()?;
        if !show.status.success() {
            continue;
        }
        if sha384_hex(&show.stdout) == checksum {
            return Some(String::from_utf8_lossy(&show.stdout).into_owned());
        }
    }
    None
}

fn repoint(version: i64, checksum: &str) -> Result<u8> {
    let sql = format!(
        "UPDATE _sqlx_migrations SET checksum = decode('{checksum}','hex') WHERE version = {version};"
    );
    let (code, _out) = psql_capture(&sql)?;
    if code != 0 {
        eprintln!("xtask db: the UPDATE failed (psql exit {code}); nothing was changed.");
        return Ok(code.clamp(0, 255) as u8);
    }
    println!("  repointed migration {version} to {checksum}");
    Ok(0)
}

/// Entry point. `version` limits the work to one migration; `force` allows a repoint when the
/// applied bytes cannot be recovered from git and a human has verified the edit by hand.
pub fn run(version: Option<i64>, force: bool) -> Result<u8> {
    echo("cargo xtask db repair-migration-checksum");
    let repo_root = crate::core::repository_root::find_repo_root()?;
    let migrations = repo_root.join(WEB).join("migrations");
    let applied = applied_checksums()?;
    if applied.is_empty() {
        println!("  no rows in _sqlx_migrations — nothing has been applied.");
        return Ok(0);
    }

    let mut drifted = 0usize;
    let mut repaired = 0usize;
    let mut refused = 0usize;
    let mut examined = 0usize;

    for (recorded_version, recorded_checksum) in &applied {
        if version.is_some_and(|v| v != *recorded_version) {
            continue;
        }
        examined += 1;
        let Some(entry) = std::fs::read_dir(&migrations)?
            .filter_map(|e| e.ok())
            .find(|e| {
                e.file_name()
                    .to_str()
                    .and_then(version_of)
                    .is_some_and(|v| v == *recorded_version)
            })
        else {
            println!(
                "  {recorded_version}: applied to the database but no migration file has that version — left alone."
            );
            refused += 1;
            continue;
        };

        let path = entry.path();
        let current = std::fs::read_to_string(&path)?;
        if sha384_hex(current.as_bytes()) == *recorded_checksum {
            continue;
        }
        drifted += 1;

        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = format!("{WEB}/migrations/{name}");
        println!("  {recorded_version} ({name}): file does not match the applied checksum.");

        let Some(applied_sql) = applied_content(&repo_root, &rel, recorded_checksum) else {
            if force {
                println!(
                    "    --force: the applied bytes are not in this checkout's history, repointing anyway."
                );
                repair_one(*recorded_version, &current, &mut repaired)?;
            } else {
                eprintln!(
                    "    REFUSED: the bytes the database applied are not in this checkout's git history,"
                );
                eprintln!(
                    "             so the edit cannot be proven harmless here. Verify the statements by"
                );
                eprintln!(
                    "             hand against the deployed schema, then re-run with --force."
                );
                refused += 1;
            }
            continue;
        };

        match classify(&current, &applied_sql) {
            Change::Identical => {}
            Change::CommentsOnly => {
                println!("    comments only — the statements are unchanged:");
                print!("{}", comment_diff(&current, &applied_sql));
                repair_one(*recorded_version, &current, &mut repaired)?;
            }
            Change::DdlChanged => {
                eprintln!("    REFUSED: the statements themselves differ, not just comments.");
                print!(
                    "{}",
                    comment_diff(&normalize_sql(&current), &normalize_sql(&applied_sql))
                );
                eprintln!(
                    "             A real schema change needs a NEW migration, not a repointed checksum."
                );
                refused += 1;
            }
        }
    }

    if examined == 0 {
        // Saying "nothing to repair" here would be a claim about migrations that were never
        // looked at: the filter matched no applied row at all.
        let asked = version.map(|v| v.to_string()).unwrap_or_default();
        eprintln!(
            "  no applied migration has version {asked}. Applied versions: {}.",
            version_list(&applied)
        );
        return Ok(1);
    }
    if drifted == 0 {
        println!("  every applied migration examined matches its file. Nothing to repair.");
        return Ok(0);
    }
    println!("  {drifted} drifted, {repaired} repaired, {refused} refused.");
    Ok(if refused > 0 { 1 } else { 0 })
}

/// Applied versions as a comma-separated list, so a bad `--version` says what was available.
fn version_list(applied: &[(i64, String)]) -> String {
    applied
        .iter()
        .map(|(v, _)| v.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn repair_one(version: i64, current: &str, repaired: &mut usize) -> Result<()> {
    if repoint(version, &sha384_hex(current.as_bytes()))? == 0 {
        *repaired += 1;
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/repair_migration_checksum/tests.rs"]
mod tests;
