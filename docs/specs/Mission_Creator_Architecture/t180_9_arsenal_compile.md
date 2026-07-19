# T-180.9 — Arsenal wire + ORBAT compile truth

**Parent:** [`t180_orbat_eden_program.md`](t180_orbat_eden_program.md) · **Depends:** T-180.7 (inspector button), T-180.1+ (graph) · **Executor:** claude-code  
**Pins:** [`t180_class_r_pins.md`](t180_class_r_pins.md) — wins on numeric/package conflict.
**Verify log:** [`.ai/artifacts/t180_9_verify_log.md`](../../../.ai/artifacts/t180_9_verify_log.md) (create on ship)  
**Related:** T-068.10 Arsenal (editor loadout) · T-068.11 compiled mod loadout (separate path — do not conflate)

---

## Problem (measured)

1. **Event ORBAT loadout is always empty.**  
   [`orbat.rs:50-51,112-115`](../../../crates/map-engine-core/src/mission/orbat.rs): comment says `` `loadout` is always `""` ``; `derive_orbat_from_editor` hardcodes `loadout: String::new()`.  
   The local `Sl` deserializer (**lines 79–85**) has **no `loadout` field at all** — even if the editor payload carries `slot.loadout`, derive cannot see it.

2. **Slots do store real loadouts.**  
   [`store.rs` `update_slot_loadout`](../../../crates/map-engine-core/src/doc/store.rs) writes embedded JSON; golden shape includes `"summary":"M16A2 · ACOG"` ([`store.rs` ~1501–1508](../../../crates/map-engine-core/src/doc/store.rs)).  
   FE [`arsenal.rs` `picks_to_loadout`](../../../apps/website/frontend/src/arsenal.rs) builds `summary` from `primary` / `optic` / `magazine` / `launcher` joined with ` · `.

3. **Compile paths.**  
   - Save Version: [`compile.rs:7-8,103-109`](../../../crates/map-engine-core/src/mission/compile.rs) — `include_orbat=false`; server re-derives via `parse_orbat_template` → `derive_orbat_from_editor`.  
   - Export: `include_orbat=true` injects derived `orbat[]`.  
   - Events: [`apps/website/api/src/handlers/events.rs`](../../../apps/website/api/src/handlers/events.rs) `orbat_template_for_mission` → `parse_orbat_template` → materializes lobby slots from `OrbatSlotTemplate.{role,loadout,tag}`.

4. **Open Arsenal.** Stitch inspector has **OPEN ARSENAL**. Live Arsenal is [`ArsenalTab`](../../../apps/website/frontend/src/arsenal.rs) inside Attributes. ORBAT Manager must open that same loadout editor for the selected slot id (not a second Arsenal).

5. **Slot line vs `orbat[].loadout`.**  
   - UI slot line (T-180.7): `N: Role (Primary + Launcher?) | TAG?` + SL badge.  
   - Compile `OrbatSlotTemplate.loadout` string: **human summary** preferred for Event Hub display (match `summary` field or reconstruct primary+launcher). Full JSON stays on `editor.slots[].loadout` for Save/reload/T-068.11.

---

## Locked

| ID | Decision |
|----|----------|
| I-L1 | `OrbatSlotTemplate.loadout` = **non-empty summary string** when slot has embedded loadout with weapons; `""` only when no loadout / no primary |
| I-L2 | Prefer `loadout.summary` if present; else build `Primary + Launcher` from `primary`/`launcher` resource display names or resource basename |
| I-L3 | Extend `Sl` to deserialize `loadout` (`Value` or struct with `summary`/`primary`/`launcher`) |
| I-L4 | Flip [`orbat.rs:174`](../../../crates/map-engine-core/src/mission/orbat.rs) `assert!(…loadout.is_empty())` — that assertion becomes **illegal** |
| I-L5 | **`open_arsenal(id)`** = `attrs_open=Some(id)` **and** Attributes tab index **3** (`TABS[3]=="Arsenal"` at [`attributes.rs:16,43`](../../../apps/website/frontend/src/attributes.rs)). Today `open_attributes` alone leaves `tab` default **1** (Identity) — that is **not** enough |
| I-L6 | **No Standardization UI** (operator deferred L8) — Stitch shows it; we omit |
| I-L7 | Do **not** implement T-068.11 mod compiled gear block here unless already shipped — this slice is Event/`orbat[]` derive + Open Arsenal |
| I-L8 | Slot `callsign`/`rank` (T-180.1): surface in inspector already in .7; if Event DTO has no fields, keep on editor graph only — do not invent backend migration without measuring `OrbatSlot` |

