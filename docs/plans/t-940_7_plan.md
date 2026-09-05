# T-940.7 — Plan

## Context
handlers/mod.rs:42-45 PageParams defaults 20 (max 100); admin.rs:60-61 returns a bare array; pages/admin/personnel.rs shows
one page. handlers/mod.rs is shared and untouched.

## Approach
1. Verify on main: 25 users → response has no total; paste the red.
2. admin.rs list_users → {items, page, per_page, total}.
3. personnel.rs: previous/next + per-page selector, query in the URL.
4. Perturbation: total = items.len() → 25-user test red; restore, touch, green.

## Risks
- admin.rs shared with T-940.11 → later wave.

## Verification
- `cargo xtask db test-it`; `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-940.7`
