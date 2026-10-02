//! The acknowledgement-dropping relay in front of the relay instance's host agent.
//!
//! **Role:** builds the payload that builds and installs the `acknowledgement-dropping-relay`
//! executable from the synced checkout, writes the relay instance's `relay.env`, restarts
//! `acknowledgement-dropping-relay@N` and reads its state back ([`relay_install_payload`]).
//!
//! **Position:** called by the deploy pipeline in `super::remote` before the host agents restart,
//! so the relay instance's agent finds the relay listening; the unit template is written by
//! `super::fleet_units`, and the relay's behaviour is `tools/developer_tools`'.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** the relay listens on `127.0.0.1` only and forwards only to the loopback API
//! origin the agents use ([`super::fleet_instances::RelaySettings::upstream`]); `relay.env` holds
//! no secret and is written mode 600; a relay that is not active fails the deploy.

use super::fleet_instances::{FleetInstance, RelaySettings};

/// The relay install for `instance`, the relay instance, forwarding to `relay.upstream`.
pub fn relay_install_payload(
    remote_dir: &str,
    instance: &FleetInstance,
    relay: &RelaySettings,
) -> String {
    let unit = instance
        .relay_unit()
        .unwrap_or_else(|| format!("acknowledgement-dropping-relay@{}.service", relay.instance));
    format!(
        "set -euo pipefail\n\
         {toolchain}\n\
         umask 077\n\
         mkdir -p \"$HOME/.local/bin\"\n\
         (cd '{remote_dir}' && cargo build --release -q -p developer_tools --bin acknowledgement-dropping-relay)\n\
         install -m 755 '{remote_dir}/target/release/acknowledgement-dropping-relay' \"$HOME/.local/bin/acknowledgement-dropping-relay\"\n\
         printf 'RELAY_LISTEN=127.0.0.1:%s\\nRELAY_UPSTREAM=%s\\n' '{port}' '{upstream}' > \"$HOME/{folder}/relay.env\"\n\
         chmod 600 \"$HOME/{folder}/relay.env\"\n\
         systemctl --user enable {unit}\n\
         systemctl --user restart {unit}\n\
         sleep 2\n\
         state=\"$(systemctl --user show -p ActiveState --value {unit} 2>/dev/null || true)\"\n\
         if [ \"$state\" != \"active\" ]; then\n\
         \x20 echo \"FAIL: {unit} is '$state', not active.\" >&2\n\
         \x20 journalctl --user -u {unit} -n 20 --no-pager >&2 || true\n\
         \x20 exit 1\n\
         fi\n\
         echo \"  {unit} active on 127.0.0.1:{port}, forwarding to {upstream}\"\n",
        toolchain = crate::commands::deploy::remote_rust_toolchain::PUT_RUST_TOOLCHAIN_ON_PATH,
        port = relay.port,
        upstream = relay.upstream,
        folder = instance.home_relative_folder(),
    )
}

#[cfg(test)]
#[path = "tests/acknowledgement_relay/tests.rs"]
mod tests;
