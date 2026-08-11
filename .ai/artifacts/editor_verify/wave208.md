# Wave 208 adversarial verification — T-801 / T-805

Verifier: Cursor Grok 4.5, 2026-08-11. Verified MERGED MAIN at **f8353eab** (`git rev-parse HEAD` = `f8353eabe886986cd08eecf95734bb763095d587`).

| Pin | Sha / note |
|---|---|
| Wave base (last close) | `bc627304` — wave 130 CLOSED — editor wave 207 |
| Merge T-801 | `bd5ca071` (slice `eb507df2`) |
| Merge T-805 / HEAD at dispatch | `f8353eab` (slices `9fefd136`, `a6f956b7`) |
| HEAD re-check at start + exit | `f8353eab` — **nothing landed after dispatch** |
| Wave gate | PASS (cargo/wasm/fmt/clippy/tests/trunk/schema/no-python/no-node/no-shell). Per §5: `wave.sh` runs **zero** editor smokes — GATE PASS ≠ editor-suite green |

**Environment left as found:** no repo files mutated except this report; no commits; no tickets filed. Probe chromium profiles under `/tmp/w208-verify/` (ephemeral). API already bound `:8080` (pre-existing `api` pid); trunk on `:3000` serving a post-merge dist whose wasm contains `pack_squad_link_drag_preview`, `bind_squad_link_preview`, `install_route_auth_guard`, `role_notice`, and `Mission Maker role required`. `git status` at exit: clean working tree aside from this untracked report path under `.ai/artifacts/`.

**Surface:** headless Chromium 1228 (`--headless=new`, SwiftShader), fresh profile per probe, CDP `Input.dispatchMouseEvent` + `Runtime.evaluate`. Class-R via `cargo xtask ai run`.

---

## FINDINGS

### F1 — `?role_notice=mission_maker` is written but never read
`MINOR | apps/website/frontend/src/router.rs:303-312 + apps/website/frontend/src/auth.rs:349-352 | denial redirect appends \`role_notice\` and a comment claims it "remains for deep links", but no SPA consumer reads the query; only the navigate-time toast fires | proven by whole-tree \`role_notice\` grep + live CDP`

- Evidence: `rg role_notice` under `apps/website/frontend/src` hits **only** `router.rs` / `auth.rs` (writers + unit asserts). No `mission_overview` / `missions` / layout effect parses the param. Live CDP: enlisted/leader/guest deep-link to `/missions/smoke/edit` → pathname `/missions/smoke`, search `?role_notice=mission_maker`, toast text `Mission Maker role required to open the editor.` present in `[role=status]`. A cold open of `/missions/smoke?role_notice=mission_maker` without going through the guard would show **no** toast (param inert).
- Impact: Primary UX (redirect + toast on denial) works. Deep-link / refresh of the overview URL alone does not re-show the notice. Claim "with `role_notice=`" is half-true.
- Disposition: **MINOR** — do not block the wave; optional follow-up to either consume the query or drop the dead-param comment. Not filed here (verifier does not file tickets).

### F2 — T-801 host wiring has no Class-R pin beside the packer tests
`NIT | apps/website/frontend/src/mission_editor.rs:7574-7598 (T-573/T-808 push_drag_preview pins) | pins require \`set_drag\` + \`bind_vehicle_preview_lane\` but not \`bind_squad_link_preview\` / \`pack_squad_link_drag_preview\` | source audit`

- Evidence: `push_drag_preview` **does** call `bind_squad_link_preview` (`select_tool.rs:299`). The existing Class-R battery that would catch a dead vehicle preview does **not** assert the new tether call, so a future edit that drops only the squad-link line stays green on that pin while breaking T-801.
- Impact: Regression risk for host wiring only; packer math is Class-R covered in `squad_links.rs` (4 dedicated tests, all green in the 10 `squad_link*` filter).
- Disposition: **NIT** — documentation of pin gap; not a shipped functional defect.

No BLOCKER. No MAJOR.

---

## Safe-line

**Yes — `main` at `f8353eab` is safe to build the next wave on.**

T-805 role vectors are live-proven. T-801 preview math and call-site wiring check out; commit path still uses authored `build_squad_link_segments`. Wave-207 drag-preview symbology binder remains on the push/clear path. Residual gaps (live mid-drag 4px pixel proximity; dead `role_notice` consumer; missing host Class-R needle) are not wave-stopping.

---

## VERIFIED-CLEAN REGISTER

### T-801 — Squad tether lines follow the drag preview

