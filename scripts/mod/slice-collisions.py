#!/usr/bin/env python3
"""Which slices can run CONCURRENTLY, computed instead of guessed.

The wave size was capped at 3 for a long time on the belief that disk was the
constraint. Measured 2026-07-25: a worktree is ~81 MB against 131 GB free, so ten
would cost under a gigabyte. Disk was never the limit.

The real limit is what SLICE_WORKFLOW rule 7 always said: FILE COLLISIONS. Worktrees
make concurrent edits safe (no clobbering) but they do not prevent merge conflicts, so
two agents must never own overlapping paths. That is a mechanical property of the
`owns` column in wave_plan.tsv, so it should be computed, not eyeballed.

  python3 scripts/mod/slice-collisions.py                 # max concurrent set from all unshipped
  python3 scripts/mod/slice-collisions.py T-181.32 T-181.27   # what can join those already running
"""
import csv, os, re, sys, itertools, json

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
PLAN = os.path.join(ROOT, 'docs/mod/wave_plan.tsv')
REG  = os.path.join(ROOT, '.ai/tickets/registry.json')
MOD  = 'apps/mod/tbd-framework/Scripts/Game/TBD/'


def resolve(owns):
    """`owns` column -> a real repo path, resolving bare filenames by searching the tree."""
    o = owns.replace(' (new)', '').split(' + ')[0].strip()
    if o.startswith(('apps/', 'packages/', 'scripts/', 'docs/')):
        return o
    if o.startswith('Scripts/Game/TBD/'):
        return 'apps/mod/tbd-framework/' + o
    p = MOD + o
    if '*' in p or os.path.exists(os.path.join(ROOT, p)):
        return p
    base = os.path.basename(p)                       # bare filename: find where it really lives
    for dirpath, _, files in os.walk(os.path.join(ROOT, 'apps/mod/tbd-framework/Scripts')):
        if base in files:
            return os.path.relpath(os.path.join(dirpath, base), ROOT)
    return p


def collide(a, b):
    """True when two `owns` paths overlap. Dir globs swallow everything beneath them."""
    a, b = a.rstrip('/'), b.rstrip('/')
    if a == b:
        return True
    for x, y in ((a, b), (b, a)):
        if x.endswith('**') and y.startswith(x[:-2]):
            return True
        if '*' in x and re.fullmatch(re.escape(x).replace(r'\*', '[^/]*'), y):
            return True
    return False


def load():
    rows = [r for r in csv.reader(open(PLAN), delimiter='\t')
            if r and not r[0].startswith('#') and r[0] != 'wave']
    plan = {r[1]: {'wave': r[0], 'title': r[2], 'owns': resolve(r[3])} for r in rows}
    try:
        reg = json.load(open(REG))
        sp = [t for t in reg['tickets'] if t.get('id') == 'T-181'][0]['slice_plan']
        for s in plan:
            plan[s]['status'] = sp.get(s, {}).get('status', '?')
    except Exception:
        for s in plan:
            plan[s]['status'] = '?'
    return plan


def main():
    plan = load()
    running = [s for s in sys.argv[1:] if s in plan]
    unknown = [s for s in sys.argv[1:] if s not in plan]
    for s in unknown:
        print(f"warning: {s} is not in wave_plan.tsv", file=sys.stderr)

    todo = [s for s, v in plan.items()
            if v['status'] not in ('shipped',) and s not in running]

    if running:
        print("IN FLIGHT:")
        for s in running:
            print(f"  {s:12} {plan[s]['owns']}")
        print()

    blocked, free = {}, []
    for s in todo:
        hits = [r for r in running if collide(plan[s]['owns'], plan[r]['owns'])]
        if hits:
            blocked[s] = hits
        else:
            free.append(s)

    # Greedy maximal set: take slices in plan order, skip any that collides with one already taken.
    chosen = []
    for s in free:
        if not any(collide(plan[s]['owns'], plan[c]['owns']) for c in chosen):
            chosen.append(s)

    print(f"CAN RUN CONCURRENTLY ({len(chosen)}):")
    for s in chosen:
        print(f"  {s:12} w{plan[s]['wave']}  {plan[s]['status']:8} {plan[s]['owns']}")

    deferred = [s for s in free if s not in chosen]
    if deferred:
        print("\nDEFERRED (collide with a chosen slice):")
        for s in deferred:
            why = [c for c in chosen if collide(plan[s]['owns'], plan[c]['owns'])]
            print(f"  {s:12} vs {why[0]:12} {plan[s]['owns']}")
    if blocked:
        print("\nBLOCKED BY IN-FLIGHT WORK:")
        for s, hits in blocked.items():
            print(f"  {s:12} vs {hits[0]:12} {plan[s]['owns']}")

    print(f"\ntotal concurrent if dispatched now: {len(running) + len(chosen)}")


if __name__ == '__main__':
    main()
