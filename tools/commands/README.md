# Command crates

The libraries behind the `cargo xtask` command groups: each crate holds one group's work (the
database lane, the deploys, the staging procedures, the CI task catalog, the platform and mod
drivers and the rest), and the xtask binary keeps only the command line that parses the arguments
and calls it. A command crate depends on the foundation, check and other tool crates below it,
never on xtask.

## Contents

```text
tools/commands/
├── agent_context_guards/   `agent_context_guards`: the AI agent tool-call guard (Bash and Read rules, the session read set) and the filtered command runner that never hides a failure (`ai`)
├── api_readiness_checks/   `api_readiness_checks`: the API readiness judge: the acceptance register, evidence receipts, fingerprints, the staging recorder and the property-test seed (`verify api-readiness`, `staging`, `db test-it`)
├── ballistics_oracle_tooling/  `ballistics_oracle_tooling`: the vanilla mortar ballistics catalog, its calibration bundle and refused bundles, trimmed from a gameplay export and the ballistics oracle (`ballistics trim-export`)
├── ci_task_catalog/        `ci_task_catalog`: the CI task table and its runner, the build lane recipes, the cargo target pin and its checks, the CI workflow checks and the map asset checks (`ci`, `help`, `mk`, `verify ci-shell`)
├── database_operations/    `database_operations`: the local database lane, the verified backup, the guarded restore, the restore drill, the container layer and the database source checks (`db`, `deploy db`, `verify wiki-seeds`)
├── deployment/             `deployment`: the website and staging deploy drivers, the paths no deploy ships and the staging compose-path check (`deploy`, `verify staging-compose-paths`)
├── enfusion_mcp/           `enfusion_mcp`: the Enfusion MCP client: daemon control, tool calls, the offline selftest, Workbench NET API calls and log verdicts (`mcp`)
├── mod_operations/         `mod_operations`: the game mod's compile gate, world boot, playtest server, equipment export publication, website API client and mod wave driver (`mod`)
├── platform_execution/     `platform_execution`: the platform factory: the wave driver, slice runs and their receipts, slice worktrees and the preflight (`platform`, `ticket run`, `mod wave`)
├── remote_debugging/       `remote_debugging`: the staging server-join probes, the direct-join report, the remote console log verdict and the mission-version upload reproduction (`debug`, `repro`, `mod remote-logs`)
├── repository_relocation/  `repository_relocation`: manifest-driven moves of tracked paths, their reference rewrites and the retired-spelling verification (`refactor relocate`)
├── schema_tooling/         `schema_tooling`: the contract codegen, the contract schema gates, the ORBAT slot flattening and the font-table generator (`schema`, `gen`, `ci schema-validate`)
├── staging_procedures/     `staging_procedures`: the staging acceptance harness: the fleet, Discord and load procedures, their receipts, the host actions and the read-only commands around them (`staging`)
└── workstation_setup/      `workstation_setup`: the server profile, the Workbench link, the MCP game root, client addons and the staging host check (`setup`, `mod bootstrap-staging`)
```
