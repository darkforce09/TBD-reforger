#!/usr/bin/env bash
# remote-log-grep.sh — assert a TBD dedicated-server console.log shows a HEALTHY boot.
#
# Usage:
#   bash scripts/mod/remote-log-grep.sh                # fetch from $TBD_SSH_HOST, then check
#   bash scripts/mod/remote-log-grep.sh --file <path>  # check a LOCAL log file (no SSH)
#   bash scripts/mod/remote-log-grep.sh --selftest     # prove the verdict logic can FAIL
#
# ── WHAT THIS SCRIPT GOT WRONG FOR A MONTH, AND WHY THE PATTERNS LOOK LIKE THIS (T-606) ────
#
# Every check below used to be pinned to a whole English sentence copied out of a `Print`.
# Three of them had since been reworded or deleted, and the failures were not loud — they were
# the WRONG WAY ROUND, which is worse:
#
#   "Mission loaded"      The current build emits this string in exactly ONE place:
#                         TBD_FrameworkManager.c:488, `"[TBD] Mission loaded but invalid —
#                         staying in LOADING."`, at LogLevel.ERROR. So the old required-string
#                         check passed ONLY when the mission had FAILED to load, and reported
#                         MISSING on every healthy server. A health check that is satisfied by
#                         the error case is not a weak check, it is an inverted one.
#
#   "built slot spawn"    Deleted outright. The per-slot line is now
#                         `[TBD][Slots] Slot-1 <id> (<faction:squad:role:n>) kit <kit> at <xyz>`.
#
#   "spawn requested"     Deleted outright — it exists in NO `Print` anywhere in apps/mod.
#                         It was ANDed into the script's only `exit 0` branch, so this script
#                         could not return success on any log, ever. `deploy-staging.sh`'s V6
#                         step runs it as its last command, so V6 could not pass either.
#
# Measured against a real boot (`world-boot.sh --mission=slot-loadout-coverage`, 2026-07-31,
# 423-line console.log, mission validated PASS, reached LOBBY): the old required list reported
# 2 of 3 MISSING and the script exited 1. The server was fine.
#
# ── THE RULE THIS FILE NOW FOLLOWS: PIN THE PREFIX, NEVER THE SENTENCE ─────────────────────
#
# This is not a one-off cleanup. T-607 documents the same defect in staging's pass criteria,
# and wave 76 REINTRODUCED it: T-605 reworded a `Print` that T-604's detector quoted verbatim,
# and the two merged in the same wave with the string dead. A detector that breaks whenever
# someone edits a `Print` will be wrong again within a wave, so:
#
#   * Match the TAG and the EVENT KEY — `[TBD][Mission] loaded id=`. The tag vocabulary is
#     enumerable from one place (`TBD_Log.c` CH_* constants) and the `k=v` shape is structural.
#   * Never match the prose after it. Names, counts, `source=`, and every summary sentence are
#     EXPECTED to vary; `[TBD][Slots] loadout settle complete` has been reworded once already.
#   * For "is this even the right build", count the FORMAT, not any sentence (see BUILD FORMAT).
#
# ── BUILD FORMAT: the one structural fact, and why it is not an exact number ────────────────
#
# `tbd-framework` is on the Workshop, unlisted, under the SAME id as the local gproj GUID, at a
# stale version 1.0.1. A `-config`-only server silently downloads and runs THAT, and it looks
# healthy — it registers a room and reaches LOBBY on months-old script. June's build logs flat
# `[TBD] …` lines with no subsystem tag; every line the current build emits is `[TBD][Sub] …`.
# So the discriminator is `grep -c '\[TBD\]\['`: stale 1.0.1 emits EXACTLY 0, any current build
# emits many.
#
# Deliberately a THRESHOLD, not an equality. The exact count is mission-dependent and rots:
# 108 was measured on 18-slot msn_8f3a2c (wave 75), 147 on 7-slot slot-loadout-coverage
# (2026-07-31, and T-608 measured the same 147 on the checkout). Two different missions, two
# different "correct" numbers, and the docs already carried 108 and 109 in different paragraphs
# as if one of them were wrong. Neither was. Asserting any single number re-creates the defect
# this file exists to remove, so the HARD gate is 0-vs-nonzero (structural, cannot rot) and the
# floor below it is advisory only.
#
# ── ENGINE NOTE: these patterns must mean the same thing in ugrep AND GNU grep ──────────────
#
# `grep` is ugrep 7.5.0 in an interactive agent shell and GNU grep 3.8 under `bash script.sh`.
# They disagree on bare `{}` in an ERE. Every pattern here uses only `\[ \] . * |`, which are
# identical in both engines, and both were tested. Do not introduce `{n,m}`, `\d`, `\b` or any
# PCRE shorthand without re-testing under both.
#
# ── EXIT CODES ─────────────────────────────────────────────────────────────────────────────
#   0  HEALTHY   current build, mission loaded, slot bodies built, reached LOBBY
#   1  FAIL      a required structural line is missing, or an error class is present
#   2  PARTIAL   boot is healthy but no player has joined yet (no slot assignment in log)
#   3  ENVIRONMENT  the log could not be obtained, or the invocation was wrong — either way
#                   no log was examined, so this says NOTHING about the mod
#
# NOTE the deviation from world-boot.sh / compile.sh, where 2 means "usage". Here 2 was already
# PARTIAL before this rewrite and `deploy-staging.sh` V6 inherits it, so usage errors go to 3
# instead. They must not land on 2: a mistyped flag returning "PARTIAL" would read as
# "server healthy, nobody joined yet" — a tool reporting a verdict over an input it never
# examined, which is the exact failure this script was rewritten to stop committing.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/paths.sh
source "$SCRIPT_DIR/lib/paths.sh"
# shellcheck source=lib/gate-grep.sh
source "$SCRIPT_DIR/lib/gate-grep.sh"

