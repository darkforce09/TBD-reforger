# src/mission/extensions

The single registry of explicitly supported authored blocks. Payload compilation and game-document compilation share this registry; unknown environment keys are not promoted.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
