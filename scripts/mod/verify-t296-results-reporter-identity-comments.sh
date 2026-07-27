#!/usr/bin/env bash
# T-296 — ResultsReporter must not claim `#tbd link` / link-confirm is unimplemented.
# T-181.35 shipped `TBD_IdentityLink` (`#tbd link <code>`, Arm()'d from MissionLoader).
# A banner/log that says otherwise is a false-green for the next reader.
#
# OWNS WIDEN: wave_plan T-296 lists only TBD_ResultsReporter.c; this script is the
# perturbation guard for that comment contract (no existing mod-comment verify path).
#
# ── T-556: this script was DEAD AND BROKEN, which is the worse of the two ─────────────
#
# Broken: its three bans read `if rg …; then FAIL=1; fi`. `rg` is installed nowhere — it
# resolves only inside an agent shell that injects a shell function of that name — so the
# missing binary exited 127, the `if` was false, and each ban printed nothing having
# compared nothing. A ban that cannot fail is not a ban.
#
# Dead: nothing invoked it. Not wave.sh, not ci.yml, not the Makefile. A dead gate
# carrying a known-broken shape is a trap for whoever wires it up next and trusts it.
#
# Both are fixed here rather than by deleting the file: the contract it guards is live
# (TBD_ResultsReporter.c really must not claim `#tbd link` is unimplemented), it is a
# named deliverable of shipped T-296 in wave_plan.tsv, and the precedent for scripts that
# exist but were never invoked is T-462/T-463/T-467 — wire them, do not bin them.
#
# Wired: scripts/platform/wave.sh gate_slice + cmd_gate, and `make verify-t296`.
#
# Non-vacuity is now proved in-script (it was not before): every ban is shown catching
# the lie it exists to catch, and every pin is shown catching its own removal, before the
# live file is asserted clean. Search goes through scripts/mod/lib/gate-grep.sh, which
# fails CLOSED when the tool is absent or the target file has moved.
#
# NOTE: deliberately no `python3` here. T-162's ban on Python in scripts/ is unenforced
# today (scripts/verify-no-python.sh has this same rg hole in its interpreter half), and
# adding another violation while that gate is blind is writing debt against a dark meter.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=lib/gate-grep.sh
source "$(cd "$(dirname "$0")" && pwd)/lib/gate-grep.sh"
FILE="$ROOT/apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_ResultsReporter.c"

if [[ ! -f "$FILE" ]]; then
	echo "FAIL: missing $FILE"
	exit 1
fi

# Truth pins: the shipped surface a rewrite must not drop quietly. Literal (-F).
PINS=(
	'TBD_IdentityLink'
	'#tbd link <code>'
	'IDENTITY LINKING (T-181.35 SHIPPED)'
)

# The whole contract, against ONE source path — the live file or a TMP perturbation.
assert_identity_comments() {
	local src="$1" label="$2" rc=0 pin

	# Forbidden: the pre-T-296 lies that T-181.35 never landed.
	gate_ban "($label) ResultsReporter still claims there is no #tbd link command" \
		-F 'There is no `#tbd link` command' "$src" || rc=1
	gate_ban "($label) ResultsReporter still claims the mod does not implement link-confirm" \
		'this mod does not implement' "$src" || rc=1
	# `(lands|ships)` is a group in ERE exactly as it was in ripgrep's engine — the
	# pattern is unchanged, only the binary evaluating it is.
	gate_ban "($label) ResultsReporter still frames attendance as inert until T-181.35" \
		'ATTENDANCE IS INERT UNTIL T-181\.35|until T-181\.35 (lands|ships)' "$src" || rc=1

	# Required: truth pins so a rewrite cannot drop the shipped surface quietly.
	for pin in "${PINS[@]}"; do
		gate_require "($label) missing truth pin: $pin" -F "$pin" "$src" || rc=1
	done

	return "$rc"
}

FAIL=0

if ! assert_identity_comments "$FILE" "live"; then
	FAIL=1
fi

TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

# ── RED 1..3: each banned lie, reintroduced one at a time ─────────────────────────────
# Appended as a `//!` comment, which is exactly the form all three originally took.
LIES=(
	'There is no `#tbd link` command'
	'this mod does not implement link-confirm'
	'ATTENDANCE IS INERT UNTIL T-181.35'
)
for lie in "${LIES[@]}"; do
	{
		cat "$FILE"
		printf '//! %s\n' "$lie"
	} >"$TMP"
	if assert_identity_comments "$TMP" "RED-lie" >/dev/null 2>&1; then
		echo "FAIL: RED lie still passed — ban is not discriminating: $lie"
		FAIL=1
	else
		echo "RED proof: reintroduced lie → FAIL (expected): $lie"
	fi
done

# ── RED 4..6: each truth pin, removed one at a time ───────────────────────────────────
for pin in "${PINS[@]}"; do
	grep -vF -- "$pin" "$FILE" >"$TMP" || true
	if assert_identity_comments "$TMP" "RED-pin" >/dev/null 2>&1; then
		echo "FAIL: RED pin removal still passed — pin is not discriminating: $pin"
		FAIL=1
	else
		echo "RED proof: truth pin removed → FAIL (expected): $pin"
	fi
done

# Live file must still PASS after every perturbation (TMP only; FILE untouched).
if ! assert_identity_comments "$FILE" "live-restore"; then
	echo "FAIL: live file no longer passes after RED proofs (FILE should be untouched)"
	FAIL=1
else
	echo "GREEN proof: live ResultsReporter — no lies, all truth pins present → PASS"
fi

if [[ "$FAIL" -ne 0 ]]; then
	echo "verify-t296-results-reporter-identity-comments: FAIL"
	exit 1
fi

echo "verify-t296-results-reporter-identity-comments: PASS"
exit 0
