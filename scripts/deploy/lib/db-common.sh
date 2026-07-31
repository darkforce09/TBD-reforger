#!/usr/bin/env bash
# shellcheck shell=bash
# db-common.sh — shared plumbing for backup-db.sh / restore-db.sh / backup-drill.sh (T-577).
#
# ── Why this is a LIBRARY and not three copies ───────────────────────────────────────
#
# scripts/mod/lib/gate-grep.sh exists because T-216 fixed a fail-open grep inline and the
# fix did not propagate: every gate written afterwards was born with the same hole. The
# same trap is live here. The dump VERIFIER and the T-381 restore allow-list are each
# security-relevant and each about to be needed by three scripts. One implementation, so
# the next script cannot be born broken by copy-paste.
#
# ── MEASURED 2026-07-31: `pg_restore --list` IS NOT A VERIFICATION ───────────────────
#
# T-577's brief specified `pg_restore --list` plus a non-zero-rows check. Measured on a
# 5,500-byte custom-format dump of a 1,000-row scratch DB, `--list` is fail-open twice:
#
#   perturbation                       --list rc   --data-only rc
#   good dump                              0             0
#   TRUNCATED to 3000/5500 bytes           0    <-- !     1
#   4 bytes corrupted at offset 4000       0    <-- !     1
#   empty file                             1             1
#   29 bytes of ASCII garbage              1             1
#   first 200 bytes only                   1             1
#
# `--list` reads only the TOC, which pg_dump writes at the HEAD of the file. Every data
# block after it is unexamined, so a dump whose body was lost to a full disk or a killed
# container passes `--list` cleanly. That is this program's signature defect — a tool
# reporting success over an input it never read — inside the very check written to stop it.
#
# `pg_restore --data-only -f -` decompresses every data block, so it is the check that
# actually reads the file. COST IS NOT A REASON TO SKIP IT: measured on an 89 MB database
# (4x the live 22 MB), dump 0.45 s, full verify 0.13 s.
#
# ── And counting TOC `TABLE DATA` entries is NOT a non-zero-rows check ───────────────
#
# MEASURED: pg_dump emits a TABLE DATA entry for every table INCLUDING EMPTY ONES. A dump
# of a database whose tables are all empty has 0 rows and 2 TABLE DATA entries. So the row
# count is taken by parsing the COPY blocks out of the decompressed stream — a count of
# rows that were actually present in the FILE, not a count of headings that describe it.

set -uo pipefail

TBD_DB_COMMON_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TBD_MONO_ROOT="$(cd "$TBD_DB_COMMON_DIR/../../.." && pwd)"

# The four-outcome grep helpers (0 match / 1 no-match / 2 file error / 127 tool absent).
# Fail CLOSED if they are missing rather than hand-rolling a boolean grep here — that
# rebirth-by-copy-paste is the defect gate-grep.sh was written to end.
TBD_GATE_GREP="$TBD_MONO_ROOT/scripts/mod/lib/gate-grep.sh"
if [ ! -f "$TBD_GATE_GREP" ]; then
	echo "FATAL: missing $TBD_GATE_GREP — refusing to run with a hand-rolled grep." >&2
	echo "       (four-outcome exit-status discipline lives there; see T-556.)" >&2
	exit 3
fi
# shellcheck source=/dev/null
. "$TBD_GATE_GREP"

die() {
	echo "FATAL: $*" >&2
	exit 1
}
info() { echo "==> $*"; }
warn() { echo "WARN: $*" >&2; }

# ─────────────────────────── container runtime resolution ───────────────────────────
#
# MEASURED 2026-07-31: pg_dump / pg_restore / psql exist on NEITHER the dev container nor
# the host — only inside the postgres:18-alpine container (pg_dump 18.4 at
# /usr/local/bin/pg_dump). podman is host-only. So every libpq tool call is routed through
# the container, and the container runtime itself may need routing through the host.
#
# Order: explicit override, then native podman/docker (this is the production host case),
# then distrobox-host-exec (the dev-container case). Fail CLOSED naming what was tried —
# "no runtime" and "runtime present but container down" send a reader to different places.
TBD_RUNTIME=()

