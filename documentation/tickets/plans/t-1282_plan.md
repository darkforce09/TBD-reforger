**Status:** live

# T-1282 — Plan

## Context

After stage S13 the folder apps holds only the Enfusion mod: the three addons tbd-framework,
tbd-export and tbd-emcp, the References lane README and two READMEs. The operator wants the mod at a
top-level `mod/` folder and the apps folder gone. The mod's documentation mirror follows to
`documentation/mod/`.

This is stage S14 of the workspace restructure. Its run folder (orchestration prompts, agent
reports, gate logs) is outside the repository, in the detached worktree's `target/restructure-s14/`.

## Approach

### Decisions

- O1–O4, operator:
  - the mod moves to `mod/` (`mod/tbd-framework`, `mod/tbd-export`, `mod/tbd-emcp`,
    `mod/References`), and the apps folder is deleted;
  - addon folder names, addon GUIDs, `.gproj` contents, `{GUID}` resource paths and `.rdb` files
    stay unchanged;
  - Workbench stays closed during the stage; the operator re-opens it on the new `.gproj` paths;
  - T-1281 (S13) ships in this stage, stamped with its landing commit `988fb3722`.
- C1–C6, coordinator (question round 1):
  - C1: the relocation tool lets a retired `path` spelling be legal again only where a later
    committed manifest names it as a `path` row's `to`; the documentation mirror then moves to
    `documentation/mod/` by the plain mirror rule;
  - C2: the two folder READMEs of the apps folders merge into the mod READMEs; link-check judges
    `apps` as a retired top-level root beside `docs`;
  - C3: one constant `ENFUSION_MOD_DIR = "mod"`; the crate-tier manifest sweep covers `crates`,
    `mod` and `tools`; `mod` is a citation scan root and a readiness fingerprint input;
  - C4: frozen records stay as written (closed tickets, run artifacts, archive, relocation
    manifests, SQL migration comments, commit-pinned permalinks);
  - C5: no migration code; the operator steps below;
  - C6: waves W0–W5.

### Waves

| Wave | Agents | Content |
|---|---|---|
| W0 | orchestrator | this ticket, T-1281 shipped, baselines |
| W1 | T1 | relocation tool: revival of a retired path spelling by a later manifest |
| W2 | M1, orchestrator | README merges, manifest `s14_w02_mod_top_level.tsv`, apply; `ENFUSION_MOD_DIR` |
| W3 | L1, O1, C1, parallel | laws and gate roots; operational paths; config, CI and agent rules |
| W4 | R1 | CLAUDE.md, standards, architecture, READMEs, runbooks, old-spelling audit |
| W5 | closing fixes, orchestrator | routed findings, stage gate, landing |

### Execution record

| Wave | Result |
|---|---|
| W0 | ticket opened; T-1281 shipped and stamped `988fb3722`; baselines: relocate --verify 2147 checks, workspace laws 5/5 PASS, file-length 5338 files 0 violations, Enfusion comment card 0, documentation gates OK, ticket check --strict OK |
| W1 T1 | the relocation tool revives a retired `path` spelling where a later manifest's `path` row `to` equals it or lies below it; seven scenarios (move back, no revival, retired again, chronology, folder above revives nothing, uncommitted manifest, partial revival), the oracle mirrored; 70 tests twice; two perturbations red then restored; 0 revivals on the real registry |
| W2 M1 | both folder READMEs merged into the mod READMEs; manifest `s14_w02_mod_top_level.tsv` (2 path rows); 1467 files moved, 916 rewritten, 0 unresolved; relocate --verify 2149 checks with `documentation/mod` revived; Enfusion comment card 0; no web path changed |
| W2b | `ENFUSION_MOD_DIR = "mod"` replaces the applications constant; the relocation crate's test modules gathered in `tests/mod.rs` (the crate root back under the 80-line anatomy cap); two archive link destinations repointed |
| W3 L1, O1, C1 | every mod root read from `ENFUSION_MOD_DIR` (pinned script roots, the Enfusion comment card, the citation scan, the readiness fingerprint, the mod, deploy and setup commands); the stray-manifest sweep reads the whole checkout; link-check, markdown-placement and readme-coverage judge the retired top-level folders `docs` and `apps`; `.gitignore`, the editorconfig exclude, `.dockerignore`, workflows and `.cursor` rules on `mod/`; 13 perturbations red then restored; wave gate: fmt, workspace clippy, workspace laws 5/5, file-length, Enfusion comment card, relocate --verify 2149, ticket check |
| W4 G1, R1 | the addon folders defined once in `repository_layout::enfusion_mod_folders`; the staging boot test fails when the framework's project file is missing; CLAUDE.md, the standards (mirror rule without an exception), the workspace layout, READMEs and the workstation runbook on `mod/`; old-spelling audit: 4021 remaining hits, each with a reason, 0 unexplained |
| Stage gate | `cargo xtask mod compile` clean (392 framework scripts, 0 warnings); `cargo xtask mod world-boot` PASS (roll-call clean); `cargo xtask ci ci-local` every step green: the API integration suite ran 154 binaries (1508 tests, 0 failed), workspace-member tests 112 packages (4858 tests), ci-local-leptos 2046 tests and the Trunk release build; `cargo xtask deploy website --dry-run` lists the `mod/References/` and `mod/.local-test-profile/` excludes, `cargo xtask deploy staging --dry-run` exit 0 (its rsync excludes and the addon link are pinned by the deployment tests) |
| Perturbations | the agents' proofs (T1 2, L1 12, O1 1, G1 1); the orchestrator re-ran four: the reviving manifest row removed (relocate --verify red), a backticked retired-folder path planted in `mod/README.md` (link-check red), a stray manifest under the retired folder (crate-tier rule 1 red), a 501-line mod script (file-length red); each restored byte-equal |

