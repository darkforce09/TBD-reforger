#!/usr/bin/env bash
# T-452 — PlayerIdentity must not claim `#tbd link` / link-confirm is unimplemented.
# T-181.35 shipped `TBD_IdentityLink` (`#tbd link <code>`, Arm()'d from MissionLoader).
# T-296 fixed the same lie in ResultsReporter; this file's header still said T-181.35.
#
# OWNS WIDEN: wave_plan T-452 lists only TBD_PlayerIdentity.c; this script is the
# perturbation guard for that comment contract (same shape as verify-t296-*).
#
# ── T-556: dead and broken, same two defects as its T-296 sibling ─────────────────────
#
# Broken: three bans in the `if rg …; then FAIL=1; fi` shape. `rg` is installed nowhere
# (it resolves only inside an agent shell that injects a shell function of that name), so
# the missing binary exited 127, the `if` was false, and each ban reported clean having
# compared nothing.
#
# Dead: invoked by nothing — not wave.sh, not ci.yml, not the Makefile.
#
# Wired here rather than deleted, for the reasons spelled out in verify-t296-*: the
# contract is live, the script is a named deliverable of shipped T-452 in wave_plan.tsv,
# and T-462/T-463/T-467 set the precedent of wiring unwired verify scripts in.
#
# Wired: scripts/platform/wave.sh gate_slice + cmd_gate, and `make verify-t452`.
#
# Every ban and every pin now carries its own RED proof, and search goes through
# scripts/mod/lib/gate-grep.sh, which fails CLOSED on an absent tool or a moved file.
# No `python3` here on purpose — see the note in verify-t296-*.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=lib/gate-grep.sh
source "$(cd "$(dirname "$0")" && pwd)/lib/gate-grep.sh"
FILE="$ROOT/apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_PlayerIdentity.c"

if [[ ! -f "$FILE" ]]; then
	echo "FAIL: missing $FILE"
	exit 1
fi

# Truth pins: the shipped surface a rewrite must not drop quietly. Literal (-F).
PINS=(
	'TBD_IdentityLink'
	'#tbd link <code>'
	'T-181.35 shipped'
	'ENGINE-resolved identity is still not a LINKED one'
)

# The whole contract, against ONE source path — the live file or a TMP perturbation.
assert_identity_comments() {
	local src="$1" label="$2" rc=0 pin

	# Forbidden: the pre-T-452 lies that T-181.35 never landed / link-confirm is future work.
	gate_ban "($label) PlayerIdentity still claims the mod does not implement link-confirm" \
		-F 'The mod does not implement it yet' "$src" || rc=1
	# The em dash and [[:space:]] class mean the same thing in ERE as they did in
	# ripgrep's engine; the pattern is byte-for-byte the one T-452 shipped.
	gate_ban "($label) PlayerIdentity still frames link-confirm as future T-181.35 work" \
		'does not implement it yet[[:space:]]*—[[:space:]]*that is T-181\.35|that is T-181\.35' "$src" || rc=1
	gate_ban "($label) PlayerIdentity still speaks of T-181.35 in the future tense for GetArmaId" \
		'T-181\.35 must not resolve' "$src" || rc=1

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
LIES=(
	'The mod does not implement it yet'
	'link-confirm — that is T-181.35'
	'T-181.35 must not resolve GetArmaId'
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

# ── RED 4..7: each truth pin, removed one at a time ───────────────────────────────────
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
	echo "GREEN proof: live PlayerIdentity — no lies, all truth pins present → PASS"
fi

if [[ "$FAIL" -ne 0 ]]; then
	echo "verify-t452-player-identity-link-comments: FAIL"
	exit 1
fi

echo "verify-t452-player-identity-link-comments: PASS"
exit 0
