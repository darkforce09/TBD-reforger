#!/usr/bin/env bash
# Preflight for an unattended platform-factory run.
#
# WHY: every check here corresponds to a way a previous run wasted hours failing for a reason that
# had nothing to do with the code. The point is to fail LOUDLY at t=0 rather than at t=6h with a
# gate-red that a fix agent then spends its whole retry budget "fixing".
#
#   bash scripts/platform/preflight.sh          # check, exit 1 on any BLOCK
#   bash scripts/platform/preflight.sh --warn   # never exit non-zero (report only)
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"; cd "$ROOT"
WARN_ONLY=0; [ "${1:-}" = "--warn" ] && WARN_ONLY=1
block=0; warn=0

ok()    { printf "  \033[32m✓\033[0m %-34s %s\n" "$1" "${2:-}"; }
nope()  { printf "  \033[31m✗ BLOCK\033[0m %-28s %s\n" "$1" "${2:-}"; block=$((block+1)); }
soft()  { printf "  \033[33m! WARN \033[0m %-28s %s\n" "$1" "${2:-}"; warn=$((warn+1)); }

if command -v distrobox-host-exec >/dev/null 2>&1; then hostrun() { distrobox-host-exec "$@"; }
else hostrun() { "$@"; }; fi

echo "═══ platform factory preflight ═══"

# 1. Host bridge. cargo/xtask/rustfmt are built against the host's glibc (2.43) and will not link
#    inside this container (2.36). Without the bridge EVERY gate fails with a linker error that
#    reads like a code fault.
if [ -f /run/.containerenv ]; then
  if hostrun true 2>/dev/null; then ok "host bridge" "distrobox-host-exec live"
  else nope "host bridge" "in a container and distrobox-host-exec is dead — every cargo gate will fail"; fi
else ok "host bridge" "not containerised"; fi

# 2. cargo actually runs. The check that matters, not just "is the binary there".
if hostrun cargo --version >/dev/null 2>&1; then ok "cargo" "$(hostrun cargo --version 2>/dev/null)"
else nope "cargo" "cargo unusable via the bridge"; fi

# 3. Disk. 8 worktrees sharing CARGO_TARGET_DIR cost ~4 GB; if a slice agent overrides it they cost
#    ~44 GB each. Below 40 GB free a mid-run build failure reads as a compile error, not a disk one.
free_gb=$(df -BG --output=avail "$ROOT" | tail -1 | tr -dc '0-9')
if   [ "$free_gb" -ge 40 ]; then ok "disk" "${free_gb}G free"
elif [ "$free_gb" -ge 20 ]; then soft "disk" "${free_gb}G free — tight; make clean-targets first"
else nope "disk" "${free_gb}G free — below the 20G floor"; fi

# 4. CARGO_TARGET_DIR must be shared, and no worktree may have grown its own target/.
[ -n "${CARGO_TARGET_DIR:-}" ] && ok "CARGO_TARGET_DIR" "$CARGO_TARGET_DIR" \
  || soft "CARGO_TARGET_DIR" "unset in this shell — wave.sh exports it, but a dispatcher must too"
stray=$(find .ai/artifacts/worktrees -maxdepth 2 -name target -type d 2>/dev/null | wc -l)
[ "$stray" -eq 0 ] && ok "no per-worktree target/" "" \
  || nope "per-worktree target/" "$stray worktree(s) built into their own target — will exhaust disk"

# 5. RAM + swap. /tmp is a 16G tmpfs (RAM-backed) and both engine gates mktemp -d into it; a killed
#    run leaks its rundir. With swap already deep, one runaway loop OOM-kills the fleet.
mem_mb=$(awk '/MemAvailable/{print int($2/1024)}' /proc/meminfo)
[ "$mem_mb" -ge 1024 ] && ok "memory" "${mem_mb}MiB available" || nope "memory" "${mem_mb}MiB — gate-env floor is 1024"
sw_t=$(awk '/SwapTotal/{print $2}' /proc/meminfo); sw_f=$(awk '/SwapFree/{print $2}' /proc/meminfo)
if [ "${sw_t:-0}" -gt 0 ]; then
  sw_used=$(( (sw_t - sw_f) * 100 / sw_t ))
  [ "$sw_used" -lt 70 ] && ok "swap" "${sw_used}% used" || soft "swap" "${sw_used}% used — OOM risk over a long run"
fi

# 6. Clean tree + synced remote. `land` merges into main; uncommitted work would be swept in.
[ -z "$(git status --porcelain)" ] && ok "working tree" "clean" || nope "working tree" "dirty — commit or stash first"
git rev-parse --abbrev-ref HEAD | grep -qx main && ok "branch" "main" || nope "branch" "not on main"
ahead=$(git rev-list --count origin/main..HEAD 2>/dev/null || echo '?')
[ "$ahead" = "0" ] && ok "remote" "in sync" || soft "remote" "$ahead commit(s) unpushed"

# 7. No worktrees left from a previous run.
wt=$(git worktree list | tail -n +2 | wc -l)
[ "$wt" -eq 0 ] && ok "worktrees" "none stale" || soft "worktrees" "$wt left over — wave.sh land will reuse or trip on them"

