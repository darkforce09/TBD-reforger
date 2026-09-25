# Deployment verifications

Source checks for the deploy commands. One gate lives here:
`cargo xtask verify staging-compose-paths` holds that the staging
[deployment](/documentation_v2/glossary.md#deployment) starts its API and Postgres stack from
`apps/website/docker-compose.staging.yml`, in the dry-run plan and on the live remote shell alike.

## Contents

```text
tools_v2/xtask/src/verifications/deployment/
├── mod.rs                    the module tree
├── staging_compose_paths/    the compose path audit: entry point, comment stripper, compose line pins
├── staging_compose_paths.rs  the staging-compose-paths gate: what it pins, and the pinned paths and keys
└── tests/                    unit tests for the staging compose path gate
```

## How it works

The gate reads `DEPLOY_SOURCE`, `tools_v2/xtask/src/commands/deploy/staging/remote/ssh_argv.rs`,
the file that prints the `cargo xtask deploy staging` plan and builds its ssh commands. After
stripping `//` and `#` comments it finds two compose lines: the one printed with the `[dry-run]`
prefix and the live one. It then requires, and reports every failure of one run:

1. both lines exist and each carries a parseable `-f` path;
2. both paths equal `apps/website/docker-compose.staging.yml`, and each other;
3. neither line names `BAD_PATH`, the same file name under `apps/website/api_v2/`;
4. the source never runs `cd` into `$TBD_REMOTE_DIR/apps/website/api_v2`, single- or
   double-quoted;
5. `apps/website/docker-compose.staging.yml` exists, and nothing, not even a dangling symlink,
   sits at `BAD_PATH`.

It prints each failure, then `staging-compose-paths: PASS` or `staging-compose-paths: FAIL`. A
missing or unreadable input is reported as "did not run", but the exit status stays 0 or 1,
because the wave gate and CI record pass or fail from it.

## Public surface

- `staging_compose_paths::verify_staging_compose_paths`: the gate, taking the repository root
  and returning 0 or 1.

## Boundaries

- Depends on: `verification-core` (`Pattern`, `gate`, `Verdict`, `NotRun`) and the `regex`
  crate; it reads the deploy source as text and never runs it.
- Used by:
  - `tools_v2/xtask/src/commands/verify/dispatch.rs`, for
    `cargo xtask verify staging-compose-paths`;
  - `tools_v2/xtask/src/commands/ci/task_definitions/verification_dispatch.rs`, for the
    `verify-staging-compose-paths` step of `ci-local`;
  - the [wave](/documentation_v2/glossary.md#wave) gate's `VERIFY_STEPS` in
    `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs`, and the `mod-gates-hosted`
    job of `.github/workflows/ci.yml`.
- Rules:
  - the dry-run plan and the live command must name the same compose file, so a rehearsal can
    never differ from the deploy (`every_source_perturbation_bites`);
  - the pinned source is the implementation, not the transport facade in front of it
    (`a_transport_facade_cannot_substitute_for_the_compose_implementation`);
  - moving the compose file or the deploy code means changing `GOOD_PATH` or `DEPLOY_SOURCE`
    in `staging_compose_paths.rs`, and every message follows from those constants.

## Related documentation

- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the staging
  host that `cargo xtask deploy staging` sets up.
