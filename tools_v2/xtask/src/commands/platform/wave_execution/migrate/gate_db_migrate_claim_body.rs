use super::*;

/// T-515 — Class-R: SQL-only claim migration 0016 must keep its claim UPDATE body.
///
/// `tests/db_migrate.rs` only asserts schema/object counts after sqlx migrate. A hollow 0016 that
/// drops the claim UPDATE (`UPDATE match_player_stats … SET discord_id`) but keeps
/// `REFRESH MATERIALIZED VIEW` still lands the same table/enum/matview census and stays gate-green
/// when claimable orphans are 0. That class of defect is invisible to the Rust gate; pin the claim
/// needles on disk here.
///
/// Needles measured from `apps/website/api_v2/migrations/0016_backfill_pre_t326_linked_match_stats.sql`
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
                .join(
                    "apps/website/api_v2/migrations/0016_backfill_pre_t326_linked_match_stats.sql",
                )
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
pub(super) fn strip_sql_comments(src: &str) -> String {
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
                .join("apps/website/api_v2/migrations")
                .display()
                .to_string()
        });
    let seed = std::env::var("TBD_GATE_MIGRATE_SEED")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            ctx.root
                .join("apps/website/api_v2/seeds/content_golden.sql")
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
pub(super) fn px(ctx: &Ctx, db: &str, sql: &str) -> Vec<String> {
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
