# src/mission/extensions/tactical_graphics

tactical graphics parsing, validation, and domain projections. The Rust module is the source of truth for supported fields and behavior.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