# ── The check vocabulary. ONE definition, used by both the remote and the --file path ───────
#
# Each pattern below is the ENTIRE dependency this script has on the mod's output. The comment
# above each one records what is deliberately left VARIABLE after the prefix, so the next person
# to edit a `Print` can tell at a glance whether they are about to break a detector.
#
# These are ERE and are fed to gate-grep.sh, whose default engine is already `grep -E` — do NOT
# pass `-E` to gate_probe_file. Its flag parser only consumes `-F`/`-i`, so a stray `-E` lands
# after the `--` and grep reads it as a FILENAME, exiting 2. That fails closed here (status > 1
# is "the check did not execute"), but it fails every check at once and the message is obscure.

PAT_TAGGED='\[TBD\]\['
PAT_MISSION='\[TBD\]\[Mission\] loaded id='
PAT_SLOTS='\[TBD\]\[Slots\] Slot-'
# Both stage formats. TBD_FrameworkManager.SetStage emits the structured
# `[TBD][Stage] LOADING -> LOBBY` (TBD_Log.Stage) AND keeps a legacy `[TBD] Stage → LOBBY`
# line verbatim because README.md and STAGING-SERVER.md quote it. Accepting either means
# this check survives whichever one is eventually retired — and it never has to carry the
# non-ASCII arrow, which is a locale hazard in its own right.
PAT_LOBBY='\[TBD\]\[Stage\].*LOBBY|\[TBD\] Stage .*LOBBY'
# The loadout tag is [Slot]. It is NOT [Player] — that string appears in no `Print` anywhere,
# though the T-068.14 spec and TBD_LoadoutEquipComponent.c:17 both still name it. Grepping for
# [Player] returns zero lines on a fully working loadout pass, which reads exactly like "the
# loadout never applied" — the single thing T-068.14 exists to confirm.
PAT_LOADOUT='\[TBD\]\[Loadout\]\[Slot\]'
# Player-join evidence. `assigned slot` is live (TBD_SpawnManager.c:675); the old companion
# `spawn requested` is deleted and must not be re-added to this condition.
PAT_ASSIGNED='\[TBD\] SpawnManager: assigned slot'
# Error classes. Kept broad on purpose: these are ENGINE strings, not our prose, so they are
# far more stable than anything we Print.
PAT_ERRORS='Can.t compile|Unknown class .TBD_|RequestSpawn failed'

