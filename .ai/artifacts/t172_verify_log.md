# T-172 verify log — Leptos SPA + Mission Creator bug bash

Date 2026-07-18 · executor Claude Code · branch `main` · spec `docs/platform/t172_leptos_bug_bash.md`.

**Scope decision (operator, this session):** B10 Arsenal = **"Full screen-04 layout"**
(AskUserQuestion answer) — icon rail + filtered item list + 3D doll + compat panel + badges +
Download JSON, not just a doll swap.

## Gates

| Gate | Result |
|------|--------|
| `make leptos-gates` | **exit 0** — 18/18 editor smokes PASS + v-suite **25/25 routes byte-equal to the frozen React oracle, zero accepts** (`scratch gates2.log`) |
| `make ci-local` | **exit 0** — editorconfig, no-python, no-node, rust-ci (fmt/clippy/build/wasm/test-it), coding standards, ci-local-leptos (fmt + clippy wasm32 + cargo test + trunk release), schema + citations |
| Native unit tests | website-frontend **69** + map-engine-core (incl. new: `is_active`/`classify_frame`, `resolve_wiki_selection`, `search_matches`, `filter_catalog_rules`, `flatten_visible_collapse_hides_subtree`, `sample_grid_meters` ×2, `retint_fill_alpha`, `slot_atlas` ×2, `time_scrubber_roundtrip`, `mission_size` ×3) |

