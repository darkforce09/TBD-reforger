//! The migration gates: the Class-R pin on 0016's claim body, and T-555's populated-database step.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::{Ctx, host, lock::GateState};
use crate::wprintln;

/// T-515 — Class-R: SQL-only claim migration 0016 must keep its claim UPDATE body.
///
/// `tests/db_migrate.rs` only asserts schema/object counts after sqlx migrate. A hollow 0016 that
/// drops the claim UPDATE (`UPDATE match_player_stats … SET discord_id`) but keeps
/// `REFRESH MATERIALIZED VIEW` still lands the same table/enum/matview census and stays gate-green
/// when claimable orphans are 0. That class of defect is invisible to the Rust gate; pin the claim
/// needles on disk here.
///
/// Needles measured from `apps/website/api/migrations/0016_backfill_pre_t326_linked_match_stats.sql`
/// claim step 2 (not comments — comment prose uses unqualified `discord_id IS NULL`).
///
/// Path override `TBD_GATE_MIGRATION_0016` is for perturbation probes only (point at a bait file
/// missing the UPDATE) — never for production gating.
pub fn gate_db_migrate_claim_body(ctx: &Ctx) -> i32 {
    let f = std::env::var("TBD_GATE_MIGRATION_0016")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            ctx.root
                .join("apps/website/api/migrations/0016_backfill_pre_t326_linked_match_stats.sql")
                .display()
                .to_string()
        });
    if !Path::new(&f).is_file() {
        wprintln!("db_migrate claim body: missing migration file: {f}");
        wprintln!(
            "        T-335 0016 is the one-shot claim for pre-T-326 linked accounts; without it"
        );
        wprintln!("        this Class-R cannot pin the UPDATE body. Restore the file or unset");
        wprintln!("        TBD_GATE_MIGRATION_0016.");
        return 1;
    }
    let src = std::fs::read_to_string(&f).unwrap_or_default();
    // Strip /*…*/ block comments (incl. multiline) then -- line comments before needle search so
    // comment-only bait cannot false-green (T-523 / verifier MAJOR).
    let body = strip_sql_comments(&src);
    let needles = [
        "UPDATE public.match_player_stats AS s",
        "SET discord_id = u.discord_id",
        "AND s.discord_id IS NULL",
    ];
    let miss: Vec<&str> = needles
        .iter()
        .copied()
        .filter(|n| !body.contains(n))
        .collect();
    if !miss.is_empty() {
        wprintln!("db_migrate claim body: FAIL — {f} is missing claim UPDATE needle(s):");
        for n in &miss {
            wprintln!("        - {n}");
        }
        wprintln!(
            "        Hollow 0016 (REFRESH kept, claim UPDATE dropped) still passes schema counts."
        );
        wprintln!("        Restore the T-335 claim UPDATE body (do not weaken this assert).");
        return 1;
    }
    wprintln!("db_migrate claim body: OK — 0016 retains claim UPDATE needles ({f})");
    0
}

/// The awk comment stripper, ported statement-for-statement.
///
/// Block comments span lines (`inblock` carries across), `--` ends the line, and whichever opener
/// comes FIRST on a line wins. Anything else and a `--` inside a block comment would terminate the
/// wrong thing.
fn strip_sql_comments(src: &str) -> String {
    let mut inblock = false;
    let mut result = String::new();
    for line in src.lines() {
        let mut s: &str = line;
        let mut out = String::new();
        while !s.is_empty() {
            if inblock {
                match s.find("*/") {
                    None => {
                        break;
                    }
                    Some(idx) => {
                        s = &s[idx + 2..];
                        inblock = false;
                        continue;
                    }
                }
            }
            let i_block = s.find("/*");
            let i_line = s.find("--");
            match (i_block, i_line) {
                (None, None) => {
                    out.push_str(s);
                    break;
                }
                (b, Some(l)) if b.is_none() || l < b.unwrap() => {
                    out.push_str(&s[..l]);
                    break;
                }
                (Some(b), _) => {
                    out.push_str(&s[..b]);
                    s = &s[b + 2..];
                    inblock = true;
                }
                _ => unreachable!("the two None case is handled above"),
            }
        }
        result.push_str(&out);
        result.push('\n');
    }
    result
}

