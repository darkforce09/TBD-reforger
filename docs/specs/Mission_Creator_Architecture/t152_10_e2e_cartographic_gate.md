# T-152.10 — E2E cartographic gate (operator + merge readiness)

**Ticket:** T-152 · **Slice:** T-152.10  
**Status:** `ready` (blocked until **T-152.9** all Gn PASS)  
**Executor:** **grok-cursor** (+ **human** operator sign-off for M-rows)  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · tag **`T-152.10`**  
**Depends on:** **T-152.0–.9** verify logs ALL PASS

## In one sentence

Run the **program-wide automated matrix** + **operator cartographic checklist**; produce merge-readiness artifact; **`./scripts/ticket done T-152`** only after **zero open FAIL**.

---

## Problem

T-152 ships across **10+ slices** (parallel agents on `.0`–`.3`, Grok on `.4`–`.9`). Without a terminal gate, partial symbology can merge with open FAIL rows hidden in per-slice logs. Cartographic view (`basemapView=map`) must read as **A3-like** at island + local zoom: vectors, labels, airfield, fences — per [`t090_1_1_map_cartographic_view.md`](t090_1_1_map_cartographic_view.md). Registry must not mark **T-152 shipped** until this slice passes.

---

## Goal

1. **Aggregator script:** `scripts/map-assets/verify-t152-cartographic.mjs` — runs sub-verifiers (or checks verify log JSON tails) for slices **.4–.9** + baseline **.0–.3**.
2. **Master verify log:** `.ai/artifacts/t152_10_verify_log.md` — table of every **G**ate from `.4`–`.9` with PASS/FAIL + commit SHA.
3. **Operator checklist** (human): cartographic Map view @ Everon — sign rows **O1–O12** in verify log.
4. **Merge readiness:** `.ai/artifacts/t152_merge_readiness.md` — CI commands, LFS pointers, known limitations, worktree → `main` promotion steps (human merge).
5. **Registry:** Grok sets slice status `ready` → human/Cursor runs `./scripts/ticket done T-152` **only** when G-master + O-rows PASS.

---

## Out of scope

- New symbology features (fix forward = new ticket).
- Arland terrain parity.
- Satellite view changes.

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | **No open FAIL** in any `t152_*_verify_log.md` | User gate |
| L2 | **`make ci-local`** (or documented subset) exit 0 on worktree tip | T-125 |
| L3 | Operator **O1–O12** required — Grok assists, cannot self-sign | Human+Grok |
| L4 | Basemap **Map** radio + cartographic tiles unchanged (no regression to T-090.1.1) | Dual view |
| L5 | Wasm size logged; no hard regression vs T-152.3 tip | Telemetry |
| L6 | Tag **`T-152.10`** then program tag **`T-152`** on merge commit | Convention |
| L7 | Fix failures **in worktree** — no "waived" without operator quote in log | No silent deferral |

---

## Operator checklist (O1–O12)

| ID | Check | Pass criterion |
|----|-------|----------------|
| **O1** | Map view loads @ Everon | No blank map / wasm panic |
| **O2** | Fences visible @ zoom ≥3 | T-152.4 |
| **O3** | Pier thin strip @ harbor | Not fat square |
| **O4** | Bridge deck + rail | T-152.4 |
| **O5** | NW airfield apron + runway | T-152.5 |
| **O6** | Hangar/tower icons @ airfield | T-152.5 |
| **O7** | Height labels on ridges | T-152.7; none in sea |
| **O8** | Town names @ island zoom | Gorey, Morton readable |
| **O9** | Major highway name on curve | T-152.9 |
| **O10** | Layer toggles | Each pref off works |
| **O11** | Pan/zoom perf | ≥55 fps @ default zoom |
| **O12** | Switch Satellite ↔ Map | No crash; state sane |

---

