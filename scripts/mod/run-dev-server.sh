#!/usr/bin/env bash
# run-dev-server.sh — DELEGATES to run-playtest-server.sh. Kept only so the name still works.
#
# ── What this file used to be (T-604) ──────────────────────────────────────────────────────
# 27 lines that resolved a path, checked the binary existed, checked the profile existed, and
# then ENDED. `grep -c ArmaReforgerServer` returned 1 and that one hit was the path variable.
# It never launched a server. It was cited in three places as the way to start one, and the
# only symptom was that nothing happened — no error, no server, exit 0.
#
# That is this codebase's signature defect: a tool reporting success over an input it never
# examined. None of it is preserved here. Everything below either execs the real launcher or
# fails loudly with a pointer.
#
# The real launcher is scripts/mod/run-playtest-server.sh. It needs a mission id, because a
# server with no mission never leaves LOADING — which is the OTHER way this script used to
# look like it worked.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REAL="$SCRIPT_DIR/run-playtest-server.sh"

if [ ! -x "$REAL" ]; then
  echo "run-dev-server.sh: the real launcher is missing at $REAL" >&2
  echo "  This shim starts nothing on its own — it never did." >&2
  exit 3
fi

# No arguments is exactly the case that used to silently do nothing. Say what to run instead.
if [ "$#" -eq 0 ]; then
  cat >&2 <<'EOF'
run-dev-server.sh starts nothing on its own — it is a shim for run-playtest-server.sh,
which has to be told WHICH mission to serve.

  bash scripts/mod/run-playtest-server.sh --mission-id=<id> [--admin=<identityId>]

  --mission-id   the mission the mod loads. Without it the stage machine never leaves
                 LOADING and the server looks healthy while being unplayable.
  --admin        your identityId (UUID) or 17-digit SteamID. Without it every '#tbd'
                 command answers "TBD: admin only." and T-181.16 cannot pass.

  bash scripts/mod/run-playtest-server.sh --help    for the rest
  docs/mod/STAGING-SERVER.md                        for what the second client needs

Offline? Add --mission-file=packages/tbd-schema/golden-missions/bridgehead-at-levie.json
to serve a golden from disk with no API running.
EOF
  exit 2
fi

exec "$REAL" "$@"
