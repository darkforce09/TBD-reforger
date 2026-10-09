//! A game server's registration in words, and the checks a registration or a change passes before
//! it is sent.
//!
//! **Role:** validates a server's name, address and game port exactly as the backend does, builds
//! the registration body, diffs an edited form against the row it was seeded from into the change
//! body that names only what differs, names each required-modpack choice, and puts an answered
//! row into the list.
//! **Position:** read by the registration sheet's form, and by the [`super::ServerRegistry`] when a
//! write is accepted.
//! **Signals & state:** none; pure over its arguments.
//! **Invariants:** the checks mirror the backend's and state its rules — a name trimmed and not
//! blank; an address that parses as a literal IPv4 or IPv6 address, sent in the parser's canonical
//! form, never a hostname and never with a `/mask`, which the database would store and then
//! silently drop; a game port from 1 to 65535 — so a registration the backend would refuse is never
//! sent. A change never names a field that did not change, sends `null` for the required modpack
//! only to clear one that was required, and is `None` when nothing changed, because the backend
//! refuses a change that names no field.

#[cfg(any(target_arch = "wasm32", test))]
use std::net::{IpAddr, SocketAddr};

#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::ModpackDto;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::identifiers::ModpackId;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::{ServerChange, ServerRegistration, ServerRowDto};

/// The one sentence for an address that is not a literal IP address, as the backend words it.
#[cfg(any(target_arch = "wasm32", test))]
const NOT_A_LITERAL_ADDRESS: &str =
    "The address must be a literal IPv4 or IPv6 address — not a hostname, and not a /mask";

/// The server's name, trimmed, or what is wrong with it.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn validated_server_name(text: &str) -> Result<String, String> {
    let name = text.trim();
    if name.is_empty() {
        return Err("The name is required".to_string());
    }
    Ok(name.to_string())
}

/// The server's address in the parser's canonical form, or what is wrong with it.
///
/// An address typed with its port is told apart from a hostname, since the port has its own
/// field.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn validated_server_address(text: &str) -> Result<String, String> {
    let address = text.trim();
    if address.is_empty() {
        return Err("The address is required".to_string());
    }
    if let Ok(parsed) = address.parse::<IpAddr>() {
        return Ok(parsed.to_string());
    }
    if address.parse::<SocketAddr>().is_ok() {
        return Err(
            "The address must not carry the port — enter the game port in its own field"
                .to_string(),
        );
    }
    Err(NOT_A_LITERAL_ADDRESS.to_string())
}

/// The game port, or what is wrong with it.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn validated_game_port(text: &str) -> Result<i64, String> {
    let port = text.trim();
    if port.is_empty() {
        return Err("The game port is required".to_string());
    }
    match port.parse::<i64>() {
        Ok(port) if (1..=65535).contains(&port) => Ok(port),
        _ => Err("The game port must be a whole number between 1 and 65535".to_string()),
    }
}

/// The required modpack a choice names; the empty choice requires none.
#[cfg(any(target_arch = "wasm32", test))]
fn chosen_modpack(choice: &str) -> Option<ModpackId> {
    let id = choice.trim();
    (!id.is_empty()).then(|| ModpackId::from(id))
}

/// The registration a filled form sends, or the first thing wrong with it.
///
/// The server is registered active, the backend's default, so the body leaves `is_active` out.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn server_registration(
    name: &str,
    address: &str,
    port: &str,
    modpack_choice: &str,
) -> Result<ServerRegistration, String> {
    Ok(ServerRegistration {
        name: validated_server_name(name)?,
        ip: validated_server_address(address)?,
        port: validated_game_port(port)?,
        required_modpack_id: chosen_modpack(modpack_choice),
        is_active: None,
    })
}

/// Whether two addresses name the same host: compared as parsed addresses, so `::1` and
/// `0:0:0:0:0:0:0:1` agree, and as text only when one does not parse.
#[cfg(any(target_arch = "wasm32", test))]
fn same_address(stored: &str, entered: &str) -> bool {
    match (stored.parse::<IpAddr>(), entered.parse::<IpAddr>()) {
        (Ok(stored), Ok(entered)) => stored == entered,
        _ => stored == entered,
    }
}

/// The change a form edited from `stored` sends: only the fields that differ, `Ok(None)` when none
/// does, or the first thing wrong with the form.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn server_change(
    stored: &ServerRowDto,
    name: &str,
    address: &str,
    port: &str,
    modpack_choice: &str,
) -> Result<Option<ServerChange>, String> {
    let name = validated_server_name(name)?;
    let ip = validated_server_address(address)?;
    let port = validated_game_port(port)?;
    let modpack = chosen_modpack(modpack_choice);
    let change = ServerChange {
        name: (name != stored.name).then_some(name),
        ip: (!same_address(&stored.ip, &ip)).then_some(ip),
        port: (port != stored.port).then_some(port),
        required_modpack_id: (modpack != stored.required_modpack_id).then_some(modpack),
        is_active: None,
    };
    Ok((change != ServerChange::default()).then_some(change))
}

/// The change that puts a deactivated server back into service.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn reactivation() -> ServerChange {
    ServerChange {
        is_active: Some(true),
        ..ServerChange::default()
    }
}

/// A modpack as the required-modpack choice names it: `name vversion`, marked when it is the
/// current modpack.
#[cfg(target_arch = "wasm32")]
pub(crate) fn modpack_choice_label(pack: &ModpackDto) -> String {
    let current = if pack.modpack.is_current {
        " (current)"
    } else {
        ""
    };
    format!("{} v{}{current}", pack.modpack.name, pack.modpack.version)
}

/// Put an answered row into the list: in place of the row with its id, else at the end.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn place_row(list: &mut Vec<ServerRowDto>, row: ServerRowDto) {
    match list.iter_mut().find(|stored| stored.id == row.id) {
        Some(stored) => *stored = row,
        None => list.push(row),
    }
}

/// Mark one server deactivated, the one field a deactivation changes.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn mark_deactivated(list: &mut [ServerRowDto], server_id: &str) {
    if let Some(stored) = list.iter_mut().find(|stored| stored.id == server_id) {
        stored.is_active = false;
    }
}
