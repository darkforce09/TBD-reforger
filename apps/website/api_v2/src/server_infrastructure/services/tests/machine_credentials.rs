use super::*;

fn secret(id: Uuid, random: &str) -> String {
    format!("tbdm_{}_{random}", id.simple())
}

#[test]
fn a_well_formed_secret_names_its_credential() {
    let id = Uuid::new_v4();
    let random = "0123456789abcdef".repeat(4);
    assert_eq!(secret_credential_id(&secret(id, &random)), Some(id));
}

#[test]
fn malformed_secrets_name_no_credential() {
    let id = Uuid::new_v4();
    let random = "0123456789abcdef".repeat(4);
    let cases = [
        String::new(),
        secret(id, &random).replacen("tbdm_", "tbdx_", 1),
        secret(id, &random[..63]),
        secret(id, &format!("{random}0")),
        secret(id, &random.to_uppercase()),
        format!("tbdm_{}_{random}", id.hyphenated()),
        format!("tbdm_{}{random}", id.simple()),
        format!("Bearer {}", secret(id, &random)),
    ];
    for case in cases {
        assert_eq!(secret_credential_id(&case), None, "{case:?}");
    }
}

#[test]
fn callers_act_only_as_their_executor_for_their_server() {
    let caller = MachineCaller {
        credential_id: Uuid::new_v4(),
        server_id: Uuid::new_v4(),
        executor: ExecutorKind::ModRuntime,
    };
    assert!(caller.require_executor(ExecutorKind::ModRuntime).is_ok());
    let refused = caller
        .require_executor(ExecutorKind::HostAgent)
        .unwrap_err();
    assert_eq!(refused.status, axum::http::StatusCode::FORBIDDEN);
    assert!(caller.require_server(caller.server_id).is_ok());
    let foreign = caller.require_server(Uuid::new_v4()).unwrap_err();
    assert_eq!(foreign.status, axum::http::StatusCode::FORBIDDEN);
}

#[test]
fn labels_and_reasons_are_trimmed_and_bounded() {
    assert_eq!(
        validated_text("  Primary runtime ", "label", 128).unwrap(),
        "Primary runtime"
    );
    assert!(validated_text("   ", "label", 128).is_err());
    assert!(validated_text(&"x".repeat(129), "label", 128).is_err());
    assert!(validated_text(&"x".repeat(128), "label", 128).is_ok());
}
