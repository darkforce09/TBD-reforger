#!/usr/bin/env bash
# shellcheck shell=bash
# gate-grep.sh — the four-outcome static-check helpers for scripts/mod gates (T-556).
#
# ── Why this file exists, and why it is a LIBRARY rather than four copies ─────────────
#
# T-216 fixed exactly this defect in scripts/verify-t180-coherency.sh in wave 5, inline.
# The fix did not propagate: every scripts/mod/verify-t*.sh written afterwards was born
# with the same two holes. This file is the propagation mechanism — one implementation,
# sourced by every mod gate, so the next gate cannot be born broken by copy-paste.
#
# ── HOLE 1: `rg` is not a dependency this repo may have ──────────────────────────────
#
# MEASURED 2026-07-27. `ripgrep` is installed NOWHERE — not in the dev container, not on
# the host, no rpm. `command -v rg` succeeds in an agent shell ONLY because Claude Code
# injects a shell FUNCTION named `rg` that routes to the agent binary's bundled copy:
#
#   type rg                   -> "rg is a function"
#   bash -c 'command -v rg'   -> ABSENT     (functions are not exported to subshells)
#
# So an `rg`-based gate returns a different verdict depending on WHO invoked it. That is
# the unpinned-external-tool failure `.cursor/rules/acceptance-gates-reproducible.mdc`
# rule 1 exists to forbid. `grep` replaces it because grep is present in the container,
# on the host, and on every CI runner — this is a dependency REMOVED, not asserted.
# A `command -v rg || fail` preflight would have been honest and would also have made
# these gates unrunnable on the host, which is where half of them have to run (T-216).
#
# The patterns are unchanged and mean the same thing in ERE: `\(`, `\[`, `|` and
# `[[:space:]]` are identical in both engines, and every call names an explicit file, so
# ripgrep's recursion and gitignore defaults were never in play. `--fixed-strings`
# becomes `-F` (gate_*'s `-F` flag), which is the same literal match.
#
# ── HOLE 2: a boolean cannot carry four outcomes ─────────────────────────────────────
#
# `if rg PAT FILE; then fail; fi` collapses four distinct results into two:
#
#   exit 0    match found          -> ban violated       -> correctly FAILED
#   exit 1    no match             -> ban holds          -> correctly passed
#   exit 2    TARGET FILE MISSING  -> check never ran    -> printed OK
#   exit 127  SEARCH TOOL ABSENT   -> check never ran    -> printed OK
#
# The last two are this program's signature defect — a tool reporting success over an
# input it never examined — living inside the scripts written to catch it. Both fail
# CLOSED here, and each names its own cause, because "the pin is genuinely gone", "the
# file moved" and "the check could not run" send a reader to three different places.
#
# NOTE on `grep -q`: with -q, GNU grep exits 0 on the first match EVEN IF it also hit a
# missing-file error. So existence must be checked separately — grep's status alone is
# not enough. That is why every helper below stats its targets first.
#
# ── Contract ─────────────────────────────────────────────────────────────────────────
#
#   gate_ban      "<message>" [-F] [-i] "<pattern>" <file>...   pattern must NOT appear
#   gate_require  "<message>" [-F] [-i] "<pattern>" <file>...   pattern MUST appear
#   gate_ban_str  "<message>" [-F] [-i] "<pattern>" "<subject>" in-memory subject
#   gate_require_str …                                          in-memory subject
#
# Default engine is `grep -E` (ERE). `-F` makes the pattern literal; `-i` case-folds.
# Return 0 = the check ran and held. Return 1 = FAIL, with the reason already printed on
# stdout. Callers decide whether to `return 1` immediately or accumulate a FAIL flag.

# Parse the optional [-F] [-i] flags into the caller's `flags` array. Internal.
_gate_flags() {
	_GATE_FLAGS=(-E)
	while :; do
		case "${1:-}" in
		-F)
			_GATE_FLAGS=(-F)
			shift
			;;
		-i)
			_GATE_FLAGS+=(-i)
			shift
			;;
		*) break ;;
		esac
	done
	_GATE_SHIFT=$#
}

# Run grep over files and echo its raw exit status. Never collapses to a boolean.
_gate_status_file() {
	local status=0
	grep "${_GATE_FLAGS[@]}" -- "$@" >/dev/null 2>&1 || status=$?
	printf '%s' "$status"
}

