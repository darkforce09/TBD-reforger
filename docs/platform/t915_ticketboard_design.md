# T-915 — Ticketboard: a native GUI projection of the ticket registry

Design contract, agreed with the operator 2026-08-14. Two programs come out of this
design: **T-915 "Ticketboard"** (the egui viewer + mutation UI) and **"Typed registry
ops"** — referred to as **T-916 throughout this doc as a placeholder**; its real id is
whatever `ticket add` derives at mint time. Nothing is minted by this doc; minting
happens through the normal registry process after operator sign-off.

**Do not write the `STRICT_LEGACY` phrase matching `Track [ABC]\b` in this spec, ticket
prose, or any commit subject/body that `ticket sync` copies into `docs/TICKET_*.md`.**
Scanner: `xtask/src/constants.rs`.

## Why

The registry is ~1173 per-file TOMLs under `.ai/tickets/`, deliberately
abstraction-free for AI readability — and therefore near-unreadable for the human
operator. There is no overview of what is done / in flight / queued / idea / deferred,
what waves exist, what is in them, or how tickets relate. Ticketboard is a **pure
projection** over the existing files. The AI factory keeps reading raw files; the app
adds eyes, not state.

## Decisions log (2026-08-14)

1. **Mutations via subprocess xtask verbs.** The app shells
   `cargo xtask ticket <verb>`; it never links writers in-process. This preserves the
   check-before-write refusal architecture (the T-237 / T-451 / T-455 / T-459 line:
   `require_check_ok` runs before any byte lands), eliminates version skew between a
   long-running GUI build and a fast-moving rule set (schema.json, `ALLOWED_NEW`,
   fossil allowlist, wave ledger all move weekly), and keeps "one legal writer" a
   literal single binary. Rejected: linked in-process writers — they invert the
   refusal architecture (check would run *after* bytes land, because
   `require_check_ok` cannot move to `tbd-tickets`; it needs jsonschema, git
   subprocesses, walkdir, and the wave ledger, all xtask territory).
2. **Read-only viewer ships as three slices** (board+detail, waves+tree+filters,
   trust+watch) — each independently landable with paste-able acceptance.
3. **`ticket remove` on a program refuses unless `--force`.** Deliberate, documented
   divergence from byte-parity on this one verb: today's save path silently
   cascade-deletes all child files (`phase2.rs::save_tree` stale-file pass), which is
   the same hazard class as the mangled-`children[]` mass deletion. Safety over parity,
   called here rather than discovered later.
4. **`ticket add-child` onto `kind = "work"` refuses; `--promote` performs the atomic
   work→program rewrite plus first child in one op.** The encoding hard-refuses
   work-with-children and program-without-children (`encoding.rs`), so a first-child
   add *must* rewrite the parent's kind in the same op — the decision is that this
   promotion requires an explicit operator flag, matching the refusal culture.
5. **The core rewrite is its own program (T-916), not slices of the UI ticket.** The
   typed-ops extraction + cmds.rs rewire is a registry-core rewrite at T-911 scale.
   The viewer lands value even if the core rewrite stalls, and a core refactor must
   not ride as slice 4-of-7 of a UI ticket in the wave ledger. Rejected alternative:
   one seven-slice program.
