**Status:** live

# Template: runbook

**When to use:** a procedure someone runs step by step: bringing services up, deploying,
recovering, running a gate. Runbooks live in `documentation_v2/runbooks/`, one file each, named in
snake_case after the procedure; a runbook longer than 500 lines becomes a folder with a README
index. The [README standard](/documentation_v2/standards/readme_standard.md) holds the writing
rules a runbook shares with READMEs.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. Steps are
numbered; each holds one command in its own block and an `Expected:` line.

````markdown
**Status:** live

# <The procedure, in plain words: what it achieves>

<One to three sentences: what the procedure does, when to run it, and how long it takes.>

## Prerequisites

- <a tool, access, file or running service the procedure needs, and how to check it is there>

## Steps

1. <What the step does, and where to run it when that is not the repository root.>

   ```bash
   <one command>
   ```

   Expected: <the output or state that shows the step worked, quoted from a safe run or from the
   code that prints it>

## Verify

<The check that proves the whole procedure worked, as a command block with its expected result.>

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| <what the operator sees, quoted> | <why it happens, as the code shows it> | <what to do> |

## Related

- [<document title>](/documentation_v2/<path to the document>) — <what it covers>
````

Every command is checked against the code before it is written: a `cargo xtask` command against
the command tree (`link-check` fails a runbook that cites one that does not exist, fenced blocks
included), anything else against its own `--help`. An `Expected:` line comes from a safe run
(`--help`, `--dry-run`, the local database, the gate doctor) or, where no safe run exists, from the
code that prints the output; the writer never runs a remote, deploy or destructive command to check
a step. A runbook never writes a host address (the deploy host is `TBD_SSH_HOST` in
`tools_v2/xtask/deploy/deploy.env`) and never shows a secret.

## Worked sample

Written from the xtask database and build commands (`tools_v2/xtask/src/commands/db/operations.rs`,
`tools_v2/xtask/src/commands/build/recipes/shell_word.rs`), the API's boot and health probe, and a
safe `cargo xtask db --help` run; the `Expected:` lines of the compose steps are quoted from the
code that prints them. The sample sits in a fenced block, so no gate reads its links.

````markdown
**Status:** live

# Local database and API

Starts the development Postgres, runs the API on port 8080 against it and loads the development
seeds: the backend the single-page app, the gates and the tools talk to. Run every step from the
repository root; the first API build takes several minutes.

## Prerequisites

- A container runtime: `TBD_CONTAINER_RUNTIME` naming one, or `podman` or `docker` on `PATH`, or
  `distrobox-host-exec` with either on the host, tried in that order.
- The Rust toolchain.

## Steps

1. Create the API's environment file; its development values (`APP_ENV=development`, the
   `DATABASE_URL` on host port 5434, `JWT_SECRET`) work as they stand.

   ```bash
   cp apps/website/api_v2/.env.example apps/website/api_v2/.env
   ```

   Expected: no output; `apps/website/api_v2/.env` exists.

2. Start Postgres 18 in the background.

   ```bash
   cargo xtask db up
   ```

   Expected: the command prints `cd apps/website/api_v2 && podman compose up -d db` (with the
   runtime it found), then compose starts the `tbd_reforger_db` container, listening on host port
   5434.

3. Run the API. It stays in the foreground; leave it running.

   ```bash
   cargo xtask mk rust-api
   ```

   Expected: cargo builds the `api` binary into `target-dev-api/`, then the API logs
   `migrations applied` and `listening on 0.0.0.0:8080`.

4. In a second terminal, once the API has applied the migrations, load the development seeds.

   ```bash
   cargo xtask db seed
   ```

   Expected: one line per seed, in order, from
   `cd apps/website/api_v2 && podman compose exec -T db psql -U tbd -d tbd_reforger < seeds/discord_roles.sql`
   through `registry_dev.sql`, `faction_library.sql` and `vehicle_database.sql` to
   `wiki_pages.sql`, each followed by psql's command tags and no `ERROR:` line. psql carries on
   past a failed statement, so the exit code alone proves nothing; the command stops early only
   when psql itself fails, as when the database is down.

## Verify

```bash
curl -sf http://127.0.0.1:8080/healthz
```

Expected: `{"status":"ok"}`, with status 200 once the database answers and the migrations are
applied.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `db up` stops with "no container runtime" | none of `TBD_CONTAINER_RUNTIME`, podman, docker or `distrobox-host-exec` resolved | install podman or docker, or set `TBD_CONTAINER_RUNTIME` |
| the API exits at boot with `DATABASE_URL is required` or `JWT_SECRET is required` | `apps/website/api_v2/.env` is missing; the API reads it from its working directory | step 1 |
| `db seed` prints `relation "discord_roles" does not exist` yet exits 0 | the seeds ran before the API applied the migrations, and psql carries on past a failed statement | run step 3 first, then seed again |
| `curl` exits 22 on `/healthz` | the probe returned 503: the database is down or the migrations are unreadable | check step 2's container, then the API's log |

## Related

- [Local development](/documentation_v2/runbooks/local_development.md) — the whole local stack,
  the single-page app included.
````
