# src/mission/validation/validator

Ordered validation rules, explicit ambient facts, and failure fixtures that prove registered rules can fire. Missing ambient facts leave the corresponding context-dependent rules inactive.

## Boundaries

This module does not access browser APIs, graphics devices, or Leptos state. Callers provide domain data explicitly. Production Rust files remain under 500 lines; test files remain under 1,000 lines.
