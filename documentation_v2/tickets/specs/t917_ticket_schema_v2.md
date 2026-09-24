# T-917 — Ticket schema v2: provenance, scope, decomposition

Design contract, agreed with the operator 2026-08-14 after a live session on the
T-915 ticketboard. Three programs come out of this design: **Program S "Registry
schema v2"** (referred to as **T-917** throughout as a placeholder), **Program B
"Ticketboard v2"** (**T-918** placeholder), and **Program T "Wall triage"** (**T-919**
placeholder, bookkeeping). Real ids are whatever `ticket add` derives at mint time.

**Do not write the `STRICT_LEGACY` phrase matching `Track [ABC]\b` in this spec, ticket
prose, or any commit subject/body that `ticket sync` copies into `docs/TICKET_*.md`.**
Scanner: `xtask/src/constants.rs`.

## KISS summary

Every ticket gets: a hard ship gate (three timestamps + a token count — real or
math-estimate, always marked with its method), a 4-level scope breadcrumb
(domain/layer/component/surface, strict vocabulary), a bug/feature class, ten short
typed fields instead of one 500-word wall, and a required per-ticket plan document (no
plan = can't go ready). All 1182 existing files migrate in one big commit; history
gets estimated stamps and tokens mined from git, marked as estimates. The board learns
to show all of it, renders markdown plans in-app, and never mixes real numbers with
estimates.

## Why

The operator opened a real ticket on the board and saw the model fail: a ~500-word
summary wall carrying requirement/repro/acceptance in one blob, `scope` rendered as an
opaque `website/frontend/editor` shared by 722 tickets, empty user_story/acceptance,
no metrics anywhere, no bug-vs-feature signal, and shipped history missing its
timestamps. v1's own depth had already failed silently: of 722 editor-scoped tickets,
exactly **one** carries a nonempty `chrome`; 199 of 357 repo tickets sit in the
`docs` mint-default landfill. Data the system does not require is data the system
does not get.

## Measured facts (2026-08-14 — instruments named)

| Fact | Value | Instrument |
|---|---|---|
| Ticket files | 1182 | `ls .ai/tickets/T-*.toml \| wc -l` |
| Shipped | 998 | grep `status = "shipped"` |
| Shipped missing shipped_at / created_at / completed_at | 770 / 988 / 985 | grep -L per field over shipped set |
| shipped_at date-shaped strays (contract: bare SHA) | 4 | value-shape scan |
| Scope: website.editor / repo / mod / engine / schema | 722 / 357 / 69 / 3 / 2 | grep `[scope.*]` headers |
| editor tickets with nonempty chrome | 1 | field scan |
| repo tickets with layers=["docs"] (mint default) | 199 | field scan |
| editor tickets with nonempty owns (surface-inferable) | 513 | field scan |
| Summaries >100 words | 208 TOML-parsed vs 382 raw-regex | **method disagreement — every acceptance line below names its counter** |
| Git recovery | 252 `T-*` tags; 2120 T-prefixed subjects; 935 distinct ids | `git tag` / `git log --pretty=%s` |
| Run receipts | zero (`.ai/tickets/metrics/` absent) | ls |

## Decisions log (operator, 2026-08-14)

1. **Ship gate is HARD**: `shipped` requires `created_at` + `completed_at` +
   `shipped_at` + a token count, for **all previous and all future tickets**. No
   "unrecorded" steady state — the audit's softer three-state proposal was
   **overruled**: "±30% is still better than nothing… hard requirement… use maths."
   Estimates are the escape hatch, always marked with their derivation method.
2. **Full history backfill** with marked estimates (chosen over an era cutoff).
3. **Scope v2 = domain / layer / component / surface — four levels final.** A fifth
   level was considered and rejected on the 1-in-722 evidence; `owns[]` already
   carries file precision. Surface is REQUIRED on live/new tickets and
   mechanically owns-inferred during migration.
4. **`class` = bug | feature | chore | audit | docs**, required on work tickets.
5. **Body decomposition, not truncation — ten fields**: `summary`, `user_story`,
   `context[]`, `requirement[]`, `current_state[]`, `approach[]`, `verify[]`,
   `acceptance[]`, `citations[]`, `notes`. `verify[]` kept by operator word with
   anti-blend definitions (below). "We don't want it to blend together."
6. **Caps** (check-enforced, never parse-enforced — old git revisions must stay
   readable): summary ≤40 words; context/requirement/current_state/approach/verify
   ≤30 words per line; citations ≤8; acceptance/notes/user_story uncapped
   (grandfathering by *field choice*, never by ticket class).
7. **Wall triage**: mechanical byte-reversible quarantine first; then AI batches of
   ~20–30 tickets per operator-reviewed commit draining a shrink-only pin.
8. **Single format after migration; single gate authority** (`ticket check --strict`).
9. **Per-ticket plan document is a ready-gate**: new `plan` key (path to
   `docs/plans/T-XXX_plan.md`), standardized template, distinct from the shared
   program `spec`. `mark-ready` refuses when the plan file is missing on disk.
10. **In-app markdown viewer**: spec/plan/citation clicks render inside the board
    (egui_commonmark — a named supply-chain event, like eframe was).

## Scope v2

### Shape: flat keys, one `[scope]` table

```toml
[scope]
domain = "website"
layer = "frontend"
component = "mission_creator"
surface = ["attr_panel", "toolbelt"]
```

- `domain` stays a closed Rust enum (website | mod | schema | engine | repo — changes
  ~never; the `deny(clippy::wildcard_enum_match_arm)` authority keeps protecting its
  matches). `layer`/`component` are single validated strings; `surface` is an array
  (a coherent slice may touch several surfaces; a ticket spanning components is
  mis-sliced — that is what owns collisions and programs are for).
- Legality is resolved at `Corpus::load` and in `check` against the vocabulary tree.
  **Documented weakening**: `parse_ticket_toml` alone becomes shape-strict only — a
  bare parse cannot know per-parent legality; every real path goes through
  `Corpus::load`/`check`, which refuse naming ticket + offending pair.
- Rejected: nested tables (compile-time legality the corpus demonstrably never
  exercised, at n² Option-table emit cost) and path strings (multi-surface breaks,
  validation-by-splitting, the typed model becomes a lie).

### Vocabulary: data file, check-validated

`.ai/tickets/scope-vocab.toml` — sorted, duplicate-free, parent-exists enforced by a
check rule; **removing a value still used by any ticket is red**. Vocabulary is
*content* that grows weekly with the codebase; the ENCODING_C governance protects
*structure*. Compiled-enum friction is what produced the 199-ticket docs landfill.

Draft tree (finalized in S.1; grounded in real modules — every leaf traceable to a
source path):

- **website** / frontend / **mission_creator**: map_canvas, top_strip, toolbelt,
  dock_left, dock_right, attr_panel, outliner, asset_browser, env_settings, tools,
  doc_store, ops_undo, validation, layout_chrome · **site_pages**: events, missions,
  wiki, leaderboards, dashboard, orbat, personnel, arsenal, modpacks, servers,
  announcements, deployments, approvals, auth_pages · **world_render**: satellite,
  forest_mass, dem_vectors, labels, bridge, world_host · **shell**: router, nav,
  layout, client_transport, toast
- **website** / backend / http_api (surfaces = handler modules), auth, db, realtime,
  services, contract, middleware · website / shared · website / tests
- **mod** / scripts / ui, gamemode, backend, core, zones, markers, objectives, radio,
  registry, spectator · mod / assets / prefabs, configs, data, missions ·
  mod / workbench · mod / worlds
- **engine** / core, render, world · **schema** / mission, registry, contract
- **repo** / ci, docs, tickets, scripts, tools · repo / **xtask**: check, wave,
  tickets, gates, deploy, db, mcp, ci, metrics

Old→new mechanical maps: chrome left→dock_left, right→dock_right, map→map_canvas,
top→top_strip, bottom→toolbelt, attr→attr_panel; backend layers 1:1; ModLayer 1:1
into scripts/assets; `feature` and the docs landfill resolve via owns-inference.
Migration populates `surface` from `owns[]` where inferable (513/722 editor tickets);
uninferable scope is listed in `estimated[]` — the provenance machinery reused for a
non-numeric field.

## Body: ten fields, anti-blend definitions

| Field | Definition (one line each — the anti-blend contract) | Cap |
|---|---|---|
| `summary` | What this ticket is, one breath | ≤40 words |
| `user_story` | Who benefits and why (existing field) | — |
| `context[]` | Why now; background facts | ≤30 w/line |
| `requirement[]` | The operator's ask, line by line | ≤30 w/line |
| `current_state[]` | What exists today; bug repro lives here | ≤30 w/line |
| `approach[]` | Planned steps | ≤30 w/line |
| `verify[]` | Commands to run — how to prove | ≤30 w/line |
| `acceptance[]` | Outcome criteria — what must be true | — |
| `citations[]` | Files/tickets/docs consulted, reference-only | ≤8 w/entry |
| `notes` | Freeform leftover (existing field) | — |

Anti-blend rules in check: a `citations[]` entry duplicating an `owns[]` entry is red
(ownership facts must not split across fields); an `acceptance[]` line that is
command-shaped (starts `cargo `/`$ `/`./`) warns pointing at `verify[]`.

## Provenance and estimates (hard-require regime)

### New on-disk keys

`class`, `plan`, `estimated[]`, `estimate_note`, `context`, `requirement`,
`current_state`, `approach`, `verify`, `citations`, `migration_legacy` + the flat
scope keys — **one governance commit**: ALLOWED_NEW + `TicketFile` +
`.ai/tickets/schema.json` together (the `on_disk_keys_are_mapped_or_allowed_new`
test in `tickets_store.rs` holds the line).

### Estimation ladder — total, every shipped ticket ends with a number

| Fact | Method 1 | Method 2 (fallback) |
|---|---|---|
| tokens | `diff_loc`: LOC changed across the ticket's commits × documented factor | `cohort_median`: median of same class+scope cohort's measured/diff_loc values |
| created_at / completed_at | `git_subject`: first/last exact-id boundary-matched commit subject dates, UTC-normalized | `id_interpolation`: interpolated between nearest id-adjacent tickets with known dates |
| shipped_at | last subject commit SHA | `estimated[]`-marked absent → re-mined or interpolated-era SHA is NOT invented; where no SHA exists the stamp comes from method 2 dates and shipped_at carries the nearest real mined SHA or the field is listed in `estimated[]` with `estimate_note` naming the gap |

Every estimate records its method and inputs — recalibration is regeneration from
recorded inputs, never untraceable mutation. The token factor starts as a **declared
constant pending calibration** (zero receipts exist today); each estimate file carries
the factor it used.

### Token estimates live OUTSIDE metrics/

`.ai/tickets/estimates/<id>.json` with its own `estimates.schema.json` and its own
check function. **Never inside `.ai/tickets/metrics/`** — the receipt walkers
(`metrics::check_as_errors`, `load_all_runs`) are `deny_unknown_fields` over every
file and would go red; worse, `has_receipt` would let an estimate impersonate a
receipt — the exact T-913 violation. Fields: `id`, `source` (diff_loc |
cohort_median), per-source inputs (`loc_changed`, `derived_from_sha`, cohort spec),
`factor`, `tokens_estimated`, `generated_at`.

Mutual exclusion, check-enforced: a measured receipt appearing means the estimate
file is deleted in that same commit (estimate + receipt for one id = red);
estimate file ⇔ `"tokens" ∈ estimated[]`; estimate for a non-shipped ticket = red
(estimates are historical reconstruction, not forecasts). Dashboards never sum
across the boundary — structurally separate trees, plus a negative acceptance
assertion in Program B.

### The gate (check rule + `ops::ship` refusal)

`status = shipped` ⇒
- `created_at`, `completed_at` present, RFC 3339 UTC valid;
- `shipped_at` present and **SHA-shaped** (7–40 lowercase hex) — the 4 date-shaped
  strays are resolved in migration (date → `completed_at` marked estimated; SHA
  re-mined or marked);
- each stamp either measured or listed in `estimated[]`;
- tokens: **exactly one of** ≥1 receipt under `metrics/<id>/` XOR
  `estimates/<id>.json`.

`ticket check --strict` prints honesty counters: "shipped tokens measured/estimated:
K/E" plus per-source breakdown — drift is visible, never silent.

### `stamp-sha` closes the loop (lands WITH the gate)

`ticket stamp-sha <id> <sha>` writes `shipped_at` canonically AND auto-generates the
`diff_loc` token estimate when no receipt exists. Without this verb every future ship
wedges between the ship edit and the SHA hand-edit (the SHA does not exist until the
operator commits). Ship flow becomes: `ticket ship <id>` → commit → `ticket stamp-sha
<id> $(git rev-parse --short HEAD)` → gate green.

### Plan documents (ready-gate)

`plan = "docs/plans/T-XXX_plan.md"` — required for ready-class from S.6 forward.
Template (pinned here): four short sections — **Context / Approach / Risks /
Verification**. `plan` ≠ `spec`: spec remains the shared program authority; plan is
this ticket's own. `ops::mark_ready` + check refuse when the file is missing on
disk (extends the existing spec-on-disk gate). The 5 currently-ready tickets get
plans written at S.6 land (measured small).

## Wall quarantine (pass 1 — mechanical, byte-reversible)

For each work ticket whose TOML-parsed `summary` exceeds 40 whitespace-split words:
move the summary **verbatim** into `migration_legacy[]`, split on newlines only —
joining with `\n` reproduces the original byte-exactly, proved per-file by the
migrator. `summary` := the ticket's existing `title` (human-written, card-length —
zero invented prose). **No sentence-splitting into semantic fields**: a semantic
field filled by a non-semantic process launders unsorted prose as classified data,
and the walls contain quoted errors, path runs, and inline lists that splitting
mangles.

