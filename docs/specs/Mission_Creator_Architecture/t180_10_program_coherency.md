# T-180.10 — Program coherency checker

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Depends:** T-180.1–.9 shipped · **Executor:** cursor-docs  
**Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md)  
**Report:** [`.ai/artifacts/t180_10_coherency_report.md`](../../../.ai/artifacts/t180_10_coherency_report.md)  
**Verify log:** [`.ai/artifacts/t180_10_verify_log.md`](../../../.ai/artifacts/t180_10_verify_log.md)  
**Gate:** `cargo xtask verify t180` → [`scripts/verify-t180-coherency.sh`](../../../scripts/verify-t180-coherency.sh)

---

## Problem

T-180.1–.9 shipped Class-R features, but hub/pins/ROADMAP/queue still describe a pre-ship world (gaps open, T-071 “ready”, T-074/T-147 queued). Risk: agents re-implement absorbed work or treat locks as unfinished. Need a thorough coherency pass + a **repeatable** gate.

## Locked

| ID | Decision |
|----|----------|
| J-L1 | Cursor owns audit report + docs/registry + verify script |
| J-L2 | Code P0 lock breaks → file **T-180.10.1** (`claude-code`); do not silently fold |
| J-L3 | Operator manuals M-* remain residual (not a .10 FAIL) |
| J-L4 | L8 Standardization stays deferred (operator-authorized) |
| J-L5 | `cargo xtask verify t180` must exit 0 on green main |
| J-L6 | Absorbed tickets T-071 / T-074 / T-147 must leave ready/queued Next lists |

## Deliverables

1. Living coherency report (L1–L10 + A–I matrix, GPU bind closed, P0/P1/P2 findings)
2. `scripts/verify-t180-coherency.sh` + `cargo xtask verify t180`
3. Doc/registry remediations (hub post-ship SoT, pins post-ship section, ROADMAP, agent_execution, t071 hub, absorb statuses)
4. Verify log + tag **T-180.10**

## Class-R gates

| ID | Assert |
|----|--------|
| **J1** | Report covers L1–L10 + A–I with evidence; GPU bind not UNKNOWN |
| **J2** | `cargo xtask verify t180` exits 0 |
| **J3** | Hub gap table / pins post-ship SoT match live code |
| **J4** | T-071 / T-074 / T-147 not ready/queued as next work after sync |
| **J5** | ROADMAP + agent_execution + t071 hub acknowledge T-180 shipped |
| **J6** | Zero P0 code findings **or** T-180.10.1 filed |
| **J7** | `t180_10_verify_log.md` + tag T-180.10 |

## Verify

```bash
cargo xtask verify t180
./scripts/ticket check
```

## Out of scope (authorized residual)

- Operator manuals M-C1…M-I3
- L8 Standardization UI
- T-068.11 mod compile gear block
- New ORBAT features
