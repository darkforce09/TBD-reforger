#!/usr/bin/env bash
# T-162 hard gate: no .py files, and no NEW Python interpreter invocations in scripts/ or Makefile.
#
# ── WHY THIS FILE WAS REWRITTEN (T-620) ──────────────────────────────────────────────────────────
#
# This gate spent four waves reporting a confident half-truth. Three independent faults, all
# measured 2026-08-01, all of them the same underlying defect — a tool reporting success over an
# input it never examined:
#
#   1. IT WAS RED AND NOBODY WAS STOPPED BY IT. The two `.py` files it named were
#      scripts/{platform,mod}/slice-collisions.py — THE FACTORY'S OWN TOOLING, landed in 1e3ea1f6
#      the day the factory opened. T-620 ported them to `cargo xtask slice-collisions` (byte-identical
#      output on default/--check/--repack, diffed before deletion) and deleted both.
#
#   2. IT WAS IN NO CI JOB. `grep verify-no-python .github/workflows/*.yml` had no hits. It lived
#      only in this file, one Makefile target, and `make ci-local` — the local replay nothing runs
#      by default. It is now a step in `.github/workflows/ci.yml` AND a wave-gate step, because a
#      gate that only runs where nobody looks is decoration.
#
#   3. HALF OF IT PHYSICALLY COULD NOT FIRE — and this is the one that matters. Line 29 used to be:
#
#          HITS="$(rg -n 'python3|#!/usr/bin/env python' scripts/ Makefile ... || true)"
#
#      `rg` DOES NOT EXIST in this repository — not in the container, not on the host, no rpm. It
#      resolves in an agent shell only because Claude Code injects a shell FUNCTION of that name,
#      which is not exported to subshells. So under `bash scripts/verify-no-python.sh` the measured
#      output was, verbatim:
#
#          scripts/verify-no-python.sh: line 29: rg: command not found
#            OK (none)
#
#      `|| true` swallowed status 127 and the gate printed OK — not because there were no Python
#      invocations, but because IT NEVER LOOKED. It had never once run in the entire life of the
#      repository. This is the purest instance of this program's signature defect the run has found,
#      and it was living inside the gate written to enforce the ban.
#
# Everything below therefore FAILS CLOSED. `scripts/mod/lib/gate-grep.sh` (T-556) already reads
# grep's four outcomes correctly — 0 match / 1 no-match / 2 read error / 127 tool absent — so it is
# SOURCED here rather than re-derived, which is how hole 3 propagated in the first place.
#
# ── WHY THE SECOND CHECK IS A RATCHET, NOT A BAN ────────────────────────────────────────────────
#
# The moment the interpreter check could see, it found 15 shell files invoking python3 — including
# wave.sh, world-boot.sh, deploy-staging.sh and every verify-t4xx mod gate. That debt is real, it is
# four waves deep, and porting it is not this ticket. Freezing it in a committed inventory is: the
# list may only SHRINK, so nothing new arrives by default while the existing debt stays visible and
# dated instead of hidden behind a `|| true`. See scripts/python-inventory.txt.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

INVENTORY="scripts/python-inventory.txt"
SELF="scripts/verify-no-python.sh"

# The helper library is a hard dependency. Silently continuing with a weaker check under the same
# name is precisely the failure this gate exists to catch, so refuse instead.
LIB="scripts/mod/lib/gate-grep.sh"
if [ ! -f "$LIB" ]; then
	echo "FAIL: $LIB is missing — the four-outcome grep helpers are unavailable." >&2
	echo "      Refusing to run a weaker check under this gate's name." >&2
	exit 1
fi
# shellcheck source=scripts/mod/lib/gate-grep.sh
. "$LIB"

FAIL=0

# ─────────────────────────── 1. zero .py files ───────────────────────────

echo "==> find *.py (excl .git / node_modules / target / worktrees)"
PY_RC=0
PY_OUT="$(
	set -o pipefail
	find . -name '*.py' -type f \
		! -path './.git/*' \
		! -path '*/node_modules/*' \
		! -path '*/target/*' \
		! -path './.ai/artifacts/worktrees/*' |
		sort
)" || PY_RC=$?
if [ "$PY_RC" -ne 0 ]; then
	# A find that died partway lists fewer files than exist. Reporting "OK (none)" off a truncated
	# sweep is hole 3 again, wearing a different tool's name.
	echo "FAIL: find exited $PY_RC — the .py sweep did not complete."
	echo "      An incomplete search must never read as 'no .py files'."
	FAIL=1
elif [ -n "$PY_OUT" ]; then
	printf 'FAIL: leftover .py files:\n'
	printf '%s\n' "$PY_OUT" | sed 's/^/  /'
	FAIL=1
else
	echo "  OK (none)"
fi

# ─────────────────── 2. Python interpreter invocations (ratcheted) ───────────────────

echo "==> python interpreter invocations in scripts/ + Makefile"