// T-555 — THE POPULATED-DATABASE MIGRATION STEP. Read this header before changing anything below.
//
// ── WHAT WAS WRONG, AND WHY NO GATE COULD SEE IT ────────────────────────────────────────────────
//
// `ensure_gate_db` force-drops `tbd_gate_migrate` at the start of EVERY run. So `db_migrate` could
// only ever run the migration chain FORWARD FROM EMPTY. Two whole classes of defect are invisible
// from there, because both need a database that already contains something:
//
//   1. EDITING AN ALREADY-APPLIED MIGRATION. sqlx checksums the WHOLE FILE (sha384) and stores it in
//      `_sqlx_migrations`. Change so much as one comment character and every database that already
//      ran that file refuses to boot: `migration N was previously applied but has been modified`.
//      From empty there is nothing to compare against, so the checksum matches BY CONSTRUCTION.
//   2. DDL THAT CANNOT SURVIVE REAL ROWS. `CREATE UNIQUE INDEX` on a column pair that already has a
//      duplicate; `SET NOT NULL` on a column that already has a NULL. From empty there are no rows,
//      so the DDL applies BY CONSTRUCTION.
//
// Both landed. a843905f (T-331) retouched an applied 0009 — comment-only, SQL byte-identical — and
// killed every existing database. 0017 (T-511) created a unique index over a duplicate seat the
// pre-T-331 seed had already inserted, and its own header asserted the row had been cleared; T-331
// had fixed the SEED FILE, which does nothing to data already seeded. EVERY WAVE GATE SINCE T-331
// WAS GREEN OVER BOTH — including, on deploy, staging and production. Not a test that examined
// nothing: a whole category, backward compatibility, that the gate architecture excluded by design.
//
// ── THE CURE: A DATABASE THAT IS NEVER DROPPED ──────────────────────────────────────────────────
//
// `tbd_gate_migrate_persist` survives across runs. There is no DROP DATABASE in this function and
// there must never be one — the persistence IS the test. Each run:
//
//   AUDIT   every migration `_sqlx_migrations` says was applied is re-hashed on disk and compared.
//   APPLY   only the migrations this database has not seen, against the rows it is already carrying.
//   SEED    re-applies seeds/content_golden.sql so the database stays POPULATED for the next wave.
//
// ── TWO MODES, AND WHY THE SLICE GATE DOES NOT COMMIT ───────────────────────────────────────────
//
//   audit    (gate_slice) read-only audit, then each pending migration is executed inside an
//            explicit transaction that is ROLLED BACK. A unique-index violation is raised while the
//            index is being BUILT, inside that transaction, so the rollback costs nothing in
//            detection. A slice must NOT advance the shared database: slices get abandoned, and a
//            persist DB carrying a migration that never reached main would fail every later run
//            with "applied version has no file on disk" — a self-inflicted red nobody could act on.
//   advance  (cmd_gate, on merged main) the same audit, then pending migrations are COMMITTED and
//            recorded. Only merged history advances the database, so its state is always some
//            prefix of main.
//
// ── CHECKSUM PARITY IS MEASURED, NOT ASSUMED ────────────────────────────────────────────────────
//
// sqlx's checksum is sha384 over the raw file bytes. That is not taken on faith from the source:
// 2026-07-27, all 17 on-disk migrations were hashed with `sha384sum` and compared against the
// `_sqlx_migrations.checksum` values sqlx ITSELF wrote into the operator's dev database. 16 of 17
// matched byte-for-byte. The seventeenth was migration 9 — the defect, not a parity failure — and
// the pre-a843905f bytes hash to exactly the value sqlx had recorded. If a future sqlx changes the
// algorithm this step goes red on everything at once, which is the correct way to find that out.
//
// The applier below is psql, not sqlx. The one behavioural difference is statement framing: sqlx
// sends a migration as ONE multi-statement simple query, psql `-f` sends them individually. Both
// run inside ONE transaction per migration — the property migrations actually depend on — and the
// bookkeeping INSERT is inside that same transaction, so a migration is never recorded as applied
// unless it applied.
//
// ── ANTI-VACUITY ────────────────────────────────────────────────────────────────────────────────
//
// This step exists because a check reported success over an input it never examined, so it is not
// permitted to do that itself. Every one of these is a FAIL, never a skip:
//   * sha384sum or psql missing / the database unreachable  (tool absent must fail closed)
//   * zero migration files found
//   * an applied version with no matching file on disk
//   * a migration recorded with success = false
//   * THE POPULATION FLOOR — after seeding, the tables migrations actually constrain must contain
//     rows, INCLUDING at least one CLAIMED orbat seat.
//
// ── WHAT THIS STEP DOES NOT CATCH, MEASURED ─────────────────────────────────────────────────────
//
// The checksum half is absolute: from the second run onward, ANY edit to an applied migration is
// caught, whatever the data. The DDL half is only ever as good as the rows this database happens to
// carry, and a VIRGIN persist DB carries only what today's seed inserts. Measured 2026-07-27:
// bootstrap a fresh persist DB with the current (T-331-fixed) content_golden and the PRE-T-555
// 0017, and it passes — because the fixed seed no longer produces the duplicate seat that 0017 died
// on. The defect only reproduces on a database that ran the OLD seed, which is what every real
// database did.
//
// So the value here compounds with age: DO NOT DROP tbd_gate_migrate_persist to "clean it up". Its
// accumulated state — rows written by older seeds at older schema versions — is the asset, and it
// is the only thing standing in for the shape of a production database. The recovery advice in the
// `missing file` branch below is a last resort and it costs exactly that history.

