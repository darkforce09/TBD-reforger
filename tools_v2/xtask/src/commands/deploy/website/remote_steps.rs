//! The commands each remote step runs over ssh, as the text the shell receives.
//!
//! Pure functions, so the dry run prints exactly what the live run executes and the tests pin the
//! parts the server depends on: the working directory, the container the checksum repair talks
//! to, and the state directory the runtime files move into.

/// The user-systemd `StateDirectory=` name the API unit declares; `%S/<this>` is where the API
/// keeps what it writes (CMS uploads), outside the checkout the rsync deletes in.
pub const STATE_DIRECTORY: &str = "tbd-website-api";

/// The Postgres container `apps/website/docker-compose.staging.yml` starts on the server.
pub const STAGING_DB_CONTAINER: &str = "tbd_staging_db";

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

/// Bring up the staging Postgres with whichever compose provider the host has.
pub fn compose_up(remote_dir: &str, postgres_port: &str) -> String {
    format!(
        "cd '{remote_dir}' &&     export TBD_POSTGRES_HOST_PORT='{postgres_port}' &&     if command -v docker >/dev/null 2>&1; then       docker compose -f apps/website/docker-compose.staging.yml up -d postgres;     else       podman compose -f apps/website/docker-compose.staging.yml up -d postgres;     fi"
    )
}

/// Build the release API binary in the remote checkout.
pub fn api_build(remote_dir: &str) -> String {
    format!(
        "cd '{remote_dir}' &&     export PATH=\"$HOME/.cargo/bin:$PATH\" &&     cargo build --release -p website-api --bin api &&     test -x target/release/api"
    )
}

/// Build the Leptos SPA into `frontend/dist`.
pub fn spa_build(remote_dir: &str) -> String {
    format!(
        "cd '{remote_dir}/apps/website/frontend' &&     export PATH=\"$HOME/.cargo/bin:$PATH\" &&     trunk build --release"
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
        "cd '{remote_dir}' &&     export PATH=\"$HOME/.cargo/bin:$PATH\" &&     TBD_DB_CONTAINER={STAGING_DB_CONTAINER} cargo xtask db repair-migration-checksum --force"
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