# An EXPLICIT file list, not `grep -r`. gate-grep.sh's helpers are built around named files (see its
# header note), and a tracked-file list is reproducible in a way that a recursive walk over whatever
# happens to be on disk is not. `scripts/ticket` is an extensionless bash wrapper, so this cannot be
# narrowed to `*.sh`.
LS_RC=0
TRACKED="$(git ls-files scripts/)" || LS_RC=$?
if [ "$LS_RC" -ne 0 ] || [ -z "$TRACKED" ]; then
	echo "FAIL: 'git ls-files scripts/' exited $LS_RC with $(printf '%s' "$TRACKED" | grep -c . || true) path(s)."
	echo "      The interpreter scan has no file list, so it did not run. Refusing to report OK."
	FAIL=1
	TRACKED=""
fi

FILES=()
while IFS= read -r f; do
	[ -n "$f" ] || continue
	# This gate names the ban in prose, and the inventory is a list of the very files that violate
	# it. Scanning either would make both permanently self-incriminating.
	[ "$f" = "$SELF" ] && continue
	[ "$f" = "$INVENTORY" ] && continue
	[ -f "$f" ] && FILES+=("$f")
done <<<"$TRACKED"
[ -f Makefile ] && FILES+=(Makefile)

# `#!.*python` catches a shebang on an extensionless script, which check 1 cannot see. Both halves
# are plain ERE alternation — no braces — so ugrep 7.5.0 (an interactive shell) and GNU grep 3.8
# (`bash script.sh`) agree; VERIFIED on both engines, because they diverge on bare `{}` and this
# repository has been bitten by exactly that.
PAT='python3|#!.*python'

if [ "${#FILES[@]}" -eq 0 ]; then
	echo "FAIL: no files to scan — refusing to report OK on a check with an empty input."
	FAIL=1
else
	# THE FAIL-CLOSED SEAM. gate_probe_file echoes grep's RAW status; anything above 1 means the
	# comparison never happened and must not be summarised as a clean result.
	ST="$(gate_probe_file "$PAT" "${FILES[@]}")"
	case "$ST" in
	0 | 1) ;;
	127)
		echo "FAIL: grep exited 127 — the search tool is ABSENT."
		echo "      This is the exact shape of the 'rg' bug this gate was rewritten to kill:"
		echo "      a missing binary must be a FAILURE, never 'OK (none)'."
		FAIL=1
		;;
	*)
		echo "FAIL: grep exited $ST — read or pattern error."
		echo "      Refusing to report OK on a scan that did not execute."
		FAIL=1
		;;
	esac

	if [ "$ST" = "0" ] || [ "$ST" = "1" ]; then
		# Safe to drop grep's status here ONLY because gate_probe_file above already established
		# that the search can run. The `|| true` covers the legitimate "every hit was a comment"
		# case, not a tool failure — that distinction is the whole point of the seam above.
		HITS="$(grep -nE "$PAT" "${FILES[@]}" 2>/dev/null | grep -vE '^[^:]+:[0-9]+:[[:space:]]*#[^!]' || true)"
		ACTUAL="$(printf '%s\n' "$HITS" | sed '/^$/d' | cut -d: -f1 | sort -u)"

		if [ ! -f "$INVENTORY" ]; then
			echo "FAIL: $INVENTORY is missing — the ratchet has no baseline and did not run."
			echo "      Restore it from git; regenerating it would re-bless whatever exists today."
			FAIL=1
		else
			LISTED="$(sed 's/#.*//' "$INVENTORY" | sed 's/[[:space:]]*$//' | sed '/^$/d' | sort -u)"
			NEW="$(comm -23 <(printf '%s\n' "$ACTUAL" | sed '/^$/d') <(printf '%s\n' "$LISTED" | sed '/^$/d'))"
			STALE="$(comm -13 <(printf '%s\n' "$ACTUAL" | sed '/^$/d') <(printf '%s\n' "$LISTED" | sed '/^$/d'))"
			if [ -n "$NEW" ]; then
				printf 'FAIL: NEW Python interpreter invocations (not in %s):\n' "$INVENTORY"
				printf '%s\n' "$NEW" | sed 's/^/  /'
				echo "      New tooling goes in xtask — CODING_STANDARDS.md LANG-1."
				FAIL=1
			fi
			if [ -n "$STALE" ]; then
				printf 'FAIL: %s lists file(s) that no longer invoke python3:\n' "$INVENTORY"
				printf '%s\n' "$STALE" | sed 's/^/  /'
				echo "      Delete these lines — the inventory is a ratchet and may only shrink."
				FAIL=1
			fi
			if [ -z "$NEW" ] && [ -z "$STALE" ]; then
				n="$(printf '%s\n' "$ACTUAL" | sed '/^$/d' | grep -c . || true)"
				echo "  OK — $n file(s) invoke python3, all inventoried, none new"
			fi
		fi
	fi
fi

if [ "$FAIL" -ne 0 ]; then
	echo "verify-no-python: FAIL" >&2
	exit 1
fi
echo "verify-no-python: PASS"
exit 0
