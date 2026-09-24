# T-940.9 — Plan

## Context
content/wiki.rs parses a minimal subset (no H3+, links, images, tables, checklists); no revisions. After T-940.8
(wiki.rs) and T-940.6 (services/mod.rs).

## Approach
1. Verify on main: a level-three heading renders as a paragraph; paste the red.
2. `services/wiki_markup.rs` (new, in services/mod.rs): typed block AST with goldens.
3. `migrations/0026_wiki_revisions.sql`; every save inserts a revision; GET revisions route.
4. pages/public/wiki.rs renders every block type and the revision list.
5. Perturbation: drop table parsing → golden red; restore, touch, green.

## Risks
- Untrusted markup: links and images are sanitized (no scripts, allowlisted schemes).

## Verification
- `cargo xtask db test-it`; `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-940.9`
