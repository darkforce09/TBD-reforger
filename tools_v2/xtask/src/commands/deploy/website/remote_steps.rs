//! The commands each remote step of `cargo xtask deploy website` runs over ssh, as the text the
//! host's shell receives.
//!
//! **Role:** builds every remote step's command: the staging compose services (Postgres, and
//! Caddy with its configuration reload), the API build, the build of the staging host tools
//! ([`STAGING_HOST_TOOLS`]), the app build, the migration checksum repair, the move of runtime
//! files into the unit's state folder, and the unit restart; and [`login_shell`], the one quoted
//! word ssh carries each of them in.
//!
//! **Position:** called by [`crate::commands::deploy::website`], which prints each command under
//! `--dry-run` and sends it through ssh as a [`login_shell`] word otherwise, so the dry run shows
//! exactly what a live run executes. `tests/website/tests.rs` pins what the host depends on, and
//! `cargo xtask verify staging-compose-paths` reads this file's compose lines.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** every compose command runs from the checkout root with
//! `TBD_POSTGRES_HOST_PORT` exported, names `apps/website/docker-compose.staging.yml`, and runs
//! under `docker compose` when the host has docker, else under `podman compose`; the Caddy reload
//! names the Caddyfile at the path the compose file's `caddy` service mounts it
//! ([`caddyfile_in_container`]); every cargo build runs in the checkout and ends by proving each
//! executable it names exists under `target/release/`.

use crate::commands::deploy::remote_rust_toolchain::PUT_RUST_TOOLCHAIN_ON_PATH;
use crate::core::repository_layout;

/// The user-systemd `StateDirectory=` name the API unit declares; `%S/<this>` is where the API
/// keeps what it writes (CMS uploads, imported equipment data), outside the checkout the rsync
/// deletes in.
pub const STATE_DIRECTORY: &str = "tbd-website-api";

/// The Postgres container `apps/website/docker-compose.staging.yml` starts on the server.
pub const STAGING_DB_CONTAINER: &str = "tbd_staging_db";

/// Where the compose file's `caddy` service mounts [`repository_layout::DEPLOY_DIR`], read-only.
pub const CADDY_CONFIG_MOUNT: &str = "/etc/tbd-caddy";

/// [`repository_layout::CADDYFILE`] as the compose file's `caddy` service sees it: the file in
/// the deploy folder mounted at [`CADDY_CONFIG_MOUNT`]. The service starts Caddy on this path.
pub fn caddyfile_in_container() -> String {
    let file_name = repository_layout::CADDYFILE
        .rsplit_once('/')
        .map_or(repository_layout::CADDYFILE, |(_, name)| name);
    format!("{CADDY_CONFIG_MOUNT}/{file_name}")
}

/// How many times the web server step asks Caddy to reload, one second apart. `compose up -d`
/// returns once the container has started, which can be before Caddy's admin endpoint listens.
pub const CADDY_RELOAD_ATTEMPTS: u32 = 5;

/// The one ssh argument that runs `command` in a login shell on the host, so the deploy user's
/// profile (`PATH` with `~/.cargo/bin`, `POSTGRES_PASSWORD`) applies.
///
/// ssh joins its remote arguments with spaces into a single line for the host's own shell. Sent
/// as the three arguments `bash`, `-lc` and the command, `bash -lc` would receive only the
/// command's first word, and the rest would run in the host's plain shell, from the home folder
/// and without the profile. The command is therefore single-quoted into one word, each `'`
/// inside it written as `'\''`.
pub fn login_shell(command: &str) -> String {
    format!("bash -lc '{}'", command.replace('\'', "'\\''"))
}

/// One remote step in the order the deploy runs them: the line the operator sees, and the shell.
pub struct RemoteStep {
    pub title: String,
    pub command: String,
}

impl RemoteStep {
    pub fn new(title: &str, command: String) -> Self {
        Self {
            title: title.to_string(),
            command,
        }
    }
}

/// The start of every compose step: the checkout root as the working folder, the Postgres host
/// port the compose file interpolates, and `staging_compose`, a shell function that runs the
/// staging compose file under docker compose when the host has docker, else under podman compose.
fn compose_session(remote_dir: &str, postgres_port: &str) -> String {
    format!(
        "cd '{remote_dir}' && export TBD_POSTGRES_HOST_PORT='{postgres_port}' && \
         staging_compose() {{ if command -v docker >/dev/null 2>&1; then \
         docker compose -f apps/website/docker-compose.staging.yml \"$@\"; else \
         podman compose -f apps/website/docker-compose.staging.yml \"$@\"; fi; }}"
    )
}

/// Start the staging Postgres, or leave it running.
pub fn postgres_start(remote_dir: &str, postgres_port: &str) -> String {
    format!(
        "{} && staging_compose up -d postgres",
        compose_session(remote_dir, postgres_port)
    )
}

