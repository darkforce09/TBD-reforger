/// Class-R: the CMS Discord push must route through `webhook.push_announcement` (the sanitised
/// sink) and the failure-audit title must use `sanitize_discord_embed_field`.
///
/// RED perturbations:
/// - call a raw HTTP client with `a.title` instead of `state.webhook.push_announcement` → FAIL
/// - restore bare `a.title` in the audit format string → FAIL
#[test]
fn push_to_discord_uses_sanitised_webhook_sink() {
    const SRC: &str = include_str!("../announcement_discord_push.rs");
    let prod = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("announcement_discord_push.rs must have a #[cfg(test)] module");

    let start = prod
        .find("async fn push_to_discord")
        .expect("push_to_discord must exist");
    let after = &prod[start..];
    let end = after[1..]
        .find("\n#[derive")
        .or_else(|| after[1..].find("\npub async fn "))
        .map(|i| i + 1)
        .unwrap_or(after.len());
    let fn_body = &after[..end];

    assert!(
        fn_body.contains("webhook.push_announcement"),
        "push_to_discord must call webhook.push_announcement (the sanitised sink)"
    );

    let sanitize = format!("{}{}", "sanitize_discord_", "embed_field");
    assert!(
        fn_body.contains(&sanitize),
        "failure-audit title must call `{sanitize}` (perturbation: raw a.title in format!)"
    );
    // Window pin: sanitize call must sit near the audit action, not a distant import bait.
    let audit_arm = fn_body
        .find("webhook.push_failed")
        .expect("CRIT audit action webhook.push_failed must remain");
    let win_start = audit_arm.saturating_sub(280);
    let win = &fn_body[win_start..fn_body.len().min(audit_arm + 200)];
    assert!(
        win.contains(&sanitize),
        "`{sanitize}` must sit in the push_failed audit window:\n{win}"
    );
}