# Display extraction: both log formats plus the error classes. `built slot spawn` and
# `spawn requested` are gone from this list too — showing an operator a pattern that can never
# match teaches them to expect a line that no longer exists.
PAT_EXTRACT='\[TBD\]|assigned slot|Can.t compile|RequestSpawn failed|Unknown class'

# Advisory floor for the tagged-line count. NOT a pass criterion — see BUILD FORMAT above.
MIN_TAGGED="${TBD_MIN_TAGGED:-20}"

usage() {
	sed -n '2,8p' "$0"
	# 3, not 2 — see the EXIT CODES note above. 2 is PARTIAL (a real, healthy verdict).
	exit 3
}

env_fail() {
	echo "ENVIRONMENT: $*" >&2
	echo "The log was never examined, so this says NOTHING about the mod." >&2
	exit 3
}

# ── The verdict, over a local file. The remote path fetches, then calls exactly this. ───────
#
# Unified deliberately: this used to run six separate `ssh grep` round trips with the patterns
# written out inline at each call site, which is how three of them drifted out of sync with the
# code in the first place. One implementation cannot drift against itself.
check_log() {
	local log="$1" fail=0 status

	if [ ! -f "$log" ]; then
		env_fail "no such log file: $log"
	fi

	echo "Log: $log"
	echo "---"
	grep -E "$PAT_EXTRACT" "$log" 2>/dev/null | tail -80
	echo "---"

	# 1. BUILD FORMAT — hard gate, structural, independent of any sentence.
	local tagged
	tagged="$(grep -c "$PAT_TAGGED" "$log" 2>/dev/null || true)"
	[ -n "$tagged" ] || tagged=0
	echo "[TBD][ tagged lines: $tagged"
	if [ "$tagged" -eq 0 ]; then
		echo "FAIL: STALE BUILD — zero '[TBD][' lines."
		echo "      Workshop 1.0.1 logs flat '[TBD] …' with no subsystem tag; the current"
		echo "      build tags every line. A '-config'-only server downloads that stale copy"
		echo "      and looks healthy while running months-old script. Boot with -addonsDir,"
		echo "      or use scripts/mod/run-playtest-server.sh which asserts the local addon won."
		fail=1
	elif [ "$tagged" -lt "$MIN_TAGGED" ]; then
		echo "WARN: only $tagged tagged lines (advisory floor $MIN_TAGGED)."
		echo "      Measured healthy boots: 108 (18-slot msn_8f3a2c), 147 (slot-loadout-coverage)."
		echo "      Not a failure — the count is mission-dependent — but a boot this quiet"
		echo "      usually means the mod stopped early. The named checks below are the verdict."
	fi

	# 2. REQUIRED STRUCTURAL LINES — tag + event key only, never the prose after it.
	#
	# Passed as explicit arguments rather than packed into delimited strings: PAT_LOBBY
	# legitimately CONTAINS `|` (it accepts both stage formats), so any single-character
	# field separator collides with ERE alternation and silently truncates the pattern to
	# its first branch. A detector quietly losing half its pattern is the same class of bug
	# this file exists to fix, so there is no parsing here at all.
	require_line() {
		local label="$1" pat="$2" varies="$3" st
		st="$(gate_probe_file "$pat" "$log")"
		case "$st" in
		0)
			echo "ok   $label"
			return 0
			;;
		1)
			echo "MISSING: $label"
			echo "         pattern: $pat"
			echo "         Everything after this prefix is expected to vary: $varies"
			return 1
			;;
		*)
			echo "FAIL: $label — grep exited $st; the check did not execute."
			return 1
			;;
		esac
	}

	require_line "mission document loaded" "$PAT_MISSION" \
		"name, slot count, source=" || fail=1
	require_line "slot bodies materialized" "$PAT_SLOTS" \
		"slot id, faction:squad:role, kit, coordinates" || fail=1
	require_line "reached LOBBY" "$PAT_LOBBY" \
		"nothing — this is a state-machine edge, not prose" || fail=1

	# 3. ERROR CLASSES — engine strings, stable.
	status="$(gate_probe_file "$PAT_ERRORS" "$log")"
	case "$status" in
	0)
		echo "FAIL: compile / unknown-class / spawn errors present:"
		grep -E "$PAT_ERRORS" "$log" 2>/dev/null | head -10
		fail=1
		;;
	1) echo "ok   no compile or spawn-logic errors" ;;
	*)
		echo "FAIL: error scan exited $status; the check did not execute."
		fail=1
		;;
	esac

	# 4. LOADOUT PASS — informational. Absent is legitimate (a mission may author no loadouts),
	#    so this must not fail the boot; it is here because "did the kit actually get applied"
	#    is the question operators come to this script with.
	status="$(gate_probe_file "$PAT_LOADOUT" "$log")"
	if [ "$status" = "0" ]; then
		echo "ok   loadout pass ran ($(grep -c "$PAT_LOADOUT" "$log" 2>/dev/null || echo 0) [Loadout][Slot] lines)"
	else
		echo "note no [TBD][Loadout][Slot] lines — legitimate if the mission authors no loadouts."
		echo "     (Do NOT grep [TBD][Loadout][Player]; no Print emits it. The tag is [Slot].)"
	fi

	[ "$fail" -eq 0 ] || { echo "VERDICT: FAIL"; return 1; }

	# 5. Player evidence decides HEALTHY vs PARTIAL. Both are non-failures.
	status="$(gate_probe_file "$PAT_ASSIGNED" "$log")"
	if [ "$status" = "0" ]; then
		echo "VERDICT: PASS — boot healthy and at least one player was seated."
		return 0
	fi

	echo "VERDICT: PARTIAL — boot healthy, no player has joined yet (join a client to finish V6)."
	return 2
}