6. **No ambiguous numbers in acceptance lines.** Every criterion is a paste-gate.
   Corpus-size assertions are phrased as measured N, never hardcoded counts; the frame
   budget is stated as 60 fps / no frame over 17 ms (an earlier draft said "60fps, no
   frame >32ms", which contradicts itself — 32 ms is 30 fps).
7. **Repo discovery is pinned** (§Read architecture): positional CLI arg, else upward
   walk for `.ai/tickets/`, else a full-window refusal with a native folder picker;
   the picked path persists to eframe Storage in the user config dir. Multiple repos
   are out of scope.

## Data layer contract (binding — the app is a projection over these files)

- `.ai/tickets/T-*.toml` — one file per parent and per dotted child, encoding C,
  parsed by `crates/tbd-tickets` (`parse_ticket_toml` / `render_ticket_toml`;
  canonical field order **is** `TicketFile` struct order, proptest-roundtripped). The
  8-value `StatusName` enum is untouched: **no new "planned" status** (`queued`
  already means that), and the UI shows **raw status names** — the operator lives in
  CLI-land, and a friendly label that differs from grep output is a tax.
- `.ai/tickets/wave.lock` — compiled wave plan. `cargo xtask wave repack` is the only
  legal writer; a status change without repack is check-red **by design**. The app
  renders the lock **verbatim** and never recomputes lanes: if tickets drifted from
  the lock, that is the trust banner's red, not silently re-derived lanes.
- `.ai/tickets/metrics/<id>/<ts>-<sha>.json` — per-run token receipts. As of
  2026-08-14 the directory does not exist (T-913.2 shipped the producer; zero runs
  recorded). The dashboard renders an explicit "no receipts yet" state, never zeros.
- `.ai/tickets/schema.json` + `cargo xtask ticket check --strict` — the validation
  authority. The app **never re-implements check; it invokes it.**
- No SQLite, no sidecar DB, no app-owned state files, no cache that becomes truth.
  App layout/prefs live in the user config dir (eframe Storage), never in `.ai/`.
  The app never commits to git — the operator commits.

## Framework: egui/eframe

- **Wayland/KDE (primary target now):** winit-based; KDE provides server-side
  decorations (GNOME is the problem child, not KDE); fractional scaling handled;
  AccessKit integrated. **Windows (next):** works unmodified. Rust only, no DSL, no
  codegen, single binary.
- Immediate mode fits a watch-reload dashboard: state is the parsed corpus, redraw is
  cheap, and ~1200 cards are trivial with `ScrollArea` row culling / `egui_extras`
  tables. The repo already speaks wgpu (`map-engine-render`); eframe's wgpu backend
  is the default, with the glow fallback behind a flag for driver quirks.
- **Rejected:** Iced (Elm-style boilerplate, API churn between releases, weaker
  table/drag-drop ecosystem); Slint (the `.slint` DSL is a second language against
  the T-165 all-Rust spirit, plus license friction); Tauri (ruled out by operator —
  the web-tooling stack was eradicated at T-165).
- **Named cost:** egui/eframe (with winit/wgpu) is a first-time dependency family for
  this workspace — a real supply-chain and build-time event; the T-915.1 ticket names
  it rather than smuggling it in.
- **Threading rule:** all IO — corpus parse, subprocesses, git — happens on worker
  threads with channel + `request_repaint`; the UI thread never blocks. Wave 0's
  ~19 KB single-line ticket array never reaches a text widget unwrapped. The text
  filter searches precomputed lowercase haystacks, not per-frame formatting.

## Read architecture

- **Repo discovery:** positional CLI arg wins; else walk up from the cwd looking for
  `.ai/tickets/`; else a full-window refusal that states both mechanisms and offers a
  native folder picker. The picked path persists to eframe Storage. One repo per app
  instance.
- **Corpus load:** link `tbd-tickets` as a workspace member and typed-parse **all**
  `T-*.toml` — parents and children. Deliberately *not*
  `phase2::load_phase2_tree`, which is parents-only; the app's whole point includes
  the children the Value projection hides.
- **Fail-closed load:** any parse failure is a full-window refusal — no partial board
  (the DidNotRun philosophy). The refusal is a trust surface, not a dead end: it
  names the file, shows the parse error verbatim, and offers "reveal in file
  manager"; the watcher keeps running, so fixing the file on disk auto-recovers the
  board.
- **Watch:** `notify` on the `.ai/tickets/` directory (one watch, not 1200) **and**
  on the sync targets (`docs/TICKET_*.md`, `CLAUDE.md`, the ROADMAP marker file);
  debounce ≥500 ms; coalesce — a new event during an in-flight reload/check schedules
  exactly one re-run; suppress self-triggered events while a verb subprocess is in
  flight plus one debounce window after it exits (otherwise every app mutation
  triggers its own multi-second strict check storm).
- **Concurrency stays dumb** (operator decision): last-writer-wins per file; reload
  on change; check after every write; red surfaces immediately.

## UI shape

- **Board** — 8 status columns in `StatusName` order; `shipped` and `cancelled`
  collapse to count chips by default. Cards show id / title / executor / order.
  Filters compose: executor, kind, program/parent, scope family, free text; the
  footer shows filtered/total.
- **Waves** — lanes rendered verbatim from `wave.lock`, one lane per open `n`
  (currently 133+). Wave 0 is **always** a count chip ("~1090 parked"), never
  expanded cards by default. Header shows `wave_base` / `max_concurrent` /
  `pack_last` trailing singletons. A missing or unreadable lock renders the DidNotRun
  refusal text (same prefix as `wave_lock::missing_lock_error`), never empty lanes.
  An "Unplanned" side bucket lists dispatchable ids absent from the lock — pure set
  arithmetic over lock + files, never a re-pack.
- **Program tree** — parent→children, status-colored, expandable.
- **Detail panel** — every field; spec path click opens via xdg-open/start;
  `depends_on` / `owns` / `parent` / `children` are clickable links. Owns-collision
  explainer: select any two tickets and the panel surfaces the colliding path pairs
  under the prefix-containment rule — "why these two can never share a wave".
- **Trust banner** — the result of `cargo xtask ticket check --strict` (streamed
  output, exit code shown), labeled **strict**. The mutator preflight is
  **non-strict** (`require_check_ok` passes `strict = false`), so banner-red does not
  always mean mutations refuse — the UI never conflates the two. A git-dirty chip
  summarizes `git status --porcelain -- .ai/tickets docs CLAUDE.md` ("N uncommitted
  registry files") — the app never commits, so the operator must see pending state.
  The banner distinguishes **"building xtask…"** from **"checking…"**: cargo rebuilds
  xtask whenever the tree changed, and xtask is the heavy bin (clap, jsonschema,
  map-engine-core, typify, syn) — the first check or verb after a `git pull` is a
  multi-minute compile, surfaced honestly with streamed build output, not hidden
  behind a generic spinner.

## Write path

### T-916 — Typed registry ops (the extraction)

Today's mutation path is the problem: `load_phase2_tree` typed-parses parents, then
converts **back** to `serde_json::Value` (legacy `slices`/`active_slice` mirrors,
`slice_plan` synthesized from child files); `cmds.rs` mutators poke the Value;
`save_tree` re-types per ticket, rewrites parents, and deletes any `T-*.toml` not in
{parents ∪ `children[]`}. The 4a2f3426 alias clash ("duplicate field `children`",
which broke every mutator) and the child-id hole (`ticket ship T-912.2` → "Unknown
ticket"; children are shipped today by hand TOML edit + repack) both live in exactly
that round-trip.

**Moves into `tbd-tickets` (mechanism only):**

- A typed corpus store: load-all-files → `BTreeMap<String, Ticket>`, parents and
  children. This becomes the shared substrate for the three near-duplicate walks that
  exist today (`wave_lock::load_views`, `check::check_open_work_owns`,
  `slice_collisions::ticket_facts`) — otherwise there will be four.
- Per-file surgical writes: render → re-parse → write to temp → rename. Re-parse
  before write follows the `migrate_live_tree` pattern; temp+rename kills torn reads
  by the watcher (which otherwise reads half-written TOMLs and flashes refusals).
  The `save_tree` full-rewrite-plus-delete pass dies with this — no mutation can
  mass-delete children again.
- Ops as pure corpus transforms with an **injected clock** (precedent:
  `metrics::stamp_land_at`, which exists so tests never race the wall clock):
  `set_status`, `ship`, `mark_ready`, `add`, `add_child`, `remove`, `reorder`,
  `advance_slice`. Semantics replicated exactly from `cmds.rs`:
  - `ship`: status→shipped, stamp `completed_at`, clear `active` — and now resolves
    child ids.
  - `set_status`: the 8-value enum gate; `cancelled` stamps `completed_at`.
  - `add`: id mint = max **parent** numeric + 1 (children never affect it —
    `derive_next_id` semantics preserved exactly), `kind = "work"`,
    `status = "idea"`, `scope.repo.layers = ["docs"]`, `created_at` stamp.
  - `mark_ready`: spec set + spec-on-disk + deps shipped/cancelled gate +
    user_story/acceptance backfill.
  - `advance_slice`: walks typed `ProgramTicket::children` (today it reads the
    mirrored `slices` key off the Value).
- **General invariant (new): no op may write a corpus its own preflight would
  refuse.** Post-image validation — render, re-parse, business rules — before any
  byte lands. The live wedge motivating it: `cmd_reorder` writes colliding duplicate
  live orders, `validate_registry` reds on them, and every subsequent verb refuses
  until the operator hand-edits. That class dies here.
- New invariants: duplicate-children refusal (plus a regression pin on the 4a2f3426
  alias class); `add_child` requires an existing program parent (work ⇒ refuse, or
  `--promote` for the atomic kind rewrite + first child); a child id must be a dotted
  extension of its parent id; `ship` on a child clears (or flags) a stale
  `parent.active`; `remove` on a program refuses unless `--force`.
- Referential integrity becomes a `check()` rule — `children[]` naming a missing
  file, or a child file whose parent is absent. Without the save_tree delete-pass,
  stray children would otherwise be permanently invisible: nothing checks
  parent↔child today.
- The `ALLOWED_NEW` / `ENCODING_C_KEYS` governance tests stay compiling and honest —
  they are the tripwire for exactly the new-field class a GUI era will generate.

**Stays in xtask (policy + heavy deps):** `require_check_ok` and all of `check.rs`
(jsonschema, git grep, walkdir, wave/metrics checks); `sync.rs` wholesale; wave
machinery (`wave_lock.rs`, `wave/` including the `base.rs` close-marker ledger
authority); the Value **read** path (`load_phase2_tree`, `ticket_to_value`,
`attach_slice_plan`, `registry.rs` helpers) serving
brief/show/list/next/prompt/sync/gap/queue.json — and `cmd_get`, whose mirrored
output external scripts may parse; do not "clean it up".
`phase2::save_tree` / `value_to_ticket` demote to migration/test-only, with the
T-912.2 regression pin retargeted at the typed ops.

**Rewiring sequence invariant:** typed op writes files → **reload**
`load_phase2_tree` from disk → `cmd_sync(reloaded)` → `repack_quiet`. Passing the
pre-mutation Value to sync regenerates docs from the *old* state — a named hazard,
pinned by test. The reload is also what makes child verbs coherent with the old read
path: `attach_slice_plan` synthesizes `slice_plan` from child files, so a typed
child-ship followed by reload surfaces the child's new status into queue.json and the
generated docs automatically.

**Preserved asymmetries (rationalize later, own ticket):** `set-status` writes only
queue.json + repack (no full sync — generated docs go stale by current design);
`mark-ready` syncs but does not repack (benign: queued→ready is
dispatchability-neutral); `ship` full-syncs. A "cleaner" unified rewrite would change
bytes and fail parity — preserve first.

**Explicit leftovers:** `shipped_at` stays hand-edited — the SHA does not exist until
the operator commits (chicken/egg), so no verb and no GUI invents it; a future
`ticket stamp-sha <id> <sha>` verb (analogous to `metrics::stamp_land`) is the
eventual fix. The bulk-triage pressure valve is a future **batched verb**
(`ticket set-status --stdin`: one preflight, N mutations, one sync, one repack) —
never in-process linking.

### App-side verb plumbing (T-915.4)

- Single-flight queue: one verb subprocess at a time; input disabled + streamed log
  while it runs.
- Resolve the repo root and run with explicit `current_dir`; resolve `cargo`
  robustly (GUI-launched processes have arbitrary cwd and a bare PATH without rustup
  shims).
- Treat **any** nonzero exit as "reload + show stderr verbatim" — several refusal
  paths are `process::exit(1)` before anyhow ever sees them; never parse messages.
  The existing refuse strings are good; show them untouched.
- UI compare-and-swap: refuse to dispatch if the target file's bytes changed since
  the clicked card was rendered (softens the app-vs-terminal/factory race; two
  concurrent xtask invocations already race today, the GUI just multiplies exposure).
- The app must **not** auto-repack after a mid-verb crash — that would make it a
  second wave writer. It shows check-red plus the recovery command
  (`cargo xtask wave repack`) and waits for the operator.

## Parity bar for T-916 (the T-911 / T-853 standard)

- **Surface:** `.ai/tickets/**` **plus** `docs/TICKET_*.md`, `docs/MILESTONES.md`,
  `CLAUDE.md`, the ROADMAP marker file, the gap-analysis file, `queue.json`, and
  `wave.lock`. A diff limited to `.ai/tickets/` lets a regression in the sync half
  ride through green — the verbs write all of it.
- **Method:** copy the live tree (plus the doc surface) into two scratch roots; run
  old binary vs new binary per verb per representative ticket; diff the full surface.
  Timestamp lines (`created_at` / `completed_at`) are normalized or the clock frozen
  — the old path cannot inject a clock, so "byte-for-byte including timestamps" can
  never pass on `ship`; byte-exact on everything else. Paste per-verb
  "0 differing files".
- **Red trees:** both binaries refuse with exit 1 and **zero** file changes
  (`git status --porcelain` empty in both scratch roots).
- **Prove-it-bites pins:** `ship` of a dotted child works end-to-end under the new
  binary while the old binary's "Unknown ticket" is documented in the test as the
  fixed hole; generated docs reflect the *post*-mutation state after a rewired ship
  (the stale-Value hazard, pinned directly); corpus roundtrip prints "N/N files
  byte-identical" where N is the on-disk ticket file count at run time (~1173 today,
  never hardcoded); reorder-collision refuses instead of writing red state.

## Programs and slices

`parallel_ok` mechanics: every T-915 slice owns `apps/ticketboard`, so T-915 slices
are serial among themselves. T-916.1 owns only `crates/tbd-tickets` and packs
parallel to T-915.1–.3. T-916.2 owns `xtask/src` and collides with anything else
touching it.

### T-916 — Typed registry ops

| Slice | What | owns | depends_on |
|---|---|---|---|
| T-916.1 | Typed corpus store + ops in `tbd-tickets`: injected clock, post-image validation, new invariants, temp+rename writes | `crates/tbd-tickets` | — |
| T-916.2 | Rewire `cmds.rs` onto typed ops; parity harness; new verbs (`add-child [--promote]`, `remove --force`, child-resolving `ship`/`set-status`); referential-integrity check rule; reload-before-sync | `xtask/src`, `crates/tbd-tickets` | T-916.1 |

**T-916.1 acceptance (paste stdout):**
1. Corpus roundtrip prints "N/N files byte-identical", N measured on disk.
2. Per-op refusal unit tests: reorder-collision, ready-without-order,
   add-child-onto-work (without `--promote`), duplicate child.
3. 4a2f3426-class regression pin (mirrored-keys condition unrepresentable/handled).
4. `ship` of a dotted child succeeds at the op layer.

**T-916.2 acceptance (paste stdout):**
1. Parity: per-verb "0 differing files" over the widened surface, timestamps
   normalized.
2. Red-tree runs: both binaries exit 1 with porcelain empty.
3. Child ship end-to-end: child file flips, repack runs, queue/docs reflect it after
   reload.
4. `cargo xtask ticket check --strict` prints `check OK`.

### T-915 — Ticketboard

| Slice | What | owns | depends_on |
|---|---|---|---|
| T-915.1 | Viewer core: `apps/ticketboard` workspace member, typed corpus load, board + detail, fail-closed parse, repo discovery | `apps/ticketboard`, root `Cargo.toml` | — |
| T-915.2 | Wave lanes + program tree + filters | `apps/ticketboard` | T-915.1 |
| T-915.3 | Trust banner + file watch | `apps/ticketboard` | T-915.1 |
| T-915.4 | Mutation UI over subprocess verbs | `apps/ticketboard` | T-915.3, T-916.2 |
| T-915.5 | Metrics dashboard | `apps/ticketboard` | T-915.1 |

**T-915.1 acceptance:**
1. `cargo build -p ticketboard` green on the pinned toolchain (1.95.0); the workspace
   `members` diff is one line plus the new crate.
2. Footer total equals `ls .ai/tickets/T-*.toml | wc -l` output at run time; parent
   and child counts shown separately and summing to the total.
3. A perturbed ticket file in a scratch copy produces the named full-window refusal
   with the verbatim parse error; fixing the file recovers without restart.
4. Release build scrolls the full board at 60 fps: no frame over 17 ms on the
   ~1200-ticket corpus.
5. `git status --porcelain` identical before and after a session exercising every
   view (the no-writes proof).

**T-915.2 acceptance:**
1. Per-lane id list equals the lock's `tickets` array for that `n` (paste: the app's
   "copy lane as TSV" against the corresponding `wave.lock` block).
2. Wave 0 renders as a count chip, never expanded cards by default.
3. Deleted/renamed `wave.lock` ⇒ the DidNotRun refusal text, not empty lanes.
4. Filters compose; clearing restores the full measured count.

**T-915.3 acceptance:**
1. Launch runs `cargo xtask ticket check --strict` streamed; banner shows green/red
   plus exit code, labeled "strict".
2. "building xtask…" and "checking…" are visibly distinct states.
3. External edit of a ticket TOML reflects in the UI within one debounce window;
   events coalesce (one re-run per burst); the app's own verb writes do not
   re-trigger a check storm.
4. Git-dirty chip shows the uncommitted-registry-file count.

**T-915.4 acceptance:**
1. Every offered transition maps to one xtask verb; refusals stream verbatim; any
   nonzero exit reloads and shows stderr.
2. Ready promotion goes through the Ready-prose form (spec on disk + user_story +
   acceptance) and surfaces the deps-unshipped refusal.
3. `running` is not offered as a manual target; raw set-status is behind an
   "advanced" affordance.
4. CAS guard: a card whose file changed since render refuses dispatch with a
   reload prompt.
5. Mid-verb SIGKILL (between save and repack): the app shows check-red with the
   wave-stale error text and the recovery command; it does not auto-repack.

**T-915.5 acceptance:**
1. With no `.ai/tickets/metrics/` dir, the dashboard shows the explicit "no receipts
   yet" state — no zeros.
2. With fixture receipts, per-ticket and per-agent token totals and elapsed sums
   match a hand computation pasted alongside.

## Non-goals

No DB / sidecar store / cache-as-truth. No git commits from the app. No CI job
initially. No recomputed wave lanes. No invented `shipped_at`. No new statuses, no
friendly status labels. No in-process writers. App layout/prefs never in `.ai/`.
Multiple repos out of scope — one repo per app instance.

## Verified pins (do not invent — measured 2026-08-14)

- Live mutation path: `phase2::load_phase2_tree` (parents only, typed→Value with
  `slices`/`active_slice` mirrors + synthesized `slice_plan`) → `cmds.rs` Value
  mutators → `phase2::save_tree` (re-type, rewrite parents, delete undesired files).
- 4a2f3426 = the alias-clash fix commit; the bug broke every registry mutator and was
  caught at ship time.
- `cmd_ship` stamps `completed_at` and clears `active`; **nothing writes the
  `shipped_at` SHA** — hand-edit today.
- `cmd_set_status` writes queue.json + repack only (no full sync); `cmd_mark_ready`
  syncs but does not repack; `cmd_ship` full-syncs. Asymmetries preserved by T-916.
- `cmd_reorder` can write colliding duplicate live orders → red-on-disk wedge
  (post-image validation motivator).
- `require_check_ok` runs `check(strict = false)`; the CI/banner surface is
  `--strict`. Not the same bar.
- Wave machinery scale: `wave_lock.rs` ~1236 lines, `wave/base.rs` ~994 lines (git
  close-marker ledger + revert-trailer disavowals) — stays in xtask.
- `tbd-tickets` deps today: serde, toml, time only. The extraction adds no external
  dependencies (std fs walk).
- `.ai/tickets/metrics/` absent; `metrics::check_as_errors` is clean on a missing
  dir.
- Ticket file count at design time: 1173 (`ls .ai/tickets/T-*.toml | wc -l`); lock
  wave 0 ≈ 1090 ids; open waves 133+ over `wave_base = 132`.