### Amendments

| Id | Source | Change |
|---|---|---|
| A1 | coordinator, plan approval | the rule that an application crate under the retired apps folder fails still holds once the folder is gone; L1 proves it with a planted top-level apps/x/Cargo.toml member |
| A2 | coordinator, W2 | accepted: the three live links and two archive link destinations to the deleted folder READMEs repointed by hand (link-check requires them) |
| A3 | coordinator, W2 | accepted: `.ai/artifacts/` rewritten by the tool's standard treatment, as in S13; C4's frozen default no longer covers artifacts |
| A4 | coordinator, W3 | the stray-manifest sweep should read tracked manifests instead of walking the checkout, unless the crate cannot list tracked files; kept as a walk (F-S14-G1-03) |

### Findings

| Id | Where | Triage | Action |
|---|---|---|---|
| F-S14-T1-01 | relocation tool | CLOSE | a revival by a `to` above the retired spelling revived 14 real rows of same-commit manifest pairs; the rule is the exact one (a `to` at or below the spelling), 0 real revivals |
| F-S14-M1-01 | relocation tool | NOTE (tool ticket) | a `text` row takes only identifier-like tokens, so a path-like link such as the folder README link cannot be rewritten by a row |
| F-S14-M1-03 | `.ai/artifacts/` | CLOSE (A3) | the tool rewrites run artifacts as live files |
| F-S14-M1-04 | relocation crate root | FIX (W2b) | 84 lines after W1; test modules declared in `tests/mod.rs` |
| F-S14-L1-01 | crate-tier law | FIX (W3) | with the applications folder gone a stray manifest there went unjudged; the sweep reads the whole checkout (hidden, test, fixture, build and npm folders skipped); the A1 perturbations turn rules 1 and 2 red |
| F-S14-L1-02 | documentation gates | FIX (W3) | one list of retired top-level folders (`docs`, `apps`) judged by link-check, markdown-placement and readme-coverage |
| F-S14-O1-01 | addon folder names | FIX (W4, G1) | defined once in `repository_layout::enfusion_mod_folders` |
| F-S14-O1-02 | staging boot test | FIX (W4, G1) | it passed with nothing checked when the framework's project file was absent; it now fails |
| F-S14-G1-03 | crate-tier law sweep | NOTE (A4) | the sweep stays a walk: the only tracked-file listing is in the tier-2 `documentation_checks` (`tracked_tree.rs`), `process_runner` shares the law crate's tier 1 so rule 4 refuses the edge, the crate reads files only, and the law's tests run on temporary checkouts without git |
| F-S14-G1-04 | addon folder literals | NOTE | 52 literals in crates outside G1's ownership (`repository_laws` `source_roots.rs` among them, at the same tier as `repository_layout`) |
| F-S14-R1-05 | open tickets and the wave lock | NOTE (T-1280) | they still name paths that earlier stages retired |
| F-S14-02 | equipment vehicle export evidence | CLOSE | its source-code manifest lists the mod's files at their new paths with unchanged digests; 14 of its 21 digests and one listed file were already stale at `988fb3722`, so the move changes nothing it proves |
| F-S14-01 | readiness fingerprint | NOTE | the source digest changes with the mod's paths (beside F-S13-T2-03): the readiness receipts are re-recorded |

## Operator steps

1. In the main checkout, move the four ignored upstream reference lanes from the old mod folder
   to `mod/References/` (`arma3_mission_frameworks`, `crf_framework`, `playable_selector`,
   `vanilla_reference`), then remove the empty apps folder.
2. Re-run `cargo xtask setup client-addons` so the client addon link points at
   `mod/tbd-framework`.
3. Open Workbench on `mod/tbd-export/addon.gproj` (or `mod/tbd-framework/addon.gproj`).
4. The next `cargo xtask deploy staging` re-points each instance's `addons/tbd-framework` link
   after its rsync; nothing else changes on the hosts.

## Risks

- A `text` row on a README link rewrites an unrelated spelling: rows are scoped and proved with
  `git grep`.
- The revival rule weakens the retired-spelling registry: scenarios prove that a spelling with no
  reviving manifest, or one retired again later, still fails.
- `mod` is a short word: no `text` row names it; the `path` row matches whole segments only.

## Verification

- `cargo xtask refactor relocate --verify`;
- `cargo xtask ci verify-workspace-laws`, `cargo xtask verify file-length`,
  `cargo xtask verify enfusion-comments`;
- `cargo xtask ci verify-documentation`, `cargo xtask ticket check --strict`;
- `cargo xtask mod compile`, `cargo xtask mod world-boot`;
- `cargo xtask ci ci-local`, with the API integration binaries counted;
- `cargo xtask deploy staging --dry-run` and `cargo xtask deploy website --dry-run`;
- perturbation proofs of every changed law root.
