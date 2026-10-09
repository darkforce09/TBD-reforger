//! The remote-shell payloads of one fleet instance, and of the checks that run before any change.
//!
//! **Role:** builds the exact text each remote `bash -s` reads on stdin: the website API probe, the
//! secret file check, the instance files (folders, RCON password, profile, server config) and the
//! V2–V4 game-runtime smoke; the profile's commands ([`instance_profile_commands`]) are the one
//! writer of an instance's `TBD_BackendConfig.json`.
//!
//! **Position:** called by the deploy pipeline in `super::remote`, and for the profile's commands by
//! the staging harness's `mod_runtime` credential promotion (the xtask binary's
//! `staging fleet` procedure); pure: every payload is a string built from its inputs.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** no payload carries a secret and no payload prints one. Machine credentials, the
//! RCON password and the join password are read on the host from mode-600 files into shell
//! variables, reach `setup server-profile` through its environment and `curl` through stdin
//! (`-H @-`), and are substituted into the server config with shell parameter expansion, so no
//! process's argument vector holds one. A template here is a raw string whose `@NAME@` markers are
//! replaced before sending; everything else is for the remote shell.

use super::config::Env;
use super::fleet_instances::{
    FLEET_ROOT_UNDER_HOME, FleetInstance, HOST_AGENT_CREDENTIAL_FILE, JOIN_PASSWORD_FILE,
    MOD_RUNTIME_CREDENTIAL_FILE, RCON_PASSWORD_FILE,
};
use super::fleet_server_config::{JOIN_PASSWORD_PLACEHOLDER, RCON_PASSWORD_PLACEHOLDER};
use repository_layout::enfusion_mod_folders::{FRAMEWORK_ADDON_DIR, FRAMEWORK_ADDON_FOLDER_NAME};

/// A machine credential as the platform issues it, as a bash regular expression.
pub const MACHINE_CREDENTIAL_SHAPE: &str = "^tbdm_[0-9a-f]{32}_[0-9a-f]{64}$";
/// The join password's allowed shape: 3 to 64 characters that need no escaping in JSON or in the
/// game client's password box.
pub const JOIN_PASSWORD_SHAPE: &str = "^[A-Za-z0-9._~+=:@%-]{3,64}$";
/// The RCON password the host generates: 32 lowercase hex digits (128 random bits).
pub const RCON_PASSWORD_SHAPE: &str = "^[0-9a-f]{32}$";

/// The website API probe, run on the host, where the mod calls the API: `GET <health_url>`.
///
/// `-f` turns any status of 400 or more into a failure, so an API that answers 503 because its
/// database is down stops the deploy as surely as one that is not running. `--max-time` bounds a
/// URL that accepts the connection and never answers. The trailing `echo` ends the printed body's
/// line.
pub fn website_api_health_payload(health_url: &str) -> String {
    format!(
        "set -euo pipefail\n\
         curl -sSf --max-time 10 '{health_url}'\n\
         echo\n"
    )
}

const SECRET_FILES_CHECK: &str = r#"set -uo pipefail
FLEET="$HOME/@FLEET@"
CREDENTIAL_SHAPE='@CREDENTIAL_SHAPE@'
JOIN_PASSWORD_SHAPE='@JOIN_SHAPE@'
problems=0
check() {
  local label="$1" file="$2" shape="$3" value
  if ! [ -f "$file" ] || [ "$(( 8#$(stat -c %a "$file") & 8#077 ))" -ne 0 ]; then
    echo "  MISSING $label: $file is absent, not a regular file, or open to other users"
    problems=$((problems + 1))
    return
  fi
  value="$(<"$file")"
  if [[ $value =~ $shape ]]; then
    echo "  ok      $label"
  else
    echo "  INVALID $label: $file does not hold the expected shape"
    problems=$((problems + 1))
  fi
}
check "join password" "$FLEET/@JOIN_FILE@" "$JOIN_PASSWORD_SHAPE"
for n in @INSTANCES@; do
  check "instance $n mod_runtime credential" "$FLEET/instance-$n/secrets/@RUNTIME_FILE@" "$CREDENTIAL_SHAPE"
  check "instance $n host_agent credential" "$FLEET/instance-$n/secrets/@AGENT_FILE@" "$CREDENTIAL_SHAPE"
done
if [ "$problems" -ne 0 ]; then
  echo "FAIL: $problems secret file(s) under $FLEET are not ready." >&2
  exit 1
fi
"#;