---

## Consumers (must not break)

| Consumer | Path | Expectation after ship |
|----------|------|------------------------|
| Event attach materialize | `events.rs` + `parse_orbat_template` | Slot rows get meaningful `loadout` text for lobby list |
| Export JSON | `compile_payload(…, true)` | Top-level `orbat[].slots[].loadout` filled |
| Save Version | `include_orbat=false` | Still omits top-level orbat; editor.slots keep full loadout object |
| FE Event Hub | `dto.rs` `OrbatSlot.loadout: Option<String>` | Displays summary when present |
| ORBAT Manager line | formatter from .7 | Updates when Arsenal saves (same doc) |

---

## File map (write here)

| File | Change |
|------|--------|
| `crates/map-engine-core/src/mission/orbat.rs` | Deserialize loadout; fill summary; new helpers `loadout_summary_from_value`; tests I1–I5; delete empty-loadout assertion |
| `crates/map-engine-core/src/mission/compile.rs` | Export golden/smoke if it asserts empty loadout — update |
| `apps/website/api` event IT | If any test expects empty loadout on derive — update |
| `apps/website/frontend/src/editor_ops.rs` | Add `open_arsenal(id)` |
| `apps/website/frontend/src/attributes.rs` | Lift `tab` to OpsCtx **or** accept initial-tab param so Arsenal can be selected (today tab is local `RwSignal::new(1)`) |
| `apps/website/frontend/src/orbat_manager.rs` | OPEN ARSENAL → `open_arsenal` |
| `apps/website/frontend/src/arsenal.rs` | No redesign |
| `crates/map-engine-core/src/mission/compile.rs:248-254` | Update export fixture expectations when loadouts filled |

---

## Algorithm (derive fill)

```text
for each slot row in derive:
  lo = slot.loadout  # JSON object or null
  if lo is null/absent → loadout_str = ""
  else if lo.summary is non-empty string → loadout_str = summary
  else → loadout_str = join_nonempty([display(primary), display(launcher)], " + ")
  # display: prefer registry name if available in pure core → use basename strip .et / last path segment
  emit OrbatSlotTemplate { role, loadout: loadout_str, tag }
```

**False-green ban:** leaving `loadout: String::new()` and only “filling” in the Leptos tree. Event attach never runs Leptos.

---

## Class-R gates (must fail if stubbed)

| ID | Assert | Test / command |
|----|--------|----------------|
| **I1** | Payload with `editor.slots[].loadout.summary = "M16A2 · ACOG"` ⇒ derived `slots[0].loadout == "M16A2 · ACOG"` | `derive_fills_loadout_from_summary` |
| **I2** | Payload with primary+launcher, no summary ⇒ loadout contains both tokens (order primary then launcher) | `derive_fills_loadout_from_weapons` |
| **I3** | No loadout key ⇒ `loadout == ""` | `derive_empty_loadout_when_absent` |
| **I4** | Index sort unchanged vs current `derives_from_editor_sorted_by_index` | keep + extend |
| **I5** | `rg 'loadout: String::new\\(\\)' crates/map-engine-core/src/mission/orbat.rs` → **0** in derive map arm | shell |
| **I6** | `compile_payload(…, true)` golden includes non-empty loadout for fixture with summary | `compile_export_orbat_loadout` or update existing compile test |
| **I7** | `open_arsenal(id)` sets `attrs_open==Some(id)` **and** tab==3; test fails if only Identity tab | `cargo test -p website-frontend open_arsenal_selects_arsenal_tab` (or ops unit with tab signal lifted to OpsCtx) |
| **I8** | `rg -i 'standardization\|IFAK\|Grenade Complement' orbat_manager/eden OrbatManager` → 0 | shell |
| **I9** | Old test body asserting `all(\|s\| s.loadout.is_empty())` **removed or inverted** | `rg 'loadout.is_empty' orbat.rs` must not assert all empty on loaded fixture |

---

## Verify / Rebuild

