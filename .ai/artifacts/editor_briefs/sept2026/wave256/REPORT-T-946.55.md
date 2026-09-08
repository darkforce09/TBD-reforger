# REPORT T-946.55 — Objective HUD replicates to every player at 1 Hz

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-946.55
slice/T-946.55
```

Command: `pwd && git branch --show-current`, first action. Then the 19 gitignored EnfusionMCP `.c`
were copied in (`find … -name '*.c' | wc -l` → `19`); `git status --porcelain` stayed empty, so
nothing of theirs is committed.

## defect_verified

**Measured: 60 Reliable Owner HUD RPCs per player per minute on a fully idle round.**

### The static trace, from the file, before any edit

| claim | path:line (pre-edit) | evidence |
|---|---|---|
| Tick is a 1 Hz repeat | `TBD_ObjectivesComponent.c:49,90` | `static const int TICK_MS = 1000;` / `CallLater(Tick, TICK_MS, true)` |
| Tick calls `Deliver` unconditionally while LIVE | `:180` | `Deliver(players, connected, objectives);` |
| `Deliver` calls `ReplicateHud` with no dirty check | `:692` | `ReplicateHud(players, connected, objectives);` — last statement of the method, nothing guards it |
| `ReplicateHud` pushes to every connected player | `:747-753` | `foreach (int playerId : connected) PushHudToPlayer(players, playerId, board, 1);` |
| the push is a Reliable Owner RPC | `:802` → `TBD_ObjectiveHud.c:312, :326` | `controller.TBD_PushObjectiveHud(...)` → `Rpc(TBD_RpcDo_ObjectiveHud, …)`, declared `[RplRpc(RplChannel.Reliable, RplRcver.Owner)]` |
| the payload is three string arrays plus four scalars | `TBD_ObjectiveHud.c:328-330` | `array<string> icons, array<string> titles, array<string> details, string barLabel, int barPercent, int barVisible, int show` |

There is no early return, no latch and no comparison anywhere on that path. 60 ticks per minute x one
RPC per connected player, and it does not fall to zero when nothing is happening — which is the wire
bill the per-tick chat pump was deleted for at T-941.4.

### The instrument, and its RED

`idle_hud_probe.awk` (verbatim at the bottom of this report; scratch, **not** committed — the slice
owns two `.c` files and nothing else). It reads the real `TBD_ObjectivesComponent.c`, strips comments
and string literals with the repo's own rule, **extracts the replication decision from the source**
(is the push gated at all; the `HudChanged` truth table; whether the per-owner store is pruned), then
simulates 60 ticks x 8 players x 4 objectives and counts RPCs.

`awk -f idle_hud_probe.awk apps/mod/tbd-framework/…/TBD_ObjectivesComponent.c` on the **unmodified**
file, exit 1:

```
SOURCE FACTS (read from apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c)
  per-owner store member m_mHudSignatures ....... NO
  ReplicateHud gates the push (if !HudChanged) ... NO
  HudChanged: unknown owner => send ............... NO
  HudChanged: equal snapshot => suppress .......... NO
  HudChanged: unconditional return true .......... NO
  store pruned against connected list ............ NO
  store cleared on OnPlayerDisconnected .......... NO

S1 idle round (8 players, 4 objectives, 60 ticks, nothing changes)
   RPCs total ................ 480
   RPCs per player per minute  60.0
   after the initial board ... 472
   RED: S1 idle-round: HUD RPCs after the initial board must be 0, got 472 (60/player/min)

S2 mid-round joiner appears at tick 30 ....... first board at tick 30
S3 capturing player, bar percent moves each tick  60/60 ticks sent
S4 side-B-only text change at tick 40 ........ B sent=1  A sent=1
   RED: S4 per-side: side A saw no change and must not be sent to at tick 40
S5 churn: 60 distinct ids over the round ..... peak store size 68 (concurrent players 9)
   RED: S5 bounded: per-owner store must be bounded by concurrent players (9), peaked at 68

