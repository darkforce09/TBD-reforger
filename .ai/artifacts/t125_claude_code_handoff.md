# T-125 — Claude Code handoff

**Status:** **in progress** · active slice **T-125.5** · **T-125.4 shipped** @ `cb508cf` (tag **T-125.4**)  
**Spec:** [`docs/platform/t125_coding_standards_enforcement.md`](../docs/platform/t125_coding_standards_enforcement.md) §T-125.5  
**Authority:** [`CODING_STANDARDS.md`](../docs/platform/CODING_STANDARDS.md) §7 FMT-2/FMT-3, §10 matrix, §11 verify replay

**Shipped:** T-125.0 @ `a54f491` · T-125.1 @ `9792182` · T-125.2/.2.1 @ `80c7f07` · T-125.3 @ `e5fbf4b` · **T-125.4 @ `cb508cf` (tag T-125.4)**

---

## T-125.4 — DONE ✓

GO-7 route-match, M6 15/15, LOG-3 two-band (140 sites), verify-* scripts, ENF-4 ×10, `ci.yml`
verify-coding-standards. Do not redo.

---

## Copy this into a **new** Claude Code chat — T-125.5 ONLY

```
═══════════════════════════════════════════════════════════════════════════════
T-125.5 — .editorconfig (FMT-2) + Prettier (FMT-3) + CI wiring
Ticket program: platform coding standards enforcement
═══════════════════════════════════════════════════════════════════════════════

You are implementing ONLY slice T-125.5. Read first, then execute.

READ ORDER (authoritative):
  1. CLAUDE.md — §T-125 block, §Conventions, §Verifying changes
  2. docs/platform/t125_coding_standards_enforcement.md — §T-125.5 (expanded task list)
  3. docs/platform/CODING_STANDARDS.md — §7 FMT-2/FMT-3, §10 matrix (FMT rows), §11 verify replay
  4. apps/website/frontend/eslint.config.js — extend with eslint-config-prettier (last)
  5. .github/workflows/ci.yml — frontend job (mirror ci-local-frontend)
  6. Makefile — ci-local / ci-local-frontend
  7. .ai/artifacts/t125_claude_code_handoff.md — this checklist

Do NOT implement T-125.6 (registry/CLAUDE/CODING_STANDARDS matrix doc sync — Cursor).
Do NOT redo T-125.4 (handlers, verify-* scripts, ENF-4, GO-7 — all live @ cb508cf).

═══════════════════════════════════════════════════════════════════════════════
PREFLIGHT (repo root)
═══════════════════════════════════════════════════════════════════════════════

  ./scripts/ticket brief T-125
  # Expect: SLICE: T-125.5, TARGETS: root, DO NOT: edit documentation

  git log -1 --oneline
  # Expect doc sync 67eb0e3+ or code cb508cf T-125.4 on main

  nvm use && node -v    # Node 26

  # Baseline (must pass before you start)
  make db-up
  make ci-local         # ~22s; all green including verify-coding-standards

  # Confirm gaps (should be absent today)
  test -f .editorconfig && echo UNEXPECTED || echo "no .editorconfig yet (expected)"
  cd apps/website/frontend && npm run format:check 2>&1 | head -3   # script missing (expected)

═══════════════════════════════════════════════════════════════════════════════
ALREADY SHIPPED — DO NOT REDO
═══════════════════════════════════════════════════════════════════════════════

  T-125.1  ci.yml + make ci-local
  T-125.2  golangci full gate
  T-125.3  strict TS + eslint TS-2..7/LOG-2/COMP-1 + TS-6
  T-125.4  @route route-match, M6, LOG-3, verify-* scripts, ENF-4, ci.yml verify-coding-standards

  FMT-1 (gofmt) already in ci-local-backend + ci.yml backend — do not change Go formatting policy.

═══════════════════════════════════════════════════════════════════════════════
EXECUTION MODEL
═══════════════════════════════════════════════════════════════════════════════

  • Work on main (single-ticket mode)
  • One commit when done; tag T-125.5; Co-Authored-By trailer
  • Expect a LARGE formatting-only diff from Prettier — keep it separate from logic edits
  • Stage paths explicitly (ignore mod .rdb, worlds/*.ent, map-assets symlinks)
  • MAY edit: docs/platform/t125_coding_standards_enforcement.md §T-125.5 Shipped note ONLY
  • DO NOT edit: registry.json, CLAUDE.md, CODING_STANDARDS.md, docs/TICKET_*.md, handoff (Cursor after slice)

═══════════════════════════════════════════════════════════════════════════════
TASK 1 — FMT-2: root .editorconfig
═══════════════════════════════════════════════════════════════════════════════

  Create repo-root .editorconfig (CODING_STANDARDS §7 FMT-2):

  [*]           → utf-8, lf, insert_final_newline, trim_trailing_whitespace
  [*.go]        → indent_style = tab
  [*.{ts,tsx,js,mjs,cjs}] → space, indent_size = 2
  [*.{json,yml,yaml,md,css}] → space, indent_size = 2

  Go formatting stays gofmt (FMT-1). EditorConfig aligns editors + checker only.

═══════════════════════════════════════════════════════════════════════════════
TASK 2 — FMT-2: editorconfig-checker in CI
═══════════════════════════════════════════════════════════════════════════════

  Install/run editorconfig-checker from REPO ROOT (covers apps/, packages/, docs/, scripts/).

  Pick ONE install path (document in Makefile comment):
    • Go: go install github.com/editorconfig-checker/editorconfig-checker/v3/cmd/editorconfig-checker@v3
    • OR npm devDep + npx (if you add a tooling dep — prefer Go to avoid new root package.json)

  Exclude: node_modules/, dist/, apps/website/frontend/src/types/contract/**,
  apps/mod/** binaries, .git/

  Fix checker violations in the same commit (trailing WS, missing final newline, wrong indent).

  Wire:
    • Makefile — run checker before or inside ci-local-frontend (from repo root)
    • ci.yml frontend job — add step (may need working-directory: repo root for this step only)

═══════════════════════════════════════════════════════════════════════════════
TASK 3 — FMT-3: Prettier config + devDeps
═══════════════════════════════════════════════════════════════════════════════

  In apps/website/frontend/:

  devDependencies:
    prettier
    eslint-config-prettier

  apps/website/frontend/.prettierrc — match EXISTING style (sample: src/lib/utils.ts):
    semi: false, singleQuote: true, tabWidth: 2, trailingComma: "all", printWidth: 100

  apps/website/frontend/.prettierignore:
    dist, node_modules, src/types/contract, package-lock.json

  Scope: **/*.{ts,tsx,css} under frontend only — NOT Go, NOT packages/tbd-schema, NOT mod .c

═══════════════════════════════════════════════════════════════════════════════
TASK 4 — FMT-3: npm scripts
═══════════════════════════════════════════════════════════════════════════════

  package.json scripts:
    "format": "prettier --write \"src/**/*.{ts,tsx,css}\" \"*.css\""
    "format:check": "prettier --check \"src/**/*.{ts,tsx,css}\" \"*.css\""

  Adjust globs if index.css lives outside src/ (include it).

═══════════════════════════════════════════════════════════════════════════════
TASK 5 — eslint-config-prettier (no eslint-plugin-prettier)
═══════════════════════════════════════════════════════════════════════════════

  eslint.config.js — import eslint-config-prettier and extend it LAST in the flat config array.
  This disables eslint formatting rules that conflict with Prettier; lint stays on TS-2..7/LOG-2/COMP-1.
  Do NOT add eslint-plugin-prettier (Prettier runs via format:check, not eslint).

═══════════════════════════════════════════════════════════════════════════════
TASK 6 — One-time Prettier pass
═══════════════════════════════════════════════════════════════════════════════

  cd apps/website/frontend && npm run format

  Large diff expected — formatting only. Verify afterward:
    npm run format:check && npm run lint && npm run build && npm test

  If any eslint-disable comments shift lines, fix so lint stays green.

═══════════════════════════════════════════════════════════════════════════════
TASK 7 — Makefile + ci.yml wiring
═══════════════════════════════════════════════════════════════════════════════

  ci-local-frontend (after npm ci, BEFORE lint):
    npm run format:check

  ci-local: ensure editorconfig-checker runs (repo root) — order should mirror ci.yml.

  ci.yml frontend job — add:
    • editorconfig-checker step (repo root)
    • npm run format:check (after npm ci, before lint)

  Do NOT break backend verify-coding-standards or schema jobs.

═══════════════════════════════════════════════════════════════════════════════
TASK 8 — Shipped note (only doc edit allowed)
═══════════════════════════════════════════════════════════════════════════════

  Append **Shipped (T-125.5):** to docs/platform/t125_coding_standards_enforcement.md §T-125.5:
    .editorconfig path; editorconfig-checker install method; prettier version;
    format:check wired ci-local + ci.yml; one-time reformat file count; make ci-local wall-clock.

═══════════════════════════════════════════════════════════════════════════════
VERIFY — ALL MUST EXIT 0
═══════════════════════════════════════════════════════════════════════════════

  editorconfig-checker                    # repo root, FMT-2
  cd apps/website/frontend && npm run format:check   # FMT-3
  cd apps/website/frontend && npm run lint && npm run build && npm test
  make ci-local                           # full gate; report wall-clock

═══════════════════════════════════════════════════════════════════════════════
COMMIT + TAG
═══════════════════════════════════════════════════════════════════════════════

  git add <explicit paths — include formatted FE files>
  git commit -m "$(cat <<'EOF'
T-125.5 platform: .editorconfig + Prettier + FMT-2/3 CI gates

Add root .editorconfig and editorconfig-checker (FMT-2); Prettier +
eslint-config-prettier + format/format:check scripts (FMT-3); one-time FE
reformat; wire into make ci-local and ci.yml frontend job.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
  git tag T-125.5

  Do NOT run ./scripts/ticket advance-slice (operator/Cursor after report).

═══════════════════════════════════════════════════════════════════════════════
RETURN REPORT (paste back to operator for Cursor doc sync → T-125.6)
═══════════════════════════════════════════════════════════════════════════════

  1. Commit hash + tag T-125.5
  2. .editorconfig: created; checker install method
  3. Prettier: version; files reformatted count
  4. eslint-config-prettier: conflicts resolved? lint still green?
  5. ci-local + ci.yml: steps added (FMT-2 + FMT-3)
  6. make ci-local wall-clock
  7. Ready for T-125.6: yes/no

═══════════════════════════════════════════════════════════════════════════════
END T-125.5 PROMPT
═══════════════════════════════════════════════════════════════════════════════
```

---

## Slice order (remaining)

| # | Slice | Executor | Focus |
|---|-------|----------|-------|
| 5 | **T-125.5** | claude-code | `.editorconfig` + Prettier ← **ACTIVE** |
| 6 | **T-125.6** | cursor-docs | Registry shipped, final hub sync |

## Return to Cursor

After T-125.5 verify → paste post-ship report → Cursor runs T-125.6 (mark T-125 shipped, matrix FMT-2/3 live, `./scripts/ticket sync`).