const LABEL: &str = "db_migrate persist";

pub fn gate_db_migrate_persist(ctx: &Ctx, state: &GateState, mode: &str) -> u8 {
    let db = std::env::var("TBD_GATE_MIGRATE_PERSIST_DB")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "tbd_gate_migrate_persist".into());
    let migdir = std::env::var("TBD_GATE_MIGRATION_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            ctx.root
                .join("apps/website/api/migrations")
                .display()
                .to_string()
        });
    let seed = std::env::var("TBD_GATE_MIGRATE_SEED")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            ctx.root
                .join("apps/website/api/seeds/content_golden.sql")
                .display()
                .to_string()
        });

    if mode != "audit" && mode != "advance" {
        wprintln!("{LABEL}: FAIL — unknown mode '{mode}' (want audit|advance)");
        return 1;
    }
    let safe = !db.is_empty()
        && db
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
        && db.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !safe {
        wprintln!("{LABEL}: FAIL — database name '{db}' is not a safe SQL identifier.");
        return 1;
    }
    // Tool-absent fails closed. A missing hasher would otherwise make every checksum compare equal
    // to the empty string and the audit would agree with itself over nothing.
    if !on_path("sha384sum") {
        wprintln!("{LABEL}: FAIL — sha384sum not on PATH; the checksum audit cannot run.");
        return 1;
    }

    // `advance` writes to a database every other gate on this machine shares. Same invariant
    // ensure_gate_db asserts, and for the same reason — assert it rather than assume it.
    if mode == "advance" && !state.held() && !state.unserialised() {
        wprintln!(
            "{LABEL}: FAIL — advance mutates the shared persist DB and the gate lock is NOT held."
        );
        return 1;
    }

    // ── the migration set on disk ───────────────────────────────────────────────────────────────
    let mut files: Vec<PathBuf> = std::fs::read_dir(&migdir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().map(|x| x == "sql").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    if files.is_empty() {
        wprintln!("{LABEL}: FAIL — no migrations found under {migdir}. Nothing would be examined.");
        return 1;
    }

    let q = |sql: &str| -> (String, i32) { host::capture(&px(ctx, &db, sql)) };
    let admin = |sql: &str| -> (String, i32) { host::capture(&px(ctx, "postgres", sql)) };

    if admin("SELECT 1;").1 != 0 {
        wprintln!(
            "{LABEL}: FAIL — cannot reach Postgres (podman exec tbd_reforger_db). Is `cargo xtask db up` running?"
        );
        wprintln!(
            "        This is a FAIL and not a skip on purpose: a migration audit that silently"
        );
        wprintln!("        examined no database is the defect this step was built to end.");
        return 1;
    }
    // already-exists is fine; never dropped
    let _ = admin(&format!("CREATE DATABASE {db};"));
    if q("CREATE TABLE IF NOT EXISTS _sqlx_migrations (\n       version bigint PRIMARY KEY, description text NOT NULL,\n       installed_on timestamptz NOT NULL DEFAULT now(), success boolean NOT NULL,\n       checksum bytea NOT NULL, execution_time bigint NOT NULL);")
        .1
        != 0
    {
        wprintln!("{LABEL}: FAIL — could not open or initialise {db}.");
        return 1;
    }

    // ── BOOTSTRAP ───────────────────────────────────────────────────────────────────────────────
    // A brand-new persist DB has nothing to audit and nothing to apply against, so bootstrapping it
    // forward-from-empty would reproduce exactly the hole this step closes. Bootstrap therefore
    // stops ONE SHORT of the newest migration and seeds there, so even the first ever run applies
    // the newest file against populated data.
    let have_any: i64 = q("SELECT count(*) FROM _sqlx_migrations;")
        .0
        .trim()
        .parse()
        .unwrap_or(0);
    if have_any == 0 {
        wprintln!(
            "  bootstrapping {db}: applying {} migration(s) minus the newest, then seeding",
            files.len()
        );
        for f in &files[..files.len().saturating_sub(1)] {
            if !persist_apply_one(ctx, &db, f, "commit") {
                wprintln!(
                    "{LABEL}: FAIL — bootstrap could not apply {}.",
                    base_name(f)
                );
                return 1;
            }
        }
        if !persist_seed(ctx, &db, &seed) {
            return 1;
        }
    }

    // ── AUDIT: every applied migration re-hashed against disk ───────────────────────────────────
    // `success` is spelled out rather than concatenated raw: `boolean || text` renders as
    // `true`/`false`, not the `t`/`f` psql prints for a bare boolean column, and comparing against
    // the wrong one flags EVERY migration as partially-applied. Caught by this step's own
    // perturbation run.
    let rows = q("SELECT version || '|' || (CASE WHEN success THEN 'ok' ELSE 'bad' END)\n                   || '|' || encode(checksum,'hex') FROM _sqlx_migrations ORDER BY version;")
        .0;
    let mut ok_n = 0usize;
    let mut drift: Vec<(String, String, String, String)> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    for row in rows.lines() {
        let mut it = row.split('|');
        let (Some(ver), Some(state_col), Some(applied_sum)) = (it.next(), it.next(), it.next())
        else {
            continue;
        };
        if ver.is_empty() {
            continue;
        }
        let Some(f) = files.iter().find(|c| mig_ver(c) == ver) else {
            missing.push(ver.to_string());
            continue;
        };
        if state_col != "ok" {
            failed.push(ver.to_string());
        }
        let sum = sha384(f);
        if sum != applied_sum {
            drift.push((ver.to_string(), base_name(f), applied_sum.to_string(), sum));
        } else {
            ok_n += 1;
        }
    }

    let applied_versions = q("SELECT string_agg(version::text, ' ') FROM _sqlx_migrations;").0;
    let applied_set: Vec<&str> = applied_versions.split_whitespace().collect();
    let pending: Vec<&PathBuf> = files
        .iter()
        .filter(|f| !applied_set.contains(&mig_ver(f).as_str()))
        .collect();

    let mut bad = false;
    if !drift.is_empty() {
        bad = true;
        wprintln!(
            "{LABEL}: FAIL — {} ALREADY-APPLIED migration(s) were MODIFIED on disk.",
            drift.len()
        );
        wprintln!(
            "        Every existing database — dev, staging, production — will refuse to boot with"
        );
        wprintln!(
            "        `migration N was previously applied but has been modified` (sqlx VersionMismatch)."
        );
        for (ver, f, applied_sum, sum) in &drift {
            wprintln!("        - migration {ver}  {f}");
            wprintln!("            applied: {applied_sum}");
            wprintln!("            on disk: {sum}");
        }
        wprintln!(
            "        An applied migration is IMMUTABLE — sqlx hashes the whole file, so a comment-only"
        );
        wprintln!(
            "        edit is as fatal as a DDL one. Restore the original bytes and put the new prose in"
        );
        wprintln!("        the migration that has not shipped yet, or in a new one.");
    }
    if !missing.is_empty() {
        bad = true;
        wprintln!(
            "{LABEL}: FAIL — applied migration(s) with NO file on disk: {}",
            missing.join(" ")
        );
        wprintln!(
            "        Either a migration was deleted/renamed after shipping (real databases can never"
        );
        wprintln!(
            "        reach the new chain), or this persist DB was advanced by something that never"
        );
        wprintln!("        merged. Recover with: DROP DATABASE {db}; the next gate rebuilds it.");
    }
    if !failed.is_empty() {
        bad = true;
        wprintln!(
            "{LABEL}: FAIL — migration(s) recorded with success=false: {} (partially applied).",
            failed.join(" ")
        );
    }
    if bad {
        return 1;
    }

    // ── APPLY the pending migrations against the rows this database already carries ─────────────
    let finish = if mode == "advance" {
        "commit"
    } else {
        "rollback"
    };
    let mut applied_n = 0usize;
    for f in &pending {
        if !persist_apply_one(ctx, &db, f, finish) {
            wprintln!(
                "{LABEL}: FAIL — {} does not apply to a POPULATED database.",
                base_name(f)
            );
            wprintln!(
                "        It applies to an empty one, which is why every gate before this step was green."
            );
            wprintln!(
                "        Neutralise the offending rows FIRST, in the same migration, then constrain —"
            );
            wprintln!(
                "        see 0010_backfill_aar_replay_url_scheme.sql (T-405) for the established shape."
            );
            return 1;
        }
        applied_n += 1;
    }

    // Re-seed so the NEXT wave still meets real rows. Only in advance mode: audit rolled its
    // pending migrations back, so the schema it would seed against is not the one that will persist.
    if mode == "advance" && !persist_seed(ctx, &db, &seed) {
        return 1;
    }

    // ── THE POPULATION FLOOR — the guard that stops this step going hollow ──────────────────────
    let floor = q("SELECT (SELECT count(*) FROM orbat_slots WHERE assigned_to IS NOT NULL) || ' ' ||\n                    (SELECT count(*) FROM matches) || ' ' ||\n                    (SELECT count(*) FROM match_player_stats);")
        .0;
    let mut fit = floor.split_whitespace();
    let seats = fit.next().unwrap_or("");
    let rows_m = fit.next().unwrap_or("");
    let rows_s = fit.next().unwrap_or("");
    let n = |s: &str| -> i64 { s.parse().unwrap_or(0) };
    let shown = |s: &str| -> String {
        if s.is_empty() {
            "?".into()
        } else {
            s.to_string()
        }
    };
    if n(seats) < 1 || n(rows_m) < 1 || n(rows_s) < 1 {
        wprintln!(
            "{LABEL}: FAIL — {db} is not populated (claimed seats={} matches={} stats={}).",
            shown(seats),
            shown(rows_m),
            shown(rows_s)
        );
        wprintln!(
            "        Every DDL check above passed over an empty table, which proves nothing. That is"
        );
        wprintln!(
            "        precisely the failure this step exists to prevent, so it is a red, not a pass."
        );
        return 1;
    }

    wprintln!(
        "{LABEL}: OK [{mode}] — audited {ok_n} applied migration(s) against disk, {applied_n} pending"
    );
    wprintln!(
        "        applied to a populated {db} (claimed seats={seats} matches={rows_m} stats={rows_s})."
    );
    0
}

