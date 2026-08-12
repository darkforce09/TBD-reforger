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

# Same host/container test as wave.sh (distrobox-host-exec:130). The binary exists on BOTH sides;
# on the host it refuses with exit 126, so `command -v` alone false-BLOCKs cargo + ticket check.
# Measured 2026-07-26: preflight printed "cargo unusable via the bridge" / "registry INVALID"
# while `cargo --version` and `./scripts/ticket check` both passed natively on the host.
in_container() { [ -f /run/.containerenv ] || [ -f /.dockerenv ] || [ -n "${container:-}" ]; }
if command -v distrobox-host-exec >/dev/null 2>&1 && in_container; then
  hostrun() { distrobox-host-exec "$@"; }
else
  hostrun() { "$@"; }
fi

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
# Name the reclaimable space rather than just the shortfall. On 2026-07-26 the disk hit 252 MB free of
# 952 GB and two gate steps failed with "No space left on device", which reads as a build error — while
# ~116 GB of shipped-slice target dirs sat in /var/tmp. A disk warning that does not say where the space
# went sends someone hunting; this says it.
orphan_mb=$(du -sm /var/tmp/*target* /var/tmp/v2-* 2>/dev/null | awk '{s+=$1} END{print s+0}')
if [ "${orphan_mb:-0}" -gt 4096 ]; then
  soft "reclaimable" "$((orphan_mb/1024))G of build caches in /var/tmp — bash scripts/platform/wave.sh reclaim"
else
  ok "reclaimable" "$((orphan_mb/1024))G of stale build caches"
fi

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
if hostrun cargo run -q -p xtask -- ticket check >/dev/null 2>&1; then ok "ticket check" "registry valid"
else nope "ticket check" "registry INVALID — every wave gate will fail"; fi

# 9. Wave plan sanity: disjoint, no directory claims, deps satisfiable.
#
# T-620: `python3 scripts/platform/slice-collisions.py` became `cargo xtask slice-collisions` when
# the Python was ported and deleted. This check had been reporting "slice-collisions.py failed" for
# real since the w76-w81 rows landed — `int('w80')` raised ValueError, so the dispatch view was dead
# and preflight was honestly saying so. T-616 normalised the labels; both are fixed together.
if hostrun cargo run -q -p xtask -- slice-collisions >/dev/null 2>&1; then
  n=$(grep -vc '^#' docs/platform/wave_plan.tsv)
  ok "wave plan" "$n tickets, dispatch set computes"
else nope "wave plan" "cargo xtask slice-collisions failed"; fi

# 10. Optional environment. Missing DB/API is NOT a code failure — but a gate will report it as one,
#     and an unattended fix agent will burn its whole budget chasing it. Know before you start.
if (exec 3<>/dev/tcp/127.0.0.1/5434) 2>/dev/null; then ok "postgres :5434" "up"
else soft "postgres :5434" "down — API integration tests will skip (make db-up on the HOST)"; fi
# A TCP CONNECT IS NOT A HEALTH CHECK, AND "UP" IS NOT "CURRENT".
# This used to be `exec 3<>/dev/tcp/127.0.0.1/8080` and nothing more, which reports "up" for any
# process holding the port — including one serving a binary six hours stale. OBSERVED 2026-07-26:
# PID 34618 started 08:51 and T-234 landed 14:27, so the dev API had none of T-234's code. T-389
# spent a full browser round trip "reproducing" a bug that was already fixed, and only caught it by
# building its own API on another port. A stale API is worse than a dead one: dead fails loudly,
# stale returns confident wrong answers that read as real defects.
# So: probe the real route (/healthz, NOT /api/v1/healthz — that 404s), and compare the process
# start time against the last commit touching the API's own crates.
api_code="$(curl -s -o /dev/null -w '%{http_code}' -m 4 http://127.0.0.1:8080/healthz 2>/dev/null)"
if [ "$api_code" = "200" ]; then
  api_pid="$(hostrun ss -ltnp 2>/dev/null | grep ':8080' | grep -o 'pid=[0-9]*' | head -1 | cut -d= -f2)"
  started="$(hostrun stat -c %Y /proc/"${api_pid:-0}" 2>/dev/null || echo 0)"
  newest="$(git log -1 --format=%ct -- apps/website/api crates/map-engine-core 2>/dev/null || echo 0)"
  if [ "${started:-0}" -gt 0 ] && [ "$newest" -gt "$started" ]; then
    soft "api :8080" "healthy but STALE — running since $(hostrun date -d "@$started" +%H:%M 2>/dev/null), \
API code changed $(hostrun date -d "@$newest" +%H:%M 2>/dev/null). Restart it or verifications lie."
  else
    ok "api :8080" "healthz 200, binary current"
  fi
elif [ -n "$api_code" ] && [ "$api_code" != "000" ]; then
  soft "api :8080" "listening but /healthz returned $api_code — wedged or mid-restart"
else
  soft "api :8080" "down — editor smokes would report gate-red for an env reason"
fi

# 11b. A running `trunk serve` (make leptos) USED TO race the gate's `trunk build --release` over the
#      same apps/website/frontend/dist. OBSERVED 2026-07-26: the wave gate failed with "error writing
#      JS loader file to stage dir / No such file or directory" while a trunk serve held it, and the
#      identical build passed seconds later — an ENVIRONMENT race that reads exactly like a code
#      defect. The mitigation was to kill the operator's dev server before every unattended run.
#
#      CURED BY T-396 (7362d1dc). The gate now builds into a private --dist AND a private
#      CARGO_TARGET_DIR, which together are trunk's complete write set (there is no staging env var —
#      TRUNK_STAGING_DIR is exported to build hooks, never read, verified against the 0.21.14 binary).
#      Private --dist ALONE was the previously-recorded dead end, because it left
#      $CARGO_TARGET_DIR/wasm-opt/... shared. gate_trunk_build now also refuses to build if either
#      private path has collapsed onto one the dev server owns, and proves after each build that both
#      took a write — because exit 0 says trunk was happy, not that trunk honoured the flags.
#      Measured: 5/5 collisions before, 15/15 clean after, plus 10/10 gate builds with trunk serve
#      live on :3000 answering 200 throughout.
#
#      So this is now INFORMATIONAL. Do NOT tell anyone to stop their dev server: that instruction is
#      what made the ritual survive, and taking :3000 down is user-hostile. If the gate's trunk step
#      ever fails with a staging error again, THAT is the signal the isolation regressed — and
#      gate_trunk_build's own guards should have caught it first.
ts=$(pgrep -f "trunk serve" 2>/dev/null | wc -l)
[ "$ts" -eq 0 ] && ok "trunk serve" "not running" \
  || ok "trunk serve" "$ts running — fine since T-396; the gate builds into private dist + target"

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
