# Deployment verifications

Source checks for the deploy commands. One gate lives here:
`cargo xtask verify staging-compose-paths` holds that the staging compose file,
`apps/website/docker-compose.staging.yml`, has one owner: every compose command of
`cargo xtask deploy website` names it, and `cargo xtask deploy staging`, the game server
[deployment](/documentation_v2/glossary/a_to_f.md#deployment), runs none.

## Contents

```text
tools_v2/xtask/src/verifications/deployment/
├── mod.rs                    the module tree
├── staging_compose_paths/    the compose path audit: entry point, comment stripper, compose line pins
├── staging_compose_paths.rs  the staging-compose-paths gate: what it pins, and the pinned paths and patterns
└── tests/                    unit tests for the staging compose path gate
```

## How it works

The gate reads the deploy sources as text: `WEBSITE_DEPLOY_SOURCE`,
`tools_v2/xtask/src/commands/deploy/website/remote_steps.rs`, which builds every compose command
the website deploy sends to the host, and every production source of `STAGING_DEPLOY_MODULE`, the
game server deploy: `tools_v2/xtask/src/commands/deploy/staging.rs` and each `.rs` file under
`tools_v2/xtask/src/commands/deploy/staging/` outside a `tests/` folder, the pipeline and each
payload module alike. The pipeline, `STAGING_DEPLOY_PIPELINE`
(`tools_v2/xtask/src/commands/deploy/staging/remote/fleet_deploy.rs`), must exist. After stripping `//` and `#` comments it takes every line that runs compose
(`docker compose`, `podman compose`, `docker-compose` or `podman-compose`; the compose file's own
name does not count) and requires, reporting every failure of one run:

1. the website deploy's source holds at least one compose line, and each carries a parseable `-f`
   path;
2. each of those paths equals `apps/website/docker-compose.staging.yml`;
3. no compose line names `BAD_PATH`, the same file name under `apps/website/api_v2/`;
4. the website deploy's source never runs `cd` into `apps/website/api_v2`, under any quoting;
5. no source of the game server deploy holds a compose line, each named by its path;
6. `apps/website/docker-compose.staging.yml` exists, and nothing, not even a dangling symlink,
   sits at `BAD_PATH`.

The website deploy prints each command under `--dry-run` from the same string it runs, so no
separate dry-run text needs pinning. The gate prints each failure, then
`staging-compose-paths: PASS` or `staging-compose-paths: FAIL`. A missing or unreadable source is
reported as "did not run", but the exit status stays 0 or 1, because the wave gate and CI record
pass or fail from it.

## Public surface

- `staging_compose_paths::verify_staging_compose_paths`: the gate, taking the repository root
  and returning 0 or 1.

## Boundaries

- Depends on: `verification-core` (`Pattern`, `gate`, `Verdict`, `NotRun`) and the `regex`
  crate; it reads the deploy sources as text and never runs them.
- Used by:
  - `tools_v2/xtask/src/commands/verify/dispatch.rs`, for
    `cargo xtask verify staging-compose-paths`;
  - `tools_v2/xtask/src/commands/ci/task_definitions/verification_dispatch.rs`, for the
    `verify-staging-compose-paths` step of `ci-local`;
  - the [wave](/documentation_v2/glossary/n_to_z.md#wave) gate's `VERIFY_STEPS` in
    `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs`, and the `mod-gates-hosted`
    job of `.github/workflows/ci.yml`.
- Rules:
  - a comment naming the compose file is never a compose command
    (`every_website_source_perturbation_bites`, `a_compose_command_in_the_game_server_deploy_bites`);
  - the pinned source is the one that builds the compose commands, not the step runner in front
    of it (`the_step_runner_cannot_substitute_for_the_compose_implementation`);
  - every production source of the game server deploy is audited, and a missing pipeline is
    "did not run", never a pass (`a_compose_command_in_any_game_server_deploy_source_bites`,
    `a_missing_fleet_pipeline_does_not_read_as_pass`);
  - moving the compose file or the deploy code means changing `GOOD_PATH`,
    `WEBSITE_DEPLOY_SOURCE`, `STAGING_DEPLOY_PIPELINE` or `STAGING_DEPLOY_MODULE` in
    `staging_compose_paths.rs`, and every message follows from those constants.

## Related documentation

- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — the deploy that owns the
  staging compose stack.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the staging
  host that `cargo xtask deploy staging` sets up.