## Mathematical acceptance matrix (master)

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | **`∀ i ∈ {0..9}: t152_i_verify_log.md` exists and ends with **SHIP PASS** | Logs |
| **G2** | **`verify-t152-cartographic.mjs` exit 0** | Aggregator |
| **G3** | **`cd apps/website/frontend && npm test && npm run build && npm run lint` exit 0** | FE |
| **G4** | **`make wasm` exit 0** | Wasm |
| **G5** | **`cargo test -p map-engine-core --all-features` exit 0** | Rust |
| **G6** | **`make map-export-validate` exit 0** | Data |
| **G7** | **`make schema-validate` exit 0** | Schema |
| **G8** | Operator **O1–O12** signed **PASS** in `t152_10_verify_log.md` | Human |
| **G9** | **`t152_merge_readiness.md` complete** (CI, LFS, promotion) | Process |
| **G10** | **Zero FAIL rows** in master gate table | Closure |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
git lfs pull && make map-assets-link

# Master aggregator (implement in this slice)
node scripts/map-assets/verify-t152-cartographic.mjs

# Full CI replay (timeboxed; skip db if unavailable)
make schema-validate
make map-export-validate
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
cargo build --workspace
make wasm
cd apps/website/frontend && npm test && npm run build && npm run lint

# Verify log tail check
for f in .ai/artifacts/t152_{0,1,2,3,4,5,6,7,8,9}_verify_log.md; do
  test -f "$f" || { echo "G1 FAIL missing $f"; exit 1; }
done
echo "G1 logs present"
```

---

## Manual acceptance

Human operator runs **`make web`**, dev-login, open Mission Creator Everon, **Map** basemap — complete **O1–O12** in verify log (Grok drafts log; human signs).

---

## Documentation sync (Cursor, after merge)

- Registry: **T-152 → shipped**; all child slices shipped.
- `CLAUDE.md` §Status **T-152** bullet + cartographic symbology summary.
- `docs/TICKET_LEAD.md` sync via `./scripts/ticket sync`.
- Link `t152_10_verify_log.md` + `t152_merge_readiness.md` from program hub (`.0`).

---

## Grok Code prompt — T-152.10 (copy-paste)

```
Read CLAUDE.md first. CWD: /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.10** — E2E cartographic gate (assist operator; do not fake sign-off).

═══ PREFLIGHT ═══
  Confirm t152_0 .. t152_9 verify logs exist
  git lfs pull && make map-assets-link

═══ READ ═══
  1. docs/specs/Mission_Creator_Architecture/t152_10_e2e_cartographic_gate.md
  2. All .ai/artifacts/t152_*_verify_log.md
  3. docs/specs/Mission_Creator_Architecture/t090_1_1_map_cartographic_view.md
  4. docs/platform/CODING_STANDARDS.md §11 (ci-local)

═══ PROBLEM ═══
  Need program closure: aggregate gates, CI replay, operator checklist, merge readiness doc.
  No registry done until G-master + O1-O12 PASS.

═══ LANGUAGE GATE ═══
  This slice is mostly verification scripts + logs. Fix symbology bugs in prior slice files only
  if G1 shows FAIL — do not add new features.

═══ LOCKED ═══
  - verify-t152-cartographic.mjs
  - t152_10_verify_log.md master table
  - t152_merge_readiness.md
  - Operator O1-O12 human signed
  - Zero open FAIL

═══ DO ═══
  1. Implement aggregator script G2
  2. Run full verify block; record SHAs in master log
  3. Draft O1-O12 checklist with screenshots paths for operator
  4. Write t152_merge_readiness.md (CI, LFS, main promotion)
  5. Fix any FAIL from .4-.9 in code (no new scope)
  6. tag T-152.10 — STOP before ticket done; wait for human O-sign + Cursor registry

═══ DO NOT ═══
  - ./scripts/ticket done T-152 without human O1-O12
  - Edit registry.json / docs/TICKET_* / CLAUDE status (Cursor)
  - Mark PASS on operator rows yourself

═══ VERIFY ═══
  Full bash block from spec §Verify

═══ RETURN ═══
  - t152_10_verify_log.md (master table G1-G10 + O1-O12 draft)
  - t152_merge_readiness.md
  - verify-t152-cartographic.mjs
  - CI output paste
  - List of open FAIL if any (block ship)
  - Ready for operator sign-off + Cursor doc/registry sync.
```
