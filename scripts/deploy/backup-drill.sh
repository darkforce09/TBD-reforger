#!/usr/bin/env bash
# backup-drill.sh — prove the backups can actually be recovered from (T-577).
#
# A verified dump proves the FILE is intact. It does not prove the file is a database you
# can run the application against. This drills that end to end:
#
#   latest backup on disk  ->  restore into a scratch DB  ->  is it BOOTABLE?
#
# It deliberately drills the backup ALREADY ON DISK rather than taking a fresh one, because
# the question during an incident is "can I recover from what I have", not "can this machine
# produce a good dump right now". `--fresh` takes a new one first when that is what you want.
#
# ── Why this is a separate script from backup-db.sh ─────────────────────────────────
#
# It needs CREATE DATABASE and DROP DATABASE. backup-db.sh must never hold those rights:
# it is the thing protecting the data, and a bug in a tool with drop privileges is how you
# lose the backups and the database in one go. Split by authority, not by convenience.
#
# ── "Run db_migrate against it" — the trap, and what this does instead ──────────────
#
# T-577 specified running `db_migrate` against the restored database. MEASURED: you cannot.
# `tests/db_migrate.rs` goes through `common::require_test_database_url`
# (apps/website/api/tests/common/mod.rs:135-230), which by T-534 DERIVES its own
# `<base>_db_migrate_it` database and DROPS AND RECREATES IT on every run. Pointing
# TEST_DATABASE_URL at a restored database therefore tests a fresh empty one instead, and
# the restored rows are never examined — the signature defect, inside the drill.
#
# So the drill asserts the property that actually matters after a restore, directly:
# WILL THE API BOOT AGAINST THIS DATABASE? `db::migrate` is `sqlx::migrate!("./migrations")`
# (apps/website/api/src/db.rs:62), which on boot compares every row of `_sqlx_migrations`
# against the sha384 of the file on disk. Two things kill it, both invisible until boot:
#
#   * `_sqlx_migrations` missing from the restore -> sqlx re-applies 0001 -> CREATE TABLE
#     fails on the tables the restore just put back -> the API refuses to start.
#   * a checksum that disagrees with the file on disk -> `migration N was previously applied
#     but has been modified` (sqlx VersionMismatch) -> the API refuses to start.
#
# Both are checked here with psql and sha384sum, the same pairing scripts/platform/wave.sh
# uses in `gate_db_migrate_persist`.
#
# Usage:
#   bash scripts/deploy/backup-drill.sh                       # drill the newest backup
#   bash scripts/deploy/backup-drill.sh --fresh               # take a new backup, then drill it
#   bash scripts/deploy/backup-drill.sh --dump path/to.dump
#   bash scripts/deploy/backup-drill.sh --help
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=/dev/null
# T-884: db-common.sh deleted — load the same tbd_* names from xtask.
eval "$(cargo run -q -p xtask -- deploy db emit-bash-fns)"

TBD_DB_CONTAINER="${TBD_DB_CONTAINER:-tbd_reforger_db}"
TBD_DB_USER="${TBD_DB_USER:-tbd}"
SOURCE_DB="${TBD_BACKUP_DB:-tbd_reforger}"
OUT="${TBD_BACKUP_DIR:-$HOME/tbd-backups/website}"
SCRATCH="${TBD_DRILL_DB:-tbd_drill_probe}"
MIGDIR="${TBD_GATE_MIGRATION_DIR:-$TBD_MONO_ROOT/apps/website/api/migrations}"
DUMP=""
FRESH=0
KEEP_SCRATCH=0
STRICT_MIGRATIONS="${TBD_DRILL_STRICT_MIGRATIONS:-1}"

usage() {
	cat <<'EOF'
Usage: backup-drill.sh [--dump FILE] [--fresh] [--db NAME] [--scratch NAME] [--keep-scratch]

  Restore a backup into a scratch database and prove it is recoverable AND bootable.

  --dump FILE      drill this dump          (default: newest in the backup dir)
  --fresh          take a new backup first, then drill that
  --db NAME        source database name     (default $TBD_BACKUP_DB, or tbd_reforger)
  --out DIR        backup directory         (default $TBD_BACKUP_DIR)
  --scratch NAME   scratch restore target   (default tbd_drill_probe; must be allow-listed)
  --keep-scratch   do not drop the scratch database at the end
  --lax-migrations warn instead of fail when the restore is not boot-ready
  -h, --help       show this help

Exit: 0 the backup is recoverable · 1 it is NOT · 2 usage · 3 missing library
EOF
}

while [ $# -gt 0 ]; do
	case "$1" in
	--dump)
		DUMP="${2:?--dump needs a value}"
		shift 2
		;;
	--db)
		SOURCE_DB="${2:?--db needs a value}"
		shift 2
		;;
	--out)
		OUT="${2:?--out needs a value}"
		shift 2
		;;
	--scratch)
		SCRATCH="${2:?--scratch needs a value}"
		shift 2
		;;
	--fresh)
		FRESH=1
		shift
		;;
	--keep-scratch)
		KEEP_SCRATCH=1
		shift
		;;
	--lax-migrations)
		STRICT_MIGRATIONS=0
		shift
		;;
	-h | --help)
		usage
		exit 0
		;;
	*)
		echo "Unknown option: $1" >&2
		usage >&2
		exit 2
		;;
	esac
