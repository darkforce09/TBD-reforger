//! The `--help` block `cargo xtask deploy website` prints.
//!
//! Built rather than declared, so the deploy file it names is the same constant the command
//! reads, and the two can never describe different paths.

use crate::core::repository_layout;

pub fn usage() -> String {
    format!(
        "\
Usage: cargo xtask deploy website [--dry-run] [--help]

  Rsync the monorepo to TBD_REMOTE_DIR, bring up staging Postgres (compose),
  build the release API binary + Leptos SPA on the server, restart the
  user-systemd API unit, and print Caddy reload hints.

  --dry-run   Print the plan (rsync/ssh/compose/build/checksum-repair/state-dir/restart)
              without executing.
  -h, --help  Show this help.

Environment ({deploy_env}):
  TBD_SSH_HOST              required (e.g. sam@192.168.0.140)
  TBD_REMOTE_DIR            required (must be under /home/sam/tbd/ — never prairielearn)
  TBD_SSH_PASS              optional (sshpass)
  TBD_SSH_IDENTITY_FILE     optional (ssh -i)
  TBD_POSTGRES_HOST_PORT    optional (default 5432) — compose host port
  TBD_WEBSITE_SYSTEMD_UNIT  optional (default tbd-website-api.service)
  TBD_SKIP_COMPOSE          set to 1 to skip docker compose postgres up
  TBD_SKIP_SPA_BUILD        set to 1 to skip remote trunk build
  TBD_SKIP_API_BUILD        set to 1 to skip remote cargo build

Smoke (no SSH):
  cargo xtask deploy website --help
  cargo xtask deploy website --dry-run   # needs a filled deploy.env

Compose validate (local):
  docker compose -f apps/website/docker-compose.staging.yml config
  # on hosts with Podman only:
  podman compose -f apps/website/docker-compose.staging.yml config
",
        deploy_env = repository_layout::DEPLOY_ENV
    )
}
