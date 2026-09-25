# Mission upload reproduction commands

The `cargo xtask repro` group: a scripted reproduction of large
[mission](/documentation_v2/glossary.md#mission) version uploads against a local
[API](/documentation_v2/glossary.md#api), and the two helpers it is built from. Developers run it
by hand when the version save of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) fails on a large document.

## Contents

```text
tools_v2/xtask/src/commands/reproduction/
├── cli.rs                     the `ReproCmd` clap enum: three commands
├── dispatch.rs                routes each `ReproCmd` to its function
├── fixtures.rs                `mission-id` and `mission-version-body`: the id reader, the padded body
├── mission_version_upload.rs  `mission-upload`: dev login, mission create, one upload per size
├── mod.rs                     the module tree
└── tests/                     unit tests for the token extraction from the dev-login redirect
```

## How it works

`tools_v2/xtask/src/cli/dispatch.rs` passes the parsed `ReproCmd` to `dispatch::run`.
`mission-upload` calls the two helpers in process and every HTTP request through curl:

```text
mission-upload   (API, ROLE and SIZES_MB from the environment)
  ├─ GET  $API/auth/dev-login?role=$ROLE      read access_token from the redirect; none ─▶ exit 1
  ├─ POST $API/missions                        a pve_coop Everon mission; its id via mission-id
  ├─ for each size in SIZES_MB (default "2 10 140"), version 1.<n>.0:
  │     mission-version-body ─▶ body_<size>mb.json in a temp folder
  │     POST $API/missions/<id>/versions       print HTTP code, bytes sent and time
  └─ point at the `cargo xtask mk rust-api` log line for the upload
```

`API` defaults to `http://localhost:8080/api/v1` and `ROLE` to `mission_maker`; the API must run
with `APP_ENV=development` so the [dev login](/documentation_v2/glossary.md#dev-login) answers. A
failed upload prints `curl exit <n>` and the loop goes on, because a connection the server cuts
is the result being reproduced; the version route's body limit comes from
`mission_version_body_limit` in `apps/website/api_v2/src/core/configuration/mod.rs`. The temp
folder is removed on exit.

## Commands

Each runs as `cargo xtask repro <command>`; a clap usage error exits 2.

### mission-upload

- Synopsis: `cargo xtask repro mission-upload`
- Does: logs in, creates a mission and posts one version body per size, printing each answer.
  Needs `cargo xtask db up` and `cargo xtask mk rust-api` running.
- Exit codes: 0 every size was tried; 1 no token from the dev login, a create answer without an
  `id`, or a size that is not an integer; the first curl's own code when the login or create
  request fails (7 when the API is down); 127 curl is not installed.
- Example: `SIZES_MB="2 10" cargo xtask repro mission-upload`

### mission-version-body

- Synopsis: `cargo xtask repro mission-version-body --out <path> --mb <n> --semver <version>`
- Does: writes a version body with `semver`, an empty `payload.spawns` and `editor_notes` padded
  with `<n>` MiB of `x`.
- Exit codes: 0 written; 1 `--mb 0` or a write failure.
- Example: `cargo xtask repro mission-version-body --out /tmp/body.json --mb 10 --semver 1.0.0`

### mission-id

- Synopsis: `cargo xtask repro mission-id` (JSON on stdin)
- Does: prints the `id` field of a mission-create response.
- Exit codes: 0 printed; 1 the input is not JSON or has no string `id`.
- Example: `curl -s … | cargo xtask repro mission-id`

## Boundaries

- Depends on: `verification_core::proc` for curl; `serde_json` and `regex`; curl; the website API
  with its dev login, `POST /api/v1/missions` and `POST /api/v1/missions/{id}/versions`.
- Used by: `tools_v2/xtask/src/cli/dispatch.rs`; people reproducing an upload failure.
- Rules: the token comes from the `access_token` of the dev-login redirect
  (`extract_token_matches_sed` in `tests/mission_version_upload/tests.rs`); the upload loop never
  stops on a failed upload.

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — the database and the API
  this reproduction runs against.
