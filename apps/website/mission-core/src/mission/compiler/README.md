# src/mission/compiler

Saved editor payloads, export envelopes, and canonical game-document compilation. Resource resolution uses the shared kit-alias registry.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