# ── Selftest: prove the verdict logic can FAIL, not just pass ───────────────────────────────
#
# A gate that has only ever been seen to pass is not a gate. This builds three synthetic logs
# and asserts the verdict for each, including the two shapes that actually shipped broken.
selftest() {
	local tmp rc bad=0
	tmp="$(mktemp -d "${TMPDIR:-/tmp}/tbd-logrep-selftest.XXXXXX")"
	trap 'rm -rf "$tmp"' RETURN

	# (a) stale Workshop 1.0.1 — flat format, no subsystem tag. MUST fail.
	{
		echo "SCRIPT : [TBD] Mission loaded from backend: something"
		echo "SCRIPT : [TBD] SpawnManager: built slot spawn"
		echo "SCRIPT : [TBD] Stage → LOBBY"
	} >"$tmp/stale.log"

	# (b) current build, healthy, no player joined. MUST be PARTIAL (2).
	{
		echo "SCRIPT : [TBD][Mission] loaded id=msn_x name='N' slots=7 source=profile"
		echo "SCRIPT : [TBD][Slots] Slot-1 s (a:b:c:0) kit kit:x at <1, 2, 3>"
		echo "SCRIPT : [TBD][Loadout][Slot] slot=a:b:c:0 loadout pass complete gear=1/1 cargo=0/0"
		echo "SCRIPT : [TBD][Stage] LOADING -> LOBBY"
	} >"$tmp/healthy.log"

	# (c) the INVERTED case that shipped: mission failed to load, and the old check passed on it.
	{
		echo "SCRIPT (E): [TBD] Mission loaded but invalid — staying in LOADING."
		echo "SCRIPT : [TBD][Validate] mission result=FAIL errors=3 warnings=0"
	} >"$tmp/invalid.log"

	expect() {
		local name="$1" want="$2" file="$3"
		check_log "$file" >/dev/null 2>&1 && rc=0 || rc=$?
		if [ "$rc" = "$want" ]; then
			echo "ok   selftest $name -> $rc"
		else
			echo "FAIL selftest $name -> $rc (expected $want)"
			bad=1
		fi
	}

	expect "stale-1.0.1-must-fail" 1 "$tmp/stale.log"
	expect "healthy-no-player-is-partial" 2 "$tmp/healthy.log"
	expect "mission-invalid-must-fail" 1 "$tmp/invalid.log"

	[ "$bad" -eq 0 ] || { echo "SELFTEST: FAIL"; return 1; }
	echo "SELFTEST: PASS"
	return 0
}

