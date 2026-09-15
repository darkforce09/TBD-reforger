# src/mission/validation

Mission validation and wire-safety checks shared by browser and API.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
