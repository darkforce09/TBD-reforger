# T-946.14 — `wave land`'s post-merge gate anchors at the wrong commit

## The defect

`cmd_land` merges each ready slice into main and then runs the wave gate over the result. It takes
its base once, before the merges:

```rust
// The base is the last known-GREEN main. It is the gate's diff anchor and the revert target.
let base = git_stdout_lossy(&["rev-parse", "HEAD"]);
```

and passes that same value to `gate::cmd_gate(ctx, &base)`.

Those are two different jobs. As the **revert target** pre-merge HEAD is exactly right: it is the
commit to roll back to. As the gate's **diff anchor** it is wrong, because the gate's scope steps
(`touch_changed`, `wasm32`, `fmt (changed)`, `trunk build`) derive what to examine from
`base..HEAD`. Anchored at pre-merge HEAD, that range holds only the merge just made, so every
commit the wave landed earlier — and every command-centre commit since the close marker — is
outside the gate's sight while the gate reports on "the wave".

The gate's own base check refuses it rather than reporting a fraction of the wave. Measured
2026-09-06 landing T-675.1 into wave 241:

```
gate: base edb4e4e4d starts AFTER this wave opened — refusing to run.
        this wave opened at 52a038a77
          wave 240 CLOSED — wave 240: roads to rkyv, water vectors + bathymetry, slot identity at schema 1.3; GATE PASS
        7 commit(s) of this wave sit OUTSIDE edb4e4e4da4b537c524abf16be88563a5c596fb4..HEAD. touch_changed, wasm32,
        fmt and the trunk build would each report PASS/SKIP without reading one of them,
        and the verdict would describe a fraction of the wave. That is T-602 verbatim.
```

That refusal is T-602 working. The anchor is the defect, not the check.

It fires on **every** wave that does any command-centre work between the close marker and the first
land — a ledger row, the wave's briefs, a machinery fix — which is every wave this run. The merge
still happens (it precedes the gate), so the operator is left with a landed slice, an aborted
bookkeeping step and a refusal that names a base nobody chose.

## The repair

Keep `base` as the revert target. Pass `""` to the gate so it derives its own base from the
close-marker ledger — the same base the end-of-wave gate and the close ceremony already use — and
drop the base from the "re-run" hint, which was advising the operator to repeat the refusal.

## What is not covered

The post-merge gate path has no unit test, for the same reason `close_ceremony` has none: it needs a
live registry, a gateable tree and a real merge, which a unit test cannot fabricate honestly. The
proof is the re-run against the real repo, recorded on the ticket.
