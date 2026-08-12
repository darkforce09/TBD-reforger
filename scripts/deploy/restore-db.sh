#!/usr/bin/env bash
# restore-db.sh — guarded PostgreSQL restore for the TBD website (T-577).
#
# `pg_restore --clean --if-exists -d X` DROPS EVERY OBJECT IN X before it recreates
# anything. Pointed at the wrong database it is not a partial failure, it is a wipe. So
# this script refuses far more than it accepts:
#
#   1. THE T-381 ALLOW-LIST. Ported from apps/website/api/tests/common/mod.rs:87 — the
#      guard that already stopped an exported TEST_DATABASE_URL from wiping the live dev
#      database once. Same names, same refusal of `tbd_reforger`. See
#      tbd_refuse_unsafe_restore_target in lib/db-common.sh for why the disaster-recovery
#      escape hatch (naming the database a second time) is STRICTLY STRONGER than the Rust
#      guard against the threat the guard exists for, rather than weaker.
#
#   2. THE DUMP IS VERIFIED BEFORE THE TARGET IS TOUCHED. This ordering is the whole
#      point. --clean drops first and restores second; if the archive turns out to be
#      truncated at that stage, the old contents are already gone and the new ones never
#      arrive. Verifying first means a bad archive costs nothing. And note from the
#      measurements in lib/db-common.sh that `pg_restore --list` alone would NOT have
#      caught a truncated archive — it returns success on one.
#
# Usage:
#   bash scripts/deploy/restore-db.sh --db rust_it dump.dump
#   bash scripts/deploy/restore-db.sh --url postgres://tbd:tbd@localhost:5434/rust_it dump.dump
#   bash scripts/deploy/restore-db.sh --db tbd_reforger --i-understand-this-destroys=tbd_reforger dump.dump
#   bash scripts/deploy/restore-db.sh --help
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=/dev/null
# T-884: db-common.sh deleted — load the same tbd_* names from xtask.
eval "$(cargo run -q -p xtask -- deploy db emit-bash-fns)"

TBD_DB_CONTAINER="${TBD_DB_CONTAINER:-tbd_reforger_db}"
TBD_DB_USER="${TBD_DB_USER:-tbd}"
DB=""
URL=""
CONFIRM=""
DUMP=""
JOBS="${TBD_RESTORE_JOBS:-1}"
MIN_ROWS="${TBD_RESTORE_MIN_ROWS:-1}"
CREATE=0
# T-588 — the database the DUMP must have been taken FROM. NOT the restore target: those
# legitimately differ (backup-drill.sh restores a tbd_reforger dump into tbd_drill_probe),
# so conflating them would fail every drill. Set `--expect-db ''` to opt out, which the
# verifier then says out loud rather than passing quietly.
EXPECT_DB="${TBD_RESTORE_EXPECT_DB:-${TBD_BACKUP_DB:-tbd_reforger}}"

usage() {
	cat <<'EOF'
Usage: restore-db.sh (--db NAME | --url URL) [options] <dump-file>

  Verify a custom-format dump, then restore it with `pg_restore --clean --if-exists`.

  --db NAME       target database name
  --url URL       target as a postgres:// URL (database name is parsed from the path)
  --create        create the target database first if it does not exist
  --jobs N        parallel restore workers (pg_restore -j; default 1)
  --min-rows N    require the dump to hold >= N data rows before restoring (default 1)
  --expect-db N   the database the dump must have been TAKEN FROM (default $TBD_BACKUP_DB,
                  or tbd_reforger). This is the source, not the target: restoring a
                  tbd_reforger dump into a scratch DB is normal. Pass '' to skip the check.
  --i-understand-this-destroys=NAME
                  required to target a database outside the T-381 allow-list.
                  NAME must exactly equal the target — a typo cannot satisfy both.
  -h, --help      show this help

  Allowed without confirmation: rust_it, tbd_gate*, *_cold, *_it, *_probe
  Refused by default:           tbd_reforger and everything else

Exit: 0 restored · 1 refused or failed · 2 usage · 3 missing library
EOF
}

while [ $# -gt 0 ]; do
	case "$1" in
	--db)
		DB="${2:?--db needs a value}"
		shift 2
		;;
	--url)
		URL="${2:?--url needs a value}"
		shift 2
		;;
	--jobs)
		JOBS="${2:?--jobs needs a value}"
		shift 2
		;;
	--min-rows)
		MIN_ROWS="${2:?--min-rows needs a value}"
		shift 2
		;;
	--expect-db)
		[ $# -ge 2 ] || die "--expect-db needs a value (pass '' to skip the identity check)"
		EXPECT_DB="$2"
		shift 2
		;;
	--create)
		CREATE=1
		shift
		;;
	--i-understand-this-destroys=*)
		CONFIRM="${1#*=}"
		shift
		;;
	-h | --help)
		usage
		exit 0
		;;
	-*)
		echo "Unknown option: $1" >&2
		usage >&2
		exit 2
		;;
	*)
		[ -n "$DUMP" ] && {
			echo "Only one dump file may be given (already had '$DUMP', then '$1')." >&2
			exit 2
		}
		DUMP="$1"
		shift
		;;
	esac
