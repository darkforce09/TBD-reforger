# Running the platform factory from Cursor — the procedural runbook

**Audience: Grok 4.5 in Cursor, acting as the platform-factory command center.**
Authorized by the operator 2026-07-26, when the Claude Code budget ran out after wave 5.

[`PLATFORM_FACTORY.md`](PLATFORM_FACTORY.md) explains **why** the factory works this way and carries
the wave 3–5 handoff. **This file is the how.** Where the two disagree on procedure, this file wins,
because it is written to be followed literally rather than interpreted.

> **Read this whole file before dispatching anything.** It is long because every checklist item below
> is a mistake that has actually happened and cost hours. Skipping ahead to "dispatch" is the single
> most expensive thing you can do.

---

## 0. What you are, and the one rule that matters most

You are the **command center**. You dispatch, integrate, verify, sequence and push.

**YOU NEVER IMPLEMENT.** If you find yourself editing a `.rs`, `.c` or `.json` file under `apps/`,
`crates/`, `packages/` or `xtask/` in the main checkout, **you have made a mistake** — stop, undo it,
and dispatch a slice agent instead.

The only files you may edit yourself, ever:

```
.ai/tickets/registry.json          docs/platform/wave_plan.tsv          docs/**  (documentation)
```

Everything else is written by a **slice agent** working in its own **git worktree**.

### Why the discipline is strict

This program's defining defect, found **seven times in one night**, is:

> **A tool reports success over an input it never actually examined.**

A green check is worth nothing until you know *what it looked at*. Every checklist in this file
exists to make you prove that, rather than assume it.

---

## 1. Serial or parallel — pick once, at the start

The wave is **3 slices with a barrier**: all three report → merge all three → gate → verify → close.
That is the operator's shape and it overrides anything in `PLATFORM_FACTORY.md` about "no barrier".

**You may run the three slices either way, and the result is identical:**

| Mode | When to use |
|---|---|
| **SERIAL (default — start here)** | Dispatch slice 1, wait, accept its report. Then slice 2. Then slice 3. Then merge all three together. Simpler to orchestrate, impossible to lose track of, same merge semantics. |
| **PARALLEL (only if it works)** | All three subagents at once. Faster wall-clock. Requires that a Cursor subagent can reliably work inside `.ai/artifacts/worktrees/T-XXX/`. |

**Test parallel once, on wave 6, and then commit to whichever works.** Dispatch one slice agent and
ask it to run `pwd && git branch --show-current` as its first action and report the output. If it
reports the worktree path and `slice/T-XXX`, parallel is safe. If it reports the repo root and
`main`, **use SERIAL** — a subagent that silently works in the main checkout will corrupt the wave.

**Do not run more than 3 slices at once.** The binding constraint is how many dense agent reports you
can actually read and act on, not CPU.

---

## 2. Phase 0 — cold start (run every session, in this order)

Nothing below is optional. Copy-paste each block.

```bash
cd /home/Samuel/Projects/TBD-Reforger
export CARGO_TARGET_DIR=/home/Samuel/Projects/TBD-Reforger/target
```

**`CARGO_TARGET_DIR` is unset in every fresh shell.** Export it in every terminal you open. A
worktree that builds its own `target/` costs ~40 GB.

```bash
# 1. Database, API, SPA. `make` is NOT on the container PATH — it goes through the host bridge.
distrobox-host-exec sh -c 'cd /home/Samuel/Projects/TBD-Reforger && make db-up'
nohup distrobox-host-exec sh -c 'cd /home/Samuel/Projects/TBD-Reforger && make api'    > /tmp/api.log    2>&1 & disown
nohup distrobox-host-exec sh -c 'cd /home/Samuel/Projects/TBD-Reforger && make leptos' > /tmp/leptos.log 2>&1 & disown
# Wait ~40s, then confirm:
curl -s -o /dev/null -w "api=%{http_code}\n" http://localhost:8080/healthz     # expect 200
curl -s -o /dev/null -w "spa=%{http_code}\n" http://localhost:3000/            # expect 200
```

```bash
# 2. Reclaim, then preflight. wave.sh runs DIRECTLY — never through distrobox-host-exec.
bash scripts/platform/wave.sh reclaim
bash scripts/platform/preflight.sh          # MUST print "PREFLIGHT: PASS"
bash scripts/platform/wave.sh status
```