```bash
cargo test -p map-engine-core derive_fills_loadout_from_summary
cargo test -p map-engine-core derive_fills_loadout_from_weapons
cargo test -p map-engine-core derive_empty_loadout_when_absent
cargo test -p map-engine-core derives_from_editor_sorted_by_index
# I5 / I9:
rg -n 'loadout: String::new\(\)' crates/map-engine-core/src/mission/orbat.rs && exit 1 || true
rg -n 'loadout\.is_empty' crates/map-engine-core/src/mission/orbat.rs
cargo test -p map-engine-core --lib
make test-it
make ci-local-leptos
```

## Manual

| ID | Check |
|----|-------|
| M-I1 | Edit Arsenal on a slot → Save Version → Event attach / GET orbat shows loadout text |
| M-I2 | ORBAT Manager → OPEN ARSENAL → same picks as Attributes Arsenal for that slot |
| M-I3 | Export JSON `orbat[].slots[].loadout` non-empty for geared slots |

## Acceptance

- [ ] I1–I9 PASS with outputs in verify log  
- [ ] Tag **T-180.9** · program complete note  
- [ ] Cursor doc sync after ship  

---

## Claude Code prompt — T-180.9 (copy-paste)

Authority: this spec + hub + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first.

Implement **T-180.9** — Open Arsenal + orbat[] compile/derive loadout truth.

═══ PREFLIGHT ═══
  git status
  ./scripts/ticket brief T-180

═══ READ (in order) ═══
  1. .ai/artifacts/t180_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t180_orbat_eden_program.md
  3. docs/specs/Mission_Creator_Architecture/t180_9_arsenal_compile.md
  4. crates/map-engine-core/src/mission/orbat.rs (Sl has NO loadout field today — ~79-85)
  5. crates/map-engine-core/src/mission/compile.rs (include_orbat export path)
  6. crates/map-engine-core/src/doc/store.rs update_slot_loadout + summary golden ~1501
  7. apps/website/frontend/src/arsenal.rs picks_to_loadout / ArsenalTab
  8. apps/website/api/src/handlers/events.rs orbat_template_for_mission

═══ PROBLEM ═══
  derive_orbat always emits loadout "". Sl deserializer ignores slot.loadout.
  Event attach and Export therefore show empty kits. ORBAT Manager needs OPEN ARSENAL
  wired to the existing ArsenalTab for the selected slot. Standardization stays out.

═══ SHIPPED (do not reopen) ═══
  T-180.1–.8 (graph, map, dock, Stitch UI, templates/vehicles)
  T-068.10 Arsenal editor loadout on slots
  T-180.0 docs

═══ LANGUAGE GATE ═══
  derive/summary fill MUST be in map-engine-core (Rust). Events call parse_orbat_template
  with no WASM/UI. Leptos only opens Arsenal and displays derived strings.

═══ LOCKED ═══
  - loadout string = summary (or primary+launcher rebuild)
  - Flip empty-loadout test; String::new() hardcode gone from derive map
  - Open Arsenal = existing ArsenalTab path
  - No Standardization UI
  - Not T-068.11 mod compile block unless already done — focus Event/Export orbat[]

═══ DO ═══
  1. Extend Sl + fill algorithm; tests I1–I4, I6
  2. Kill String::new() hardcode (I5) and empty-all assertion (I9)
  3. Wire OPEN ARSENAL (I7)
  4. Confirm I8 no standardization
  5. .ai/artifacts/t180_9_verify_log.md with outputs
  6. Tag T-180.9 · commit prefix T-180.9:

═══ DO NOT ═══
  - Edit docs/registry/CLAUDE markers
  - Leave fill only in FE formatters
  - Keep assert all loadouts empty
  - Build Standardization dropdowns
  - Defer derive fill

═══ VERIFY ═══
  cargo test -p map-engine-core derive_fills_loadout_from_summary
  cargo test -p map-engine-core derive_fills_loadout_from_weapons
  cargo test -p map-engine-core derive_empty_loadout_when_absent
  cargo test -p map-engine-core --lib
  # I5: hardcode gone
  if rg -n 'loadout: String::new\(\)' crates/map-engine-core/src/mission/orbat.rs; then exit 1; fi
  cargo test -p website-frontend open_arsenal_selects_arsenal_tab
  make test-it
  make ci-local-leptos

═══ MANUAL ═══
  M-I1 Event/orbat shows kit text · M-I2 Open Arsenal same as Attributes · M-I3 Export filled

═══ RETURN ═══
  - Commit SHA + tag T-180.9
  - .ai/artifacts/t180_9_verify_log.md
  - Automated PASS
  - Ready for Cursor doc sync · T-180 program complete
```
