use super::*;

fn member(roles: &[&str]) -> GuildMember {
    GuildMember {
        nick: String::new(),
        roles: roles.iter().map(|s| (*s).to_string()).collect(),
    }
}

#[test]
fn transport_failure_writes_nothing() {
    // A Discord timeout must not collapse into an empty role vec, which sync_roles would
    // turn into "DELETE every stored role for this user" → Enlisted, with no snapshot left
    // for resync_all_roles to restore from. `None` here is the proof that sync_roles — the
    // only thing that deletes — is never reached on a failure.
    let snap = classify_member_lookup("42", Err(anyhow::anyhow!("connection reset by peer")));
    assert!(
        snap.ids_to_persist().is_none(),
        "an unreachable Discord must not write roles"
    );
}

#[test]
fn real_non_member_still_demotes() {
    // Discord's 404 is an answer: the user genuinely holds no guild roles, so the
    // empty write (and the resulting Enlisted) is correct.
    let snap = classify_member_lookup("42", Ok(None));
    assert!(
        matches!(snap.ids_to_persist(), Some(ids) if ids.is_empty()),
        "a 404 non-member must still sync to no roles"
    );
}

#[test]
fn member_roles_are_persisted_verbatim() {
    let snap = classify_member_lookup("42", Ok(Some(member(&["1517", "8899"]))));
    assert_eq!(
        snap.ids_to_persist().expect("authoritative"),
        ["1517", "8899"]
    );
}

/// Mirror the production decode seam for a 200: `decode_2xx` is
/// `Ok(resp.json::<GuildMember>().await?)`, so a body that fails to deserialize leaves
/// as `Err` and one that succeeds arrives as `Ok(Some(..))`. Deserializing here rather
/// than hand-building a `GuildMember` is the point — the risk lives in the derive.
fn lookup_from_200_body(body: &str) -> anyhow::Result<Option<GuildMember>> {
    Ok(Some(serde_json::from_str::<GuildMember>(body)?))
}

