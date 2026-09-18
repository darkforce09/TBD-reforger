use super::*;

pub(crate) fn verify_dump(
    file: &Path,
    min_rows: u64,
    expect_db: Option<&str>,
) -> Result<u64, VerifyFail> {
    if !file.is_file() {
        eprintln!(
            "VERIFY FAIL: '{}' does not exist or is not a regular file.",
            file.display()
        );
        return Err(VerifyFail);
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
        return Err(VerifyFail);
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
            return Err(VerifyFail);
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
    .map_err(|_| {
        eprintln!(
            "VERIFY FAIL: '{}' — `pg_restore --list` could not read the archive table of contents.",
            file.display()
        );
        VerifyFail
    })?;
    let toc = String::from_utf8_lossy(&toc_bytes);
    if list_rc != 0 || toc.is_empty() {
        eprintln!(
            "VERIFY FAIL: '{}' — `pg_restore --list` could not read the archive table of contents.",
            file.display()
        );
        return Err(VerifyFail);
    }

    // 4. IDENTITY (T-588)
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
            return Err(VerifyFail);
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
            return Err(VerifyFail);
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
                return Err(VerifyFail);
            }
            Err(nr) => {
                eprintln!(
                    "VERIFY FAIL: '{}' — the identity check could not RUN (grep status {nr:?}).",
                    file.display()
                );
                eprintln!(
                    "             A check that did not execute is not a pass. Failing closed."
                );
                return Err(VerifyFail);
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
    .map_err(|_| VerifyFail)?;
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
        return Err(VerifyFail);
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
        return Err(VerifyFail);
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
    // bash: head -c 5 | od -An -c | tr -s ' '
    match Command::new("head")
        .args(["-c", "5"])
        .arg(file)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .and_then(|head| {
            let od = Command::new("od")
                .args(["-An", "-c"])
                .stdin(head.stdout.unwrap())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()?;
            Ok(od)
        }) {
        Ok(od) => {
            let s = String::from_utf8_lossy(&od.stdout);
            s.split_whitespace().collect::<Vec<_>>().join(" ")
        }
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
        bail!("tbd_count_db_rows: psql exited {rc}");
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

/// Print bash function definitions that forward to `cargo xtask deploy db …`.
///
/// Legacy bash bridge. T-885…T-887 call Rust helpers directly; no remaining in-tree
/// `eval` caller after backup-drill.sh deletion. Kept so external wrappers do not break.
/// No new `.sh` library file — inventory shrinks only when a script is deleted.
pub fn emit_bash_fns() -> String {
    // Resolve via `cargo run -q` so cargo status lines do not leak into script stderr.
    // die()/exit in the old sourced lib killed THIS shell; a child xtask exit alone would
    // not — wrappers that map to die/exit must `exit` the caller on failure.
    r#"# T-884 bridge — db-common.sh → cargo xtask deploy db (do not add a new .sh lib)
die() { echo "FATAL: $*" >&2; exit 1; }
info() { echo "==> $*"; }
warn() { echo "WARN: $*" >&2; }
_tbd_xtask_db() { cargo run -q -p xtask -- deploy db "$@"; }
tbd_resolve_runtime() { :; } # runtime resolved inside xtask on each call
tbd_require_container() { _tbd_xtask_db require-container || exit $?; }
tbd_require_pg_tool() { _tbd_xtask_db require-pg-tool "$1" || exit $?; }
tbd_database_name_from_url() { _tbd_xtask_db database-name-from-url "$1"; }
tbd_is_safe_scratch_database_name() { _tbd_xtask_db is-safe-scratch --db "$1"; }
tbd_refuse_unsafe_restore_target() { _tbd_xtask_db refuse-unsafe --db "$1" ${2:+--confirm "$2"} || exit $?; }
tbd_verify_dump() {
	local file="$1" min_rows="${2:-1}" expect_db="${3:-}"
	_tbd_xtask_db verify-dump --file "$file" --min-rows "$min_rows" --expect-db "$expect_db"
}
tbd_count_db_rows() { _tbd_xtask_db count-rows --db "$1"; }
tbd_database_exists() { _tbd_xtask_db database-exists --db "$1"; }
tbd_ct() { _tbd_xtask_db ct -- "$@"; }
tbd_ct_i() { _tbd_xtask_db ct-i -- "$@"; }
"#
    .to_string()
}