/// Start the Caddy web server, or leave it running, then have it reload the Caddyfile, so an
/// edited Caddyfile applies without a restart. A container that `up` has just (re)created reads
/// the current file as it starts, and the reload then finds nothing to change.
pub fn web_server_start_and_reload(remote_dir: &str, postgres_port: &str) -> String {
    format!(
        "{session} && staging_compose up -d caddy && attempt=1 && \
         until staging_compose exec -T caddy caddy reload --config {caddyfile} \
         --adapter caddyfile; do if [ \"$attempt\" -ge {CADDY_RELOAD_ATTEMPTS} ]; then exit 1; fi; \
         attempt=$((attempt + 1)); sleep 1; done",
        session = compose_session(remote_dir, postgres_port),
        caddyfile = caddyfile_in_container(),
    )
}

/// Build the release API binary in the remote checkout.
pub fn api_build(remote_dir: &str) -> String {
    format!(
        "cd '{remote_dir}' &&     {PUT_RUST_TOOLCHAIN_ON_PATH} &&     cargo build --release -p website-api --bin api &&     test -x target/release/api"
    )
}

/// One executable a staging verification run calls on the host: the cargo package that declares
/// it, and its `[[bin]]` name.
pub struct StagingHostTool {
    pub package: &'static str,
    pub executable: &'static str,
}

/// The staging host tools every website deploy builds into the checkout's `target/release/`, so
/// the staging harness finds them current on the host: `staging-fixtures`, which stages the fleet's
/// servers, credentials and load fixtures against the API's database, and
/// `acknowledgement-dropping-relay`, which `cargo xtask deploy staging` installs in front of the
/// relay instance's host agent.
pub const STAGING_HOST_TOOLS: [StagingHostTool; 2] = [
    StagingHostTool {
        package: "website-api",
        executable: "staging-fixtures",
    },
    StagingHostTool {
        package: "developer-tools",
        executable: "acknowledgement-dropping-relay",
    },
];

/// Build every [`STAGING_HOST_TOOLS`] executable in the remote checkout, one `cargo build` per
/// tool, then prove each one is an executable file.
pub fn staging_host_tools_build(remote_dir: &str) -> String {
    let builds = STAGING_HOST_TOOLS
        .iter()
        .map(|tool| {
            format!(
                "cargo build --release -p {} --bin {}",
                tool.package, tool.executable
            )
        })
        .collect::<Vec<_>>()
        .join(" && ");
    let proofs = STAGING_HOST_TOOLS
        .iter()
        .map(|tool| format!("test -x target/release/{}", tool.executable))
        .collect::<Vec<_>>()
        .join(" && ");
    format!("cd '{remote_dir}' && {PUT_RUST_TOOLCHAIN_ON_PATH} && {builds} && {proofs}")
}

/// Build the Leptos SPA into `frontend/dist`.
pub fn spa_build(remote_dir: &str) -> String {
    format!(
        "cd '{remote_dir}/apps/website/frontend' &&     {PUT_RUST_TOOLCHAIN_ON_PATH} &&     trunk build --release"
    )
}

/// Repoint `_sqlx_migrations` for every applied migration whose file changed in comments only.
///
/// Runs after the new tree is on the server and before the unit restarts: the new binary compares
/// every migration file's hash against the recorded one at boot and refuses to start on a
/// mismatch, so the repoint has to land first. The server has no `.git/` to recover the applied
/// bytes from, which is what `--force` states; the edit is proven comments-only on the developer's
/// checkout before it is committed (`tests/migrations_are_immutable.rs` pins the hashes).
pub fn migration_checksum_repair(remote_dir: &str) -> String {
    format!(
        "cd '{remote_dir}' &&     {PUT_RUST_TOOLCHAIN_ON_PATH} &&     TBD_DB_CONTAINER={STAGING_DB_CONTAINER} cargo xtask db repair-migration-checksum --force"
    )
}

/// Create the unit's state directory and move any uploads an older layout left inside the
/// checkout (`apps/website/api_v2/uploads`) into it. Idempotent: with nothing left to move it
/// only ensures the directory exists.
pub fn runtime_state_move(remote_dir: &str) -> String {
    format!(
        "state=\"${{XDG_STATE_HOME:-$HOME/.local/state}}/{STATE_DIRECTORY}\" &&     mkdir -p \"$state/uploads\" &&     for tree in uploads; do       src='{remote_dir}/apps/website/api_v2/'\"$tree\";       if [ -d \"$src\" ]; then         rsync -a --remove-source-files \"$src/\" \"$state/$tree/\" &&         find \"$src\" -depth -type d -empty -delete;       fi;     done"
    )
}

/// Restart the API unit and report whether it came up.
pub fn restart(unit: &str) -> String {
    format!("systemctl --user restart '{unit}' &&   systemctl --user is-active '{unit}'")
}
