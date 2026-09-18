//! The migration gates: the Class-R pin on 0016's claim body, and T-555's populated-database step.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::{Ctx, host, lock::GateState};
use crate::wprintln;

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

#[cfg(test)]
#[path = "tests/migrate/tests.rs"]
mod tests;

mod gate_db_migrate_claim_body;
pub use gate_db_migrate_claim_body::gate_db_migrate_claim_body;
pub use gate_db_migrate_claim_body::gate_db_migrate_persist;

mod persist_feed;
use persist_feed::base_name;
use persist_feed::mig_ver;
use persist_feed::on_path;
use persist_feed::persist_apply_one;
use persist_feed::persist_seed;
use persist_feed::sha384;

#[cfg(test)]
use persist_feed::mig_desc;

#[cfg(test)]
use gate_db_migrate_claim_body::strip_sql_comments;