done

# The scratch target goes through the SAME guard as a manual restore. A drill that can be
# pointed at the live database is a scheduled outage waiting to happen — and this one runs
# unattended on a timer, so it is the single most dangerous caller in the set.
tbd_refuse_unsafe_restore_target "$SCRATCH" ""

# Fail closed on the hasher: without it every checksum compares equal to the empty string
# and the boot-readiness audit would agree with itself over nothing.
command -v sha384sum >/dev/null 2>&1 ||
	die "sha384sum is not on PATH — the migration checksum audit cannot run, and a drill that skips it is not a drill."

tbd_require_container
tbd_require_pg_tool pg_restore >/dev/null
tbd_require_pg_tool psql >/dev/null

FAILED=0
note_fail() {
	echo "DRILL FAIL: $*" >&2
	FAILED=1
}

echo "═══ backup restore drill ═══"

# ── 1. pick the dump ────────────────────────────────────────────────────────────────
if [ "$FRESH" -eq 1 ]; then
	info "taking a fresh backup of '$SOURCE_DB' first"
	cargo run -q -p xtask -- deploy db backup --db "$SOURCE_DB" --out "$OUT" || die "the fresh backup failed; nothing to drill."
fi
if [ -z "$DUMP" ]; then
	# Built by an explicit loop, NOT `mapfile < <(printf '%s\n' glob | sort)`: with nullglob
	# and no matches, `printf '%s\n'` receives no arguments and still prints ONE EMPTY LINE,
	# so the array has length 1 and the "no backups at all" branch below is unreachable.
	# Measured — the drill then called restore-db.sh with an empty path and reported a
	# restore failure instead of "there is nothing to recover from".
	shopt -s nullglob
	CANDIDATES=()
	for _f in "$OUT/${SOURCE_DB}-"*.dump; do [ -f "$_f" ] && CANDIDATES+=("$_f"); done
	shopt -u nullglob
	if [ "${#CANDIDATES[@]}" -gt 1 ]; then
		mapfile -t CANDIDATES < <(printf '%s\n' "${CANDIDATES[@]}" | LC_ALL=C sort -r)
	fi
	if [ "${#CANDIDATES[@]}" -eq 0 ]; then
		die "no backups matching '$OUT/${SOURCE_DB}-*.dump'.
       THERE IS NOTHING TO RECOVER FROM. This is the loudest possible result and it is correct:
       an empty backup directory is the failure the drill exists to surface."
	fi
	DUMP="${CANDIDATES[0]}"
	info "drilling newest of ${#CANDIDATES[@]} backup(s)"
fi
info "dump       $DUMP"
info "age        $(( ($(date +%s) - $(stat -c%Y "$DUMP" 2>/dev/null || date +%s)) / 3600 )) hour(s) old"

# ── 2. restore into the scratch DB ──────────────────────────────────────────────────
cleanup_scratch() {
	if [ "$KEEP_SCRATCH" -eq 0 ]; then
		tbd_ct psql -U "$TBD_DB_USER" -d postgres -qc \
			"DROP DATABASE IF EXISTS \"$SCRATCH\" WITH (FORCE);" >/dev/null 2>&1
	fi
}
trap cleanup_scratch EXIT INT TERM

tbd_ct psql -U "$TBD_DB_USER" -d postgres -qc "DROP DATABASE IF EXISTS \"$SCRATCH\" WITH (FORCE);" >/dev/null 2>&1
info "restoring into scratch database '$SCRATCH'"
# T-588 — --expect-db is the SOURCE database, not $SCRATCH. Passed explicitly rather than
# left to deploy-db-restore's default so that `backup-drill.sh --db <other>` still checks the
# identity of the archive it was actually pointed at.
if ! cargo run -q -p xtask -- deploy db restore --db "$SCRATCH" --expect-db "$SOURCE_DB" --create "$DUMP"; then
	echo "DRILL FAIL: the backup could NOT be restored. It is not a usable backup." >&2
	exit 1
fi

# ── 3. did anything actually arrive? ────────────────────────────────────────────────
ROWS="$(tbd_count_db_rows "$SCRATCH")"
TABLES="$(tbd_ct psql -U "$TBD_DB_USER" -d "$SCRATCH" -tAc \
	"SELECT count(*) FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE';" 2>/dev/null | tr -d '[:space:]')"
ENUMS="$(tbd_ct psql -U "$TBD_DB_USER" -d "$SCRATCH" -tAc \
	"SELECT count(*) FROM pg_type t JOIN pg_namespace n ON n.oid=t.typnamespace WHERE t.typtype='e' AND n.nspname='public';" 2>/dev/null | tr -d '[:space:]')"
IDX="$(tbd_ct psql -U "$TBD_DB_USER" -d "$SCRATCH" -tAc \
	"SELECT count(*) FROM pg_indexes WHERE schemaname='public';" 2>/dev/null | tr -d '[:space:]')"
