# src/mission/compiler/flatten

Compile authored factions, squads, slots, vehicles, zones, and extensions into game documents. Diagnostics and substitution reports come from the same traversal as the emitted bytes.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
