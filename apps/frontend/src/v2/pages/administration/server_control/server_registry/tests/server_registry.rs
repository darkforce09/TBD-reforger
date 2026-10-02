//! The server registry's checks and bodies, held against the backend's rules and the captured
//! registration and change, and the wiring of its four writes.

use super::registration_wording::*;
use crate::v2::core::api::dto::{
    DataEnvelope, ModpackDto, ServerChange, ServerRegistration, ServerRowDto,
};
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
use crate::v2::core::test_support::fixtures::golden;

const REGISTRATION: &str = golden!("POST__servers.request.json");
const REGISTERED: &str = golden!("POST__servers.json");
const CHANGE: &str = golden!("PATCH__servers__00000000-0000-4000-d000-000000000002.request.json");
const CHANGED: &str = golden!("PATCH__servers__00000000-0000-4000-d000-000000000002.json");

fn servers() -> Vec<ServerRowDto> {
    serde_json::from_str::<DataEnvelope<ServerRowDto>>(golden!("GET__servers.json"))
        .unwrap()
        .data
}

fn json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap()
}

/// A name is trimmed and required, as the backend stores it.
#[test]
fn names_are_trimmed_and_required() {
    assert_eq!(
        validated_server_name("  TBD Staging — Everon "),
        Ok("TBD Staging — Everon".to_string())
    );
    assert_eq!(
        validated_server_name(" \t "),
        Err("The name is required".to_string())
    );
}

/// An address is a literal IPv4 or IPv6 address in its canonical form; a hostname, a `/mask` and
/// an address with its port are refused before anything is sent, each with the backend's rule.
#[test]
fn addresses_are_checked_as_the_backend_checks_them() {
    assert_eq!(
        validated_server_address(" 203.0.113.24 "),
        Ok("203.0.113.24".to_string())
    );
    assert_eq!(
        validated_server_address("0:0:0:0:0:0:0:1"),
        Ok("::1".to_string())
    );
    assert_eq!(
        validated_server_address("2001:DB8::10"),
        Ok("2001:db8::10".to_string())
    );
    let literal_only =
        "The address must be a literal IPv4 or IPv6 address — not a hostname, and not a /mask";
    for refused in [
        "play.tbd.example.com",
        "10.0.0.5/24",
        "2001:db8::/32",
        "[::1]",
        "203.0.113",
        "256.1.1.1",
    ] {
        assert_eq!(
            validated_server_address(refused),
            Err(literal_only.to_string()),
            "{refused:?} must be refused"
        );
    }
    for with_port in ["203.0.113.24:2001", "[2001:db8::10]:2001"] {
        assert_eq!(
            validated_server_address(with_port),
            Err(
                "The address must not carry the port — enter the game port in its own field"
                    .to_string()
            ),
            "{with_port:?} names its port"
        );
    }
    assert_eq!(
        validated_server_address(""),
        Err("The address is required".to_string())
    );
}

/// A game port is a whole number from 1 to 65535, the backend's bounds.
#[test]
fn game_ports_are_bounded_as_the_backend_bounds_them() {
    assert_eq!(validated_game_port(" 2001 "), Ok(2001));
    assert_eq!(validated_game_port("1"), Ok(1));
    assert_eq!(validated_game_port("65535"), Ok(65535));
    for refused in ["0", "65536", "-1", "20.5", "2001a", "port"] {
        assert_eq!(
            validated_game_port(refused),
            Err("The game port must be a whole number between 1 and 65535".to_string()),
            "{refused:?} must be refused"
        );
    }
    assert_eq!(
        validated_game_port("  "),
        Err("The game port is required".to_string())
    );
}

/// The form's registration is the captured request byte for byte, and the captured answer is a
/// row the list reads.
#[test]
fn a_filled_form_sends_the_captured_registration() {
    let built = server_registration(
        " TBD Reserve — Everon ",
        "198.51.100.40",
        "2031",
        "00000000-0000-4000-a000-000000000001",
    )
    .unwrap();
    assert_eq!(serde_json::to_value(&built).unwrap(), json(REGISTRATION));
    assert_eq!(
        serde_json::from_str::<ServerRegistration>(REGISTRATION).unwrap(),
        built
    );
    let without_modpack = server_registration("Reserve", "198.51.100.40", "2031", "").unwrap();
    assert_eq!(
        serde_json::to_value(&without_modpack).unwrap(),
        serde_json::json!({"ip": "198.51.100.40", "name": "Reserve", "port": 2031})
    );
    assert_eq!(
        server_registration("Reserve", "reserve.example.com", "2031", ""),
        Err(
            "The address must be a literal IPv4 or IPv6 address — not a hostname, and not a /mask"
                .to_string()
        )
    );
    let row: ServerRowDto = serde_json::from_str(REGISTERED).unwrap();
    assert!(row.is_active && row.status.is_none() && row.terrain.is_none());
    assert_eq!(row.required_modpack_id, built.required_modpack_id);
}

