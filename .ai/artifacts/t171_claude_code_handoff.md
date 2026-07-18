# T-171 — Claude Code handoff (monorepo / website hygiene)

**Start only after T-169 is tagged/shipped on `main`.**  
**Do not touch `apps/mod/`.**  
**Do not edit docs/registry/CLAUDE status markers** — return a **complete** fix list; Cursor applies as T-171.docs (role split, not content deferral).

Authority: [`docs/platform/t171_monorepo_hygiene_program.md`](../../docs/platform/t171_monorepo_hygiene_program.md).

**HARD:** `.cursor/rules/no-silent-deferrals.mdc` — do the whole program. No “fold forward”, no self-authored Out-of-scope, no “optional later” for inventory rows. Blocked → ASK operator.

---

## Claude Code prompt — T-171 (copy-paste)

```
Read CLAUDE.md first.
HARD GATE: .cursor/rules/no-silent-deferrals.mdc — finish the whole T-171 ask.
HARD GATE: agent split — you do not edit docs/** / registry / CLAUDE sync markers; you RETURN a complete Cursor fix list (T-171.docs). That is not deferring the content.

Implement **T-171** — full monorepo/website hygiene (not mod).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger
  git status -sb && git log -5 --oneline --decorate
  # BLOCKER: T-169 must be tagged/shipped. If not: STOP.
  git tag -l 'T-169' && ./scripts/ticket brief T-169
  ./scripts/ticket brief T-171

═══ READ ═══
  1. docs/platform/t171_monorepo_hygiene_program.md
  2. .ai/artifacts/t171_claude_code_handoff.md
  3. Cargo.toml · Makefile · .github/workflows/ci.yml
  4. apps/website/ (API + internal/ + Makefile) · apps/website-leptos/
  5. Fixture homes: .ai/artifacts/t159_gates/fixtures · packages/tbd-schema/golden* · crates/map-engine-core/tests/fixtures
  6. packages/map-assets + CI LFS usage

═══ PROBLEM ═══
  Website/monorepo layout and residue are disjointed after React→Leptos and Node eradication.
  Operator wants one thorough hygiene ship so structure, names, dead code, fixtures, map-assets
  story, tooling, and conventions are coherent — not a partial MVP with “later” appendix.

═══ SHIPPED (do not reopen product work) ═══
  T-166..T-169 (T-169 must be done before you start). T-170 = human prod flip — leave env secrets,
  but fix any path strings T-170 will need (SPA_DIST_DIR → frontend/dist).

═══ LOCKED ═══
  End layout:
    apps/website/api/       ← Axum API crate (from today’s apps/website crate root)
    apps/website/frontend/  ← Leptos SPA (from apps/website-leptos)
  Package names aligned: website-api + website-frontend (prefer; ASK only if truly blocked)
  apps/mod/** OFF LIMITS
  No silent deferrals of hygiene items found in inventory
  Forest/site lag are product polish already operator-noted on T-166 — not a reason to skip
  structure/fixture/ADR-path hygiene

═══ DO (all phases) ═══
  Phase 0 — .ai/artifacts/t171_inventory.md (Class-R). No deletes until written.
    Include: layout, dead code, dual SoT, rename blast, golden/fixture homes, map-assets story,
    doc/ADR/rule rot, tickets, conventions gaps.

  Phase 1 — Layout + renames:
    Move SPA → apps/website/frontend; move API → apps/website/api;
    rename packages; fix workspace, root Makefile, CI, Trunk, compose, .env.example, scripts, gates.
    Prove make api + make leptos; /map-assets 200.

  Phase 2 — Dead code + dual-SoT:
    SAFE deletes; relocate seeds then purge internal/; kill nested Go/Vite Makefile;
    purge __pycache__; fix go/npm lies in website scripts.

  Phase 3 — Fixtures + map-assets:
    Consolidate golden/fixture homes to one documented convention (update all refs).
    Finish the map-assets consumption story (CI selective LFS vs local sat vs gate map_assets_dir)
    so the tree isn’t a landmine — ASK only if an external constraint blocks (credentials, LFS host).

  Phase 4 — Tooling one story:
    ticket CLI · leptos-gates · ci-local · verify-no-node · CI names/paths match api+frontend.

  Phase 5 — Return complete Cursor list for T-171.docs:
    ROADMAPs, AGENT_COMMIT_CHECKLIST, DOCUMENTATION_STANDARDS, ADRs/stubs that still say
    Deck/React/Vite/Go, apps/website README/CLAUDE/compose, .cursor/rules paths,
    “Where does X go?” conventions pin (page/handler/smoke/ticket/migration/fixture/asset).

  Ship: make leptos-gates · make ci-local · make verify-no-node · ./scripts/ticket check
  Commit on main · tag **T-171** · push.
  Prefix T-171: · Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

═══ DO NOT ═══
  - Start before T-169 shipped
  - Edit docs/** / registry / CLAUDE sync markers (return list instead)
  - Touch apps/mod/**
  - Invent Out-of-scope / “fold forward” / “P1 later” for anything in the phases above
  - Blind rm -rf internal/ while make seed still reads it
  - Regress /map-assets always-on, Trunk no_redirect, FRONTEND_URL=:3000
  - Ship an MVP layout move and call T-171 done while fixtures/api nest/renames remain

═══ VERIFY ═══
  make leptos-gates && make ci-local && make verify-no-node && ./scripts/ticket check
  test -d apps/website/frontend && test -f apps/website/frontend/Trunk.toml
  test -d apps/website/api && test -f apps/website/api/Cargo.toml
  test ! -e apps/website-leptos
  # no leftover dual API root src/ at apps/website/src unless intentional thin shim — prefer clean

═══ RETURN ═══
  t171_inventory.md path · commit SHAs · tag T-171 · gate exits ·
  COMPLETE Cursor T-171.docs fix list · any ASK that blocked a row (quote operator if they deferred)
```