Ratchet: a shrink-only pinned count of tickets carrying `migration_legacy` (the
`HAND_EDITED_NOT_CANONICAL` self-tightening pattern in `store.rs`); a new ticket
minting the field is red (field-scoped rule, not a ticket class). Pass 2 (Program T)
drains it: AI decomposes each wall into the typed fields, deleting `migration_legacy`
in the same edit, batches of 20–30 per operator-reviewed commit.

## Migration mechanics

Sequence on main — every commit build-green and check-green:

1. **S.1** vocab file + its check rule (additive, harmless).
2. **S.2 THE cutover — one commit by compile-time physics**: `apps/ticketboard`,
   `phase2.rs`, `cmds.rs`, `ops.rs` all match `Scope` exhaustively, so the type
   change and every consumer fix must ride together. Contents: tbd-tickets types +
   encoding (flat scope, new keys), schema.json, governance widen, the migrator,
   all 1182 files rewritten, regenerated sync surface (docs/TICKET_*.md, queue.json —
   `sync.rs` copies summaries verbatim), consumer compile fixes
   (`board.rs::scope_compact`, `phase2.rs::infer_scope`/`override_scope` — the
   per-id override table retires into the migrator — `targets_from_scope`, mint
   sites in `cmds.rs`/`ops.rs`). Precedent: the T-911.2 encoding-C cutover.
   **The migrator parses v1 as `toml::Value`** (typed v2 refuses v1 by definition),
   transforms Value→Value via the committed mapping table, then validates every
   output through the new `parse_ticket_toml` + byte-stable re-render — the
   `migrate_live_tree` re-parse-gate precedent.