/// A change names only the fields that differ from the row the form was seeded from, clears the
/// required modpack with `null`, and is nothing at all when nothing changed.
#[test]
fn a_change_names_only_what_differs() {
    let secondary = servers().remove(1);
    let untouched = server_change(
        &secondary,
        &secondary.name,
        &format!(" {} ", secondary.ip),
        &secondary.port.to_string(),
        secondary.required_modpack_id.as_deref().unwrap_or_default(),
    );
    assert_eq!(untouched, Ok(None));
    let captured = server_change(&secondary, &secondary.name, &secondary.ip, "2012", "")
        .unwrap()
        .expect("the port and the modpack changed");
    assert_eq!(serde_json::to_value(&captured).unwrap(), json(CHANGE));
    assert_eq!(
        serde_json::from_str::<ServerChange>(CHANGE).unwrap(),
        captured
    );
    let renamed = server_change(
        &secondary,
        "TBD Secondary — Everon",
        &secondary.ip,
        &secondary.port.to_string(),
        "00000000-0000-4000-a000-000000000001",
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        serde_json::to_value(&renamed).unwrap(),
        serde_json::json!({"name": "TBD Secondary — Everon"})
    );
    let staging = servers().remove(2);
    let required = server_change(
        &staging,
        &staging.name,
        &staging.ip,
        &staging.port.to_string(),
        "00000000-0000-4000-a000-000000000001",
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        serde_json::to_value(&required).unwrap(),
        serde_json::json!({"required_modpack_id": "00000000-0000-4000-a000-000000000001"})
    );
    assert!(server_change(&staging, "", &staging.ip, "2021", "").is_err());
    let changed: ServerRowDto = serde_json::from_str(CHANGED).unwrap();
    assert_eq!(changed.port, 2012);
    assert!(changed.required_modpack_id.is_none() && changed.required_modpack.is_none());
}

/// Reactivating names `is_active` alone.
#[test]
fn a_reactivation_names_only_the_active_flag() {
    assert_eq!(
        serde_json::to_value(reactivation()).unwrap(),
        serde_json::json!({"is_active": true})
    );
}

/// An answered row takes the place of its own row, or joins the end; a deactivation clears the
/// active flag of its row and nothing else.
#[test]
fn answered_rows_take_their_place() {
    let mut list = servers();
    let registered: ServerRowDto = serde_json::from_str(REGISTERED).unwrap();
    place_row(&mut list, registered.clone());
    assert_eq!(list.len(), 4);
    assert!(list.last() == Some(&registered));
    let changed: ServerRowDto = serde_json::from_str(CHANGED).unwrap();
    place_row(&mut list, changed.clone());
    assert_eq!(list.len(), 4);
    assert!(list[1] == changed);
    let before = list.clone();
    mark_deactivated(&mut list, "00000000-0000-4000-d000-000000000001");
    assert!(!list[0].is_active);
    let mut expected = before[0].clone();
    expected.is_active = false;
    assert!(list[0] == expected && list[1..] == before[1..]);
    mark_deactivated(&mut list, "no such server");
    assert!(list[1..] == before[1..]);
}

/// Each modpack choice reads as its name and version, and the current modpack says so.
#[test]
fn modpack_choices_name_the_pack_and_its_version() {
    let packs: DataEnvelope<ModpackDto> =
        serde_json::from_str(golden!("GET__modpacks.json")).unwrap();
    assert_eq!(
        modpack_choice_label(&packs.data[0]),
        "Core Modern Expansion v2.1 (current)"
    );
    let mut retired = packs.data[0].clone();
    retired.modpack.is_current = false;
    assert_eq!(modpack_choice_label(&retired), "Core Modern Expansion v2.1");
}

/// The four writes go through the typed endpoints, and the sheet checks the form before sending.
#[test]
fn writes_go_through_the_typed_endpoints() {
    let src = live_code(include_str!("../mod.rs"));
    let compact = |text: &str| {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .replace(",)", ")")
    };
    let load = compact(only_body(&src, "pub(super) fn load("));
    assert!(load.contains("load_servers(self.store)"));
    assert!(load.contains("pick_default_id(&list.data)"));
    let register = compact(only_body(&src, "pub(super) fn register("));
    assert!(register.contains("register_server(self.store,&registration)"));
    assert!(register.contains("place_row(list,row)") && register.contains("self.selected_id"));
    let change = compact(only_body(&src, "fn send_change("));
    assert!(change.contains("change_server(self.store,&server_id,&change)"));
    assert!(change.contains("place_row(list,row)"));
    let deactivate = compact(only_body(&src, "pub(super) fn deactivate("));
    assert!(deactivate.contains("deactivate_server(self.store,&server_id)"));
    assert!(deactivate.contains("mark_deactivated(list,&server_id)"));
    let reactivate = compact(only_body(&src, "pub(super) fn reactivate("));
    assert!(reactivate.contains("registration_wording::reactivation()"));
    let sheet = compact(&live_code(include_str!("../registration_sheet.rs")));
    assert!(sheet.contains("server_registration(&name,&address,&port,&modpack)"));
    assert!(sheet.contains("server_change(&row,&name,&address,&port,&modpack)"));
}
