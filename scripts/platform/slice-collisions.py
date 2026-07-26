#!/usr/bin/env python3
"""Compute the maximum FILE-DISJOINT set of platform tickets that can run concurrently.

The parallelism limit on this program is not disk and not CPU — it is merge conflicts.
Worktrees make concurrent edits *safe* (no clobbering) but do nothing to prevent two agents
editing the same file and colliding at merge. That is a mechanical property of the `owns`
column in docs/platform/wave_plan.tsv, so it is computed here rather than eyeballed.

  python3 scripts/platform/slice-collisions.py                 # max concurrent set from the next wave
  python3 scripts/platform/slice-collisions.py T-190 T-191     # what may JOIN those already in flight
  python3 scripts/platform/slice-collisions.py --repack        # rebuild wave_plan.tsv from the registry
  python3 scripts/platform/slice-collisions.py --check T-190   # is T-190 safe against everything running?

Unlike the T-181 mod version this takes the plan path from TBD_WAVE_PLAN, so the same logic
serves any program. Default is the platform plan.
"""
import csv, json, os, sys, subprocess, collections

ROOT = subprocess.run(['git', 'rev-parse', '--show-toplevel'],
                      capture_output=True, text=True).stdout.strip()
PLAN = os.environ.get('TBD_WAVE_PLAN', os.path.join(ROOT, 'docs/platform/wave_plan.tsv'))
REGISTRY = os.path.join(ROOT, '.ai/tickets/registry.json')
# Integration attention, not disk, is the real ceiling: every agent returns a dense report the
# command center must actually read. Measured on T-181: three was far too low, twenty is too many
# to integrate in one sitting. Eight is the working compromise — raise it if you are keeping up.
MAX_CONCURRENT = int(os.environ.get('TBD_MAX_CONCURRENT', '8'))

# Ordering constraints that file-disjointness cannot express. Each of these is a case where two
# tickets touch DIFFERENT files but one still has to land first, so the collision computation alone
# would happily run them together and produce a broken tree.
#
#   T-273 -> T-237 -> T-238  `ticket check` is inside the wave gate. T-237 wires it to validate
#                            against schema.json, and schema.json is a month stale — every one of
#                            the 113 tickets violates it today. Land T-237 first and the gate goes
#                            red on the whole registry, failing every subsequent wave.
#   T-241 -> zones consumers The doc has no `zones` root at all. Four tickets would each invent a
#                            different one; T-241 declares the vocabulary once.
#   T-222 -> sync consumers  CLIENT_ID is hardcoded to 1. Any sync transport that lands first
#                            corrupts documents on every multi-peer merge.
#   T-257 -> undo consumers  `objectives`/`markers` are cleared by hydrate but not undo-scoped, so
#                            both features would ship non-undoable.
#   T-186 -> T-209 -> T-251  test lane, then CI wiring, then deploy.
#   T-290 LAST               it annotates fields as non-consumed that five earlier tickets build.
DEPS = {
    'T-237': ['T-273'], 'T-238': ['T-273', 'T-237'],
    'T-201': ['T-241'], 'T-211': ['T-241'], 'T-212': ['T-241', 'T-257'], 'T-275': ['T-241'],
    'T-190': ['T-222'], 'T-295': ['T-222'], 'T-213': ['T-257'],
    'T-209': ['T-186'], 'T-251': ['T-209'],
}
RUN_LAST = {'T-290'}


def plan_rows():
    if not os.path.exists(PLAN):
        sys.exit(f"no wave plan at {PLAN} (set TBD_WAVE_PLAN)")
    with open(PLAN) as fh:
        for r in csv.reader(fh, delimiter='\t'):
            if not r or r[0].startswith('#') or r[0] == 'wave':
                continue
            if len(r) < 4:
                continue
            yield {'wave': r[0], 'id': r[1], 'title': r[2],
                   'owns': [p.strip() for p in r[3].split(';') if p.strip()]}


def registry():
    with open(REGISTRY) as fh:
        return {t['id']: t for t in json.load(fh)['tickets']}


def shipped(tid, reg):
    return reg.get(tid, {}).get('status') in ('shipped', 'cancelled')


def dispatchable(tid, reg):
    """Can a slice AGENT take this ticket, or is a human the only one who can?

    Two ways a ticket is undispatchable even though it is not shipped:

      status `deferred`  — a slice agent already took it and refused with cause. T-205 and T-206
                           are the live case: the missing vehicle/item data only exists behind a
                           Workbench export pass. Re-dispatching burns a whole agent to re-derive
                           the same refusal.
      executor != claude-code — the D5 executor gate in CLAUDE.md. `workbench`, `human` and `ci`
                           rows are operator work by definition.

    Without this, `pack()` filtered on shipped/cancelled ALONE and kept offering both tickets at
    the head of every dispatch set, where they would have consumed 2 of 8 slots per wave forever.
    """
    t = reg.get(tid, {})
    if t.get('status') in ('shipped', 'cancelled', 'deferred', 'blocked'):
        return False
    return t.get('executor', 'claude-code') == 'claude-code'


def collides(a, b):
    """Two tickets collide if any owned path overlaps — including prefix containment,
    so `apps/website/api/src/` collides with `apps/website/api/src/handlers/admin.rs`."""
    for x in a:
        for y in b:
            if x == y or x.startswith(y.rstrip('/') + '/') or y.startswith(x.rstrip('/') + '/'):
                return True
    return False