3. **S.3** wall quarantine (own commit — the diff is exactly the walls).
4. **S.4** stamp backfill (two-pass forced: `estimated[]` is illegal on disk until
   S.2 widens governance; the miner reads immutable `git log` metadata, indifferent
   to working-tree contents). Exact-id boundary matching (`T-90` must not match
   `T-902`); offsets normalized to UTC `Z` or `validate_rfc3339_utc` refuses.
5. **S.5** token estimates (factor doc + generator + estimates check).
6. **S.6** gate ON + `stamp-sha` + plan ready-gate.

**wave.lock byte-neutrality is an acceptance tripwire on S.2/S.3/S.4**: the lock's
per-ticket inputs are exactly `owns`/`depends_on`/`pack_last` + status/executor
membership — scope, summaries, and stamps are not lock inputs, so
`git diff --stat -- .ai/tickets/wave.lock` must print empty; a dirty lock means the
migrator perturbed something it must not.

## Programs and slices

Packing: S.2–S.4 own `.ai/tickets/**` — singleton waves by construction. B slices
own `apps/ticketboard` — serial among themselves, parallel to S where owns allow.

### Program S — Registry schema v2

| Slice | What | owns | depends_on |
|---|---|---|---|
| S.1 | scope-vocab.toml + vocab check rule | `.ai/tickets/scope-vocab.toml`, `xtask/src` | — |
| S.2 | THE cutover (types, migrator, 1182 rewrite, consumers, sync) | `crates/tbd-tickets`, `xtask/src`, `apps/ticketboard/src`, `.ai/tickets`, `docs` | S.1 |
| S.3 | Wall quarantine + caps + ratchet | `.ai/tickets`, `xtask/src`, `crates/tbd-tickets` | S.2 |
| S.4 | Stamp backfill (git miner) | `xtask/src`, `.ai/tickets` | S.2 |
| S.5 | Token estimates (factor doc, generator, estimates check) | `xtask/src`, `.ai/tickets/estimates`, `docs/platform` | S.4 |
| S.6 | Gate ON + stamp-sha + plan ready-gate | `xtask/src`, `crates/tbd-tickets`, `docs/plans` | S.4, S.5 |