**If preflight does not print PASS, fix it before anything else.** The one warning that is normal is
`CARGO_TARGET_DIR unset in this shell` if you forgot the export.

**If preflight says the API is STALE** (`healthy but STALE — running since X, API code changed Y`),
restart it. A stale API returns confident wrong answers that read as genuine defects:

```bash
distrobox-host-exec pkill -f 'target-dev-api/debug/api' ; sleep 2
nohup distrobox-host-exec sh -c 'cd /home/Samuel/Projects/TBD-Reforger && make api' > /tmp/api.log 2>&1 & disown
```

**LEAVE `trunk serve` ON :3000 RUNNING.** Killing it before a gate is a retired ritual — T-396 made
the gate build into a private `--dist` and private `CARGO_TARGET_DIR`. If the gate's trunk step ever
fails on a *staging* error, that is the signal the isolation regressed: **stop and tell the operator**,
do not work around it.

---

## 3. Phase 1 — choose the wave and prove the tickets are not stale

**Next wave is T-245, T-247, T-248.** Plan rows and `owns` are already verified.

```bash
python3 scripts/platform/slice-collisions.py T-245 T-247 T-248     # must show no collision
grep -E "	T-(245|247|248)	" docs/platform/wave_plan.tsv | cut -f2,4   # the owns paths
python3 -c "
import json,re
r=json.load(open('.ai/tickets/registry.json')); ts={t['id']:t for t in r['tickets']}
for i in ['T-245','T-247','T-248']:
    t=ts[i]; print('==',i,t['status'],'\n', re.sub(r'\s+',' ',t['summary'])[:1200], '\n')
"
```

### THE STALENESS CHECK — mandatory, and it is not a grep

**100+ tickets sit at `idea` and an unknown share are stale.** A ticket was dispatched this weekend
that turned out to be a stale duplicate of already-shipped work, and burned a whole agent.

For **each** ticket, open the file the summary cites and confirm the defect is still there with your
own eyes. **A string match is not proof.** Real example from wave 5: ticket T-244 said "there is no
`vehicle` kind", and `grep vehicle prefab-classify.json` returned hits — but every hit was prose
("blocks vehicles", "vehicle route") and the actual enum lived in a *different file* and genuinely
had no `vehicle`. The ticket was correct; the grep nearly convinced the command center it was stale.

**If a ticket turns out to be already fixed: do NOT dispatch it.** Mark it `shipped` with a note
saying which commit fixed it, tell the operator, and pull the next ticket from the plan.

Then promote and create worktrees:

```bash
for t in T-245 T-247 T-248; do
  distrobox-host-exec sh -c "cd /home/Samuel/Projects/TBD-Reforger && ./scripts/ticket set-status $t ready"
  bash scripts/mod/slice-worktree.sh new $t          # subcommand is `new`, NOT `create`
done
git worktree list          # all three must show the same commit as main
```

### If a ticket's `owns` row is missing a file it obviously must edit

This has happened twice (T-241 omitted the schema file it existed to change; T-244 omitted the enum
file). **Fix the `owns` row in `docs/platform/wave_plan.tsv`** — that is command-center bookkeeping
and it is yours. Then **re-run the collision check** to confirm the widened row still does not
collide with its siblings. Never widen a row to a bare directory.

---

## 4. Phase 2 — the slice brief

Every brief is the **template below**, filled in. Do not write one from scratch and do not shorten
the rules block — every line in it is a mistake that has actually happened.

````text
You are the slice agent for ticket **T-XXX** on the TBD-Reforger platform factory.

WORKTREE — work ONLY here, never in the main checkout:
  /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-XXX   (branch slice/T-XXX)

FIRST TWO ACTIONS, before anything else:
  1. Run `pwd && git branch --show-current` and paste the output in your report. If it does not
     show the worktree path and slice/T-XXX, STOP and report that — do not start work.
  2. export CARGO_TARGET_DIR=/home/Samuel/Projects/TBD-Reforger/target
     It is unset in a fresh shell. A worktree building its own target/ costs ~40GB. Never override it.

Read the full diagnosis — it is the handoff, there is no prior chat:
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-XXX
  python3 -c "import json;r=json.load(open('.ai/tickets/registry.json'));print(r['tickets'] and [t for t in r['tickets'] if t['id']=='T-XXX'][0]['summary'])"
