#!/usr/bin/env bash
# backup-db.sh — verified PostgreSQL backups for the TBD website (T-577).
#
# T-280 established that this repo had NO backup tooling at all: `pg_dump`/`pg_restore`
# appeared nowhere but three comments about the Go->Rust schema parity check. There was no
# way to recover from a bad migration, a dropped table, or a disk failure.
#
# ── The two things that make this a backup rather than a file ────────────────────────
#
# 1. IT VERIFIES THE DUMP IT JUST WROTE, by opening it. See tbd_verify_dump in
#    lib/db-common.sh, and read the measurement block at the top of that file: a
#    `pg_restore --list` check — which is what this ticket originally specified — returns
#    SUCCESS on a dump truncated to 55% of its length and on a dump with corrupted bytes
#    in the body. Only a full `--data-only` read catches those. A backup script that
#    reports success over a file it never opened is this program's signature defect in a
#    new hat, so the verification is not optional and has no --skip flag.
#
# 2. THE DUMP IS WRITTEN TO A `.part` FILE AND ONLY RENAMED INTO PLACE AFTER IT VERIFIES.
#    Otherwise a dump killed halfway lands under a real backup name, counts against
#    retention, and evicts a good backup — the newest file being the broken one is
#    precisely the worst case during an incident.
#
# ── This script is READ-ONLY against PostgreSQL ─────────────────────────────────────
#
# It runs pg_dump and SELECTs. It never CREATEs or DROPs a database. The round-trip drill
# needs those rights, so the drill is a SEPARATE script (backup-drill.sh) and this one is
# never given the authority to destroy what it is protecting.
#
# Usage:
#   bash scripts/deploy/backup-db.sh                      # back up $TBD_BACKUP_DB
#   bash scripts/deploy/backup-db.sh --db tbd_x_probe     # a specific database
#   bash scripts/deploy/backup-db.sh --keep 30            # retention: keep newest 30
#   bash scripts/deploy/backup-db.sh --verify-only F.dump # re-verify an existing dump
#   bash scripts/deploy/backup-db.sh --help
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=/dev/null
. "$SCRIPT_DIR/lib/db-common.sh"

TBD_DB_CONTAINER="${TBD_DB_CONTAINER:-tbd_reforger_db}"
TBD_DB_USER="${TBD_DB_USER:-tbd}"
DB="${TBD_BACKUP_DB:-tbd_reforger}"
OUT="${TBD_BACKUP_DIR:-$HOME/tbd-backups/website}"
KEEP="${TBD_BACKUP_KEEP:-14}"
MIN_ROWS="${TBD_BACKUP_MIN_ROWS:-1}"
VERIFY_ONLY=""

usage() {
	cat <<'EOF'
Usage: backup-db.sh [--db NAME] [--out DIR] [--keep N] [--min-rows N] [--verify-only FILE]

  Dump a PostgreSQL database through the compose container in custom format (-Fc),
  VERIFY the resulting file by reading it back, and prune old backups BY COUNT.

  --db NAME           database to dump          (default $TBD_BACKUP_DB, or tbd_reforger)
  --out DIR           backup directory          (default $TBD_BACKUP_DIR, or ~/tbd-backups/website)
  --keep N            retain the newest N dumps (default $TBD_BACKUP_KEEP, or 14). N>=1.
  --min-rows N        fail if the dump contains fewer than N data rows (default 1)
  --verify-only FILE  verify an existing dump and exit; take no new backup
  -h, --help          show this help

Environment:
  TBD_DB_CONTAINER      compose container name   (default tbd_reforger_db)
  TBD_DB_USER           postgres role            (default tbd)
  TBD_CONTAINER_RUNTIME override runtime, e.g. "distrobox-host-exec podman"

Exit: 0 verified backup written · 1 failure (nothing promoted) · 2 usage · 3 missing library
EOF
}

