//! The `--help` block `cargo xtask deploy website` prints.
//!
//! **Role:** the `--help` text of `cargo xtask deploy website`.
//! **Position:** a child of [`crate::website`], which prints [`usage`] for `--help` and usage
//! errors.
//! **Signals & state:** none; a pure function.
//! **Invariants:** the deploy file the text names is the constant the command reads.
//!
//! Built rather than declared, so the deploy file it names is the same constant the command
//! reads, and the two can never describe different paths.

/// The usage text: what the deploy does, its flags, and every `deploy.env` key it reads.
pub(crate) fn usage() -> String {
    format!(
        "\
Usage: cargo xtask deploy website [--dry-run] [--help]

  Rsync the monorepo to TBD_REMOTE_DIR, bring up staging Postgres (compose),
  build the release API binary, the staging host tools (staging-fixtures,
  acknowledgement-dropping-relay) and the Leptos SPA on the server, start the
  staging Caddy (compose) and reload its Caddyfile, and restart the user-systemd
  API unit. The rsync runs only once the host holds a readable
  crates/api/api_server/.env in TBD_REMOTE_DIR.

  --dry-run   Print the plan (probes/rsync/ssh/compose/builds/web server/
              checksum-repair/restart) without executing.
  -h, --help  Show this help.

Settings ({deploy_env}, or the file DEPLOY_ENV names):
  The file decides every key it sets (an empty value counts as unset); the process
  environment fills only keys the file never sets.
  TBD_SSH_HOST              required, user@host (e.g. sam@dooley.local)
  TBD_REMOTE_DIR            optional (default /home/<user>/tbd/repo; must stay under
                            /home/<user>/tbd/ — never prairielearn)
  TBD_SSH_PASS              optional (sshpass)
  TBD_SSH_IDENTITY_FILE     optional (ssh -i)
  TBD_POSTGRES_HOST_PORT    optional (default 5432) — compose host port
  TBD_WEBSITE_SYSTEMD_UNIT  optional (default tbd-website-api.service)
  TBD_SKIP_COMPOSE          set to 1 to skip both compose steps (Postgres, Caddy)
  TBD_SKIP_SPA_BUILD        set to 1 to skip remote trunk build (Caddy still
                            starts and serves the dist already on the server)
  TBD_SKIP_API_BUILD        set to 1 to skip both remote cargo builds (the API
                            and the staging host tools)

Smoke (no SSH):
  cargo xtask deploy website --help
  cargo xtask deploy website --dry-run   # needs a filled deploy.env

Compose validate (local):
  docker compose -f deploy/compose.staging.yml config
  # on hosts with Podman only:
  podman compose -f deploy/compose.staging.yml config
",
        deploy_env = repository_layout::DEPLOY_ENV
    )
}
