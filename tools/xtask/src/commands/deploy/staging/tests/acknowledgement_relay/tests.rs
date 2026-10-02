//! The relay install for the relay instance.
use super::*;
use crate::commands::deploy::staging::config::tests::base;

#[test]
fn the_relay_is_built_configured_started_and_read_back() {
    let env = base();
    let relay = env.fleet.relay.clone().expect("the fixture runs a relay");
    let fifth = env.fleet.instances()[4].clone();
    let p = relay_install_payload(&env.remote_dir, &fifth, &relay);
    assert!(
        p.starts_with("set -euo pipefail\nexport PATH=\"$HOME/.cargo/bin:$PATH\"\numask 077\n")
    );
    assert!(p.contains(
        "(cd '/home/deploy/tbd/repo' && cargo build --release -q -p developer_tools --bin acknowledgement-dropping-relay)\n"
    ));
    assert!(p.contains(
        "install -m 755 '/home/deploy/tbd/repo/target/release/acknowledgement-dropping-relay' \"$HOME/.local/bin/acknowledgement-dropping-relay\"\n"
    ));
    assert!(p.contains(
        "printf 'RELAY_LISTEN=127.0.0.1:%s\\nRELAY_UPSTREAM=%s\\n' '18085' 'http://127.0.0.1:8080' > \"$HOME/tbd/fleet/instance-5/relay.env\"\n"
    ));
    assert!(p.contains("chmod 600 \"$HOME/tbd/fleet/instance-5/relay.env\"\n"));
    assert!(p.contains("systemctl --user restart acknowledgement-dropping-relay@5.service\n"));
    assert!(p.contains("show -p ActiveState --value acknowledgement-dropping-relay@5.service"));
    assert!(p.contains("if [ \"$state\" != \"active\" ]; then\n"));
}