while [ $# -gt 0 ]; do
	case "$1" in
	--db)
		DB="${2:?--db needs a value}"
		shift 2
		;;
	--out)
		OUT="${2:?--out needs a value}"
		shift 2
		;;
	--keep)
		KEEP="${2:?--keep needs a value}"
		shift 2
		;;
	--min-rows)
		MIN_ROWS="${2:?--min-rows needs a value}"
		shift 2
		;;
	--verify-only)
		VERIFY_ONLY="${2:?--verify-only needs a value}"
		shift 2
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

case "$KEEP" in '' | *[!0-9]*) die "--keep must be a non-negative integer (got '$KEEP')" ;; esac
[ "$KEEP" -lt 1 ] && die "--keep must be >= 1 — a retention policy that keeps zero backups is not a policy."
case "$MIN_ROWS" in '' | *[!0-9]*) die "--min-rows must be a non-negative integer (got '$MIN_ROWS')" ;; esac

# ── --verify-only: the verifier, standalone ─────────────────────────────────────────
# Exposed so the guard can be exercised against a deliberately broken file without taking
# a backup — non-vacuity has to be checkable by an operator, not just claimed in a comment.
if [ -n "$VERIFY_ONLY" ]; then
	tbd_require_container
	tbd_require_pg_tool pg_restore >/dev/null
	info "verifying $VERIFY_ONLY"
	if rows="$(tbd_verify_dump "$VERIFY_ONLY" "$MIN_ROWS")"; then
		echo "OK: $VERIFY_ONLY verified — $rows data row(s), TOC and full archive body read back."
		exit 0
	fi
	echo "FAIL: $VERIFY_ONLY did NOT verify." >&2
	exit 1
fi

# ── preflight ───────────────────────────────────────────────────────────────────────
case "$DB" in '' | *[!A-Za-z0-9_]*) die "--db '$DB' is not a plain ASCII database name." ;; esac
[ -z "$OUT" ] && die "--out is empty."
case "$OUT" in
/ | /root | /home | /usr | /etc | /var) die "refusing to use '$OUT' as a backup directory." ;;
esac

tbd_require_container
tbd_require_pg_tool pg_dump >/dev/null
tbd_require_pg_tool pg_restore >/dev/null # needed to verify; absent means we could not check our own work

tbd_database_exists "$DB" || die "database '$DB' does not exist in container '$TBD_DB_CONTAINER'.
       Refusing: pg_dump would fail and leave a zero-byte file that looks like a backup."

mkdir -p "$OUT" || die "cannot create backup directory '$OUT'"
[ -w "$OUT" ] || die "backup directory '$OUT' is not writable"

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
FINAL="$OUT/${DB}-${STAMP}.dump"
# `.part` deliberately does NOT match the retention glob, so a half-written dump can never
# be counted as a backup nor evict a good one.
PART="$FINAL.part"

cleanup_part() { [ -n "${PART:-}" ] && [ -f "$PART" ] && rm -f "$PART"; }
trap cleanup_part EXIT INT TERM

info "database   $DB (container $TBD_DB_CONTAINER, runtime ${TBD_RUNTIME[*]})"
info "target     $FINAL"

SRC_ROWS="$(tbd_count_db_rows "$DB")"
case "$SRC_ROWS" in '' | *[!0-9]*) SRC_ROWS="" ;; esac
[ -n "$SRC_ROWS" ] && info "source     $SRC_ROWS live row(s) across user tables"

# ── dump ────────────────────────────────────────────────────────────────────────────
# -Fc: custom format, for parallel (-j) and selective (-t/-n) restore, and it is the only
# format the verifier below can read a TOC from. No -t: never allocate a TTY on a binary
# stream. stderr is captured so a pg_dump warning does not end up inside the dump file.
DUMP_ERR="$(mktemp)"
info "dumping…"
tbd_ct_i pg_dump -U "$TBD_DB_USER" -Fc -d "$DB" >"$PART" 2>"$DUMP_ERR"
DUMP_RC=$?
if [ "$DUMP_RC" -ne 0 ]; then
	echo "FAIL: pg_dump exited $DUMP_RC — no backup written." >&2
	sed 's/^/      /' "$DUMP_ERR" >&2
	rm -f "$DUMP_ERR"
	exit 1
