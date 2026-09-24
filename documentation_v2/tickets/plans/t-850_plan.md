# T-850 — Plan

## Context

T-801 eye-pass: the squad tether does not follow a drag on auto-grouped same-squad units. T-801 shipped `pack_squad_link_drag_preview` (`squad_links.rs:7`) and `bind_squad_link_preview` (`select_tool.rs:299`); the verifier could not re-prove mid-drag with the 4 px probe. May depend on clean membership (T-848).

## Approach

1. Reproduce on the release build with auto-grouped members (single and two-selected drag); paste the probe numbers.
2. If SQUAD_LINKS misses that membership shape: fix `bind_squad_link_preview`/`pack_squad_link_drag_preview`; if membership is wrong, re-test after T-848 and say so.

## Risks

- Class-R packer pins: the byte-parity test scrubs its own source — keep the packer change minimal.

## Verification

- `cargo test -p map-engine-core --all-features` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-850`