| Claim | Result | Evidence |
|---|---|---|
| Preview pack offsets dragged endpoint by `(dx,dy)` | **PASS** | `squad_link_drag_preview_offsets_single_dragged_endpoint` — verts `[10,20]→[37.5,36.75]` for drag `a` by `(7.5,-3.25)` |
| Multi-select moves **both** ends | **PASS** | `…_offsets_both_ends_when_multi_selected` — leader `(17.5,16.75)`, member `(37.5,36.75)` |
| Only affected squads re-resolve; idle squad byte-identical | **PASS** | `…_repacks_only_affected_squads` — `preview[12..] == authored[12..]` |
| Empty drag / zero delta = identity restore | **PASS** | `…_identity_on_clear` |
| Host wires preview into `push_drag_preview` / `clear_drag_preview` | **PASS** | `select_tool.rs:299,317` → `bind_squad_link_preview` → `pack_squad_link_drag_preview` + `upload_hairline_segments(role_id::SQUAD_LINKS, …)` |
| Commit path unchanged (authored rebuild) | **PASS** | `mission_history.rs` still `build_squad_link_segments` only; preview pack imported only from `select_tool.rs` |
| W207 vehicle symbology mid-drag not broken by T-801 | **PASS (source)** | `push_drag_preview` still `bind_vehicle_preview_lane` → `vehicles_bind_symbology`; T-808 Class-R needles still name that binder |
| Mid-drag ≤4px endpoint proximity (agent admitted UNTESTED) | **NOT RE-PROVED live** | CDP sessions repeatedly hit `Loading world objects…` overlay / `__editorCamSet` NaN; screenshots are load chrome, not tether pixels. Class-R substitutes for math; **pixel proximity remains an open measurement gap**, not a refuted claim |
| 9-slot `rf≤1ms` median (agent admitted UNTESTED) | **NOT RE-PROVED** | Debug HUD `rf` not readable in probe sessions (`rf_idle`/`rf_mid` null). No perf finding either way |
| MissionConnections hairlines still doc_tick-only | **CONFIRMED outside owns** | Agent honesty stands; not a silent deferral of T-801 scope |
| `slot_line.rs` "owns" | **Comment-only redirect** to `squad_links` — intentional; no hollow behavior claim |

`cargo xtask ai run -- 'cargo test -p map-engine-core squad_link'`: **10 passed** (includes prior segment pins + 4 T-801 preview pins).

### T-805 — Editor route enforces declared tier

| Vector | Result | Evidence |
|---|---|---|
| Editor declares `mission_maker` | **PASS** | Class-R `editor_route_declares_mission_maker` + live ROUTES |
| Enlisted deep-link `/missions/smoke/edit` | **PASS** | CDP → `/missions/smoke?role_notice=mission_maker` + toast |
| Leader (below maker) | **PASS** | Same redirect + toast |
| Guest (no auth) | **PASS** | Same redirect + toast |
| Mission maker stays on edit | **PASS** | pathname `/missions/smoke/edit`, `canvas=true` |
| Admin stays on edit | **PASS** | same |
| Bootstrapping avoids false-deny | **PASS (logic + live)** | `route_auth_redirect(_, None, true) == None` (Class-R); maker with persisted refresh token reached editor canvas (bootstrap completed without bounce) |
| Admin routes not broken by T-805 | **PASS** | Fresh-token CDP: admin `/admin/events` → Operations Calendar (no AdminGate denial); enlisted + mission_maker → **"Admin access required."** (`AdminGate` unchanged; `auth_denial_redirect` returns `None` for admin paths by design) |
| Draft / IDB leak from investigation | **REFUTED as T-805 defect** | T-805 diffs touch **only** `auth.rs` / `nav.rs` / `router.rs`. `yrs_persist.rs` not in the commit. 8-slot seed HUD on 404 stay-local is pre-existing `yrs_persist` / editor seed behavior, not an IDB overwrite introduced here |
| `yrs_persist` read-only for this ticket | **PASS** | `git show --name-only` on `9fefd136` / `a6f956b7` / merge |

`cargo xtask ai run -- 'cargo test -p website-frontend route_auth'`: **6 passed** earlier in session.

---

## Attacked and FAILED to break

1. **T-801 preview math** — single-end, multi-end, unaffected-squad isolation, identity clear (Class-R all green).
2. **T-801 commit/authored path** — still `build_squad_link_segments` on doc tick; preview-only pack not on commit.
3. **T-801 vs W207 symbology register** — vehicle preview still goes through `bind_vehicle_preview_lane` / `vehicles_bind_symbology` after the tether upload was added.
4. **T-805 enlisted / leader / guest editor entry** — all redirected with notice + toast; canvas never stays.
5. **T-805 maker / admin editor entry** — both stay on `/edit` with canvas.
6. **T-805 bootstrapping false-deny** — maker deep-link with refresh-on-disk reached the editor (no bounce-to-overview).
7. **T-805 AdminGate regression** — admin still sees Event Manager calendar; non-admins still see "Admin access required." (not a new redirect).
8. **T-805 silent yrs_persist / IDB draft wipe** — owns paths exclude `yrs_persist`; no evidence the ticket wrote drafts.
9. **HEAD drift after dispatch** — remained `f8353eab` for the whole verification.
10. **Gate lying about editor smokes** — not re-litigated as new; standing note from wave 207 still applies (`wave.sh` ≠ `editor-suite`), but this wave's **claimed** tickets are not "green only because the gate never looked."

---

## Environment left as found

- Repo: HEAD `f8353eab`, no verifier commits, no ticket registry edits, no app source edits.
- Ephemeral probe debris: `/tmp/w208-verify/**` (chrome profiles, screenshots, JSON) — not in the repo.
- Pre-existing `:8080` API and `:3000` trunk left running as found (not torn down by this verifier).