IDLE HUD PROBE: FAIL (3 assertion(s))
```

**480 RPCs / 8 players / 1 minute = 60 per player per minute, 472 of them after the initial board.**

What each scenario is for: S1 is the ticket's metric. S3 is the trap — a bar that legitimately moves
every tick must keep sending, so "0 RPCs" is not the goal, "0 RPCs *when nothing changed*" is. S4 is
the per-side trap: a change visible only to one faction must reach that faction and only that faction,
which a global board hash gets wrong in both directions. S5 is the boundedness requirement. S5's
pre-fix number is the probe's model store growing unbounded because the source names no prune; the
honest reading is "nothing in the file bounds it", not "the shipped code leaks a map that does not
exist yet".

Exit code, both before and after, is the assertion result: `EXIT=1` pre-fix, `EXIT=0` post-fix.

## changes

Two files, both twins, identical after ASCII folding.

| path | change |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c` | the gate |
| `apps/mod/tbd-export/Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c` | mirror |

1. **`m_mHudSignatures`** — `protected ref map<int, string>`, built in `OnPostInit` **before** the
   `RplMode.Client` bail-out so no path can reach a null map. Cleared in `OnDelete`.
2. **`ReplicateHud`** now builds each player's snapshot with the existing `FillHudSnapshot`, signs it,
   and calls the push only when the signature differs from what that owner was **last actually sent**.
   The signature is of the **rendered per-player snapshot**, not the shared board: `FillHudSnapshot`
   resolves `factionKey` per viewer and carries that viewer's own capture bar, so a global dirty flag
   would both miss a per-side text change (S4) and suppress a bar that moves every tick (S3).
3. **`HudChanged`** — a map miss is a **send**, never a skip. The map holds only owners the server has
   provably pushed to, so a mid-round joiner, a recycled playerId, or a player who has only just
   gained a controller reads as unknown and gets the whole board on the next tick.
4. **The record is written only when the RPC went out.** `PushHudToPlayer` was split: `PushHudSnapshot`
   is the wire (controller cast + RPC) and **returns whether it actually sent**. A connected player
   still in the lobby has no `SCR_PlayerController`; recording a signature for them would mark them up
   to date for a frame that never left the server and starve them for the rest of the round.
5. **Bounded by concurrent players, two ways.** `PruneHudSignatures(connected)` runs at the top of
   `ReplicateHud` and drops every key absent from the list `Tick` re-reads each second — collect-then-
   remove, because mutating a map while iterating is unsafe and `array.Remove` is by index (the shape
   `TBD_PlayAreaComponent.PruneDeparted` already uses). `OnPlayerDisconnected(int, KickCauseCode, int)`
   drops the leaver immediately; `SCR_BaseGameMode` dispatches it to every `SCR_BaseGameModeComponent`
   and `TBD_SpawnManager.c:3348` is the exact-signature precedent on the same base class.
6. **`PushHudToPlayer` stays ungated** for its two callers that must never be suppressed — `HideAllHuds`
   on the LIVE falling edge, and `PushHudTo` — and **resets** the owner's record in both directions:
   `show == 0` removes the key (a hidden HUD is not a clean one; re-entering LIVE on an unchanged board
   must still re-open the panel), `show == 1` records what was sent so the next tick does not resend.
7. **`HudSignature` is length-prefixed.** Plain separators collide — `("A|B","C")` and `("A","B|C")`
   concatenate to the same bytes — and a HUD that *misses* a real change is worse than one that sends a
   redundant frame. Row count is clamped to the shortest of the three arrays so a later edit to
   `FillHudSnapshot` cannot turn this into an out-of-range read at 1 Hz.

Post-fix probe, exit 0:

```
S1 idle round (8 players, 4 objectives, 60 ticks, nothing changes)
   RPCs total ................ 8
   RPCs per player per minute  1.0
   after the initial board ... 0

S2 mid-round joiner appears at tick 30 ....... first board at tick 30
S3 capturing player, bar percent moves each tick  60/60 ticks sent
S4 side-B-only text change at tick 40 ........ B sent=1  A sent=0
S5 churn: 60 distinct ids over the round ..... peak store size 9 (concurrent players 9)

IDLE HUD PROBE: PASS
```

60 → 1 RPC per player per minute on an idle round, and 0 after the initial board.

### The `objectives` pin was not disturbed

`xtask/src/schema_gates.rs:2455` asserts `objectives == 13` by exact equality over
`apps/mod/tbd-framework` after comments and string literals are stripped. Measured with a
transcription of the gate's own `strip_enfusion_comments_and_strings` rule, per file across the whole
tree:

```
before:  TOTAL(framework) = 13   (all 13 in TBD_ObjectivesComponent.c, 18 raw incl. prose)
after:   TOTAL = 13 (pin 13)
```