# 7b. An UNDISPATCHED worktree — created, never given an agent. OBSERVED 2026-07-26: five worktrees
#     (T-331, T-337, T-348, T-349, T-352) sat pristine for up to 90 minutes because the command center
#     created them in a batch and then dispatched only a subset. The operator noticed, not the tooling.
#     A live agent writes files within a couple of minutes, so "no commits AND no uncommitted files"
#     is a reliable signature of a slot that is burning wall-clock for nothing. Cheap to check, and it
#     is pure lost throughput rather than a correctness fault, which is exactly the kind of thing
#     nobody goes looking for.
#     THRESHOLD IS LOAD-BEARING: a just-dispatched agent legitimately has nothing yet, so an
#     unconditional check warns on every healthy fresh wave — and a gate that cries wolf is one
#     everybody learns to ignore, which is precisely how `make ci-local` became decorative here.
#     Only worktrees older than IDLE_MIN minutes with nothing to show are suspicious.
IDLE_MIN="${TBD_IDLE_WORKTREE_MIN:-10}"
idle=""
for w in $(git worktree list | tail -n +2 | awk '{print $1}'); do
  t="$(basename "$w")"
  ahead="$(git -C "$w" rev-list --count main..HEAD 2>/dev/null || echo 0)"
  dirty="$(git -C "$w" status --porcelain 2>/dev/null | wc -l)"
  # The worktree's .git file is written once at creation and never touched again, so its mtime is
  # the dispatch time. Anything the agent writes is newer; nothing else in here ages it.
  young="$(find "$w" -maxdepth 1 -name .git -newermt "-${IDLE_MIN} minutes" 2>/dev/null)"
  [ "$ahead" = "0" ] && [ "$dirty" -eq 0 ] && [ -z "$young" ] && idle="$idle $t"
done
if [ -n "$idle" ]; then
  soft "idle worktrees" "nothing written in ${IDLE_MIN}m+ —$idle — created and never dispatched?"
else
  ok "worktrees busy" "every worktree is working or newer than ${IDLE_MIN}m"
fi

# 8. The registry must validate, because `ticket check` is INSIDE the wave gate. A registry that
#    fails here fails every wave, forever, and a fix agent cannot repair it from inside a slice.
if hostrun ./scripts/ticket check >/dev/null 2>&1; then ok "ticket check" "registry valid"
else nope "ticket check" "registry INVALID — every wave gate will fail"; fi

# 9. Wave plan sanity: disjoint, no directory claims, deps satisfiable.
if python3 scripts/platform/slice-collisions.py >/dev/null 2>&1; then
  n=$(grep -vc '^#' docs/platform/wave_plan.tsv)
  ok "wave plan" "$n tickets, dispatch set computes"
else nope "wave plan" "slice-collisions.py failed"; fi

# 10. Optional environment. Missing DB/API is NOT a code failure — but a gate will report it as one,
#     and an unattended fix agent will burn its whole budget chasing it. Know before you start.
if (exec 3<>/dev/tcp/127.0.0.1/5434) 2>/dev/null; then ok "postgres :5434" "up"
else soft "postgres :5434" "down — API integration tests will skip (make db-up on the HOST)"; fi
if (exec 3<>/dev/tcp/127.0.0.1/8080) 2>/dev/null; then ok "api :8080" "up"
else soft "api :8080" "down — editor smokes would report gate-red for an env reason"; fi

# 11b. A running `trunk serve` (make leptos) races the gate's `trunk build --release` over the same
#      apps/website/frontend/dist. OBSERVED 2026-07-26: the wave gate failed with "error writing JS
#      loader file to stage dir / No such file or directory" while PID 158382 held it; the identical
#      build passed seconds later. This is the worst failure shape there is — an ENVIRONMENT race
#      that reads exactly like a code defect, so an unattended fix agent burns its whole retry budget
#      on working code. Catch it at t=0.
ts=$(pgrep -f "trunk serve" 2>/dev/null | wc -l)
[ "$ts" -eq 0 ] && ok "trunk serve" "not running" \
  || soft "trunk serve" "$ts running — races the gate's trunk build over dist/; stop it before an unattended run"

# 11. Stray chrome starves the editor gates (doctor.rs:166-172 refuses to run with any alive).
# `pgrep -fc` prints 0 AND exits 1 when there is no match, so a `|| echo 0` fallback yields "0\n0"
# and the arithmetic test below blows up. Count lines instead.
ch=$(pgrep -f "chrome-linux64/chrome" 2>/dev/null | wc -l)
[ "$ch" -eq 0 ] && ok "chrome" "none stray" || soft "chrome" "$ch process(es) alive — leptos-gates will refuse"

echo
if [ "$block" -gt 0 ]; then
  echo "PREFLIGHT: $block BLOCK, $warn warn — DO NOT START"
  [ "$WARN_ONLY" -eq 1 ] && exit 0 || exit 1
fi
echo "PREFLIGHT: PASS ($warn warn)"
