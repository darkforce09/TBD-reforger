# Language Bans & File Hygiene (`verifications/language_bans`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Enforces repository-wide language eradication policies and file size limits.

---

## Verifications

- **`no_shell_scripts.rs`** (formerly `shell_free.rs`): LANG-1: Guarantees zero tracked shell scripts (`.sh`, `.bash`) exist in the repository.
- **`no_python_scripts.rs`** (formerly `gate_no_python.rs`): LANG-2: Guarantees zero tracked Python scripts (`.py`) exist in the repository.
- **`no_node_scripts.rs`** (formerly `node_free.rs`): LANG-3: Guarantees zero Node/npm scripts (`.js`, `.mjs`, `package.json`) exist in the repository.
- **`file_length_limits.rs`**: Enforces **Law 7**:
  - Production files must stay strictly under **500 LOC**.
  - Test files must stay strictly under **1,000 LOC**.
