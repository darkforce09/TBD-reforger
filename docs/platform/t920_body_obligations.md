# T-920 — Body obligations, main_goal, title repair, side-by-side viewer

Design contract, agreed with the operator 2026-08-15 after a live board session on the
v2 tree (T-917/T-918 shipped). Program **T-920** (schema + board), stream ticket
**T-921** (history reconstruction). Extends `docs/platform/t917_ticket_schema_v2.md`;
everything there stays binding.

**Do not write the `STRICT_LEGACY` phrase matching `Track [ABC]\b` in this spec, ticket
prose, or any synced commit text.** Scanner: `xtask/src/constants.rs`.

## Decisions log (operator, 2026-08-15)

1. **`user_story` → `main_goal`** — measured: only 46/1195 carry it and the content is
   already goal-shaped, not persona prose. Rename everywhere (serde alias keeps old git
   revisions parseable); renders at the TOP of the detail panel, directly under the
   title — the first thing read. Obligatory from `queued` upward.
2. **Body fields become obligatory, tiered by status, enforced at transitions:**

   | Status | Must be nonempty |
   |---|---|
   | idea | title (real: ≠ id, ≤10 words), summary, class |
   | queued | + main_goal |
   | ready / running / review | + context, requirement, current_state, approach, verify, acceptance |
   | shipped (future ships) | same as ready — refused at the ship verb |

   `citations` and `notes` stay optional (forcing citations breeds fake references;
   notes is the triage fallback bucket). `depends_on`/`unblocks`/`children` stay
   relational-optional. Quarantine exemption: nonempty `migration_legacy` exempts the
   body-tier rules (content exists, unprocessed) — same field-scoped pattern as the
   summary cap.
3. **History gets backfilled too** (operator overruled transition-time-only): every
   shipped ticket's body fields get filled by reviewed AI reconstruction from real
   sources — its spec, its commits, its diffs — in T-919-style batches. Reviewed
   commits are the honesty mechanism; reconstruction cites its sources in `citations`.
4. **Title repair**: gate new/edited titles (≠ id, nonempty, ≤10 words); history debt
   (measured 2026-08-15: **99 id-as-title + 340 over-10-words**) drains via the
   streams — T-919 batches fix titles on tickets they touch; T-921 covers the rest.
5. **Markdown viewer opens BESIDE the detail panel** (third column to its right), not
   replacing it. Detail stays visible while reading a plan.
6. The "person who supports/updated" ask was a dictation artifact — dropped.

## Schema changes (T-920.1)

- `main_goal` key replaces `user_story` on disk: TicketFile field renamed with
  `#[serde(alias = "user_story")]` (old revisions parse; emit writes `main_goal` in
  the same canonical slot). Governance: `main_goal` joins ALLOWED_NEW;
  `user_story` remains in the frozen ENCODING_C_KEYS as history (on-disk keys must be
  a SUBSET of the union — a vanished key is legal). 46-file migration via write_back.
  Typed model renames `WorkTicket::user_story`/`ProgramTicket::user_story` and the
  `Status::live_ready` parameter through to every consumer (board, ops, check, sync).
- Tier rules: check binds the idea/queued tiers corpus-wide (cheap, already true or
  trivially fixable) and the ready-tier on ready-class work tickets;
  `ops::mark_ready` refuses promotion with any ready-tier field empty (naming each);
  `ops::ship` refuses a ship with any ready-tier field empty (future ships only —
  shipped history is untouched by check until T-921 drains it, then the corpus-wide
  rule turns on with the debt pin at zero).
- Title gate: `ops` post-image refuses a changed work OR program ticket whose title is
  empty, equals its id, or exceeds 10 words; check carries a shrink-only
  **TITLE_DEBT_PIN** (measured at land time ≈ 439 minus overlap, exact number pinned)
  counting offenders, drift-red both directions — the streams shrink it per batch.
- Queued-tier main_goal debt: same pin pattern (**MAIN_GOAL_DEBT_PIN**) counting
  queued+ tickets without main_goal at land time; streams drain it.

## Board changes (T-920.2)

- main_goal renders under the title in the detail header (prominent, wrapped); the
  body section list drops user_story and starts at context.
- Viewer becomes a third column beside the detail panel (detail stays live); Back
  collapses the column; width persisted in eframe Storage.
- Card tooltip shows main_goal when present.

## T-921 — History reconstruction stream (work ticket, T-919 sibling)

Batches of ~20 per reviewed commit over shipped tickets WITHOUT quarantined walls
(the walls belong to T-919, whose batches now ALSO fill the full tier set + fix
titles on the tickets they drain): reconstruct context / requirement / current_state /
approach / verify (and main_goal, title where deficient) from the ticket's spec doc,
commit subjects/bodies, and diffs; `citations` lists the sources used; content is
derived, never invented — thin evidence yields thin honest fields, not padding.
Per batch: TITLE_DEBT_PIN and MAIN_GOAL_DEBT_PIN shrink by the measured amount in the
same commit; check --strict green; wave.lock byte-identical.

## Acceptance (instrument-named)

- T-920.1: 46/46 user_story files migrated (`grep -c '^user_story' == 0`,
  `grep -c '^main_goal' == 46` immediately post-migration); corpus roundtrip N/N
  byte-identical; wave.lock diff empty; mark-ready refusal names each empty
  ready-tier field on a scratch fixture; ship refusal same; title post-image refusal
  on id-title/11-word fixtures; pins equal measured counts (numbers printed by a
  check-side counter with the instrument in the line); check --strict OK on the live
  tree; workspace build green.
- T-920.2: viewer opens beside detail with both visible; Back collapses; main_goal
  under title; body sections start at context; tests pin the layout model;
  `git status --porcelain` unchanged after a session.
- T-921 per batch: pins shrink by the measured batch amount same commit;
  reconstruction cites sources; check --strict OK; wave.lock byte-identical.

## Non-goals

No new statuses; no obligatory citations/notes; no attribution fields; no
check-redding of shipped history before the drain completes; no invented prose in
reconstruction (thin evidence = thin fields).
