//! The five dump checks, the row count and the database existence probe.
//!
//! **Role:** the dump verifier (size, header, database identity, migrations table of contents, a
//! data-only read whose COPY rows must reach the minimum), the live row count and the database
//! existence probe.
//! **Position:** a child of [`crate::container_database`], which re-exports [`verify_dump`],
//! [`count_db_rows`] and [`database_exists`]; the backup, restore, drill and `deploy db
//! verify-dump` call them.
//! **Signals & state:** none; reads the dump file and queries the container per call.
//! **Invariants:** a dump is vouched for only when all five checks hold; a check that could not run
//! rejects the dump; an operator stop of the container layer is an error, distinct from a rejected
//! dump.

use super::*;

/// Runs the five checks on `file`: `Ok(Ok(rows))` when every check holds, `Ok(Err(VerifyFail))`
/// when one fails (its reason already on stderr), and `Err` for an operator stop of the container
/// layer, which checked nothing.
pub(crate) fn verify_dump(
    file: &Path,
    min_rows: u64,
    expect_db: Option<&str>,
) -> Result<Result<u64, VerifyFail>> {
    match check_dump(file, min_rows, expect_db) {
        Ok(rows) => Ok(Ok(rows)),
        Err(DumpCheckFailure::Rejected) => Ok(Err(VerifyFail)),
        Err(DumpCheckFailure::Stopped(stop)) => Err(stop),
    }
}

/// An operator stop passes through; any other container failure is reported by `reject`.
fn rejected_unless_stop(error: Error, reject: impl FnOnce()) -> DumpCheckFailure {
    if error.is_stop() {
        return DumpCheckFailure::Stopped(error);
    }
    reject();
    DumpCheckFailure::Rejected
}

fn check_dump(
    file: &Path,
    min_rows: u64,
    expect_db: Option<&str>,
) -> std::result::Result<u64, DumpCheckFailure> {
    if !file.is_file() {
        eprintln!(
            "VERIFY FAIL: '{}' does not exist or is not a regular file.",
            file.display()
        );
        return Err(DumpCheckFailure::Rejected);
    }
    let meta = fs::metadata(file).map_err(|_| {
        eprintln!(
            "VERIFY FAIL: '{}' does not exist or is not a regular file.",
            file.display()
        );
        VerifyFail
    })?;
    let size = meta.len();
    if size == 0 {
        eprintln!(
            "VERIFY FAIL: '{}' is empty (0 bytes). A zero-byte backup is the failure this check exists for.",
            file.display()
        );
        return Err(DumpCheckFailure::Rejected);
    }

    let mut magic_buf = [0u8; 5];
    {
        let mut f = fs::File::open(file).map_err(|_| VerifyFail)?;
        let n = f.read(&mut magic_buf).unwrap_or(0);
        if n < 5 || &magic_buf != b"PGDMP" {
            let got = od_c_preview(file);
            eprintln!(
                "VERIFY FAIL: '{}' does not start with the PGDMP custom-format magic (got: {got}).",
                file.display()
            );
            eprintln!(
                "             Was it written by `pg_dump -Fc`? A plain-SQL or gzip dump cannot be verified or restored by this tooling."
            );
            return Err(DumpCheckFailure::Rejected);
        }
    }

    let bytes = fs::read(file).map_err(|_| {
        eprintln!(
            "VERIFY FAIL: '{}' — could not read the archive for verification.",
            file.display()
        );
        VerifyFail
    })?;

    // 3. TOC
    let (list_rc, toc_bytes, _) = ct_i_stdin_capture(
        &["pg_restore".into(), "--list".into()],
        &bytes,
    )
    .map_err(|error| {
        rejected_unless_stop(error, || {
            eprintln!(
                "VERIFY FAIL: '{}' — `pg_restore --list` could not read the archive table of contents.",
                file.display()
            );
        })
    })?;
    let toc = String::from_utf8_lossy(&toc_bytes);
    if list_rc != 0 || toc.is_empty() {
        eprintln!(
            "VERIFY FAIL: '{}' — `pg_restore --list` could not read the archive table of contents.",
            file.display()
        );
        return Err(DumpCheckFailure::Rejected);
    }

    // 4. IDENTITY
    if let Some(expect) = expect_db {
        let dbname = toc
            .lines()
            .find_map(|line| {
                let trimmed = line.trim_start();
                trimmed.strip_prefix(';').and_then(|rest| {
                    let rest = rest.trim_start();
                    rest.strip_prefix("dbname:").map(|v| v.trim().to_string())
                })
            })
            .unwrap_or_default();
        if dbname.is_empty() {
            eprintln!(
                "VERIFY FAIL: '{}' — the archive header carries no `dbname:` line, so the source",
                file.display()
            );
            eprintln!(
                "             database cannot be established. Refusing to vouch for an archive whose"
            );
            eprintln!("             identity is unknown; expected '{expect}'.");
            return Err(DumpCheckFailure::Rejected);
        }
        if dbname != expect {
            eprintln!(
                "VERIFY FAIL: '{}' is a dump of database '{dbname}', but '{expect}' was expected.",
                file.display()
            );
            eprintln!(
                "             This archive is structurally VALID — it is simply the wrong database."
            );
            eprintln!(
                "             Restoring it would `pg_restore --clean` the target and repopulate it"
            );
            eprintln!(
                "             from the wrong source. If you meant this, pass the real source name."
            );
            return Err(DumpCheckFailure::Rejected);
        }
        let pat = Pattern::literal("_sqlx_migrations");
        match gate::probe_str(&pat, &toc) {
            Ok(true) => {}
            Ok(false) => {
                eprintln!(
                    "VERIFY FAIL: '{}' names database '{dbname}' but its TOC has no `_sqlx_migrations`",
                    file.display()
                );
                eprintln!(
                    "             table. Every TBD-Reforger database carries one; an archive without it is"
                );
                eprintln!(
                    "             not a backup of this platform's schema, whatever it is named."
                );
                return Err(DumpCheckFailure::Rejected);
            }
            Err(nr) => {
                eprintln!(
                    "VERIFY FAIL: '{}' — the identity check could not RUN (grep status {nr:?}).",
                    file.display()
                );
                eprintln!(
                    "             A check that did not execute is not a pass. Failing closed."
                );
                return Err(DumpCheckFailure::Rejected);
            }
        }
    } else {
        eprintln!(
            "VERIFY NOTE: no expected database name was given, so '{}' was NOT checked for",
            file.display()
        );
        eprintln!(
            "             database identity — a valid dump of a DIFFERENT database would pass"
        );
        eprintln!("             everything below. Pass a third argument to close that gap.");
    }

    // 5. Full body read + COPY row count
    let (data_rc, data_out, data_err) = ct_i_stdin_capture(
        &[
            "pg_restore".into(),
            "--data-only".into(),
            "-f".into(),
            "-".into(),
        ],
        &bytes,
    )
    .map_err(|error| rejected_unless_stop(error, || {}))?;
    if data_rc != 0 {
        eprintln!(
            "VERIFY FAIL: '{}' — `pg_restore --data-only` exited {data_rc} while reading the archive body.",
            file.display()
        );
        eprintln!(
            "             The file is TRUNCATED or CORRUPT. Note that `pg_restore --list` PASSES on both"
        );
        eprintln!("             of those (measured) — this is the check that catches them.");
        for line in data_err.lines() {
            eprintln!("             {line}");
        }
        return Err(DumpCheckFailure::Rejected);
    }
    let data_text = String::from_utf8_lossy(&data_out);
    let rows = count_copy_rows(&data_text);
    if rows < min_rows {
        eprintln!(
            "VERIFY FAIL: '{}' restored cleanly but contains {rows} data row(s), below the required minimum of {min_rows}.",
            file.display()
        );
        eprintln!(
            "             A schema-only or all-empty archive is not a backup of a live database."
        );
        eprintln!(
            "             (If backing up a genuinely empty database is intended, pass --min-rows 0.)"
        );
        return Err(DumpCheckFailure::Rejected);
    }
    Ok(rows)
}

