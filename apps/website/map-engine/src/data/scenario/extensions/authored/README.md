# src/mission/extensions/authored

Register authored blocks, validate their carriers, and serialize supported extension values.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