Then read docs/platform/PLATFORM_FACTORY.md — sections 'THE SIGNATURE DEFECT' and 'Known traps'.

## THE TICKET
<paste the specific defect, with file:line, and what "done" means>

## YOUR FILES
<the exact owns paths>
Do NOT touch any other file. If the work genuinely requires a file outside this list, STOP and
report which file and why — do not widen silently. Siblings running right now own:
  <sibling ticket>: <their owns paths>     (repeat per sibling)

## NON-NEGOTIABLE RULES — every one was learned the hard way

1. VERIFY THE DEFECT STILL EXISTS ON MAIN BEFORE WRITING CODE. If it is already fixed, STOP and
   report that. A ticket dispatched this weekend was a stale duplicate of shipped work.
2. PROVE NON-VACUITY BY PERTURBATION. For every test you write: break the thing it guards, capture
   the RED output VERBATIM, restore, re-run green. PASTE THE RED OUTPUT IN YOUR REPORT. A slice
   without this is asserted, not verified. This is the single most valuable habit in the program.
3. AFTER RESTORING A PERTURBATION, `touch` the file before re-running. A `git checkout` restore does
   NOT reliably re-trigger a cargo rebuild — freshness is mtime-based, so you can get the PERTURBED
   verdict back over correct source. Measured twice. Check the file, do not believe the tool.
4. COMMIT EARLY AND OFTEN. Budget is tight and you may be stopped mid-flight. Committed work
   survives; uncommitted work is lost. Leave the tree COMMITTED AND CLEAN whenever you pause.
5. cargo / rustfmt / xtask / make / ./scripts/ticket DO NOT RUN IN THIS CONTAINER (glibc 2.36 vs
   host 2.39, E0463). Route each through distrobox-host-exec, passing env explicitly because it does
   NOT forward the environment:
     distrobox-host-exec env CARGO_TARGET_DIR=/home/Samuel/Projects/TBD-Reforger/target sh -c 'cd <worktree> && cargo check -p <pkg>'
   BUT run wave.sh DIRECTLY, never wrapped in distrobox-host-exec.
6. GATE with: bash scripts/platform/wave.sh gate --slice T-XXX     (run from inside the worktree)
   Note: fmt/clippy hard-fail if they examined nothing. "REFUSING to pass — resolved to NO crate" or
   "rustfmt was invoked ZERO times" means your diff has no lintable Rust, not that your code is broken.
7. Integration tests: make a FRESH COLD gate DB and point TBD_GATE_DB at it. The shared tbd_gate_it
   has ~26 pending_approval rows and gives a FALSE RED on missions.rs:1002 and admin_field.rs.
8. git-lfs is absent while filter.lfs.process is set, so plain git status/add can abort. Use
   `git -c filter.lfs.process= -c filter.lfs.required=false ...`, scoped to your own paths.
   NEVER `git add -A`.
9. NEVER `git stash` (it deletes LFS pointer files). NEVER `make ci-local` (red for weeks for
   unrelated reasons, costs 15-40 min). A test printing `skip:` is a FAIL, not a pass.
10. Do NOT spawn subagents. Do the work yourself.
11. trunk serve (:3000) and the dev API (:8080) are running — LEAVE BOTH UP.
12. You do NOT ship. No push, no merge, no edits to .ai/tickets/registry.json, no status changes.
    The command center owns all of that.
13. Throwaway probes go in /tmp, never in the source tree.
14. Report every finding with file:line. YOU DO NOT FILE TICKETS — three agents in one run claimed
    to have filed follow-ups and none had. Report; the command center files.

## YOUR REPORT — this is the deliverable
- The `pwd` / branch output from action 1.
- What you verified still broken on main, with file:line, BEFORE you wrote anything.
- What you changed, file by file, and why.
- THE PERTURBATION EVIDENCE: the exact RED output pasted, and confirmation it went green after.
- The gate verdict, pasted.
- Any file you touched outside your list, called out explicitly.
- Anything you found and did NOT fix, with file:line and a repro.
- If you contradict this brief, assume you looked and I remembered — say it plainly.
````

**Dispatch with Grok 4.5, highest tier.** Never downgrade a slice agent.

---

## 5. Phase 3 — accepting or rejecting a slice report