/// `podman exec tbd_reforger_db psql -U tbd -d <db> -qtA -v ON_ERROR_STOP=1 -c <sql>`.
fn px(ctx: &Ctx, db: &str, sql: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    if ctx.host.bridge {
        v.push("distrobox-host-exec".into());
    }
    v.extend(host::v(&[
        "podman",
        "exec",
        "tbd_reforger_db",
        "psql",
        "-U",
        "tbd",
        "-d",
        db,
        "-qtA",
        "-v",
        "ON_ERROR_STOP=1",
        "-c",
    ]));
    v.push(sql.to_string());
    v
}

/// Stdin -> psql on `<db>`. Explicit rather than inherited: bash's dynamic scoping would let the
/// helpers read the caller's locals, and a helper whose database depends on who called it is
/// exactly the kind of thing that quietly runs against the wrong one.
fn persist_feed(ctx: &Ctx, db: &str, body: &str) -> (String, i32) {
    let mut argv: Vec<String> = Vec::new();
    if ctx.host.bridge {
        argv.push("distrobox-host-exec".into());
    }
    argv.extend(host::v(&[
        "podman",
        "exec",
        "-i",
        "tbd_reforger_db",
        "psql",
        "-U",
        "tbd",
        "-d",
        db,
        "-q",
        "-v",
        "ON_ERROR_STOP=1",
        "-f",
        "-",
    ]));
    super::flush();
    let Ok(mut child) = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    else {
        return (String::new(), 127);
    };
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(body.as_bytes());
    }
    match child.wait_with_output() {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            (s, host::status_code(&o.status))
        }
        Err(_) => (String::new(), 127),
    }
}

