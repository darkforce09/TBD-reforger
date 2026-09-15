# src/mission/validation/wire_safety

Inspect authored strings and cargo limits before save/compile. Findings retain their existing wording and bounded reporting behavior.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