fi
if [ -s "$DUMP_ERR" ]; then
	warn "pg_dump wrote to stderr:"
	sed 's/^/      /' "$DUMP_ERR" >&2
fi
rm -f "$DUMP_ERR"

# ── VERIFY BEFORE PROMOTING. This is the line the ticket is about. ──────────────────
info "verifying the file just written…"
if ! ROWS="$(tbd_verify_dump "$PART" "$MIN_ROWS")"; then
	echo "FAIL: the dump did NOT verify — refusing to promote it to $FINAL." >&2
	echo "      The partial file has been removed. THERE IS NO NEW BACKUP; the previous ones are untouched." >&2
	exit 1
fi

# Cross-check the file against the database: rows counted by reading the archive vs rows
# counted by the server. They will not match exactly (materialized views are dumped as
# data, and rows can change mid-dump), so this is a loud WARN, not a failure — but a dump
# holding a small fraction of the source is worth a human look.
if [ -n "$SRC_ROWS" ] && [ "$SRC_ROWS" -gt 0 ]; then
	if [ "$ROWS" -lt $((SRC_ROWS / 2)) ]; then
		warn "dump holds $ROWS row(s) but the database reports $SRC_ROWS — less than half. Investigate."
	fi
fi

mv -f "$PART" "$FINAL" || die "verified dump could not be moved into place at $FINAL"
trap - EXIT INT TERM
chmod 600 "$FINAL" 2>/dev/null || true
info "VERIFIED  $FINAL ($(stat -c%s "$FINAL") bytes, $ROWS data rows)"

# ── retention BY COUNT, never by age ────────────────────────────────────────────────
#
# `find -mtime +N -delete` is the obvious idiom and it is the wrong one: it answers "is
# this file old?" when the question is "do I still have N backups?". If the timer stops —
# a failed unit, a machine that was off, a full disk — an age rule keeps deleting until
# NOTHING is left, and does it silently, because each individual deletion is correct by
# its own rule. A count rule degrades the opposite way: it stops deleting and the newest
# backup simply gets staler, which is recoverable and visible.
#
# Sorted by FILENAME, not mtime: the names carry a UTC ISO-8601 stamp, which sorts
# lexicographically in exactly chronological order, and unlike mtime it cannot be
# rewritten by a copy, an rsync, or a `touch`.
shopt -s nullglob
mapfile -t ALL < <(printf '%s\n' "$OUT/${DB}-"*.dump | LC_ALL=C sort -r)
shopt -u nullglob

if [ "${#ALL[@]}" -eq 0 ]; then
	warn "retention found no dumps matching ${DB}-*.dump — expected at least the one just written."
else
	# The dump just written must be the newest; if it is not, something else is writing
	# here and pruning would be guesswork.
	if [ "${ALL[0]}" != "$FINAL" ]; then
		warn "newest dump on disk is ${ALL[0]}, not the one just written ($FINAL) — skipping prune."
	elif [ "${#ALL[@]}" -gt "$KEEP" ]; then
		PRUNED=0
		for ((i = KEEP; i < ${#ALL[@]}; i++)); do
			# Belt: never delete the file just verified, whatever the sort said.
			[ "${ALL[$i]}" = "$FINAL" ] && continue
			rm -f -- "${ALL[$i]}" && PRUNED=$((PRUNED + 1))
		done
		info "retention  kept newest $KEEP, removed $PRUNED older dump(s)"
	else
		info "retention  ${#ALL[@]}/$KEEP dumps on disk, nothing to prune"
	fi
fi

info "done"
exit 0