No added identifier is spelled `objectives`; `Deliver`'s parameter and the call at the old `:692` are
untouched; the new code works on `board`. The gate agrees — `schema PASS` in the run below.

### Twin lockstep

The transliteration was proven exact before use, not assumed: `sed -e 's/—/-/g' -e 's/═/=/g'` applied
to `fw@HEAD` reproduces `ex@HEAD` **byte for byte** (`TRANSLITERATION EXACT`). The export twin was then
regenerated from the edited framework twin by the same two substitutions. Only those two non-ASCII
code points exist in the file (U+2014, U+2550); the export twin scans clean for `[^\x00-\x7F]`. Both
twins carry the identical hunk structure (`git diff --unified=0 | grep '^@@'` matches line for line).
`mirror_lockstep` (`xtask/src/gate_mod_compile.rs:427`) runs inside `mod compile` and did not fire.

## perturbation

The dirty check was made to **always report dirty**, in **both** twins so lockstep could not mask it:

```
	protected bool HudChanged(int playerId, string signature)
	{
		return true;
	}
```

Twins re-diffed after perturbing: `TWINS STILL IN LOCKSTEP (both perturbed)`.

**RED, verbatim** (`awk -f idle_hud_probe.awk apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c`):

```
################ PERTURBED: idle probe (framework twin) ################
SOURCE FACTS (read from apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c)
  per-owner store member m_mHudSignatures ....... yes
  ReplicateHud gates the push (if !HudChanged) ... yes
  HudChanged: unknown owner => send ............... NO
  HudChanged: equal snapshot => suppress .......... NO
  HudChanged: unconditional return true .......... yes
  store pruned against connected list ............ yes
  store cleared on OnPlayerDisconnected .......... yes

S1 idle round (8 players, 4 objectives, 60 ticks, nothing changes)
   RPCs total ................ 480
   RPCs per player per minute  60.0
   after the initial board ... 472
   RED: S1 idle-round: HUD RPCs after the initial board must be 0, got 472 (60/player/min)

S2 mid-round joiner appears at tick 30 ....... first board at tick 30
S3 capturing player, bar percent moves each tick  60/60 ticks sent
S4 side-B-only text change at tick 40 ........ B sent=1  A sent=1
   RED: S4 per-side: side A saw no change and must not be sent to at tick 40
S5 churn: 60 distinct ids over the round ..... peak store size 9 (concurrent players 9)

IDLE HUD PROBE: FAIL (2 assertion(s))
EXIT=1
```

**The RED is behavioural, not a compile break** — the perturbed tree still compiles clean, so the
assertion is failing on what the code *does*, not on the file being broken:

```
--- perturbed tree mod compile ---
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)
OK: compiled clean
    Module: Game; loaded 5761x files; 11484x classes
    0 warning(s) in TBD sources
```

**Restored** with `git checkout --` on the two explicit paths, then **`touch`**ed on both (mandatory:
a restored file can be older than the artifact built from the perturbed one, so the green half of the
loop is stale otherwise):

```
=== restored HudChanged ===
838:	protected bool HudChanged(int playerId, string signature)
839-	{
840-		string previous;
841-		if (!m_mHudSignatures.Find(playerId, previous))
842-			return true;
843-
844-		return previous != signature;
845-	}

S1 idle round (8 players, 4 objectives, 60 ticks, nothing changes)
   RPCs total ................ 8
   RPCs per player per minute  1.0
   after the initial board ... 0
...
IDLE HUD PROBE: PASS
EXIT=0
```

and, post-`touch`:

```
--- restored tree mod compile (post-touch) ---
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)
OK: compiled clean
    Module: Game; loaded 5761x files; 11484x classes
    Compiling Game scripts took: 857.534000 ms
    0 warning(s) in TBD sources
```

## gate_verdict_tail

`hcargo xtask platform wave gate --slice T-946.55`, run detached, polled on `^SLICE GATE:` only.
Final run is at the **final** commit (`885d6b7a2eb3`), not the first:

```
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ 885d6b7a2eb3 recorded: .ai/artifacts/verdicts/T-946.55.json
SLICE GATE: PASS
```

All 20 steps PASS, including `schema PASS` — that is the step carrying the `objectives == 13`
`UNREAD_WIRE_FIELDS` assertion.

**`cargo xtask mod compile`** (the gate compiles no Enfusion; run by hand, final tree):

