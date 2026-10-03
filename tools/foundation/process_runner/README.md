# Process runner

The `process_runner` crate: how the repository tooling runs an external program without losing
the reason it stopped. A child runs in its own process group with both pipes drained, a deadline
kills the whole group, and a signal death, a timeout or a missing program comes back as a
`NotRun` cause instead of an exit code a check could read as a result. The crate also holds the
two ways a command leaves the development container: the bridge to the host's binaries and the
ssh transport to a remote host.

## Contents

```text
tools/foundation/process_runner/
├── Cargo.toml  the `process_runner` library package: `verification_core`, `libc`, `thiserror`, layout tier 1
└── src/        the command builder and runner, the pipe drains, the lookups, the host bridge, the ssh transport
```

## How it works

```text
Run::new(program).args(..).timeout(..) ──► output() / merged_output() / status() / expect_code()
                                                 │
                     setsid child, drained pipes, killpg at the deadline
                                                 │
                     Ok(Output | Merged | code)  or  Err(NotRun::{ToolAbsent, Signalled, Timeout, ToolError})

Host::detect() ──► argv with `distrobox-host-exec` only inside a container ──► Run
SshBase::from_settings(password, identity) ──► ssh_argv(base, host, remote) ──► Run (+ SSHPASS)
```

`libc` carries the two calls std does not offer: `setsid` in the child before exec, so it leads a
new session and process group, and `killpg`, so a timeout reaches every grandchild. Both sit in
`src/runner.rs` behind a reasoned `#[allow(unsafe_code)]`. `src/README.md` describes each module.

## Getting started

Run from the repository root:

```bash
cargo test -p process_runner   # the runner, the host bridge and the ssh transport; they run sh, cat and sleep
```

## Configuration

No feature. The crate reads `PATH` (in `which` and in `Host::detect`) and the container markers
`/run/.containerenv` and `/.dockerenv`; the settings that choose an `SshBase` belong to the caller
(xtask reads `TBD_SSH_PASS` and `TBD_SSH_IDENTITY_FILE` from `deploy/deploy.env`).

## Public surface

- At the crate root: `Run`, `Output`, `Merged`, `which`, `retry`, `wait_for`, `Error` and
  `Result`.
- `host_execution`: `Host`, `in_container` and `NO_BRIDGE_RC`.
- `secure_shell_transport`: `SshBase`, `ssh_argv` and `SSH_PASSWORD_VARIABLE`.
- `prelude`: the runner, the lookups, `Host`, `SshBase` and `ssh_argv`.

## Boundaries

- Depends on: `verification_core` (the `NotRun` and `Verdict` vocabulary), `libc` and
  `thiserror`.
- Used by: `xtask` — every command group and verification that runs an external program, the
  `db` group, the playtest server and the platform wave driver through `host_execution`, and the
  staging deploy and the staging harness through `secure_shell_transport`.
- Rules: tier 1 of `tools/foundation`, depending only on `verification_core` among the workspace
  crates (`foundation_crates_depend_only_on_lower_foundation_crates` in
  `tools/xtask/src/tests/tooling_dependency_boundaries.rs`; `cargo xtask verify crate-tiers`); a
  signal is never an exit code and a timeout kills the process group, which `src/tests/` holds.

## Related documentation

- [Tooling foundation crates](/tools/foundation/README.md) — the three crates and their tiers.
- [Verification core](/tools/foundation/verification_core/README.md) — the outcome vocabulary
  the runner reports in.
- [Tooling architecture](/documentation/tools/tooling_architecture.md) — how the tooling crates
  fit together.
