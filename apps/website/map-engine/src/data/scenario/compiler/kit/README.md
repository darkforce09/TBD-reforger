# src/mission/compiler/kit

Load the canonical kit-alias registry once and resolve character/vehicle resources and faction defaults.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
