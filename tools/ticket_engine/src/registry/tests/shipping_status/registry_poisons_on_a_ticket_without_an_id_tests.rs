use super::*;

#[test]
fn registry_poisons_on_a_ticket_without_an_id() {
    // python's KeyError inside the comprehension makes EVERY query answer "not shipped".
    let dir = std::env::temp_dir().join(format!("t853-reg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("shipping_status.json");
    std::fs::write(
        &p,
        r#"{"tickets":[{"id":"T-1","status":"shipped"},{"status":"shipped"}]}"#,
    )
    .unwrap();
    let r = ShippingStatus::load(&p);
    assert!(
        !r.is_shipped("T-1"),
        "one id-less ticket poisons every lookup"
    );
    std::fs::write(&p, r#"{"tickets":[{"id":"T-1","status":"shipped"}]}"#).unwrap();
    assert!(ShippingStatus::load(&p).is_shipped("T-1"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn registry_unreadable_is_not_shipped() {
    let r = ShippingStatus::load(Path::new("/nonexistent/shipping_status.json"));
    assert!(!r.is_shipped("T-1"));
}

#[test]
fn cancelled_counts_as_shipped() {
    let dir = std::env::temp_dir().join(format!("t853-reg2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("shipping_status.json");
    std::fs::write(&p, r#"{"tickets":[{"id":"T-9","status":"cancelled"}]}"#).unwrap();
    assert!(ShippingStatus::load(&p).is_shipped("T-9"));
    let _ = std::fs::remove_dir_all(&dir);
}