/// The instance numbers as a shell word list: `1 2 3 4 5`.
pub fn instance_numbers(instances: &[FleetInstance]) -> String {
    instances
        .iter()
        .map(|instance| instance.number.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The check that every secret file the fleet reads exists on the host: the join password and
/// each instance's two machine credentials, as regular files with no group or other permission and
/// with the expected shape. It prints one line per file, never a value, and exits 1 when any file
/// is not ready.
pub fn fleet_secret_files_check_payload(instances: &[FleetInstance]) -> String {
    SECRET_FILES_CHECK
        .replace("@FLEET@", FLEET_ROOT_UNDER_HOME)
        .replace("@CREDENTIAL_SHAPE@", MACHINE_CREDENTIAL_SHAPE)
        .replace("@JOIN_SHAPE@", JOIN_PASSWORD_SHAPE)
        .replace("@JOIN_FILE@", JOIN_PASSWORD_FILE)
        .replace("@INSTANCES@", &instance_numbers(instances))
        .replace("@RUNTIME_FILE@", MOD_RUNTIME_CREDENTIAL_FILE)
        .replace("@AGENT_FILE@", HOST_AGENT_CREDENTIAL_FILE)
}

const INSTANCE_FILES: &str = r#"set -euo pipefail
@TOOLCHAIN@
umask 077
FLEET="$HOME/@FLEET@"
INSTANCE="$HOME/@INSTANCE_FOLDER@"
SECRETS="$INSTANCE/secrets"
mkdir -p "$SECRETS" "$INSTANCE/profile" "@ADDONS@"
chmod 700 "$FLEET" "$INSTANCE" "$SECRETS" "$INSTANCE/profile"
ln -sfn "@REMOTE@/@FRAMEWORK_FOLDER@" "@ADDONS@/@FRAMEWORK_ADDON@"
if [ ! -s "$SECRETS/@RCON_FILE@" ]; then
  RCON_PASSWORD="$(head -c 16 /dev/urandom | od -An -tx1 | tr -d ' \n')"
  (set -o noclobber; printf '%s\n' "$RCON_PASSWORD" > "$SECRETS/@RCON_FILE@")
  echo "  instance @N@: generated its RCON password on the host"
fi
chmod 600 "$SECRETS/@RCON_FILE@"
RCON_PASSWORD="$(<"$SECRETS/@RCON_FILE@")"
RCON_PASSWORD_SHAPE='@RCON_SHAPE@'
if ! [[ $RCON_PASSWORD =~ $RCON_PASSWORD_SHAPE ]]; then
  echo "FAIL: $SECRETS/@RCON_FILE@ does not hold 32 lowercase hex digits; delete it to generate a new one" >&2
  exit 1
fi
JOIN_PASSWORD="$(<"$FLEET/@JOIN_FILE@")"
JOIN_PASSWORD_SHAPE='@JOIN_SHAPE@'
if ! [[ $JOIN_PASSWORD =~ $JOIN_PASSWORD_SHAPE ]]; then
  echo "FAIL: $FLEET/@JOIN_FILE@ does not hold 3 to 64 of A-Z a-z 0-9 . _ ~ + = : @ % -" >&2
  exit 1
fi
@PROFILE@cat > "$INSTANCE/server.config.template.json" <<'CONFIGEOF'
@CONFIG@CONFIGEOF
CONFIG="$(<"$INSTANCE/server.config.template.json")"
rm -f "$INSTANCE/server.config.template.json"
CONFIG="${CONFIG//@RCON_PLACEHOLDER@/"$RCON_PASSWORD"}"
CONFIG="${CONFIG//@JOIN_PLACEHOLDER@/"$JOIN_PASSWORD"}"
case "$CONFIG" in
  *_FROM_HOST_FILE*) echo "FAIL: a password placeholder survived in instance @N@'s config" >&2; exit 1 ;;
esac
printf '%s\n' "$CONFIG" > "$INSTANCE/server.config.json.next"
mv -f "$INSTANCE/server.config.json.next" "$INSTANCE/server.config.json"
echo "  instance @N@: profile, RCON password and server config in place"
"#;

const INSTANCE_PROFILE: &str = r#"TBD_MACHINE_CREDENTIAL="$(<"$SECRETS/@RUNTIME_FILE@")"
export TBD_MACHINE_CREDENTIAL
(cd "@REMOTE@" && cargo run -q -p xtask -- setup server-profile "$INSTANCE/profile")
unset TBD_MACHINE_CREDENTIAL
sed -i 's|"backendUrl": "[^"]*"|"backendUrl": "@BACKEND@"|' "$INSTANCE/profile/profile/TBD_BackendConfig.json"
"#;

/// The commands that write a fleet instance's game profile: `setup server-profile` run from the
/// host checkout `checkout` with the instance's `mod_runtime` credential in its environment only,
/// then `backend_url` as the profile's `backendUrl`. They run in a `bash` whose `INSTANCE` holds
/// the instance's folder and `SECRETS` its `secrets/` folder, as [`instance_files_payload`] sets
/// them, with the Rust toolchain on `PATH`
/// ([`crate::remote_rust_toolchain::PUT_RUST_TOOLCHAIN_ON_PATH`]).
pub fn instance_profile_commands(checkout: &str, backend_url: &str) -> String {
    INSTANCE_PROFILE
        .replace("@REMOTE@", checkout)
        .replace("@RUNTIME_FILE@", MOD_RUNTIME_CREDENTIAL_FILE)
        .replace("@BACKEND@", backend_url)
}

/// Instance `instance`'s files on the host: its folders (mode 700), the shared addon link, its RCON
/// password (generated once, mode 600, never sent anywhere), its profile with its own `mod_runtime`
/// credential and this deployment's `backendUrl` ([`instance_profile_commands`]), and its server
/// config: `rendered_config` with both password placeholders replaced from the host's files,
/// written mode 600 and moved into place whole.
pub fn instance_files_payload(
    env: &Env,
    instance: &FleetInstance,
    rendered_config: &str,
) -> String {
    let mut config = rendered_config.to_string();
    if !config.ends_with('\n') {
        config.push('\n');
    }
    INSTANCE_FILES
        .replace(
            "@TOOLCHAIN@",
            crate::remote_rust_toolchain::PUT_RUST_TOOLCHAIN_ON_PATH,
        )
        .replace("@FLEET@", FLEET_ROOT_UNDER_HOME)
        .replace("@INSTANCE_FOLDER@", &instance.home_relative_folder())
        .replace("@ADDONS@", &env.addons_staging)
        .replace("@REMOTE@", &env.remote_dir)
        .replace("@FRAMEWORK_FOLDER@", FRAMEWORK_ADDON_DIR)
        .replace("@FRAMEWORK_ADDON@", FRAMEWORK_ADDON_FOLDER_NAME)
        .replace("@RCON_FILE@", RCON_PASSWORD_FILE)
        .replace("@RCON_SHAPE@", RCON_PASSWORD_SHAPE)
        .replace("@JOIN_FILE@", JOIN_PASSWORD_FILE)
        .replace("@JOIN_SHAPE@", JOIN_PASSWORD_SHAPE)
        .replace(
            "@PROFILE@",
            &instance_profile_commands(&env.remote_dir, &env.backend_url),
        )
        .replace("@RCON_PLACEHOLDER@", RCON_PASSWORD_PLACEHOLDER)
        .replace("@JOIN_PLACEHOLDER@", JOIN_PASSWORD_PLACEHOLDER)
        .replace("@N@", &instance.number.to_string())
        .replace("@CONFIG@", &config)
}

const SMOKE: &str = r#"set -euo pipefail
CREDENTIAL="$(<"$HOME/@INSTANCE_FOLDER@/secrets/@RUNTIME_FILE@")"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
API='http://127.0.0.1:8080/api/v1/game-runtime'
bearer() { printf 'Authorization: Bearer %s\n' "$CREDENTIAL"; }
code=$(bearer | curl -sS -o "$WORK/deployment.json" -w '%{http_code}' -H @- "$API/deployment")
echo "V2 instance @N@ game-runtime deployment: HTTP $code"
if [ "$code" = "404" ]; then
  grep -q '"NO_DEPLOYMENT"' "$WORK/deployment.json" || exit 1
  echo "V3 instance @N@ artifact: no deployment yet — deploy an approved mission to this server"
elif [ "$code" = "200" ]; then
  ARTIFACT=$(sed -n 's/.*"artifact_id": *"\([^"]*\)".*/\1/p' "$WORK/deployment.json")
  EXPECTED=$(sed -n 's/.*"artifact_sha256": *"\([0-9a-f]*\)".*/\1/p' "$WORK/deployment.json")
  code=$(bearer | curl -sS -o "$WORK/artifact.json" -w '%{http_code}' -H @- "$API/artifacts/$ARTIFACT")
  ACTUAL=$(sha256sum "$WORK/artifact.json" | cut -d' ' -f1)
  echo "V3 instance @N@ artifact $ARTIFACT: HTTP $code, sha256 $ACTUAL"
  [ "$code" = "200" ] && [ -n "$EXPECTED" ] && [ "$ACTUAL" = "$EXPECTED" ] || exit 1
else
  exit 1
fi
code=$(curl -sS -o /dev/null -w '%{http_code}' "$API/deployment")
echo "V4 instance @N@ without a credential: HTTP $code"
[ "$code" = "401" ] || exit 1
"#;

/// The V2–V4 game-runtime smoke of one instance, run on the host against its own API with the
/// instance's `mod_runtime` credential:
///
/// * V2 — `GET /api/v1/game-runtime/deployment` answers the deployment this server runs (200)
///   or `NO_DEPLOYMENT` (404): the credential is accepted and scoped to this server.
/// * V3 — with a deployment, `GET /api/v1/game-runtime/artifacts/{id}` answers the artifact's
///   bytes, and they hash to the deployment's `artifact_sha256`.
/// * V4 — the same deployment read without a credential answers 401.
pub fn smoke_payload(instance: &FleetInstance) -> String {
    SMOKE
        .replace("@INSTANCE_FOLDER@", &instance.home_relative_folder())
        .replace("@RUNTIME_FILE@", MOD_RUNTIME_CREDENTIAL_FILE)
        .replace("@N@", &instance.number.to_string())
}

#[cfg(test)]
#[path = "tests/payloads/tests.rs"]
mod tests;