/// One migration, one transaction — the migration body AND its `_sqlx_migrations` row together, so
/// a migration can never be recorded as applied unless it applied. `rollback` runs the identical
/// transaction and throws it away: used by the slice gate, which must detect without advancing.
fn persist_apply_one(ctx: &Ctx, db: &str, f: &Path, finish: &str) -> bool {
    let ver = mig_ver(f);
    let desc = mig_desc(f);
    let sum = sha384(f);
    let src = std::fs::read_to_string(f).unwrap_or_default();
    let mut body = String::from("BEGIN;\n");
    body.push_str(&src);
    body.push('\n');
    body.push_str(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)\n",
    );
    body.push_str(&format!(
        "VALUES ({ver}, '{desc}', true, decode('{sum}','hex'), 0);\n"
    ));
    body.push_str(if finish == "commit" {
        "COMMIT;\n"
    } else {
        "ROLLBACK;\n"
    });
    persist_feed(ctx, db, &body).1 == 0
}

fn persist_seed(ctx: &Ctx, db: &str, seed: &str) -> bool {
    let Ok(body) = std::fs::read_to_string(seed) else {
        wprintln!("db_migrate persist: FAIL — seed not found: {seed}");
        return false;
    };
    let (out, rc) = persist_feed(ctx, db, &body);
    if rc != 0 {
        wprintln!(
            "db_migrate persist: FAIL — the committed seed no longer loads into the migrated schema."
        );
        wprintln!(
            "        A migration that makes seeds/content_golden.sql unloadable breaks every fresh"
        );
        wprintln!("        environment, and leaves this persist DB unpopulated for the next wave.");
        let lines: Vec<&str> = out.lines().collect();
        for l in lines.iter().skip(lines.len().saturating_sub(8)) {
            wprintln!("        {l}");
        }
        return false;
    }
    true
}

