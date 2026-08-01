#!/usr/bin/env bash
# mcp-wb-logs.sh — grep the latest Workbench Play console.log for TBD spawn diagnostics and
# assert the spawn pipeline actually ran. Run after MCP wb_play (and optional sleep) —
# enfusion-mcp has no wb_log tool, so this is the read-back half of a wb_play loop.
#
# Usage:
#   mcp-wb-logs.sh [extended-grep-pattern]     # latest Workbench log; pattern filters DISPLAY only
#   mcp-wb-logs.sh --file <path> [pattern]     # verdict over a specific log file (no Workbench)
#   mcp-wb-logs.sh --selftest                  # prove the verdict logic can FAIL
#
# ── WHAT THIS SCRIPT GOT WRONG, AND WHY THE PATTERNS LOOK LIKE THIS (T-612, after T-606) ────
#
# The old PASS branch required `spawn requested` and the old PARTIAL branch required
# `built slot spawn`. BOTH strings are deleted — neither exists in any `Print` in apps/mod —
# so neither branch could fire: this script had NO REACHABLE exit 0 and no reachable exit 2.
# Every verdict it could produce was FAIL. On a fully healthy session with a seated player it
# printed "FAIL: expected TBD spawn lines missing." — while a log written by the stale June
# build (which still emits both dead strings, flat-tagged) would have PASSED. A checker that
# fails every current log and passes a stale one is inverted, not merely stale.
#
# THE RULE (T-606, remote-log-grep.sh:34): pin the TAG and the EVENT KEY — `[TBD][Slots] Slot-`
# — never the sentence after it. Names, counts and coordinates are EXPECTED to vary; a detector
# that breaks whenever someone rewords a `Print` will be wrong again within a wave. The tag
# vocabulary is enumerable from TBD_Log.c CH_* constants plus the fixed SpawnManager prefixes.
#
# ── EXIT CODES ──────────────────────────────────────────────────────────────────────────────
#   0  PASS      slot bodies built AND a player was assigned a slot
#   2  PARTIAL   slot bodies built; no player has deployed yet (join/deploy to finish)
#   1  FAIL      required structural line missing, error class present, or stale/absent build
#   3  ENVIRONMENT  no log to examine, or bad invocation — the log was never examined, so this
#                   says NOTHING about the mod. NOT 1: "no Workbench log directory" misread as
#                   "spawn pipeline broken" sends an agent auditing a mod that never ran. NOT 2:
#                   2 is already a real verdict (PARTIAL) here. Same deviation, same reason as
#                   remote-log-grep.sh's EXIT CODES note.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/gate-grep.sh
source "$SCRIPT_DIR/lib/gate-grep.sh"

PROTON_LOG_DIR="$HOME/.local/share/Steam/steamapps/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/logs"
NATIVE_LOG_DIR="$HOME/Documents/Games/ArmaReforgerWorkbench/logs"

# ── The check vocabulary — a HAND-SYNCED COPY of remote-log-grep.sh's. NOT one definition. ──
#
# T-619 corrected this header, because what it used to say — "shared with remote-log-grep.sh.
# ONE definition per pattern" — was a claim with no mechanism behind it. Nothing is shared: these
# are two independent assignments in two files, and "one definition" was a promise that the next
# editor would keep them equal, not a thing the code enforced. It was already false when it was
# written. Five of the six patterns matched; PAT_ASSIGNED did not, and had not since T-614
# broadened the sibling without a way to notice this copy existed.
#
# THE DRIFT, measured 2026-08-01:
#     here (before)          '\[TBD\] SpawnManager: assigned slot'                    flat only
#     remote-log-grep.sh:136 '\[TBD\]\[Spawn\].*assigned|\[TBD\] SpawnManager: assigned'  both
# TBD_SpawnManager.c is 70 of 74 Prints onto the tagged `[TBD][Sub]` format and `:675` is one of
# the last flat stragglers, so on the day it is reworded this script reports PARTIAL — "no player
# has deployed yet" — over a boot where a player really was seated. Measured: exit 2 on a
# tagged-seat log before this change, exit 0 after.
#
# WHAT WOULD MAKE THE OLD CLAIM TRUE, and why it is not done here: lift these six into
# scripts/mod/lib/ (next to gate-grep.sh) and `source` it from BOTH files. That is a two-line
# change to remote-log-grep.sh, which this slice does not own — and a lib sourced by only one of
# two copies is not one definition, it is the same two copies plus a file, which would make this
# header MORE misleading rather than less. So the fix here is one-sided on purpose and the header
# now says so. Until that lib exists, EDIT BOTH FILES: this one and remote-log-grep.sh:102-144.
#
# ERE only, restricted to `\[ \] . * |` + POSIX classes: identical meaning under ugrep 7.5.0
# (interactive agent shells) and GNU grep 3.8 (bash script.sh). Do not add `{n,m}`, `\d`, `\b`
# or any PCRE shorthand without re-testing under both engines.
#
# Current-build discriminator. Every line the current build emits is `[TBD][Sub] …`; the stale
# June build logs flat `[TBD] …` only. 0-vs-nonzero on purpose — T-606 measured 147 and 155
# tagged lines on two goldens and +47 drift on an unchanged mission, so any pinned count rots.
PAT_TAGGED='\[TBD\]\['
PAT_MISSION='\[TBD\]\[Mission\] loaded id='
PAT_SLOTS='\[TBD\]\[Slots\] Slot-'
# Player-join evidence (TBD_SpawnManager.c:675). BOTH FORMATS, byte-identical to
# remote-log-grep.sh:136 — see the drift note in the header above.
#
# Matching the TAG plus the event key `assigned`, never the sentence after it, is this file's own
# stated rule (see THE RULE, T-606) and the old value broke it: it pinned the legacy flat prefix
# plus three words of prose. Checked against every Print in TBD_SpawnManager.c — no other
# `[TBD][Spawn]` line contains the word (the failure case reads "could not be seated"), and the
# only other `assigned slot` in the mod is TBD_SpectatorHost.c's refusal text, which carries the
# `[TBD][spectator]` tag and so cannot match this.
#
# The old companion `spawn requested` is deleted and must never be re-added to this condition —
# it is what made exit 0 unreachable.
PAT_ASSIGNED='\[TBD\]\[Spawn\].*assigned|\[TBD\] SpawnManager: assigned'
# Engine strings, not our prose — far more stable than anything we Print.
PAT_ERRORS='Can.t compile|Unknown class .TBD_|RequestSpawn failed'
# The loadout tag is [Slot] (also [TestNPC] for the dev harness). It is NOT [Player] — that
# string appears in no `Print` anywhere; grepping for it returns nothing on a working pass.
PAT_LOADOUT='\[TBD\]\[Loadout\]\[Slot\]'