```
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)
OK: compiled clean
    Module: Game; loaded 5761x files; 11484x classes
    Compiling Game scripts took: 953.294000 ms
    0 warning(s) in TBD sources
```

**On baseline exactly** — `OK`, 5761 files, 11484 classes, 0 warnings. `mirror_lockstep` runs inside
this command and did not fire.

**`hcargo xtask schema validate`** (worktree LFS form, not `ci schema-validate`): `All contracts valid.`

## files_outside_owns

`[]`

Only the two owned twins were edited. The mandated report
(`.ai/artifacts/editor_briefs/sept2026/wave256/REPORT-T-946.55.md`) is the single other write. The
probe is a scratch instrument outside the repo and is reproduced verbatim below instead of committed.
Nothing in `xtask/`, `TBD_ObjectiveHud.c`, `schema_gates.rs` or any sibling's paths was touched.

## found_not_fixed

- **The client's HUD pull path is dead code.** The brief lists `PushHudTo` as reachable from
  `TBD_ObjectiveHud.c:288 TBD_RequestObjectiveHud()`; that line is the **declaration**, not a call
  site. `grep -rn "TBD_RequestObjectiveHud" apps/mod/` returns four hits: the declaration in each
  twin's `TBD_ObjectiveHud.c:287`, and the two new comment references I added. **Nothing invokes it**,
  so neither `TBD_RpcAsk_ObjectiveHud` nor `PushHudTo` can fire on this build. This does not weaken the
  fix — the anti-starvation guarantee is `HudChanged` returning true on a map miss, which needs no
  client cooperation — but it does mean there is currently no client-initiated resync at all, and a
  reader who assumed the pull was live would gate too aggressively. Recorded in-code on
  `PushHudToPlayer` so the next reader does not lean on it. `TBD_ObjectiveHud.c` is outside my owns; I
  did not add the caller.
- **No behavioural lane exists for this.** `world-boot` loads zero players and never leaves LOBBY, so
  no HUD is ever instantiated and no RPC is ever emitted on any automated lane. The RPC counts here are
  a source-driven simulation, not an observation on a live server. Two clients on a dedicated server
  remain the only way to see the real wire (the same `executor: human` gap that keeps T-181 deferred).
- **CPU cost is unchanged, deliberately.** The server still renders every player's snapshot every tick;
  only the *wire* is gated. A content-based dirty check cannot detect a change without computing the
  new value. `PushHudToPlayer` (the ungated path) does render twice when it sends — once to push, once
  to sign — but it runs on the LIVE edge and on request, not at 1 Hz.
- **`Tick` returns before `Deliver` when `UsableCount() == 0`** (`:149-150`, pre-existing). If every
  objective goes inert mid-round the HUD freezes on its last frame rather than being hidden. Not
  introduced here and not in scope, but the gate makes it slightly more visible: previously the same
  frame was resent every second, now nothing is sent at all.

## deviations

- **The "idle-round assertion" had to be built.** The slice owns two `.c` files; Enfusion has no unit
  test harness in this repo and I may not add a Rust test to `xtask`. The assertion is therefore an
  external awk probe that reads the real source. It is reproduced verbatim below so the loop is
  re-runnable by anyone; it is not committed, because committing it would be a third file.
