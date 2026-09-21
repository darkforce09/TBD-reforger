# Tooling architecture

The four crates under `tools_v2/`, the invariants that keep them apart, and the structural tests
that enforce both. The module-by-module map is in
[ANALYSIS_AND_INVENTORY.md](ANALYSIS_AND_INVENTORY.md).

---

## 1. The shape

```text
tools_v2/
  verification-core/            fail-closed assertion primitives
  ticket-engine/                the ticket domain
  developer-tools/              the heavy services, behind six executables
  xtask/                        the command router and every repository verification
  enfusion_mcp_node_package/    the pinned enfusion-mcp server (not a crate)
```

One word per layer, and no layer borrows another's vocabulary. "Gate" names exactly one thing:
a repository verification that reaches a verdict. The headless browser harness is
`browser_testing`; the assertion primitives are `verification-core`; the ticket rules are
`ticket-engine::validation`.

---

## 2. Dependency direction

```text
verification-core   ticket-engine        (foundational: no workspace dependency at all)
        ^                  ^
        |                  |
   developer-tools  ------>|             (heavy services; no dependency on xtask)
        ^                  ^
        |                  |
      xtask ---------------+             (the router: depends on all three)
```

Four rules, each asserted by `xtask/src/tests/tooling_dependency_boundaries.rs`:

1. **`verification-core` and `ticket-engine` depend on no workspace member.** Either can be read,
   tested and reasoned about without the rest of the repository. The test walks every workspace
   member's package name and refuses to find any of them in either manifest.
2. **`developer-tools` does not depend on `xtask`.** The router may call the services; the
   services may not call the router.
3. **`xtask` does not depend on `website-map-engine` or `website-graphics-engine` directly.** The
   map engine reaches the router only transitively, through `developer-tools`, so the router stays
   dependency-light and a graphics change does not rebuild the whole command surface.
4. **`apps/ticketboard` consumes `ticket-engine`'s public model and operations**, never a second
   copy of them.

`cargo xtask ...` resolves through `.cargo/config.toml`:

```toml
[alias]
xtask = "run --package xtask --"
```

---

## 3. Invariants

### 3.1 One owner per path

Nothing outside a layout module spells a repository path. Three modules own them all:

| Module | Owns |
|---|---|
| `ticket-engine/src/repository.rs` | The ticket registry, its schemas, receipts, estimates and the documents generated from it, plus the `documentation` submodule for everything under the documentation tree. |
| `xtask/src/core/repository_layout.rs` | The deployment tree, the server profiles, the MCP fixtures, the worktree base, the verdict receipts and the documentation pointers the router prints. |
| `developer-tools/src/repository_layout.rs` | The contract and asset trees, the Enfusion index, the operations logs and the node package directory. |

`xtask` and `ticketboard` resolve ticket paths from `ticket-engine`; neither declares its own.
Checkout-root discovery is implemented twice on purpose — once in each foundational crate — because
neither may depend on the other, and `xtask` delegates to `ticket-engine`'s copy.

### 3.2 One outcome vocabulary

Every verification returns a `verification_core::Verdict`. A check whose input is missing, whose
search tool is absent, or whose corpus will not load reports did-not-run, never pass. An empty
input is never a clean tree. This is what makes a green run evidence.

### 3.3 Structural limits

- Production files stay below 500 lines; test files below 1,000; `xtask/src/main.rs` below 150.
  The browser smoke scenarios are capped at 450 and the executables' `src/bin` shims at 250.
- Unit tests live in sibling files declared with `#[cfg(test)] #[path = "tests/..."] mod tests;`.
  Inline `mod tests {}` is rejected by a syn-based walk that also descends into macro bodies and
  function-local modules.
- No crate under `tools_v2/` carries a file-size exemption in `.coding-standards-allowlist.yaml`.

### 3.4 Present-tense prose

Comments, doc comments, help strings and READMEs describe what the code does now and why: the
invariant, the measurement, the refusal reason. They carry no ticket identifiers, no names of
files that no longer exist, no deleted script names and no narrative about how the code got here.
Commit history owns history.

---

## 4. The verification surface

`cargo xtask verify <name>` runs one check; `cargo xtask ci ci-local` runs the composite every CI
job runs. Each verification is spelled after the module that implements it — the module file name
with underscores written as hyphens — and its entry function is `verify_` plus that module name.
The language bans (`no-python`, `no-shell`, `no-node`, `ci-shell`) and `file-length` keep the names
their CI jobs use.

Two checks close the loop on the surface itself:

- `verify ci-schema-parity` compares the verification surface against the CI task index by reading
  the source text of both, so a check that exists but is wired into nothing fails.
- `schema specification-consistency` compares the specification documents against the live task
  names, so a document citing a command that does not exist fails.

The structural tests in `xtask/src/tests/` enforce the rest: dependency direction, file limits,
test placement, path ownership, and the prose rules.

---

## 5. Data beside the crates

`enfusion_mcp_node_package/` sits outside every crate root deliberately. `verify file-length`
walks whole crate directories, and the structural walker refuses symlinks under `src/`, so a
vendored `.rs` inside an installed `node_modules` would become a subject of both. Outside a crate
root it is neither.

The remaining data directories — `xtask/deploy/`, `xtask/dedicated_server_profiles/`,
`xtask/fixtures/mcp/`, `developer-tools/fixtures/`, `developer-tools/test_fixtures/`,
`ticket-engine/tests/fixtures/` — sit beside the code that reads them, and a layout module names
each location once.