Deliberate smoke edits (all green): `arsenal` (Forge layout + 3D doll proof), `outliner-palette`
(expands `US_Army` first — palette now honors `default_expanded`), `doc` (asserts
`__wgpuSlotStats` `atlas_ready` + `slot_len == seed`), `hydrate` (pins `?force=webgl` on its
self-built URL — with the slot atlas live, the first hydrated bind allocates a GPU buffer and
headless software WebGPU rejects any createBuffer; the suite's known wedge).

## Behavioral sweep (headless, release dist)

Driven via `gate render-check` upgraded for this pass (`--seed-auth` = the v-suite admin
localStorage seed; awaited asserts; value echo). Log: `scratchpad/probes*.log`.

| Probe | Proves | Result |
|-------|--------|--------|
| A2-nav-active | click sidebar Mission Library → exactly one `nav-item-active`, on `/missions`, breadcrumb "Mission Library" | PASS |
| A8-breadcrumb | `/vehicles` breadcrumb correct → SPA-nav to `/wiki` → breadcrumb follows | PASS |
| A2b-roundtrip | `/vehicles` → `/wiki` → `/` — active highlight lands on Dashboard only | PASS |
| A1-user-menu | avatar click → dropdown with `/settings#arma-link` + Sign Out | PASS |
| A9-drawer | hamburger click → drawer aside + scrim appear; scrim click closes | PASS |
| A4-wiki-select | click "Decision-Making Under Pressure" row → h1 changes AND URL becomes `/wiki/lead-decisions` (router-param selection live) | PASS |
| A5-vehicle-select | click "M1A1 Abrams" row → dossier h2 swaps | PASS |
| A6-modpack-select | click 2nd pack row → dossier h2 matches the clicked pack | PASS |
| H7-wiki-edit | `[ EDIT ]` click → Markdown textarea mounts | PASS |
| H8-launch-toast | Launch Game click → "requires the Reforger client" toast | PASS |
| A3-css | computed styles: body has NO fixed background-attachment; `body::before` is `position:fixed` carrying the gradient backdrop | PASS |

H11 note: a direct probe of `/wiki/comms-dynamic` shows the AuthGate sign-in — an artifact of the
probe harness (no API behind it, so the seeded session's `/me` fails and T-126 clears it), not an
app bug. The slug→article path is proven three ways: the A4 probe lands on `/wiki/lead-decisions`
with the right article via the same route-param resolution, `resolve_wiki_selection` is
unit-tested, and the frozen `wikislug` v-suite golden (deep-link render) matches 25/25.

## Matrix — operator-seeded rows

| Row | Fix | Evidence |
|-----|-----|----------|
| A1 user menu dead | dropdown (Settings / Link Arma Identity / Sign Out w/ token revoke + persisted clear) | probe A1 · commit `7ab2cf3a` |
| A2 nav highlight stuck | reactive SidebarNav closure over pathname | probes A2/A2b · v-suite 25/25 (byte-equal initial DOM) |
| A3 scroll lag | `body::before` fixed layer replaces `background-attachment: fixed` (composited scrolling restored) | probe A3-css · commit `8a2fd47e` |
| A4 wiki dead selection | route-param selection + navigate-on-click + all 5 manual bodies restored from React | probe A4 · unit `resolve_wiki_selection` |
| A5 vehicles dead selection | signal selection + reactive dossier | probe A5 |
| A6 modpacks dead selection | signal selection + reactive dossier | probe A6 |
| A7 dossier open UX | Sheet slide-in + scrim fade animations; mc-load-bar Suspense fallback | commit `d37da813` (animation classes render only in open state — v-suite untouched) |
| A8 breadcrumb stale | reactive breadcrumb + FrameKind Memo (no chrome remount between chrome routes) | probe A8 |
| A9 hamburger dead | slide-over drawer (scrim/Esc/nav-click close) | probe A9 |
| B1 editor lag | boot: manifest dedupe + concurrent sat/DEM + forest re-upload memo + Rc grid; interact: B8 selection path | commits `e9471e8d` `20a21665` |
| B2 CUR no Z | DEM-fed Z (retained 1600² grid bilinear sample) + Z cell + CUR↔SEL swap | `cur` smoke green (X/Y untouched) · sampler units |
| B3 opaque forest | zoom-band alpha re-tint at upload (0.45/0.35/0.12/0) | `fullmap` smoke `forest_polygons > 0` · `retint_fill_alpha` unit |
| B4 invisible slots | core `build_slot_atlas` + `ensure_slot_atlas` at mount → lane live | `doc` smoke `atlas_ready && slot_len==8`; gates show `atlas_bytes: 32768` |
| B5 drop-place invisible | same root cause; `after_doc_change` rebind draws | `outliner-palette` smoke place path green |
| B6 trees can't collapse | `flatten_visible` + chevrons; palette seeds from `default_expanded` | `outliner-palette` (expands US_Army) + `virtual-outliner` green · unit |
| B7 no folder icons/guides | `folder`/`folder_open` variants + border-l guide spans | in B6 render path (smokes traverse rows) |
| B8 laggy selection | `refresh_selection()` — no tree rebuild on click/marquee/outliner select | `select` smoke green · code: `editor_ops.rs` `mission_history.rs` |
| B9 chrome gaps | menu bar, editable title, time scrubber + weather, History, Save dialog (size est + bar), SZ cell, FACTIONS/VEHICLES/MARKERS tabs, Asset Browser search, OUTLINER header + icon strip, debug HUD | `keyboard-settings`/`doc`/`save-export` smokes green · commit `e3662e42` |
| B10 2D arsenal | intact T-154 DollEngine mounted (rotate/hover/pick/callout/set_states, SVG fallback) + full screen-04 Forge layout | `arsenal` smoke 18 checks green incl. `r7_dollBackend`/`r7_dollAnchorPick`/`r7_dollCallout` |

## Found-by-hunt (all fixed here)

| ID | Find | Fix / evidence |
|----|------|----------------|
| H1 | SidebarNav sampled auth role once — nav never reacted to login | reactive closure (with A2); v-suite still byte-equal |
| H2 | forest mesh re-uploaded identically every drain pass (12× boot, 6×/settle) | pushed-state memo; `fullmap` green |
| H3 | 10 MB DEM grid deep-cloned every camera settle | `Rc<DemVectorGrid>` |
| H4 | manifest.json fetched twice per bootstrap | single fetch feeds DEM + satellite (now concurrent) |
| H5 | `engine.on_camera_changed()` never called — slot px sizing + cluster gate stale | wired at wheel/pan/set_view/center; `pan`+`select` smokes green |
| H6 | editor dialogs popped in (no enter animation) | dialog-in/overlay-fade on Attributes/Settings/FactionManager |
| H7 | wiki READ/EDIT dead | real session-local Markdown editor (React parity); probe H7 |
| H8 | modpacks Launch dead + editor dead | Launch toast (React parity) + full ModpackEditor port (rename/add/remove/[REQUIRED]/Save w/ toast; list+search honor overrides); probe H8 |
| H9 | login "Sign in with Discord" no handler | full-page redirect to `GET /api/v1/auth/discord/login` |
| H10 | Arsenal missing Download loadout JSON + COMPAT/VALID badges (page-doc-pinned) | restored in the Forge rebuild; arsenal smoke asserts |
| H11 | `/wiki/:slug` param ignored | selection derives from the route; probe A4 + unit + wikislug golden |

Hunt count: **operator-seeded 19** (A1–A9, B1–B10) · **found-by-hunt 11** (H1–H11) — all fixed,
zero deferrals.

## Perf before/after notes

- Site scroll: `background-attachment: fixed` removed from `body` — scrolling is composited again
  (the backdrop no longer repaints + re-blurs ~20 backdrop-filter surfaces per frame). CSS-level
  proof: probe A3-css.
- Editor selection: click/marquee/outliner select no longer rebuilds both dock trees (O(n)
  flatten per click) — only `sel_count` + the `selected_ids` highlight mirror update.
- Editor boot: one manifest fetch (was 2), satellite download overlaps DEM decode + hillshade,
  forest composite uploads once per state instead of 18× around boot, camera settles stopped
  deep-cloning the 10 MB grid.

## Commits (this ticket, `main`)

`1782e4cc` inventory · `8a2fd47e` A3 · `efbd4019` A2/A8/H1 · `7ab2cf3a` A1 · `7ae48bee` A9 ·
`e9e4c293` A4/A5/A6 · `d37da813` A7/H6 · `8918a1ab` H7/H8/H9 · `e9471e8d` B8/B1 ·
`3e5da91d` B2 · `20a21665` B3/H2 · `aea722f2` B4/B5/H5 · `3573fe7d` B6/B7 · `e3662e42` B9 ·
`66b64590` B10/H10 · hydrate-smoke pin · render-check probe infra.

## Residual

None in the T-172 matrix. T-069 markers / T-070 vehicles remain their own tickets (their chrome
tabs exist as stubs, matching the React reference). T-170 prod flip stays `human`.
