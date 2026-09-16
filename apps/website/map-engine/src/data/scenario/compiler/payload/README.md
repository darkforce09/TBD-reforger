# src/mission/compiler/payload

Project document maps into ordered editor payloads, construct export envelopes, and serialize version bodies without an additional payload copy.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