done

[ -z "$DUMP" ] && {
	echo "No dump file given." >&2
	usage >&2
	exit 2
}
case "$JOBS" in '' | *[!0-9]*) die "--jobs must be a positive integer (got '$JOBS')" ;; esac
[ "$JOBS" -lt 1 ] && die "--jobs must be >= 1"
case "$MIN_ROWS" in '' | *[!0-9]*) die "--min-rows must be a non-negative integer (got '$MIN_ROWS')" ;; esac

# Resolve the target name. --url goes through the same parser shape as the Rust guard, so
# an unparseable URL is a refusal rather than a silent fallback to some default.
if [ -n "$URL" ]; then
	if [ -n "$DB" ]; then
		die "pass --db or --url, not both (got db='$DB' url='$URL')."
	fi
	DB="$(tbd_database_name_from_url "$URL")" || die "could not parse a database name out of --url '$URL'.
       Expected e.g. postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable"
fi
[ -z "$DB" ] && {
	echo "One of --db or --url is required." >&2
	usage >&2
	exit 2
}

# ── THE GUARD. Before the container is even contacted. ──────────────────────────────
tbd_refuse_unsafe_restore_target "$DB" "$CONFIRM"

tbd_require_container
tbd_require_pg_tool pg_restore >/dev/null
tbd_require_pg_tool psql >/dev/null

# ── VERIFY THE ARCHIVE BEFORE DROPPING ANYTHING ─────────────────────────────────────
info "verifying $DUMP before touching '$DB'"
if ! ROWS="$(tbd_verify_dump "$DUMP" "$MIN_ROWS" "$EXPECT_DB")"; then
	echo "FAIL: refusing to restore — the archive did not verify. Database '$DB' is UNTOUCHED." >&2
	echo "      \`--clean\` drops before it restores, so restoring an unreadable archive would" >&2
	echo "      have destroyed '$DB' and put nothing back." >&2
	exit 1
fi
info "archive OK — $ROWS data row(s)"

if ! tbd_database_exists "$DB"; then
	if [ "$CREATE" -eq 1 ]; then
		info "creating database '$DB'"
		tbd_ct psql -U "$TBD_DB_USER" -d postgres -c "CREATE DATABASE \"$DB\";" >/dev/null ||
			die "could not create database '$DB'"
	else
		die "database '$DB' does not exist. Pass --create to create it first."
	fi
fi

info "restoring into '$DB' (pg_restore --clean --if-exists -j $JOBS)"
# --exit-on-error so a restore that hit errors cannot report success; pg_restore's default
# is to continue and exit 0, which is the fail-open shape this whole ticket is about.
# NOTE: --single-transaction is incompatible with -j>1, so it is only used for -j 1.
RESTORE_ARGS=(--clean --if-exists --no-owner --no-privileges --exit-on-error -U "$TBD_DB_USER" -d "$DB")
[ "$JOBS" -gt 1 ] && RESTORE_ARGS+=(-j "$JOBS")

RES_ERR="$(mktemp)"
tbd_ct_i pg_restore "${RESTORE_ARGS[@]}" <"$DUMP" >"$RES_ERR" 2>&1
RC=$?
if [ "$RC" -ne 0 ]; then
	echo "FAIL: pg_restore exited $RC. Database '$DB' may be in a partial state." >&2
	tail -40 "$RES_ERR" | sed 's/^/      /' >&2
	rm -f "$RES_ERR"
	exit 1
fi
if [ -s "$RES_ERR" ]; then
	warn "pg_restore output:"
	tail -20 "$RES_ERR" | sed 's/^/      /' >&2
fi
rm -f "$RES_ERR"

# Confirm the restore landed by counting rows in the TARGET — a restore step that reports
# success without reading back the database it wrote is the same defect one layer down.
GOT="$(tbd_count_db_rows "$DB")"
info "restored   '$DB' now reports $GOT live row(s) (archive held $ROWS)"
if [ -n "$GOT" ] && [ "$GOT" -eq 0 ] && [ "$ROWS" -gt 0 ]; then
	echo "FAIL: archive held $ROWS rows but '$DB' reports 0 after restore." >&2
	exit 1
fi
info "done"
exit 0
