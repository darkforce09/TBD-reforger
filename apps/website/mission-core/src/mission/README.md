# src/mission

Public mission interfaces. Existing compiler import paths remain available while implementations live in the AST, compiler, extensions, and validation modules.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
