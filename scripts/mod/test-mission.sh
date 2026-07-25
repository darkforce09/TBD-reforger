#!/usr/bin/env bash
# Switch the mission the Workbench client loads, without hand-editing JSON.
#
# WHY THIS EXISTS
# ---------------
# Testing slotting needs a mission with several seats on BOTH sides; the backend dev mission
# ("T-068.11 verify") has exactly one blufor seat, which cannot exercise seat contention, side
# discipline, or the one-life path. The golden fixtures can: bridgehead-at-levie is 18 seats,
# 9 blufor / 9 opfor, and is already the mission the wave gate boots.
#
# A golden is loaded through TBD_MissionLoader's PROFILE FALLBACK: the backend is tried first,
# 404s (the id is not in the DB), and OnBackendFetchError -> TryProfileFallbackAfterRestFailure
# reads $profile:missions/<id>.json. So a red `backend refused the mission fetch — http=404` line
# in the log is this working as designed, not a fault.
#
# CAVEAT worth knowing before you trust a green run: the profile path applies NO json-schema
# validation — only TBD_MissionValidator, which is strictly more permissive. A golden staged this
# way can carry shapes the schema forbids. Use the backend path when validating the CONTRACT; use
# this when exercising the GAME.
#
#   bash scripts/mod/test-mission.sh                      # what is loaded now?
#   bash scripts/mod/test-mission.sh bridgehead-at-levie  # 18 seats, both sides (profile)
#   bash scripts/mod/test-mission.sh backend              # back to the DB mission
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PROF="$HOME/.local/share/Steam/steamapps/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile"
CFG="$PROF/TBD_BackendConfig.json"
# The backend dev mission, restored by `backend`. Kept here rather than read from a .bak so the
# script is idempotent — running `bridgehead-at-levie` twice must not lose the way home.
BACKEND_MISSION="6d291619-8182-4164-866d-4e165a5516af"

[ -f "$CFG" ] || { echo "no config at $CFG — has the mod ever run?" >&2; exit 1; }

show() {
  python3 - "$CFG" "$PROF" <<'PY'
import json, os, sys
cfg, prof = sys.argv[1], sys.argv[2]
mid = json.load(open(cfg))["missionId"]
local = os.path.join(prof, "missions", mid + ".json")
# Do NOT infer the load source from this file existing. The mod caches every SUCCESSFUL backend
# fetch to exactly this path, so a backend mission has one too. The real discriminator is whether
# the id is a uuid the DB knows: a golden's `msn_*` id 404s and falls back, a uuid does not.
looks_golden = mid.startswith("msn_")
print(f"  missionId = {mid}")
print("  loads via: profile fallback (backend will 404 — expected)" if looks_golden
      else "  loads via: backend /compiled (schema-validated)")
if os.path.exists(local):
    d = json.load(open(local))
    fac = {}
    for s in d.get("slots", []):
        fac[s["faction"]] = fac.get(s["faction"], 0) + 1
    seats = ", ".join(f"{k} {v}" for k, v in sorted(fac.items()))
    what = "staged" if looks_golden else "last cached"
    print(f"  {len(d.get('slots', []))} seats — {seats}   ({what})")
PY
}

case "${1:-}" in
  "")        echo "current:"; show ;;
  backend)   python3 -c "
import json,sys
p=sys.argv[1]; c=json.load(open(p)); c['missionId']=sys.argv[2]
json.dump(c, open(p,'w'), indent=2)" "$CFG" "$BACKEND_MISSION"
             echo "switched to the backend mission:"; show ;;
  *)         G="$(find "$ROOT/packages/tbd-schema" -name "$1.json" | head -1)"
             [ -n "$G" ] || { echo "no golden named '$1' under packages/tbd-schema" >&2; exit 1; }
             MID="$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['meta']['id'])" "$G")"
             mkdir -p "$PROF/missions"
             cp "$G" "$PROF/missions/$MID.json"
             # The registry alias $TBD_Framework:Data/registry.json does not resolve for a loose
             # addon, so without this every slot fails "kit resolve failed" (measured, world-boot.sh).
             cp "$ROOT/apps/mod/tbd-framework/Data/registry.json" "$PROF/TBD_Registry.json" 2>/dev/null || true
             python3 -c "
import json,sys
p=sys.argv[1]; c=json.load(open(p)); c['missionId']=sys.argv[2]
json.dump(c, open(p,'w'), indent=2)" "$CFG" "$MID"
             echo "staged $1 and switched:"; show ;;
esac