#[test]
fn absent_roles_field_on_a_200_does_not_demote() {
    // `RoleSnapshot` alone stops a transport failure from erasing roles, but a
    // `#[serde(default)]` on `GuildMember::roles` would let a gateway or proxy serving a
    // JSON error envelope with a 200 status deserialize to `roles: []`, become
    // Authoritative(vec![]), and send sync_roles off to DELETE every stored role — the same
    // permanent damage through a different door. An absent field is not an answer about this
    // user's roles, and must reach the Unavailable branch.
    let snap = classify_member_lookup(
        "42",
        lookup_from_200_body(r#"{"code":0,"message":"502 Bad Gateway"}"#),
    );
    assert!(
        snap.ids_to_persist().is_none(),
        "a 200 whose body omits `roles` must not be read as an authoritative empty role list"
    );
}

#[test]
fn explicitly_empty_roles_array_still_demotes() {
    // The other half of the contract, and the reason this is a serde rule rather than a
    // "treat empty as unavailable" rule: a guild member who genuinely holds no roles gets
    // `"roles": []` from Discord. That IS an answer, so it must still write — and the
    // resulting demotion to enlisted is correct, not a regression.
    let snap = classify_member_lookup("42", lookup_from_200_body(r#"{"nick":"B","roles":[]}"#));
    assert!(
        matches!(snap.ids_to_persist(), Some(ids) if ids.is_empty()),
        "an explicit `roles: []` is a real answer and must still sync to no roles"
    );
}

#[test]
fn populated_roles_on_a_200_are_authoritative() {
    // Guard the happy path against an over-broad guard: tightening `roles` must not make
    // ordinary logins fall through to Unavailable and freeze everyone's role forever.
    let snap = classify_member_lookup(
        "42",
        lookup_from_200_body(r#"{"nick":null,"roles":["1517285898817896559"]}"#),
    );
    assert_eq!(
        snap.ids_to_persist().expect("authoritative"),
        ["1517285898817896559"]
    );
}

#[test]
fn blank_guild_id_is_not_configured() {
    // A blank id must never reach Discord: the resulting 404 would read as Ok(None)
    // and demote every user who logs in.
    assert!(!guild_configured(""));
    assert!(!guild_configured("   "));
    assert!(guild_configured("1517285898817896559"));
}

/* ──── Telling a bad secret apart from a Discord outage ──── */

/// A wrong `DISCORD_CLIENT_SECRET` is the case that is easiest to misread as an outage.
/// Discord answers 401 and `decode_2xx` turns that into a plain `anyhow::bail!` with no
/// `reqwest::Error` in the chain — so "no transport error" is the typed signal that Discord
/// *answered and refused*, i.e. a CONFIG fault.
#[test]
fn a_rejected_credential_classifies_as_config_not_outage() {
    // Byte-shaped like the Discord client's real `decode_2xx` bail.
    let e = anyhow::anyhow!(
        "discord: status {}: {}",
        401,
        r#"{"error":"invalid_client"}"#
    );
    let (fault, _) = classify_discord_failure(&e);
    assert_eq!(
        fault, "config",
        "a 401 from Discord must not be reported as an outage — conflating them is the \
         whole defect this classifier exists to fix"
    );
}

/// A **real** `reqwest::Error` of the exact type production produces, from a refused
/// loopback connection — nothing listens on port 1, and no external network is touched.
/// A hand-built stand-in would be free to diverge from the type the classifier downcasts
/// to, which is the one thing this must not do.
///
/// The provider install mirrors the Discord client's own: the crate is built with reqwest's
/// `rustls-no-provider`, so constructing a `Client` panics without it. Idempotent —
/// `install_default` returns `Err` when one is already set.
async fn loopback_transport_error(path: &str) -> reqwest::Error {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::new()
        .get(format!("http://127.0.0.1:1{path}"))
        .send()
        .await
        .expect_err("nothing listens on loopback port 1")
}

/// The other half, and the one that keeps the classifier honest: without it, a
/// classifier that returned "config" unconditionally would still pass the test above.
#[tokio::test]
async fn a_refused_connection_classifies_as_outage_not_config() {
    let e = anyhow::Error::new(loopback_transport_error("/oauth2/token").await);
    let (fault, _) = classify_discord_failure(&e);
    assert_eq!(
        fault, "outage",
        "a transport failure must not be reported as a config fault; the classifier would \
         be vacuous if every input returned the same verdict"
    );
}

/// The classification must survive `anyhow`'s `?` wrapping, which is how the error
/// actually arrives from `retry_429` — a downcast of only the outermost error would
/// silently relabel every wrapped outage as a config fault.
#[tokio::test]
async fn a_wrapped_transport_error_is_still_found_in_the_chain() {
    let e = anyhow::Error::new(loopback_transport_error("/users/@me").await)
        .context("discord: fetching the user profile");
    assert_eq!(classify_discord_failure(&e).0, "outage");
}

/* ──── The no-secret guarantee, made able to FAIL ──── */

/// The three values that must never reach a log: the authorization code, the access token
/// and the client secret. Deliberately distinctive strings — a substring search for them
/// cannot collide with anything reqwest, anyhow or `tracing` emits on its own.
const CREDENTIAL_SENTINELS: [&str; 3] = [
    "invalid-authorization-code",
    "invalid-access-token",
    "invalid-client-secret",
];

/// The detector both tests below are asserted with — ONE function, so the positive control
/// proves the sensitivity of the exact code that returns the verdict.
fn credential_in(haystack: &str) -> Option<&'static str> {
    CREDENTIAL_SENTINELS
        .into_iter()
        .find(|s| haystack.contains(s))
}

/// Everything [`log_discord_call_failure`] actually emits, captured as a `tracing` layer
/// sees it — the whole event, not just the `error` field.
///
/// This is what makes the guarantee cover the LOG rather than a private re-render of the
/// error inside the test. Asserting over the test's own `format!("{:#}")` would miss a log
/// site that grew a fourth field carrying a credential.
///
/// `with_default` installs the subscriber on THIS THREAD only, and `#[tokio::test]` runs a
/// current-thread runtime, so there is no global subscriber and no bleed into the other
/// tests `cargo test` runs in parallel.
fn capture_call_failure_log(stage: &'static str, e: &anyhow::Error) -> String {
    #[derive(Clone, Default)]
    struct SharedBuf(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
    impl std::io::Write for SharedBuf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let buf = SharedBuf::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer({
            let b = buf.clone();
            move || b.clone()
        })
        .with_ansi(false)
        .finish();
    tracing::subscriber::with_default(subscriber, || log_discord_call_failure(stage, e));
    let bytes = buf.0.lock().unwrap().clone();
    String::from_utf8(bytes).expect("tracing writes UTF-8")
}

/// **The positive control, and the reason the guarantee below can fail at all.**
///
/// A lone "no credential leaked" assertion over a haystack that never contained one is
/// unfalsifiable: no rendering of `reqwest::Error` could make it fail. A
/// `contains("127.0.0.1:1")` check only rules out an EMPTY haystack; it says nothing about
/// whether the detector can see a credential that is really there.
///
/// So: same `loopback_transport_error`, same `reqwest::Error` type, same
/// `log_discord_call_failure`, same capture, same `credential_in` — the ONE difference is
/// that the credential is a genuine input, carried in the failing URL's query string.
/// Measured: reqwest renders the full URL, query included, so this is a real leak path and
/// not a hypothetical one.
///
/// If reqwest ever stops rendering the URL, or the capture silently stops capturing, or
/// `credential_in` is weakened, THIS test goes red — and the guarantee below is exposed as
/// vacuous instead of quietly becoming so.
#[tokio::test]
async fn a_credential_in_the_failing_url_does_reach_the_log() {
    for secret in CREDENTIAL_SENTINELS {
        let e = anyhow::Error::new(
            loopback_transport_error(&format!("/oauth2/token?leaked={secret}")).await,
        );
        let logged = capture_call_failure_log("token_exchange", &e);
        assert_eq!(
            credential_in(&logged),
            Some(secret),
            "the detector must SEE a credential that is genuinely in the log line; if it \
             cannot, the no-leak assertion is vacuous. Captured: {logged}"
        );
    }
}

/// The guarantee itself, on the real path. `log_discord_call_failure` is handed only a
/// stage label and the error — `q.code`, the access token and the client secret are not in
/// scope at the log site, and the production call carries them in the POST **body**, which
/// no `reqwest::Error` renders. This asserts that end-to-end over the emitted event.
#[tokio::test]
async fn the_logged_call_failure_carries_no_credential() {
    let e = anyhow::Error::new(loopback_transport_error("/oauth2/token").await);
    let logged = capture_call_failure_log("token_exchange", &e);
    assert_eq!(
        credential_in(&logged),
        None,
        "a logged Discord failure must never carry a credential. Captured: {logged}"
    );
    // Still the operator-facing contract: the line has to identify the failed call and its
    // verdict, or it is clean only because it is useless.
    assert!(
        logged.contains("127.0.0.1:1") && logged.contains("outage"),
        "the log line must still name the failed call and its fault class: {logged}"
    );
}
