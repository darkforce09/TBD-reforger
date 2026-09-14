# Dedicated Test Suite (`tests`)

This directory isolates unit and integration tests from production code files.

---

## The Zero-Pollution Policy
In `editor/v1`, over 30,000 lines of code were `#[cfg(test)] mod tests` blocks appended to the bottom of production files.

In `editor/v2`:
- **NO test code is permitted inside production `.rs` files.**
- Feature unit tests live in adjacent test files or under this `tests/` tree.
- Complex integration and UI regression tests live here.