def pack(cands, already=(), landed=None, enforce_deps=True):
    """Greedy maximum disjoint set, honouring plan order (which is priority order) and DEPS.

    `landed` defaults to everything already shipped. It MUST NOT default to an empty set: repack()
    seeded it explicitly but main() did not, so `wave.sh prep` — the only dispatch view — silently
    skipped every ticket carrying a DEPS edge, forever. 11 tickets were unreachable, including
    T-209 whose dependency T-186 had already shipped. Computing it here covers both callers.
    """
    if landed is None:
        landed = {tid for tid, t in registry().items()
                  if t.get('status') in ('shipped', 'cancelled')}
    chosen, used = [], list(already)
    for c in cands:
        if enforce_deps:
            if c['id'] in RUN_LAST:
                continue
            if any(d not in landed for d in DEPS.get(c['id'], ())):
                continue
        if any(collides(c['owns'], u) for u in used):
            continue
        chosen.append(c)
        used.append(c['owns'])
        if len(chosen) + len(already) >= MAX_CONCURRENT:
            break
    return chosen


def repack():
    """Rebuild the plan from the registry, re-packing every unshipped ticket by disjointness.
    Preserves each row's `owns` — only the wave numbers move."""
    reg = registry()
    rows = [r for r in plan_rows() if dispatchable(r['id'], reg)]
    done = [r for r in plan_rows() if not dispatchable(r['id'], reg)]
    waves, remaining, n = [], list(rows), 0
    # Seed `landed` with everything already shipped. Without this, a DEPS edge pointing at a shipped
    # ticket can never be satisfied — the dependency is filtered out of `rows` as done, so it never
    # enters `landed`, and every dependent deadlocks. Hit for real on 2026-07-26 once T-186 shipped:
    # T-209 -> T-186 and T-251 -> T-209 both became unschedulable.
    landed = {tid for tid, t in registry().items()
              if t.get('status') in ('shipped', 'cancelled')}
    last = [r for r in remaining if r['id'] in RUN_LAST]
    remaining = [r for r in remaining if r['id'] not in RUN_LAST]
    while remaining:
        n += 1
        w = pack(remaining, landed=landed)
        if not w:
            # Everything left is either colliding or dep-blocked. Take the first whose deps are
            # satisfied; if none are, the DEPS table has a cycle and that is a bug worth shouting about.
            free = [r for r in remaining if all(d in landed for d in DEPS.get(r['id'], ()))]
            if not free:
                sys.exit(f"DEPS deadlock: {[r['id'] for r in remaining][:8]} — check the DEPS table")
            w = [free[0]]
        for r in w:
            remaining.remove(r)
            landed.add(r['id'])
        waves.append(w)
    for r in last:                     # RUN_LAST tickets get their own trailing wave
        waves.append([r])
    out = ["# Platform wave plan — WHICH tickets run together, and in what order.",
           "# Columns: wave <TAB> ticket <TAB> title <TAB> owns (semicolon-separated paths)",
           "# Waves are packed by FILE-DISJOINTNESS in priority order.",
           "# Regenerate: python3 scripts/platform/slice-collisions.py --repack", "#"]
    for r in done:
        out.append(f"0\t{r['id']}\t{r['title']}\t{'; '.join(r['owns'])}")
    for i, w in enumerate(waves, 1):
        for r in w:
            out.append(f"{i}\t{r['id']}\t{r['title']}\t{'; '.join(r['owns'])}")
    with open(PLAN, 'w') as fh:
        fh.write('\n'.join(out) + '\n')
    print(f"repacked {sum(len(w) for w in waves)} open tickets into {len(waves)} waves "
          f"({len(done)} already shipped, parked at wave 0)")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    flags = {a for a in sys.argv[1:] if a.startswith('--')}
    if '--repack' in flags:
        return repack()

    reg = registry()
    rows = [r for r in plan_rows() if dispatchable(r['id'], reg)]
    by_id = {r['id']: r for r in rows}

    if '--check' in flags:
        if not args:
            sys.exit("--check needs a ticket id")
        t = by_id.get(args[0])
        if not t:
            sys.exit(f"{args[0]} is not an open ticket in {os.path.relpath(PLAN, ROOT)}")
        bad = [o['id'] for o in rows if o['id'] != t['id'] and collides(t['owns'], o['owns'])]
        print(f"{t['id']} owns: {'; '.join(t['owns'])}")
        print("collides with:", ', '.join(bad) if bad else "nothing — safe to run alongside anything")
        return

    running = [by_id[a] for a in args if a in by_id]
    for a in args:
        if a not in by_id:
            print(f"warning: {a} is not an open ticket in the plan", file=sys.stderr)

    cands = [r for r in rows if r['id'] not in {x['id'] for x in running}]
    picked = pack(cands, already=[r['owns'] for r in running])

    if running:
        print(f"already in flight ({len(running)}):")
        for r in running:
            print(f"  {r['id']:8s} {r['title'][:60]}")
        print(f"\nmay join them ({len(picked)}, cap {MAX_CONCURRENT}):")
    else:
        nxt = min((r['wave'] for r in rows), key=lambda w: int(w))
        print(f"next wave is {nxt}. Max disjoint dispatch set ({len(picked)}, cap {MAX_CONCURRENT}):")
    for r in picked:
        print(f"  {r['id']:8s} {r['title'][:60]}")
        print(f"           owns: {'; '.join(r['owns'])}")
    if not picked:
        print("  (none — everything left collides with what is already running)")

    blocked = collections.Counter()
    for c in cands:
        if c not in picked:
            for r in picked + running:
                if collides(c['owns'], r['owns']):
                    blocked[r['id']] += 1
    if blocked:
        print("\nmost-contended tickets (blocking the most others):")
        for tid, n in blocked.most_common(5):
            print(f"  {tid} blocks {n}")


if __name__ == '__main__':
    main()