info "restored   ${TABLES:-?} tables · ${ENUMS:-?} enums · ${IDX:-?} indexes · ${ROWS:-?} rows"
[ "${TABLES:-0}" -gt 0 ] || note_fail "the restored database has no tables."

# Structural parity against the live source, when it is reachable. Compares the restore to
# what it is supposed to be a copy of, rather than to a number hardcoded here that would
# silently stop meaning anything the next time a migration lands.
if tbd_database_exists "$SOURCE_DB"; then
	SRC_TABLES="$(tbd_ct psql -U "$TBD_DB_USER" -d "$SOURCE_DB" -tAc \
		"SELECT count(*) FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE';" 2>/dev/null | tr -d '[:space:]')"
	if [ -n "$SRC_TABLES" ] && [ "$SRC_TABLES" != "${TABLES:-}" ]; then
		note_fail "table count differs from the source: '$SOURCE_DB' has $SRC_TABLES, the restore has $TABLES."
	else
		info "parity     table count matches the live source ($SRC_TABLES)"
	fi
fi

# ── 4. BOOT READINESS — the "db_migrate against it" step ────────────────────────────
#
# See the header: the API's boot-time migrate compares _sqlx_migrations against the files
# on disk. Anything it would reject is caught here, in a drill, instead of at 3am.
info "boot check migration state vs $MIGDIR"
if [ ! -d "$MIGDIR" ]; then
	note_fail "migration directory '$MIGDIR' does not exist — the boot-readiness audit could not run (fail closed)."
else
	mapfile -t MIGFILES < <(ls -1 "$MIGDIR"/*.sql 2>/dev/null | sort)
	if [ "${#MIGFILES[@]}" -eq 0 ]; then
		note_fail "no .sql migrations under '$MIGDIR' — refusing to report a boot-ready restore from an audit with nothing to compare."
	else
		HAS_TBL="$(tbd_ct psql -U "$TBD_DB_USER" -d "$SCRATCH" -tAc \
			"SELECT to_regclass('public._sqlx_migrations') IS NOT NULL;" 2>/dev/null | tr -d '[:space:]')"
		if [ "$HAS_TBL" != "t" ]; then
			MSG="the restore has NO _sqlx_migrations table. On boot, sqlx would try to apply 0001 over the
            tables this restore just recreated, CREATE TABLE would fail, and the API would not start."
			if [ "$STRICT_MIGRATIONS" -eq 1 ]; then note_fail "$MSG"; else warn "$MSG"; fi
		else
			mig_ver() { basename "$1" | sed 's/^0*\([0-9][0-9]*\)_.*/\1/'; }
			APPLIED="$(tbd_ct psql -U "$TBD_DB_USER" -d "$SCRATCH" -tAc \
				"SELECT version || '|' || (CASE WHEN success THEN 'ok' ELSE 'bad' END) || '|' || encode(checksum,'hex')
				 FROM _sqlx_migrations ORDER BY version;" 2>/dev/null)"
			DRIFT=0 OKN=0 BADN=0 UNKNOWN=0
			while IFS='|' read -r ver state sum; do
				[ -z "$ver" ] && continue
				f=""
				for cand in "${MIGFILES[@]}"; do [ "$(mig_ver "$cand")" = "$ver" ] && {
					f="$cand"
					break
				}; done
				if [ -z "$f" ]; then
					UNKNOWN=$((UNKNOWN + 1))
					note_fail "restore records migration $ver, which has no file in $MIGDIR."
					continue
				fi
				[ "$state" = ok ] || {
					BADN=$((BADN + 1))
					note_fail "migration $ver is recorded as NOT successful in the restore."
				}
				disk="$(sha384sum <"$f" | cut -d' ' -f1)"
				if [ "$disk" != "$sum" ]; then
					DRIFT=$((DRIFT + 1))
					note_fail "migration $ver ($(basename "$f")) checksum drift — the API would refuse to boot with sqlx VersionMismatch.
            recorded: $sum
            on disk:  $disk"
				else
					OKN=$((OKN + 1))
				fi
			done <<<"$APPLIED"
			PENDING=0
			ALLV=" $(tbd_ct psql -U "$TBD_DB_USER" -d "$SCRATCH" -tAc \
				"SELECT string_agg(version::text,' ') FROM _sqlx_migrations;" 2>/dev/null | tr -d '\n') "
			for f in "${MIGFILES[@]}"; do
				case "$ALLV" in *" $(mig_ver "$f") "*) ;; *) PENDING=$((PENDING + 1)) ;; esac
			done
			info "boot check $OKN migration(s) match on disk · $PENDING pending · $DRIFT drifted · $BADN failed · $UNKNOWN unknown"
			[ "$PENDING" -gt 0 ] && info "           ($PENDING newer migration(s) would be applied on boot — expected after a deploy)"
		fi
	fi
fi

echo
if [ "$FAILED" -eq 0 ]; then
	echo "DRILL PASS — $DUMP restored into '$SCRATCH' with $ROWS row(s) across $TABLES table(s), and is boot-ready."
	exit 0
fi
echo "DRILL FAIL — the backups are NOT proven recoverable. See the failures above." >&2
exit 1