# Run grep over an in-memory subject and echo its raw exit status.
_gate_status_str() {
	local pat="$1" subject="$2" status=0
	printf '%s\n' "$subject" | grep "${_GATE_FLAGS[@]}" -- "$pat" >/dev/null 2>&1 || status=$?
	printf '%s' "$status"
}

# Every target must exist before any comparison is attributed to it.
_gate_files_present() {
	local msg="$1" what="$2"
	shift 2
	local f
	for f in "$@"; do
		if [ ! -f "$f" ]; then
			echo "FAIL: $msg — target file missing: $f"
			echo "      The $what could not run. A moved or deleted file must not read as a clean result."
			return 1
		fi
	done
	return 0
}

_gate_tool_fail() {
	local msg="$1" what="$2" status="$3"
	echo "FAIL: $msg — grep exited $status"
	case "$status" in
	127) echo "      The search tool is ABSENT. Refusing to report OK on a $what that did not execute." ;;
	2) echo "      grep reported a read/pattern error. Refusing to report OK on a $what that did not execute." ;;
	*) echo "      Unexpected grep status. Refusing to report OK on a $what that did not execute." ;;
	esac
	return 1
}

gate_ban() {
	local msg="$1"
	shift
	_gate_flags "$@"
	shift $(($# - _GATE_SHIFT))
	local pat="$1"
	shift
	_gate_files_present "$msg" "ban" "$@" || return 1
	local status
	status="$(_gate_status_file "$pat" "$@")"
	case "$status" in
	0)
		echo "FAIL: $msg"
		return 1
		;;
	1) return 0 ;; # no match — the ban holds, and we know it holds because the search ran
	*) _gate_tool_fail "$msg" "ban" "$status" ;;
	esac
}

gate_require() {
	local msg="$1"
	shift
	_gate_flags "$@"
	shift $(($# - _GATE_SHIFT))
	local pat="$1"
	shift
	_gate_files_present "$msg" "pin" "$@" || return 1
	local status
	status="$(_gate_status_file "$pat" "$@")"
	case "$status" in
	0) return 0 ;;
	1)
		echo "FAIL: $msg"
		return 1
		;;
	*) _gate_tool_fail "$msg" "pin" "$status" ;;
	esac
}

# gate_probe_str [-F] [-i] "<pattern>" "<subject>" — echo grep's RAW exit status.
#
# For COMPOUND conditions, where neither "match" nor "no match" is a failure on its own
# and the two must be combined (`grep A && ! grep B`). Such a chain short-circuits to
# "clean" the moment one of its greps cannot run, which is the fail-open shape again in
# a costume. The caller reads the raw status and must still treat anything above 1 as a
# check that did not execute.
gate_probe_str() {
	_gate_flags "$@"
	shift $(($# - _GATE_SHIFT))
	_gate_status_str "$1" "$2"
}

# gate_probe_file [-F] [-i] "<pattern>" <file>... — echo grep's RAW exit status.
#
# The file twin of gate_probe_str, for callers whose failure message has to carry run
# context a generic helper cannot know (per-run log assertions, loop counters). Status 2
# covers "target file missing" here rather than a separate stat, because for a runtime
# log the two questions collapse: a log the gate could not read is a run it did not
# examine either way. Anything above 1 is a check that did not execute.
gate_probe_file() {
	_gate_flags "$@"
	shift $(($# - _GATE_SHIFT))
	_gate_status_file "$@"
}

gate_ban_str() {
	local msg="$1"
	shift
	_gate_flags "$@"
	shift $(($# - _GATE_SHIFT))
	local pat="$1" subject="$2"
	local status
	status="$(_gate_status_str "$pat" "$subject")"
	case "$status" in
	0)
		echo "FAIL: $msg"
		return 1
		;;
	1) return 0 ;;
	*) _gate_tool_fail "$msg" "ban" "$status" ;;
	esac
}

gate_require_str() {
	local msg="$1"
	shift
	_gate_flags "$@"
	shift $(($# - _GATE_SHIFT))
	local pat="$1" subject="$2"
	local status
	status="$(_gate_status_str "$pat" "$subject")"
	case "$status" in
	0) return 0 ;;
	1)
		echo "FAIL: $msg"
		return 1
		;;
	*) _gate_tool_fail "$msg" "pin" "$status" ;;
	esac
}
