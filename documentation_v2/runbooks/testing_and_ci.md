# Testing & Quality Gates Runbook

This runbook details local CI verification, headless Chrome CDP automated testing, and gate assertion rules.

---

## 1. Local CI Suite (`cargo xtask ci ci-local`)

Run the full local quality gate suite before pushing commits to `main`:

```bash
cargo xtask ci ci-local
```

### Checks Enforced:
1. **Formatting & Lints**: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`.
2. **Coding Standards & File Limits**:
   - Production files under 500 lines of code.
   - Test files under 1000 lines of code.
   - No inline test modules (tests reside in sibling files declared via `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`).
3. **Language Bans**: Fail-closed verifications ensuring zero Python scripts, zero shell scripts, and zero Node.js scripts in production runtime pipelines.
4. **Backend Tests**: Rust unit and integration tests across `apps/website/api_v2/`.
5. **Frontend WASM Build**: Compiles `apps/website/frontend/` to `wasm32-unknown-unknown` and asserts clean compilation without warnings.
6. **Contract Parity**: Verifies Rust backend models and frontend DTOs strictly match schemas in `packages/tbd-schema/`.

---

## 2. Headless Chrome CDP Gates (`cargo xtask mk leptos-gates`)

The repository includes a headless Chrome DevTools Protocol (CDP) test harness (`tools_v2/developer-tools/src/bin/gate.rs`) for end-to-end frontend verification.

```bash
cargo xtask mk leptos-gates
```

### Gate Doctor Preflight (`doctor.rs`):
Before launching browser tests, Gate Doctor executes preflight checks:
- Verifies Google Chrome binary availability (prefers full `google-chrome` over `chrome-headless-shell` to avoid Skia font fallback aborts).
- Isolates fontconfig cache to `$TMPDIR/tbd-gate-cache`.
- Asserts minimum available memory.
- Kills any orphaned zombie Chrome instances from prior runs.

### 21 Editor Smoke Tests:
The harness programmatically interacts with the Scenario Creator canvas:
- `save-dialog-rect`: Verifies modal bounding rectangles and alignment.
- `entrance-motion-rect`: Validates opacity-only entrance animations (no layout shift).
- `hydrate`: Confirms WASM client hydration without DOM mismatch errors.
- `mutations`: Tests entity placement, property modification, and undo/redo operations.
- `hillshade` & `fullmap`: Validates WebGPU terrain canvas rendering and tile loading.

### V-Suite Frozen DOM Oracle:
- Normalizes DOM trees across all platform routes.
- Compares rendered DOM structures against golden fixtures in `apps/website/frontend/tests/fixtures/api/`.