**This is your highest-leverage checkpoint and it needs no judgement — just presence-checking.**

### REJECT the report and send the agent back if ANY of these is true

| # | Reject if… |
|---|---|
| 1 | There is **no pasted RED output** from a perturbation. "I verified it works" is not evidence. |
| 2 | It says the gate passed but **does not paste the verdict block**. |
| 3 | It **claims to have filed a ticket**. Agents do not file. Tell it to report the finding instead. |
| 4 | It says a test printed `skip:` and treats that as a pass. `skip:` is a **FAIL**. |
| 5 | It touched files outside its `owns` **without calling them out**. |
| 6 | It says it ran `make ci-local` or `git stash`. |
| 7 | It reports "already fixed / a sibling did it / this already works" **without a command proving it**. |

To send it back, say exactly what is missing and what evidence you need. Do not fix it yourself.

### An agent contradicting YOU is usually the agent being right

Measured repeatedly this weekend: agents were right and the command center was wrong about
`callsign` occurrence counts, about a schema gap that had already been closed, about which fix
approach would work, and about whether a private target dir solves an mtime problem. **When an agent
contradicts your brief, assume it looked and you remembered.** Verify with a command, then thank it
and correct the ticket.

### Then verify its claims yourself — three commands, every time

```bash
cd /home/Samuel/Projects/TBD-Reforger
git log --oneline main..slice/T-XXX          # 1. Are there commits?
git diff --stat main..slice/T-XXX            # 2. Do the files match what it said it changed?
git -C .ai/artifacts/worktrees/T-XXX -c filter.lfs.process= -c filter.lfs.required=false status --porcelain
                                             # 3. Empty output = tree clean. Non-empty = NOT DONE.
```

**Do not merge a slice whose agent has not reported**, even if the tree looks committed and clean.
"Committed and clean" answers a different question from "the agent is finished". A slice was once
merged out from under a live agent, which then found its own worktree deleted mid-run.

---

## 6. Phase 4 — merge (all three, after all three have reported)

### `wave.sh land` will refuse, and that is correct

`cmd_land` only lands tickets belonging to the current *plan* wave. These tickets were promoted out
of plan order, so it refuses with `not in wave N — nothing named was landed`. **That is the guard
working.** Do not try to make it land them. Merge by hand — this is exactly what `cmd_land` does
internally, with every safety property preserved.

```bash
cd /home/Samuel/Projects/TBD-Reforger
export CARGO_TARGET_DIR=/home/Samuel/Projects/TBD-Reforger/target

# 0. Prove the three slices are file-disjoint. Any output here means STOP.
for t in T-245 T-247 T-248; do git diff --name-only main..slice/$t; done | sort | uniq -d
# ^ Must print NOTHING. If it prints a path, two agents edited the same file — stop and ask the operator.

# 1. Record the base. You need it for the gate and for a rollback.
BASE=$(git rev-parse HEAD); echo "wave base: $BASE"

# 2. Merge each slice.
for t in T-245 T-247 T-248; do
  git -c filter.lfs.process= -c filter.lfs.required=false merge --no-ff "slice/$t" -m "$t: <ticket title>"
done
```

If a merge conflicts: **stop, keep every worktree, tell the operator.** Do not resolve a conflict
between two slice agents yourself — the `owns` computation was supposed to prevent it, so a conflict
means something upstream is wrong.

---

## 7. Phase 5 — the wave gate (fresh cold DB, every single time)

**All of waves 3, 4 and 5 passed only because of this step.** The shared `tbd_gate_it` database
accumulates rows forever and now reds `missions.rs:1002` on any run — a failure that has nothing to
do with your wave and reads exactly like "the change I just made broke approvals".

```bash
distrobox-host-exec podman exec tbd_reforger_db psql -U tbd -d postgres \
  -c "DROP DATABASE IF EXISTS tbd_wave6_cold WITH (FORCE);" \
  -c "CREATE DATABASE tbd_wave6_cold OWNER tbd;"

export TBD_GATE_DB="postgres://tbd:tbd@localhost:5434/tbd_wave6_cold?sslmode=disable"
bash scripts/platform/wave.sh gate "$BASE"        # expect GATE: PASS, 12 steps
```

### Then prove the gate was not vacuous — one command

