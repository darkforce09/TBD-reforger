# T-304 — Plan

## Context
T-206 found two scanner defects: weapons hang ItemPhysAttributes off a *StorageComponent so the pass skips them
(0/107 weights), and the class-keyed `foreach` (TBD_RegistryScan.c:324-432) resolves overrides in hash order,
leaving 32 rows at Item_Base.et's 0.01 kg — values that would poison any cargo budget.

## Approach
1. Read ReadPhysAttrsPass in `apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_RegistryScan.c`; paste the 32 rows from the registry JSON.
2. Read ItemPhysAttributes regardless of the storage flag; build an ordered list of buckets by derivation depth
   (most-derived first) and iterate that instead of the map.
3. Mirror the change into `apps/mod/tbd-export/Scripts/WorkbenchGame/TBD_RegistryScan.c`; diff the two files empty.
4. `cargo xtask mod compile`; MANUAL: operator re-runs the scan and diffs the 32 rows.
## Risks
- Derivation depth is not exposed directly; derive it by walking the class ancestry once and caching. Fallback:
  explicit ordering by the prefab's own component list order.

## Verification
- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-304`
- MANUAL: registry diff shows 107/107 weapon weights and the 32 rows corrected.