# ── Argument handling ───────────────────────────────────────────────────────────────────────
MODE=remote
FILE=""
while [ $# -gt 0 ]; do
	case "$1" in
	--file)
		MODE=file
		FILE="${2:-}"
		[ -n "$FILE" ] || usage
		shift 2
		;;
	--file=*)
		MODE=file
		FILE="${1#*=}"
		shift
		;;
	--selftest)
		MODE=selftest
		shift
		;;
	-h | --help) usage ;;
	*)
		echo "unknown argument: $1" >&2
		usage
		;;
	esac
done

case "$MODE" in
selftest)
	selftest
	exit $?
	;;
file)
	check_log "$FILE"
	exit $?
	;;
esac

# ── Remote path: fetch the log ONCE, then run the identical local verdict ───────────────────
ENV_FILE="$DEPLOY_ENV"
# shellcheck source=/dev/null
[ -f "$ENV_FILE" ] && source "$ENV_FILE"

: "${TBD_SSH_HOST:?Set TBD_SSH_HOST in scripts/deploy/deploy.env}"
: "${TBD_PROFILE_DIR:?Set TBD_PROFILE_DIR in scripts/deploy/deploy.env}"

ssh_cmd() {
	if [ -n "${TBD_SSH_PASS:-}" ]; then
		sshpass -p "$TBD_SSH_PASS" ssh -o StrictHostKeyChecking=no "$TBD_SSH_HOST" "$@"
	elif [ -n "${TBD_SSH_IDENTITY_FILE:-}" ]; then
		ssh -i "$TBD_SSH_IDENTITY_FILE" -o StrictHostKeyChecking=no "$TBD_SSH_HOST" "$@"
	else
		ssh -o StrictHostKeyChecking=no "$TBD_SSH_HOST" "$@"
	fi
}

FIND_LOG="
ls -td '$TBD_PROFILE_DIR'/logs/logs_* '$TBD_PROFILE_DIR'/profile/logs/logs_* 2>/dev/null | while read -r d; do
  [ -f \"\$d/console.log\" ] && echo \"\$d/console.log\" && exit 0
done
exit 1
"

REMOTE_LOG="$(ssh_cmd "bash -lc $(printf '%q' "$FIND_LOG")" 2>/dev/null || true)"
if [ -z "$REMOTE_LOG" ]; then
	env_fail "no console.log found under $TBD_PROFILE_DIR (logs/ or profile/logs/) on $TBD_SSH_HOST"
fi

LOCAL_COPY="$(mktemp "${TMPDIR:-/tmp}/tbd-remote-log.XXXXXX")"
trap 'rm -f "$LOCAL_COPY"' EXIT

# Fetching once and asserting locally is not just fewer round trips: it means the remote and
# --file paths cannot drift apart, and an SSH failure mid-run can no longer read as "no match"
# on an individual check (which is the fail-open shape this repo keeps finding).
if ! ssh_cmd "cat \"$REMOTE_LOG\"" >"$LOCAL_COPY" 2>/dev/null; then
	env_fail "could not read $REMOTE_LOG from $TBD_SSH_HOST"
fi
if [ ! -s "$LOCAL_COPY" ]; then
	env_fail "$REMOTE_LOG on $TBD_SSH_HOST is empty"
fi

echo "Remote log: $TBD_SSH_HOST:$REMOTE_LOG"
check_log "$LOCAL_COPY"
exit $?
