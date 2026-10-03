//! A verified `pg_dump -Fc` of the staging database, taken on the host before a procedure
//! changes it.
//!
//! **Role:** builds the backup script `staging backup --label` runs, and reads the backup's path
//! back out of its answer.
//!
//! **Position:** used by `staging_dispatch.rs` (`staging backup`) and by the load and Discord procedures'
//! action lists, which name it as their first action.
//!
//! **Signals & state:** none; pure builder and parser.
//!
//! **Invariants:** the dump lands under `~/tbd/backups/<UTC date>/` (folder mode 700, file mode
//! 600), is written to a `.partial` name first, and becomes the backup only after `pg_restore
//! --list` reads its table of contents back; a failed or empty dump leaves no backup-named file.

use crate::error::{Result, ensure};

use crate::remote_observers::remote_command::{RemoteCommand, shell_quote};
use crate::staging_settings::{STAGING_DATABASE, STAGING_DATABASE_USER, StagingSettings};

/// The answer line naming the finished backup.
const BACKUP_LINE_PREFIX: &str = "backup: ";

/// The backup of the staging database labelled `label` (`[a-z0-9-]+`).
pub(crate) fn backup(settings: &StagingSettings, label: &str) -> Result<RemoteCommand> {
    ensure!(
        !label.is_empty()
            && label.len() <= 64
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
        "a backup label is 1-64 of [a-z0-9-], not {label:?}"
    );
    let backups = shell_quote(&format!("{}/tbd/backups", settings.home));
    let container = shell_quote(&settings.database_container);
    Ok(RemoteCommand::change_script(
        "database backup",
        format!(
            "set -euo pipefail\n\
             umask 077\n\
             dir={backups}/\"$(date -u +%Y-%m-%d)\"\n\
             mkdir -p \"$dir\"\n\
             chmod 700 \"$dir\"\n\
             file=\"$dir/{STAGING_DATABASE}-{label}-$(date -u +%Y%m%dT%H%M%SZ).dump\"\n\
             docker exec {container} pg_dump -U {STAGING_DATABASE_USER} -d {STAGING_DATABASE} -Fc > \"$file.partial\"\n\
             test -s \"$file.partial\"\n\
             docker exec -i {container} pg_restore --list < \"$file.partial\" > /dev/null\n\
             mv \"$file.partial\" \"$file\"\n\
             chmod 600 \"$file\"\n\
             echo \"{BACKUP_LINE_PREFIX}$file ($(stat -c %s \"$file\") bytes)\"\n"
        ),
    ))
}

/// The `backup: <path> (<n> bytes)` line of a backup's answer.
pub(crate) fn backup_line(output: &str) -> Option<&str> {
    output
        .lines()
        .find_map(|line| line.strip_prefix(BACKUP_LINE_PREFIX))
}