Acceptance (paste-stdout; N always measured at run time, instrument named):

- **S.1**: check OK with file present; planted duplicate surface under one parent →
  red naming file+parent+value; restore → check OK; test prints "D domains, L layers,
  C components, F surfaces" counted from the file.
- **S.2**: migrator prints "migrated N/N files" (N = `ls .ai/tickets/T-*.toml | wc
  -l`), unmapped-scope list printed and empty; corpus roundtrip "N/N files
  byte-identical" on the migrated tree; `git diff --stat -- .ai/tickets/wave.lock`
  empty; workspace `cargo build` green; check --strict OK; new `ticket
  scope-histogram` pastes per-level counts — former editor bucket shows ≥K distinct
  surfaces with counts plus "U surface-empty (scope ∈ estimated: E)", K/U/E measured;
  before/after pasted for 3 named representatives (editor+chrome ticket, repo-docs
  landfill ticket, frozen-unmappable ticket).
- **S.3**: "M summaries >40 TOML-parsed whitespace-split words moved to
  migration_legacy" printed by the migrator (instrument in the output line); "M/M
  reversible" — newline-join byte-compare vs `git show HEAD^:` per file; ratchet pin
  == measured count (test red on drift either way); planted 41-word summary in a
  scratch tree → red naming ticket+field+count+cap, live tree green; docs/TICKET_*.md
  regenerated in-commit.
