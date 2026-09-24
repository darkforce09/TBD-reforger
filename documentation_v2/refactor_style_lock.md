**Status:** live — Documentation V2 style lock (CP3b)

# Documentation V2 style lock

This lock binds every Phase 4 README writer and every Phase 5 doc writer, alongside the locked
[README standard](/documentation_v2/standards/readme_standard.md) and the
[templates](/documentation_v2/standards/templates/README.md): where the standard leaves a choice
open, the rules below settle it. Each rule names its model, a passage of a checked pilot file that
shows the rule applied, as `path:first-last` line ranges; copy the pattern of the model, not its
facts.

## Rules

1. **Routes.** One bullet per path; the full `/api/v1/…` path everywhere; methods joined with
   "and"; then the access tier and the meaning.
   Model: `apps/website/api_v2/src/server_infrastructure/README.md:44-61`.
2. **Contents roles.** A purpose, lowercase, no closing period, at most about 100 characters per
   line; entries in name order (`mod.rs` before `modpack_lookup.rs`); a symbol first only when the
   file exists for that type; `mod.rs` is "the module tree", plus "; re-exports X" when it
   re-exports; `tests/` is "unit tests for …".
   Model: `apps/website/api_v2/src/core/middleware/README.md:10-19`.
3. **Purpose.** One to three sentences that link the glossary term for the folder's subject.
   Model: `apps/website/api_v2/README.md:3-7`.
4. **How it works.** The flow first, then the invariants that span files; an ASCII diagram for a
   flow or a state machine, a table for parallel units, constants by name; a leaf stays near 60
   lines. Models: `apps/website/api_v2/src/bin/README.md:23-30` (a flow),
   `apps/website/api_v2/src/server_infrastructure/services/fleet_commands/README.md:22-30` (a state
   machine), `apps/website/api_v2/src/background_workers/README.md:45-58` (a table).
5. **Depends on.** Grouped: `core` first, then the other domains with what each supplies, then the
   crates and external services.
   Model: `apps/website/api_v2/src/server_infrastructure/README.md:78-84`.
6. **Used by.** Found with `git grep`: the module callers, then the workers, binaries and tests by
   repository path, then "over HTTP," the client folders.
   Model: `apps/website/api_v2/src/server_infrastructure/README.md:85-92`.
7. **Rules.** Each rule followed by its test function or gate command; at least one rule
   particular to the folder. Models: `apps/website/api_v2/migrations/README.md:70-74`,
   `apps/website/api_v2/src/administration/README.md:69-70`.
8. **Glossary links.** The first prose use of every glossary term in every file, leaf and set
   READMEs included, links its entry once per file, never in an H1, a Contents role or backticks.
   "Registry" meaning the ticket registry, "event" inside "SSE event", quoted interface text and
   the text of another link take no glossary link; the link goes on the next use outside them.
   Model: `apps/website/api_v2/src/server_infrastructure/README.md:3-12`.
9. **Paths.** A full repository path outside the folder, a folder-relative path inside it;
   Getting started commands keep repository-root-relative paths. Models:
   `apps/website/api_v2/README.md:52-61` (Getting started), `apps/website/api_v2/README.md:89-118`
   (Configuration), `apps/website/api_v2/src/bin/README.md:36-65` (Commands).
10. **Feature doc Behaviour.** Numbered steps under `###` sub-flows, interface text quoted exactly;
    it ends with `### Known discrepancies`, each "the UI says X (file) — the API does Y (handler in
    file)"; every discrepancy's open ticket goes in Open work, after checking `.ai/tickets/` for
    the statuses `idea`, `queued`, `ready`, `running`, `review` and `deferred`. Models:
    `documentation_v2/website/frontend/pages/administration/personnel/personnel_roster_page.md:30-74`,
    `documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md:143-161`.
11. **Feature doc Data, Design, Open work and Decisions.** Data: one bullet per call,
    "`METHOD /api/v1/…` (`handler` in file): read as `DTO`; server-side meaning". Design: the
    layout as built, then "Design target: <set>, a design-phase reference", then the differences.
    Open work: "(status, plan): what changes", where plan is the ticket's `plan` field as a link or
    "no plan". Decisions: "decision: why". Model:
    `documentation_v2/website/frontend/pages/administration/personnel/personnel_roster_page.md:80-141`.
12. **Documentation folder and set READMEs.** A folder README indexes its pages in a table of page,
    route and feature doc (model:
    `documentation_v2/website/frontend/pages/administration/README.md:39-46`); a set README opens
    "Design-phase reference for …" and gives the differences in one paragraph that links the
    feature doc (model:
    `documentation_v2/website/frontend/pages/administration/personnel/visual_references/personnel_roster_blueprint/README.md:5-33`).
13. **Checks before handover.** Writers check their new files with
    `cargo xtask verify <gate> --path <slice> --with-untracked` before handing over. With the flag,
    the gates also see another session's untracked files: ignore violations in paths you do not
    own, and never document untracked files you did not create.
    Model: `documentation_v2/standards/readme_standard.md:317-318`.

## Don't copy

- The "The page README lists no calls" Data openers of the administration feature docs, and the
  state texts and route facts they repeat from the page READMEs: interim until R05 rewrites those
  READMEs.
- Open work that reads "None." without a check of `.ai/tickets/`.
- Short `/admin/…` route paths.
- `mod.rs` role variants.
- Leaves of 70 to 95 lines as a target.
- Partial paths such as `bin/api.rs`.
- The backticked H1s and "Not present in the legacy page" sections of the administration page
  READMEs under `apps/website/frontend/src/v2/pages/administration/`, which R05 rewrites.