tbd_resolve_runtime() {
	if [ "${#TBD_RUNTIME[@]}" -gt 0 ]; then return 0; fi
	if [ -n "${TBD_CONTAINER_RUNTIME:-}" ]; then
		# Deliberate word-split: the override may legitimately be "distrobox-host-exec podman".
		read -r -a TBD_RUNTIME <<<"$TBD_CONTAINER_RUNTIME"
		command -v "${TBD_RUNTIME[0]}" >/dev/null 2>&1 ||
			die "TBD_CONTAINER_RUNTIME='$TBD_CONTAINER_RUNTIME' but '${TBD_RUNTIME[0]}' is not executable."
		return 0
	fi
	if command -v podman >/dev/null 2>&1; then
		TBD_RUNTIME=(podman)
	elif command -v docker >/dev/null 2>&1; then
		TBD_RUNTIME=(docker)
	elif command -v distrobox-host-exec >/dev/null 2>&1 &&
		distrobox-host-exec command -v podman >/dev/null 2>&1; then
		TBD_RUNTIME=(distrobox-host-exec podman)
	elif command -v distrobox-host-exec >/dev/null 2>&1 &&
		distrobox-host-exec command -v docker >/dev/null 2>&1; then
		TBD_RUNTIME=(distrobox-host-exec docker)
	else
		die "no container runtime. Tried: \$TBD_CONTAINER_RUNTIME, podman, docker, distrobox-host-exec {podman,docker}.
       pg_dump does not exist on this host either, so there is no fallback path.
       Refusing to report a successful backup from a tool that cannot run."
	fi
	return 0
}

# Never pass -t: a TTY would mangle the binary dump stream.
tbd_ct() { tbd_resolve_runtime && "${TBD_RUNTIME[@]}" exec "$TBD_DB_CONTAINER" "$@"; }
tbd_ct_i() { tbd_resolve_runtime && "${TBD_RUNTIME[@]}" exec -i "$TBD_DB_CONTAINER" "$@"; }

tbd_require_container() {
	tbd_resolve_runtime
	local out rc
	out="$("${TBD_RUNTIME[@]}" inspect -f '{{.State.Running}}' "$TBD_DB_CONTAINER" 2>&1)"
	rc=$?
	if [ "$rc" -ne 0 ]; then
		die "container '$TBD_DB_CONTAINER' not found by '${TBD_RUNTIME[*]}' (rc=$rc).
       $out
       Start it with: make db-up"
	fi
	if [ "$out" != "true" ]; then
		die "container '$TBD_DB_CONTAINER' exists but is not running (State.Running=$out).
       A backup taken against a stopped database is not a backup. Start it: make db-up"
	fi
}

# Fail CLOSED when the postgres tool is absent, and say so distinctly from "it ran and
# failed". `command -v` inside the container returns 127-ish semantics via the shell.
tbd_require_pg_tool() {
	local tool="$1" path rc
	path="$(tbd_ct sh -c "command -v $tool" 2>/dev/null)"
	rc=$?
	if [ "$rc" -ne 0 ] || [ -z "$path" ]; then
		die "'$tool' is ABSENT inside container '$TBD_DB_CONTAINER' (rc=$rc).
       This is not a failed backup, it is a backup that never ran. Refusing to report success.
       Expected a postgres image that ships $tool (postgres:18-alpine has it at /usr/local/bin/$tool)."
	fi
	printf '%s' "$path"
}

# ───────────────────────────── T-381 restore target guard ────────────────────────────
#
# Ported verbatim in behaviour from apps/website/api/tests/common/mod.rs:63-96, which is
# the guard that already stopped an exported TEST_DATABASE_URL from wiping the live dev
# database. Same allow-list, same refusal of `tbd_reforger`, same ASCII-name parsing.
# Kept character-for-character in its RULES so the two cannot drift into disagreeing about
# which databases are throwaways.

# postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable -> rust_it
# Empty path, multi-segment path, or a non [A-Za-z0-9_] name -> "" (caller must refuse).
tbd_database_name_from_url() {
	local url="$1" rest name
	case "$url" in
	*://*) rest="${url#*://}" ;;
	*) return 1 ;;
	esac
	rest="${rest#*/}"     # strip authority, keep path+query
	name="${rest%%\?*}"   # drop query string
	name="${name%%#*}"    # drop fragment
	[ -z "$name" ] && return 1
	case "$name" in */*) return 1 ;; esac
	# Reject weirdness rather than percent-decoding — same call the Rust guard makes.
	case "$name" in *[!A-Za-z0-9_]*) return 1 ;; esac
	printf '%s' "$name"
}

# Whether `name` is a dedicated integration / gate / probe database — never the live
# `tbd_reforger` database. Mirrors is_safe_test_database_name (tests/common/mod.rs:87).
tbd_is_safe_scratch_database_name() {
	local name="$1"
	[ -z "$name" ] && return 1
	[ "$name" = "tbd_reforger" ] && return 1
	case "$name" in
	rust_it) return 0 ;;
	tbd_gate*) return 0 ;;
	*_cold) return 0 ;;
	*_it) return 0 ;;
	*_probe) return 0 ;;
	esac
	return 1
}

tbd_refuse_unsafe_restore_target() {
	local name="$1" confirm="${2:-}"
	if [ -z "$name" ]; then
		die "restore target database name is empty or unparseable.
       Expected a single ASCII name, e.g. --db rust_it"
	fi
	case "$name" in *[!A-Za-z0-9_]*)
		die "restore target '$name' is not a plain ASCII database name ([A-Za-z0-9_])."
		;;
	esac
	if tbd_is_safe_scratch_database_name "$name"; then
		return 0
	fi
	# ── Escape hatch, deliberately NOT weaker than the Rust guard ──────────────────
	# The Rust guard is a TEST harness: it never needs to touch the live DB, so it has
	# no escape at all. A restore tool that can never target the live database cannot
	# do the one job it exists for — recovering `tbd_reforger` after a bad migration.
	# So the allow-list stays the DEFAULT and the only way past it is to type the
	# destructive name a SECOND time in a flag that spells out what it does. A typo
	# cannot satisfy that: it would have to be made identically twice, in two different
	# arguments. This closes the typo threat the guard exists for while leaving the
	# disaster-recovery path open.
	if [ -n "$confirm" ] && [ "$confirm" = "$name" ]; then
		warn "restoring over NON-scratch database '$name' — confirmed via --i-understand-this-destroys=$name"
		return 0
	fi
	cat >&2 <<EOF
───────────────────────────────────────────────────────────────────────
REFUSING to restore into database \`$name\` (T-381 allow-list).

  Allowed without confirmation: rust_it, tbd_gate*, *_cold, *_it, *_probe

  The live database \`tbd_reforger\` is never allowed by default — a
  \`pg_restore --clean --if-exists\` against it DROPS EVERY OBJECT FIRST,
  so a typo here is unrecoverable without another backup.

  This is the same allow-list the integration harness carries at
  apps/website/api/tests/common/mod.rs:87, which already stopped one
  exported TEST_DATABASE_URL from wiping the live database.

  If you genuinely mean it (disaster recovery), name it twice:
    restore-db.sh --db $name --i-understand-this-destroys=$name <dump>
───────────────────────────────────────────────────────────────────────
EOF
	exit 1
}

# ──────────────────────────────── dump verification ──────────────────────────────────
#
# The single most important function in T-577. It must open the file it is asked about.
# Four independent checks, each catching something the one before it cannot:
#
#   1. exists + non-empty        catches "the redirect never wrote anything"
#   2. PGDMP magic header        catches "this is not a custom-format archive at all"
#   3. pg_restore --list         catches a destroyed TOC (and gives us the header)
#   4. pg_restore --data-only    THE ONE THAT MATTERS — decompresses every data block,
#                                catching truncation and corruption that (3) passes,
#                                and yielding the row count FROM THE FILE.
#
# Emits the row count on stdout when it succeeds. Returns non-zero on any failure.
tbd_verify_dump() {
	local file="$1" min_rows="${2:-1}"
	local size rows magic rc

	if [ ! -f "$file" ]; then
		echo "VERIFY FAIL: '$file' does not exist or is not a regular file." >&2
		return 1
	fi
	size="$(stat -c%s "$file" 2>/dev/null || echo 0)"
	if [ "$size" -le 0 ]; then
		echo "VERIFY FAIL: '$file' is empty (0 bytes). A zero-byte backup is the failure this check exists for." >&2
		return 1
	fi

	# 2. Custom-format archives begin with the literal bytes "PGDMP".
	magic="$(head -c 5 "$file" 2>/dev/null)"
	if [ "$magic" != "PGDMP" ]; then
		echo "VERIFY FAIL: '$file' does not start with the PGDMP custom-format magic (got: $(head -c 5 "$file" | od -An -c | tr -s ' '))." >&2
		echo "             Was it written by \`pg_dump -Fc\`? A plain-SQL or gzip dump cannot be verified or restored by this tooling." >&2
		return 1
	fi

	# 3. TOC readable.
	if ! tbd_ct_i pg_restore --list <"$file" >/dev/null 2>&1; then
		echo "VERIFY FAIL: '$file' — \`pg_restore --list\` could not read the archive table of contents." >&2
		return 1
	fi

	# 4. Full body read. Decompresses every data block; this is the check `--list` cannot
	#    make. Row count is parsed out of the COPY blocks in the decompressed stream, so
	#    it counts rows that are ACTUALLY IN THE FILE — not TABLE DATA headings, which
	#    pg_dump emits for empty tables too (measured).
	local dataerr
	dataerr="$(mktemp)"
	rows="$(tbd_ct_i pg_restore --data-only -f - <"$file" 2>"$dataerr" |
		awk '/^COPY .* FROM stdin;$/{inc=1;next} inc&&/^\\\.$/{inc=0;next} inc{n++} END{print n+0}')"
	rc="${PIPESTATUS[0]}"
	if [ "$rc" -ne 0 ]; then
		echo "VERIFY FAIL: '$file' — \`pg_restore --data-only\` exited $rc while reading the archive body." >&2
		echo "             The file is TRUNCATED or CORRUPT. Note that \`pg_restore --list\` PASSES on both" >&2
		echo "             of those (measured) — this is the check that catches them." >&2
		sed 's/^/             /' "$dataerr" >&2
		rm -f "$dataerr"
		return 1
	fi
	rm -f "$dataerr"

	if [ -z "$rows" ] || [ "$rows" -lt "$min_rows" ]; then
		echo "VERIFY FAIL: '$file' restored cleanly but contains $rows data row(s), below the required minimum of $min_rows." >&2
		echo "             A schema-only or all-empty archive is not a backup of a live database." >&2
		echo "             (If backing up a genuinely empty database is intended, pass --min-rows 0.)" >&2
		return 1
	fi

	printf '%s' "$rows"
	return 0
}

# Total live rows across user tables, counted in the DATABASE — the cross-check partner
# to tbd_verify_dump's count, which is taken from the FILE. Exact counts, not estimates:
# pg_class.reltuples is stale between ANALYZEs and would make this comparison a lie.
tbd_count_db_rows() {
	local db="$1"
	tbd_ct psql -U "$TBD_DB_USER" -d "$db" -tAc "
		SELECT COALESCE(sum(cnt),0) FROM (
			SELECT (xpath('/row/c/text()',
				query_to_xml(format('SELECT count(*) AS c FROM %I.%I', schemaname, relname),
				false, true, '')))[1]::text::bigint AS cnt
			FROM pg_stat_user_tables
		) t;" 2>/dev/null | tr -d '[:space:]'
}

tbd_database_exists() {
	local db="$1" out
	out="$(tbd_ct psql -U "$TBD_DB_USER" -d postgres -tAc \
		"SELECT 1 FROM pg_database WHERE datname = '$db';" 2>/dev/null | tr -d '[:space:]')"
	[ "$out" = "1" ]
}