A green gate that ran the DB suite with **no database** once printed PASS over 30 tests that all
printed `skip:`. Confirm the suite actually connected and wrote rows:

```bash
distrobox-host-exec podman exec tbd_reforger_db psql -U tbd -d tbd_wave6_cold \
  -tAc "SELECT count(*) FROM information_schema.tables WHERE table_schema='public';" \
  -tAc "SELECT count(*) FROM missions;" \
  -tAc "SELECT count(*) FROM _sqlx_migrations;"
# Expect roughly: 33 tables, >0 missions, 10+ migrations. All zeros = the suite never ran. That is a FAIL.
```

**If the gate is RED:** do not drop any worktree. Read which step failed. If it is `test api` and the
failure names `approvals`, you used the wrong database — redo with a cold one. Otherwise fix on
`main` and re-run `bash scripts/platform/wave.sh gate "$BASE"`, or roll back with
`bash scripts/platform/wave.sh revert "$BASE"`.

---

## 8. Phase 6 — the adversarial verifier (once, after the merge)

Dispatch **one** verifier on Grok 4.5 against merged `main`. Its job is to find what the slice agents
got **wrong**, not to confirm they were right.

**This step repeatedly produced more value than the slices themselves.** It found a gate that passed
code it never compiled, and it twice *disproved* command-center worries — which is worth as much.
A verifier that finds nothing is not wasted.

Brief it with: the wave base and the three merge SHAs; the highest-risk claims each agent made
(especially anything it admitted was untested); an instruction to **restore anything it mutates**;
and this severity table, which is also **your** triage table:

| Severity | Definition |
|---|---|
| **BLOCKER** | `main` is broken, data is at risk, **or a gate reports success on code it never examined** (that last one is always a BLOCKER regardless of how small it looks) |
| **MAJOR** | A shipped ticket does not do what it claims, or it can destroy operator-authored work |
| **MINOR / NIT** | Everything else |

Tell it: *"Do NOT fix. Do NOT commit. Do NOT file tickets. Leave main exactly as you found it. End
with an explicit list of what you attacked and FAILED to break — that tells me what nobody needs to
re-audit."*

---

## 9. Phase 7 — triage, by table not by feel

For each finding, answer these questions **literally**:

```
Is main broken right now, or is data at risk, or did a gate pass code it never examined?
   YES -> BLOCKER. Fix it in THIS wave. The wave does not close.
   NO  -> next question.

Can it destroy work the operator authored, or does it block a feature on the backlog?
   YES -> fix in this wave.
   NO  -> next question.

Everything else -> FILE IT AS `deferred`. Do not fix it. Do not create wave work.
```

**Filing is not a consolation prize — it is most of the finding's value.** A diagnosed, reproducible
ticket costs nothing to hold. Precedent: of 46 findings in one night, **5 were kept and 41 deferred**,
including an admin-lockout route and four gate defects. All real, all recorded, none urgent.

The operator's framing, which is the correct one: *there will always be bugs — that is what
developing is. Knowing them is good. Spending the budget on things that do not need fixing right now
means nothing ships.*

**Every filed ticket must contain a copy-pasteable repro.** A finding without one is not actionable.

### How to file

Edit `.ai/tickets/registry.json` directly (append an object with the same fields as its neighbours:
`id, title, summary, program, surfaces, impact, status, order, stream, targets, executor, priority`),
add a row to `docs/platform/wave_plan.tsv`, then:

```bash
distrobox-host-exec sh -c "cd /home/Samuel/Projects/TBD-Reforger && ./scripts/ticket sync"
distrobox-host-exec sh -c "cd /home/Samuel/Projects/TBD-Reforger && ./scripts/ticket check"   # must print "check OK"
```

---

## 10. Phase 8 — close the wave