/// `basename "$1" | sed 's/^0*\([0-9][0-9]*\)_.*/\1/'` — leading zeros stripped, or the basename
/// unchanged when the pattern does not match (sed leaves non-matching lines alone).
fn mig_ver(f: &Path) -> String {
    let b = base_name(f);
    let digits: String = b
        .chars()
        .skip_while(|c| *c == '0')
        .take_while(char::is_ascii_digit)
        .collect();
    let consumed = b.chars().take_while(|c| c.is_ascii_digit()).count();
    if consumed > 0 && b.chars().nth(consumed) == Some('_') {
        // `0*` is greedy but `[0-9][0-9]*` needs one digit, so `0016` -> `16` and `000` -> `0`.
        if digits.is_empty() {
            "0".into()
        } else {
            digits
        }
    } else {
        b
    }
}

/// `basename "$1" .sql | sed 's/^[0-9][0-9]*_//; s/_/ /g'`.
fn mig_desc(f: &Path) -> String {
    let b = base_name(f);
    let stem = b.strip_suffix(".sql").unwrap_or(&b);
    let n = stem.chars().take_while(char::is_ascii_digit).count();
    let rest = if n > 0 && stem.chars().nth(n) == Some('_') {
        &stem[n + 1..]
    } else {
        stem
    };
    rest.replace('_', " ")
}