- **S.4**: miner report "of S shipped: A git_subject, B id_interpolation, C already
  measured" with A+B+C=S; corpus loads green (the load IS the RFC 3339 proof); the 4
  shipped_at strays printed before/after; `git diff -- .ai/tickets/wave.lock` empty.
- **S.5**: factor constant pasted from the doc + a check that every estimate.json
  factor equals it; generator prints "E diff_loc, C cohort_median, E+C = shipped
  count"; planted estimate inside `metrics/<id>/` → red; estimate + receipt same id →
  red; `summarize_by_agent` on a mixed fixture equals the receipts-only hand
  computation pasted alongside.
- **S.6**: planted shipped ticket missing completed_at → red naming ticket+field;
  scratch end-to-end ship → commit → `stamp-sha` → check green (auto-estimate
  written, factor matches); `ops::ship` on a created_at-less ticket refuses pre-write
  (corpus byte-untouched, the ops refusal-test pattern); `mark-ready` without plan
  file refuses naming the path; the 5 live ready tickets carry plans; strict prints
  "shipped tokens measured/estimated: K/E" + per-source breakdown.

### Program B — Ticketboard v2

| Slice | What | depends_on |
|---|---|---|
| B.1 | Breadcrumb scope rendering + facet filters + class chips | S.2 |
| B.2 | Provenance rendering: measured vs estimated distinct everywhere, per-source tooltip, NO mixed sums (negative assertion) | S.5 |
| B.3 | Ten-field detail sections + migration_legacy triage affordance | S.3 |
| B.4 | In-app markdown viewer (egui_commonmark — named dep event): spec/plan/citation clicks render in a pane, raw-text fallback, external-open still offered | S.2 (renders S.6 plans) |

T-915-style acceptance: counts equal measured file counts; screenshot + negative
assertion that no UI element renders receipt+estimate arithmetic; `git status
--porcelain` unchanged after a full session.

### Program T — Wall triage (bookkeeping)

AI batches of 20–30 draining the ratchet, operator-reviewed commits. Per batch:
pin shrinks by exactly the batch size; remaining files still pass the reversibility
join-proof; check --strict green.

## Non-goals

No schema_version dual format. No 5th scope level. No compiled vocab enums (except
`domain`). No estimates inside `metrics/`. No receipt+estimate summing anywhere. No
parse-time caps. No sentence-split semantic laundering. No silent backfill — every
derived value marked with its method. No board writes to the registry outside the
xtask verbs.

## Verified pins (measured 2026-08-14 — do not invent)

- v1 scope depth is empirically dead: 1 nonempty `chrome` / 722 editor tickets; the
  `capability` field is 0-populated. Depth without requirement rots.
- `metrics::has_receipt` returns true for ANY file under `metrics/<id>/` — estimates
  colocated there would impersonate receipts and satisfy `land_receipt_refusal`.
- `Status::Shipped` carries `shipped_at` inside the status for programs while work
  tickets carry the field — gate implementation reads through both
  (`ops.rs::current_shipped_at` documents the asymmetry).
- `sync.rs` copies `summary` verbatim into docs/TICKET_*.md ×5 + queue.json — the
  quarantine commit regenerates them or the parity class regresses.
- 935 distinct ids in commit subjects vs 998 shipped: children mostly lack subjects —
  `id_interpolation` will carry a large share; per-source counters make that visible.
- The T-916 verbs + `store::Corpus`/`ops` are the mutation substrate for everything
  here; the migrator alone works Value→Value (v1 files are unparseable by v2 types)
  with the re-parse + byte-stable render gate per file.