/// Port of the bash awk COPY-block row counter.
pub(super) fn count_copy_rows(data: &str) -> u64 {
    let mut n: u64 = 0;
    let mut inc = false;
    for line in data.lines() {
        if line.starts_with("COPY ") && line.ends_with(" FROM stdin;") {
            inc = true;
            continue;
        }
        if inc && line == "\\." {
            inc = false;
            continue;
        }
        if inc {
            n += 1;
        }
    }
    n
}

pub(super) fn od_c_preview(file: &Path) -> String {
    // bash: head -c 5 | od -An -c | tr -s ' ' — `od -N 5` reads the same first five bytes.
    match Run::new("od")
        .args(["-An", "-c", "-N", "5"])
        .arg(file)
        .output()
    {
        Ok(od) => od.stdout.split_whitespace().collect::<Vec<_>>().join(" "),
        Err(_) => "<unreadable>".into(),
    }
}

pub(crate) fn count_db_rows(db: &str) -> Result<String> {
    let user = db_user();
    let sql = "\
		SELECT COALESCE(sum(cnt),0) FROM (
			SELECT (xpath('/row/c/text()',
				query_to_xml(format('SELECT count(*) AS c FROM %I.%I', schemaname, relname),
				false, true, '')))[1]::text::bigint AS cnt
			FROM pg_stat_user_tables
		) t;";
    let (rc, stdout, _) = ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            db.to_string(),
            "-tAc".into(),
            sql.into(),
        ],
    )?;
    if rc != 0 {
        refuse!("tbd_count_db_rows: psql exited {rc}");
    }
    Ok(stdout.chars().filter(|c| !c.is_whitespace()).collect())
}

pub(crate) fn database_exists(db: &str) -> Result<bool> {
    let user = db_user();
    // bash interpolates $db into SQL; keep the same shape (callers already ASCII-guard names).
    let sql = format!("SELECT 1 FROM pg_database WHERE datname = '{db}';");
    let (rc, stdout, _) = ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            "postgres".into(),
            "-tAc".into(),
            sql,
        ],
    )?;
    if rc != 0 {
        return Ok(false);
    }
    let out: String = stdout.chars().filter(|c| !c.is_whitespace()).collect();
    Ok(out == "1")
}