# Display extraction only — the verdict below never depends on this. `SpawnLogic` catches the
# engine naming TBD_SCR_MenuSpawnLogic in its own diagnostics; dead strings removed (showing an
# operator a pattern that can never match teaches them to expect a line that no longer exists).
DEFAULT_EXTRACT='\[TBD\]|SpawnLogic|assigned slot'

usage() {
	sed -n '2,9p' "$0"
	exit 3
}

env_fail() {
	echo "ENVIRONMENT: $*" >&2
	echo "The log was never examined, so this says NOTHING about the mod." >&2
	exit 3
}

# ── The verdict, over one log file. Both the Workbench-latest and --file paths call this. ───
check_log() {
	local log="$1" extract="$2" fail=0 status

	if [ ! -f "$log" ]; then
		env_fail "no such log file: $log"
	fi

	echo "Log: $log"
	echo "---"
	grep -E "$extract" "$log" 2>/dev/null | tail -60
	echo "---"

	# 1. BUILD FORMAT — hard gate, structural, independent of any sentence.
	local tagged
	tagged="$(grep -c "$PAT_TAGGED" "$log" 2>/dev/null || true)"
	[ -n "$tagged" ] || tagged=0
	echo "[TBD][ tagged lines: $tagged"
	if [ "$tagged" -eq 0 ]; then
		echo "FAIL: zero '[TBD][' subsystem-tagged lines — the current mod never logged."
		echo "      Flat '[TBD] …' lines only = a stale (June-era) build; none at all = the mod"
		echo "      is not loaded in this session. Either way the pipeline under test did not run."
		fail=1
	fi

	# 2. REQUIRED STRUCTURAL LINES — tag + event key only, never the prose after it.
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

	# 3. ERROR CLASSES.
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

	# 4. LOADOUT PASS — informational; a mission may legitimately author no loadouts.
	status="$(gate_probe_file "$PAT_LOADOUT" "$log")"
	if [ "$status" = "0" ]; then
		echo "ok   loadout pass ran ($(grep -c "$PAT_LOADOUT" "$log" 2>/dev/null || echo 0) [Loadout][Slot] lines)"
	else
		echo "note no [TBD][Loadout][Slot] lines — legitimate if the mission authors no loadouts."
		echo "     (Do NOT grep [TBD][Loadout][Player]; no Print emits it. The tag is [Slot].)"
	fi

	[ "$fail" -eq 0 ] || { echo "VERDICT: FAIL"; return 1; }

	# 5. Player evidence decides PASS vs PARTIAL. Both are non-failures.
	status="$(gate_probe_file "$PAT_ASSIGNED" "$log")"
	if [ "$status" = "0" ]; then
		echo "PASS: slot bodies built and a player was assigned a slot."
		return 0
	fi

	echo "PARTIAL: slot bodies built; no player has deployed yet."
	return 2
}

