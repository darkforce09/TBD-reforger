# Deployment checks

Source checks for the deploy commands. One gate lives here:
`cargo xtask verify staging-compose-paths` holds that the staging compose file,
`deploy/compose.staging.yml`, has one owner: every compose command of
`cargo xtask deploy website` names it, and `cargo xtask deploy staging`, the game server
[deployment](/documentation/glossary/a_to_f.md#deployment), runs none.

## Contents

```text
tools/commands/deployment/src/deployment_checks/
├── staging_compose_paths/    the compose path audit: entry point, comment stripper, compose line pins
├── staging_compose_paths.rs  the staging-compose-paths gate: what it pins, and the pinned paths and patterns
└── tests/                    unit tests for the staging compose path gate
```

## How it works

The gate reads the deploy sources as text: `WEBSITE_DEPLOY_SOURCE`,
`tools/commands/deployment/src/website/remote_steps.rs`, which builds every compose command
the website deploy sends to the host, and every production source of `STAGING_DEPLOY_MODULE`, the
game server deploy: `tools/commands/deployment/src/staging.rs` and each `.rs` file under
`tools/commands/deployment/src/staging/` outside a `tests/` folder, the pipeline and each
payload module alike. The pipeline, `STAGING_DEPLOY_PIPELINE`
(`tools/commands/deployment/src/staging/remote/fleet_deploy.rs`), must exist. After stripping `//` and `#` comments it takes every line that runs compose
(`docker compose`, `podman compose`, `docker-compose` or `podman-compose`; the compose file's own
name does not count) and requires, reporting every failure of one run:

1. the website deploy's source holds at least one compose line, and each carries a parseable `-f`
   path;
2. each of those paths equals `deploy/compose.staging.yml`;
3. no compose line names any other compose file (`COMPOSE_FILE_NAME`, any folder in front): not
   the development stack `deploy/compose.dev.yml` beside it, and no compose file outside
   `deploy/`, whether after `-f`, in a second `-f` overlay or in an `--env-file=`;
4. the website deploy's source never runs `cd` into `deploy/`, under any quoting, where a relative
   `-f` sits one word from the development stack (a path through another folder of that name,
   such as `/home/deploy/`, does not count);
5. no source of the game server deploy holds a compose line, each named by its path;
6. `deploy/compose.staging.yml` exists, and nothing named like a compose file, not even a folder
   or a dangling symlink, sits in the checkout root, the folder every compose command runs from.

The website deploy prints each command under `--dry-run` from the same string it runs, so no
separate dry-run text needs pinning. The gate prints each failure, then
`staging-compose-paths: PASS` or `staging-compose-paths: FAIL`. A missing or unreadable source is
reported as "did not run", but the exit status stays 0 or 1, because the wave gate and CI record
pass or fail from it.

## Public surface

- `staging_compose_paths::verify_staging_compose_paths`: the gate, taking the repository root
  and returning 0 or 1.

## Boundaries

- Depends on: `verification_core` (`Pattern`, `gate`, `Verdict`, `NotRun`) and the `regex`
  crate; it reads the deploy sources as text and never runs them.
- Used by:
  - `tools/xtask/src/commands/verify/dispatch.rs`, for
    `cargo xtask verify staging-compose-paths`;
  - `tools/commands/ci_task_catalog/src/task_definitions/verification_dispatch.rs`, for the
    `verify-staging-compose-paths` step of `ci-local`;
  - the [wave](/documentation/glossary/n_to_z.md#wave) gate's `VERIFY_STEPS` in
    `tools/commands/platform_execution/src/wave_execution/gate.rs`, and the `mod-gates-hosted`
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

- [Website deployment](/documentation/runbooks/website_deployment.md) — the deploy that owns the
  staging compose stack.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the staging
  host that `cargo xtask deploy staging` sets up.
