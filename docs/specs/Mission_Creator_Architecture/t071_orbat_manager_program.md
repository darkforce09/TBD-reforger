# T-071 — ORBAT Manager (web program)

**Status:** **SHIPPED / SUPERSEDED** — **T-071.0** via **T-177** @ `e97a01c6`; remaining authoring (**T-071.1+**) completed by **[T-180](t180_orbat_eden_program.md)** (.1–.9 @ `cba837b3`).  
**Ticket:** T-071 · **Route:** `/missions/:id/edit` · **Registry:** [`.ai/tickets/registry.json`](../../../.ai/tickets/registry.json)

**Do not** implement thin T-071.1–.4 in parallel. Authority for ORBAT + Eden placement is **T-180**.

**T-071.0 (done):** left Outliner → Editor Layers only; top-strip **ORBAT Manager** browse/select shell. Spec/verify: [`t177_mc_chrome_orbat_cutover.md`](../../platform/t177_mc_chrome_orbat_cutover.md) · [`.ai/artifacts/t177_verify_log.md`](../../../.ai/artifacts/t177_verify_log.md).

**T-180 delivered (was T-071.1+ intent):** foundation graph · map tint + leader lines · Eden side chips · Stitch ORBAT Manager · Faction Library Apply/Save + vehicles · Open Arsenal + `derive_orbat` loadout. Coherency: [`t180_10_program_coherency.md`](t180_10_program_coherency.md) · `make verify-t180`.

---

## Residual (not T-071)

| Item | Owner |
|------|-------|
| Event lobby / admin ORBAT polish | **T-118** |
| Production mod LOBBY slot picker | **T-068.13** → **T-114** |
| Standardization (IFAK/grenades) | **Deferred** (T-180 L8 — operator) |
| Faction logos in Manager | Not ticketed under T-180 |

---

## Slice ladder (historical)

| Slice | Focus | Status |
|-------|-------|--------|
| **T-071.0** | Modal shell; left → Editor Layers only | ✅ via **T-177** |
| **T-071.1–.4** | Squad CRUD / order / logos / arsenal | **Superseded by T-180** |

---

## Related

| Ticket | Role |
|--------|------|
| **T-180** | ORBAT + Eden placement program (SoT) |
| **T-068.11–T-068.14** | Loadout Phase 2 + production slot picker + E2E |
| **T-118** | Event admin ORBAT + identity linking |