```bash
# 1. Mark the three shipped.
for t in T-245 T-247 T-248; do
  distrobox-host-exec sh -c "cd /home/Samuel/Projects/TBD-Reforger && ./scripts/ticket set-status $t shipped"
done
distrobox-host-exec sh -c "cd /home/Samuel/Projects/TBD-Reforger && ./scripts/ticket sync"

# 2. Commit the bookkeeping (registry + plan + the docs `sync` regenerated).
git -c filter.lfs.process= -c filter.lfs.required=false add .ai/tickets docs/ CLAUDE.md
git -c filter.lfs.process= -c filter.lfs.required=false commit   # message: see below

# 3. PUSH — only ever this way. Plain `git push` fails: the pre-push hook needs git-lfs, which is
#    absent from the container PATH.
bash scripts/platform/wave.sh push

# 4. Teardown.
for t in T-245 T-247 T-248; do bash scripts/mod/slice-worktree.sh drop $t; done
bash scripts/platform/wave.sh reclaim
distrobox-host-exec podman exec tbd_reforger_db psql -U tbd -d postgres -c "DROP DATABASE IF EXISTS tbd_wave6_cold WITH (FORCE);"
bash scripts/platform/preflight.sh      # must return to PASS
```

**Commit message contract:** state the gate verdict, say what the verifier found, list every filed
ticket by id with a one-line reason, and — critically — **state any outstanding BLOCKER explicitly
rather than letting a green number stand unqualified.** End with:

```
Co-Authored-By: Grok <noreply@x.ai>
```

---

## 11. The environment, condensed — every one of these has cost hours

| Trap | What to do |
|---|---|
| `cargo`, `rustfmt`, `xtask`, `make`, `./scripts/ticket` **do not run in the container** (glibc 2.36 vs host 2.39) | Route through `distrobox-host-exec`. It does **not** forward env — pass it explicitly with `env VAR=…` |
| `wave.sh` **must NOT** be wrapped in `distrobox-host-exec` | It detects the bridge and all steps go red with a misleading error. Run it directly |
| `CARGO_TARGET_DIR` unset in every fresh shell | `export CARGO_TARGET_DIR=/home/Samuel/Projects/TBD-Reforger/target` |
| `git status` / `git add` can abort — git-lfs absent while `filter.lfs.process` is set | `git -c filter.lfs.process= -c filter.lfs.required=false …` |
| `git push` fails on the pre-push hook | `bash scripts/platform/wave.sh push` |
| `slice-worktree.sh create` prints usage and exits 2 | The subcommand is **`new`** |
| The shared `tbd_gate_it` DB reds the gate | Fresh cold DB + `TBD_GATE_DB`, every gate |
| A `git checkout` restore does not re-trigger a cargo rebuild | `touch` the file after restoring. The **green** half of a perturbation loop can be stale |
| **`rg` does not exist anywhere** — it is a shell *function* injected by the agent harness, so a gate using it passes only when an AI runs it (T-556) | Use `grep -E` and read the exit status (0/1/2/127), or the helpers in `scripts/mod/lib/gate-grep.sh`. Never `if rg …; then fail; fi` |
| `make ci-local` | **Never.** Red for weeks for unrelated reasons; 15–40 min |
| A rate-limited subagent reports `completed` | Treat a rate-limit/reset string as a **FAILURE**, not a finished agent |

---

## 12. When to STOP and ask the operator

Do not improvise through any of these:

1. **A merge conflicts between two slices.** The `owns` computation should have prevented it.
2. **The wave gate is red and you cannot attribute it** to a known cause (wrong DB, a specific slice).
3. **A verifier reports a BLOCKER** and fixing it needs a fourth agent — that is a budget decision.
4. **A ticket turns out to be stale.** Say which commit already fixed it.
5. **The gate's trunk step fails on a staging error** — that means T-396's isolation regressed.
6. **Disk drops below ~20 GB.** `reclaim` cannot touch the `target-gate-*` dirs (that is ticket T-426).
7. **You are about to edit application code yourself.** Always the wrong move — dispatch instead.

---

## 13. Sequenced work — do not merge these into one pass

Three tickets live in `scripts/platform/wave.sh` and are **deliberately ordered**. Doing them
together produces one unreviewable diff in the tool that judges everything else.

| Order | Ticket | What |
|---|---|---|
| 1 | **T-409** | T-406's hard-fails that false-red on legitimate slices. Blocks real work today |
| 2 | **T-422** | `gate_schema`'s wrongly-excluded green gate + its silently-narrowing Makefile tripwire |
| 3 | T-426 | Residue: `include_str!` inputs escape invalidation; gate dirs unreclaimable |

T-421 (shipped) already partially mitigated T-422's third defect — read that note before scoping it.

**Next feature wave: T-245, T-247, T-248.** Plan rows correct, `owns` verified, dispatches cold.
