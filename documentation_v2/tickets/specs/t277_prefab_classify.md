# T-277 — 27.4% of the map catalogue is unclassified

Owner: command center. 444/1,623 prefabs fall to `fallback` (prefab-classify.json:3629); rules are first-match by
resourceNameContains, so new rules are appended. The catalogue rebuild is a separate committed artifact.

## Claude Code prompt — T-277

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-277 && pwd && git branch --show-current   # must be slice/T-277
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
packages/tbd-schema/rules/prefab-classify.json (header :1-40 + last 200 lines), packages/map-assets/everon/objects/type-inventory.json (fallback rows), docs/plans/t-277_plan.md
═══ PROBLEM ═══
444 prefabs are unclassified; vegetation and utility lanes are empty; roads are not censused.
═══ SHIPPED ═══
T-244 vehicle lane (append pattern); `_turret`/CannonWreck rules must stay ahead of vehicle rules.
═══ LANGUAGE GATE ═══
JSON rules only; Rust only for a test. No scripts.
═══ LOCKED ═══
- Append only; never reorder or edit existing rules.
- Every new rule has gameplay, spatial, render.iconKey.
- The rebuilt catalogue is NOT committed; paste the local counts.
═══ DO ═══
1. Paste the fallback count on main. 2. Group the 444 names; append rules. 3. Rebuild locally, paste counts.
4. Perturb (remove one rule) → red → restore → touch → green. 5. Gates + cargo xtask platform wave gate --slice T-277
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no edits under packages/map-assets/ or tools/.
═══ VERIFY ═══
cargo xtask ci schema-validate ; cargo xtask ci verify type-inventory ; cargo xtask ci verify map-object-enums ; cargo xtask platform wave gate --slice T-277
═══ MANUAL ═══
None; if the road census is a build.rs counter bug, report found_not_fixed.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