# ── Selftest: prove the verdict logic can FAIL, not just pass ───────────────────────────────
# Four synthetic logs, including the exact shape the OLD script passed (stale) and the exact
# shape it failed (healthy). A gate that has only ever been seen to pass is not a gate.
selftest() {
	local tmp rc bad=0
	tmp="$(mktemp -d "${TMPDIR:-/tmp}/tbd-wblogs-selftest.XXXXXX")"
	trap 'rm -rf "$tmp"' RETURN

	# (a) stale June build — flat tags, both dead strings present. The OLD script exited 0 on
	#     exactly this log (`assigned slot` + `spawn requested` both matched). MUST fail now.
	{
		echo "SCRIPT       : [TBD] Mission loaded from backend: Bridgehead at Levie"
		echo "SCRIPT       : [TBD] SpawnManager: built slot spawn blufor:Alpha:SL:0"
		echo "SCRIPT       : [TBD] Stage → LOBBY"
		echo "SCRIPT       : [TBD] SpawnManager: assigned slot blufor:Alpha:SL:0"
		echo "SCRIPT       : [TBD] SpawnManager: spawn requested"
	} >"$tmp/stale.log"

	# (b) current build, healthy, player seated. The OLD script exited 1 on exactly this log
	#     (no `spawn requested` anywhere). MUST pass now.
	{
		echo "SCRIPT       : [TBD][Mission] loaded id=msn_x name='N' slots=2 source=profile"
		echo "SCRIPT       : [TBD][Slots] Slot-1 blufor:Alpha:SL:0 (blufor:Alpha:SL:0) kit kit:rifleman_m16 at <4870, 135, 7760>"
		echo "SCRIPT       : [TBD][Loadout][Slot] slot=blufor:Alpha:SL:0 loadout pass complete gear=4/4 cargo=6/6"
		echo "SCRIPT       : [TBD][Stage] LOADING -> LOBBY"
		echo "SCRIPT       : [TBD] SpawnManager: assigned slot blufor:Alpha:SL:0 to player 1 at (4870,7760)"
	} >"$tmp/healthy.log"

	# (c) current build, healthy, nobody joined yet. MUST be PARTIAL (2).
	{
		echo "SCRIPT       : [TBD][Mission] loaded id=msn_x name='N' slots=2 source=profile"
		echo "SCRIPT       : [TBD][Slots] Slot-1 blufor:Alpha:SL:0 (blufor:Alpha:SL:0) kit kit:rifleman_m16 at <4870, 135, 7760>"
		echo "SCRIPT       : [TBD][Stage] LOADING -> LOBBY"
	} >"$tmp/healthy-nojoin.log"

	# (d) mission failed to load — the ERROR string is the ONLY `Mission loaded` in the codebase
	#     (TBD_FrameworkManager.c:488). It must satisfy nothing. MUST fail.
	{
		echo "SCRIPT    (E): [TBD] Mission loaded but invalid — staying in LOADING."
		echo "SCRIPT    (E): [TBD][Validate] mission result=FAIL errors=3 warnings=0"
	} >"$tmp/invalid.log"

	expect() {
		local name="$1" want="$2" file="$3"
		check_log "$file" "$DEFAULT_EXTRACT" >/dev/null 2>&1 && rc=0 || rc=$?
		if [ "$rc" = "$want" ]; then
			echo "ok   selftest $name -> $rc"
		else
			echo "FAIL selftest $name -> $rc (expected $want)"
			bad=1
		fi
	}

	expect "stale-build-must-fail" 1 "$tmp/stale.log"
	expect "healthy-with-player-passes" 0 "$tmp/healthy.log"
	expect "healthy-no-player-is-partial" 2 "$tmp/healthy-nojoin.log"
	expect "mission-invalid-must-fail" 1 "$tmp/invalid.log"

	[ "$bad" -eq 0 ] || { echo "SELFTEST: FAIL"; return 1; }
	echo "SELFTEST: PASS"
	return 0
}

# ── Argument handling. Bare [pattern] is kept for back-compat with every existing caller. ───
MODE=latest
FILE=""
PATTERN="$DEFAULT_EXTRACT"
while [ $# -gt 0 ]; do
	case "$1" in
	--selftest)
		MODE=selftest
		shift
		;;
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
	-h | --help) usage ;;
	*)
		PATTERN="$1"
		shift
		;;
	esac
done

case "$MODE" in
selftest)
	selftest
	exit $?
	;;
file)
	check_log "$FILE" "$PATTERN"
	exit $?
	;;
esac

latest_log_dir() {
	local d picked=""
	for d in "$PROTON_LOG_DIR" "$NATIVE_LOG_DIR"; do
		[ -d "$d" ] || continue
		picked="$(ls -td "$d"/logs_* 2>/dev/null | head -1)"
		[ -n "$picked" ] && echo "$picked" && return
	done
}

LATEST="$(latest_log_dir)"
if [ -z "$LATEST" ]; then
	env_fail "no Workbench log directory found (looked in $PROTON_LOG_DIR and $NATIVE_LOG_DIR)"
fi

check_log "$LATEST/console.log" "$PATTERN"
exit $?
