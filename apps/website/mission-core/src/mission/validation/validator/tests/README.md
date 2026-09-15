# src/mission/validation/validator/tests

Regression tests and shared fixtures for the enclosing domain module. Test bodies, assertions, and literal values are preserved; source-reading assertions follow the current implementation files.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