fn base_name(f: &Path) -> String {
    f.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `sha384sum < "$f" | cut -d' ' -f1`.
///
/// Shelled out rather than reimplemented: the bash asserts the tool is on PATH and FAILS CLOSED
/// when it is not, and that assertion is one of the step's stated anti-vacuity properties. A
/// compiled-in hasher would delete a branch the header promises.
fn sha384(f: &Path) -> String {
    let Ok(body) = std::fs::read(f) else {
        return String::new();
    };
    let Ok(mut child) = Command::new("sha384sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    else {
        return String::new();
    };
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(&body);
    }
    let Ok(o) = child.wait_with_output() else {
        return String::new();
    };
    String::from_utf8_lossy(&o.stdout)
        .split(' ')
        .next()
        .unwrap_or("")
        .to_string()
}

fn on_path(prog: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(prog).is_file()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_stripping_survives_multiline_blocks_and_line_comments() {
        // Comment-only bait must not false-green the claim-body pin (T-523 / verifier MAJOR).
        let src = "UPDATE public.match_player_stats AS s\n/* SET discord_id = u.discord_id\n   still commented */\n-- AND s.discord_id IS NULL\nSELECT 1;\n";
        let out = strip_sql_comments(src);
        assert!(out.contains("UPDATE public.match_player_stats AS s"));
        assert!(
            !out.contains("SET discord_id = u.discord_id"),
            "block comment leaked: {out}"
        );
        assert!(
            !out.contains("AND s.discord_id IS NULL"),
            "line comment leaked: {out}"
        );
        assert!(out.contains("SELECT 1;"));
    }

    #[test]
    fn a_line_comment_inside_a_block_does_not_end_it() {
        let out = strip_sql_comments("A /* x -- y\nz */ B\n");
        assert!(out.contains('A') && out.contains('B'));
        assert!(!out.contains('z'));
    }

    #[test]
    fn migration_version_and_description_match_the_sed_pipeline() {
        let p = PathBuf::from(
            "apps/website/api/migrations/0016_backfill_pre_t326_linked_match_stats.sql",
        );
        assert_eq!(mig_ver(&p), "16");
        assert_eq!(mig_desc(&p), "backfill pre t326 linked match stats");
    }
}
