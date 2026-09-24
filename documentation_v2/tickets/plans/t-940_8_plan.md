# T-940.8 — Plan

## Context
app.rs:628 routes list/create only; vehicle handlers sit in content/wiki.rs:47 and :208. After T-940.2 (core/dto.rs).

## Approach
1. Verify on main: PUT /vehicles/{id} → 405; paste the red.
2. `handlers/content/vehicles.rs` (new, in content/mod.rs): list, create, PUT, PATCH, soft DELETE.
3. app.rs routes; wiki.rs loses the vehicle handlers; core/dto.rs mirrors VehicleUpdate/VehiclePatch.
4. Perturbation: PATCH ignores an unknown field → reject test red; restore, touch, green.

## Risks
- wiki.rs shared with T-940.9 → later wave.

## Verification
- `cargo xtask db test-it`; `cargo xtask mk ci-local-leptos`
- `cargo xtask platform wave gate --slice T-940.8`
