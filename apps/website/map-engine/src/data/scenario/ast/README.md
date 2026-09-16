# src/mission/ast

Authored input records and compiled entity/scenario wire structures. Faction hierarchy projection lives in `factions/`. Slot and vehicle structures are grouped in `entities.rs`; no separate duplicate model is introduced.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