- **What the probe does and does not prove.** It genuinely reads the shipped file — the perturbation
  was applied to the `.c`, not to the probe, and the RED came out without the probe being edited. What
  it evaluates is the decision the source encodes (gated / not gated, and `HudChanged`'s truth table)
  against a modelled tick loop. It is not an EnfScript interpreter and it does not execute
  `FillHudSnapshot`. It cannot catch an engine-level surprise; `mod compile` and a live two-client test
  are the other two halves.
- **S5's pre-fix number** (peak 68) is the probe's model store growing unbounded because the source
  named no prune. Before the fix there is no store at all, so read that row as "nothing in the file
  bounds this", not as a leak in shipped code.
- **The disconnect hook is belt *and* braces, on purpose.** The brief noted that per-tick pruning
  against `connected` needs no hook. Both are in: the prune is the load-bearing bound (it cannot be
  skipped by a hook that fails to fire), and `OnPlayerDisconnected` closes the one-tick window in which
  a recycled playerId could inherit the previous holder's signature.
- `hrustfmt` was never needed — no Rust file changed.

## commits

Branch `slice/T-946.55`, committed and clean. No push, no merge, no ship.

```
885d6b7a2 T-946.55: record that the client HUD pull path has no caller
2b0b33984 T-946.55: gate the objective HUD RPC on a per-owner rendered-snapshot difference
```

`git status --porcelain` on `apps/` is empty; the four `*.log` scratch files were deleted after their
contents were pasted above. The 19 EnfusionMCP `.c` remain gitignored and uncommitted.

## manual_checklist

Needs a dedicated server with at least two real clients on opposite sides — none of it is reachable
from `world-boot`.

1. **Idle round.** Two players, LIVE, nobody on an objective. After the first board lands, no further
   HUD RPC for 60 s. The panel must stay populated, not blank — this is the regression the gate could
   plausibly cause and the one to look at hardest.
2. **Mid-round joiner.** Third player connects during LIVE with nothing else changing. Their HUD fills
   within one tick of gaining a player controller. They must not sit on an empty panel waiting for
   somebody to capture something.
3. **Live capture bar.** Walk one player into a capture zone. Their bar advances smoothly at 1 Hz while
   the other player's HUD does not change and receives nothing.
4. **Per-side text.** Capture an objective. The icon flips to `+` for the owning side and `-` for the
   other in the same tick, and each side sees its own `StatusText`.
5. **LIVE → SAFE_START → LIVE.** HUD hides on the way out and re-appears on the way back in even though
   the board is unchanged (this is the `show == 0` record reset; if it is wrong the panel stays hidden).
6. **Disconnect and reconnect on a recycled playerId.** The reconnecting player gets a full board, not
   the previous holder's frame.
7. **Ops.** Watch server bandwidth over an idle minute before/after if a counter is available; the
   expected drop is 60 → 1 HUD RPC per player per minute.

---

## Appendix — `idle_hud_probe.awk`, verbatim

Run as
`awk -f idle_hud_probe.awk apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c`
(exit 0 = green, 1 = at least one RED). Works on either twin.

```awk
# idle_hud_probe.awk — T-946.55 measurement instrument (scratch; NOT a repo file).
#
# Reads the REAL TBD_ObjectivesComponent.c, extracts the HUD replication decision from its source,
# then simulates one minute of 1 Hz ticks and counts Reliable-Owner HUD RPCs.
#
# What it reads from the file (nothing is assumed):
#   * whether the per-player push inside ReplicateHud is gated at all, and by what;
#   * the truth table of the gate helper HudChanged(known, equal);
#   * whether the per-owner store is pruned against the connected list and/or on disconnect.
# What it simulates (the probe's own model, stated plainly):
#   * P players x M objectives, 60 ticks; per-player rendered snapshots; the store as a map.
#
# Exit 0 = all assertions green. Exit 1 = at least one RED.
#
# Usage: awk -f idle_hud_probe.awk <path to TBD_ObjectivesComponent.c>

# ---------------------------------------------------------------------------- strip + collect
{
    line = $0
    gsub(/"[^"]*"/, "\"\"", line)             # blank string literal contents
    i = index(line, "//")
    if (i > 0) line = substr(line, 1, i - 1)  # cut line comments
    src[NR] = line
    n = NR
}

END {
    # ------------------------------------------------------------------ FACT 1: the store member
    has_store = 0
    for (k = 1; k <= n; k++)
        if (src[k] ~ /ref[ \t]+map<int,[ \t]*string>[ \t]+m_mHudSignatures[ \t]*;/) has_store = 1

    # ------------------------------------------------- FACT 2: is the per-player push gated?
    rs = body_start(n, "void[ \t]+ReplicateHud[ \t]*\\(")
    re = body_end(n, rs)
    gate_line = 0; gate_continues = 0; push_line = 0; prune_line = 0
    for (k = rs; k <= re && rs > 0; k++) {
        if (!gate_line && src[k] ~ /if[ \t]*\([ \t]*![ \t]*HudChanged[ \t]*\(/) {
            gate_line = k
            if (src[k] ~ /continue[ \t]*;/ || src[k + 1] ~ /^[ \t]*continue[ \t]*;/) gate_continues = 1
        }
        if (!push_line && src[k] ~ /Push(HudSnapshot|HudToPlayer)[ \t]*\(/) push_line = k
        if (!prune_line && src[k] ~ /PruneHudSignatures[ \t]*\(/) prune_line = k
    }
    guarded = (rs > 0 && gate_line > 0 && gate_continues && push_line > gate_line)

    # ------------------------------------------- FACT 3: the HudChanged truth table, from source
    cs = body_start(n, "bool[ \t]+HudChanged[ \t]*\\(")
    ce = body_end(n, cs)
    unknown_sends = 0; equal_suppresses = 0; always_true = 0; always_false = 0; first_stmt = ""
    for (k = cs; k <= ce && cs > 0; k++) {
        s = src[k]; gsub(/^[ \t]+|[ \t]+$/, "", s)
        if (s == "") continue
        if (first_stmt == "") first_stmt = s
        if (s ~ /if[ \t]*\([ \t]*![ \t]*m_mHudSignatures\.Find[ \t]*\(/) {
            t = src[k + 1]; gsub(/^[ \t]+|[ \t]+$/, "", t)
            if (s ~ /return[ \t]+true[ \t]*;/ || t ~ /^return[ \t]+true[ \t]*;/) unknown_sends = 1
        }
        if (s ~ /return[ \t]+previous[ \t]*!=[ \t]*signature[ \t]*;/) equal_suppresses = 1
    }
    if (first_stmt ~ /^return[ \t]+true[ \t]*;/)  always_true = 1
    if (first_stmt ~ /^return[ \t]+false[ \t]*;/) always_false = 1

    # ------------------------------------- FACT 4: is the store bounded (pruned / disconnect hook)?
    ps = body_start(n, "void[ \t]+PruneHudSignatures[ \t]*\\(")
    pe = body_end(n, ps)
    prune_removes = 0
    for (k = ps; k <= pe && ps > 0; k++)
        if (src[k] ~ /m_mHudSignatures\.Remove[ \t]*\(/) prune_removes = 1
    prunes = (prune_line > 0 && prune_removes)

    ds = body_start(n, "void[ \t]+OnPlayerDisconnected[ \t]*\\(")
    de = body_end(n, ds)
    disconnect_hook = 0
    for (k = ds; k <= de && ds > 0; k++)
        if (src[k] ~ /m_mHudSignatures\.Remove[ \t]*\(/) disconnect_hook = 1

    printf "SOURCE FACTS (read from %s)\n", FILENAME
    printf "  per-owner store member m_mHudSignatures ....... %s\n", yn(has_store)
    printf "  ReplicateHud gates the push (if !HudChanged) ... %s\n", yn(guarded)
    printf "  HudChanged: unknown owner => send ............... %s\n", yn(unknown_sends)
    printf "  HudChanged: equal snapshot => suppress .......... %s\n", yn(equal_suppresses)
    printf "  HudChanged: unconditional return true .......... %s\n", yn(always_true)
    printf "  store pruned against connected list ............ %s\n", yn(prunes)
    printf "  store cleared on OnPlayerDisconnected .......... %s\n", yn(disconnect_hook)
    printf "\n"

    P = 8; M = 4; T = 60

    # ------------------------------------------------------------- S1  IDLE ROUND, NOTHING CHANGES
    reset()
    initial = 0; after_initial = 0
    for (t = 1; t <= T; t++)
        for (p = 1; p <= P; p++) {
            sig = snapshot(p, M, 0, 0, 0)
            if (send(p, sig)) { if (t == 1) initial++; else after_initial++ }
        }
    per_player_min = (initial + after_initial) / P
    printf "S1 idle round (%d players, %d objectives, %d ticks, nothing changes)\n", P, M, T
    printf "   RPCs total ................ %d\n", initial + after_initial
    printf "   RPCs per player per minute  %.1f\n", per_player_min
    printf "   after the initial board ... %d\n", after_initial
    assert(after_initial == 0, "S1 idle-round: HUD RPCs after the initial board must be 0, got " after_initial " (" per_player_min "/player/min)")

    # --------------------------------------------------------------- S2  MID-ROUND JOINER, TICK 30
    reset()
    joiner_got = 0
    for (t = 1; t <= T; t++) {
        for (p = 1; p <= P; p++) { sig = snapshot(p, M, 0, 0, 0); send(p, sig) }
        if (t >= 30) { sig = snapshot(99, M, 0, 0, 0); if (send(99, sig) && !joiner_got) joiner_got = t }
    }
    printf "\nS2 mid-round joiner appears at tick 30 ....... first board at tick %d\n", joiner_got
    assert(joiner_got == 30, "S2 joiner: unknown owner must get a full board on first sight (tick 30), got tick " joiner_got)

    # ------------------------------------------------- S3  LIVE CAPTURE BAR, PERCENT MOVES EVERY TICK
    reset()
    capt = 0
    for (t = 1; t <= T; t++) { sig = snapshot(1, M, 1, t, 1); if (send(1, sig)) capt++ }
    printf "S3 capturing player, bar percent moves each tick  %d/%d ticks sent\n", capt, T
    assert(capt == T, "S3 capture bar: a per-player bar that changes every tick must send every tick, got " capt "/" T)

    # ------------------------------------------ S4  PER-SIDE TEXT: ONE SIDE'S ROW CHANGES AT TICK 40
    reset()
    a_at40 = 0; b_at40 = 0
    for (t = 1; t <= T; t++) {
        sa = snapshot(1, M, 0, 0, 0)                       # side A: unchanged all round
        sb = snapshot(2, M, 0, 0, (t >= 40) ? 1 : 0)       # side B: its rendered text flips at 40
        if (send(1, sa) && t == 40) a_at40 = 1
        if (send(2, sb) && t == 40) b_at40 = 1
    }
    printf "S4 side-B-only text change at tick 40 ........ B sent=%d  A sent=%d\n", b_at40, a_at40
    assert(b_at40 == 1, "S4 per-side: a change visible only to side B must reach B at tick 40")
    assert(a_at40 == 0, "S4 per-side: side A saw no change and must not be sent to at tick 40")

    # ------------------------------------------------ S5  BOUNDED STORE ACROSS CONNECT/DISCONNECT CHURN
    reset()
    peak = 0; live = P
    for (t = 1; t <= T; t++) {
        # one player leaves and a fresh id joins every tick: 60 distinct ids over the round
        gone = 1000 + t - 1
        for (p = 1; p <= P; p++) { sig = snapshot(p, M, 0, 0, 0); send(p, sig) }
        sig = snapshot(1000 + t, M, 0, 0, 0); send(1000 + t, sig)
        if (prunes || disconnect_hook) delete store[gone]
        c = 0; for (key in store) c++
        if (c > peak) peak = c
    }
    printf "S5 churn: 60 distinct ids over the round ..... peak store size %d (concurrent players %d)\n", peak, P + 1
    assert(peak <= P + 1, "S5 bounded: per-owner store must be bounded by concurrent players (" P + 1 "), peaked at " peak)

    printf "\n"
    if (fails == 0) { printf "IDLE HUD PROBE: PASS\n"; exit 0 }
    printf "IDLE HUD PROBE: FAIL (%d assertion(s))\n", fails
    exit 1
}

# ------------------------------------------------------------------------------------- helpers
function yn(b) { if (b) return "yes"; return "NO" }

function assert(cond, msg) {
    if (cond) return
    fails++
    printf "   RED: %s\n", msg
}

function reset(   key) { for (key in store) delete store[key] }

# The per-player RENDERED snapshot, the thing FillHudSnapshot actually emits: M rows of
# icon/title/detail plus the capture bar triple. `variant` flips this player's own row text.
function snapshot(pid, rows, barVisible, barPercent, variant,   s, r) {
    s = barVisible "|" barPercent "|bar"
    for (r = 1; r <= rows; r++) s = s "|o|OBJ" r "|held " variant
    return s
}

# The decision the SOURCE makes, applied to this player's stored state.
function send(pid, sig,   known, equal, go) {
    known = (pid in store)
    equal = (known && store[pid] == sig)
    if (!guarded)        go = 1
    else if (always_true)  go = 1
    else if (always_false) go = 0
    else if (!known)       go = unknown_sends
    else                   go = (equal ? 0 : (equal_suppresses ? 1 : 0))
    if (go) store[pid] = sig
    return go
}

# First line of the body of the function whose declaration matches `pat` (0 when absent).
function body_start(last, pat,   k) {
    for (k = 1; k <= last; k++)
        if (src[k] ~ pat) {
            while (k <= last && index(src[k], "{") == 0) k++
            return k + 1
        }
    return 0
}

# Last line of the body opened at `start-1` (brace depth back to zero).
function body_end(last, start,   k, depth, c, j) {
    if (start <= 0) return 0
    depth = 1
    for (k = start; k <= last; k++) {
        for (j = 1; j <= length(src[k]); j++) {
            c = substr(src[k], j, 1)
            if (c == "{") depth++
            else if (c == "}") { depth--; if (depth == 0) return k - 1 }
        }
    }
    return last
}
```
