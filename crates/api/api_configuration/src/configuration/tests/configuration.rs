use super::*;

fn production_base() -> Config {
    let mut cfg = Config::for_tests("postgres://x/x", "jwt-secret");
    cfg.env = "production".into();
    cfg.discord_client_id = "client-id".into();
    cfg.discord_client_secret = "secret".into();
    cfg.discord_redirect_url = "https://example.com/callback".into();
    cfg
}

/// A change that leaves one production setting unusable.
type BreakSetting = fn(&mut Config);

/// The variable a refusal names, whichever refusal kind it is.
fn refused_variable(error: ConfigError) -> &'static str {
    match error {
        ConfigError::Missing(name)
        | ConfigError::Malformed(name, _)
        | ConfigError::MalformedEntry(name, _, _) => name,
        other => panic!("expected a variable refusal, got {other:?}"),
    }
}

/// A complete production configuration boots; each unusable production setting is refused at
/// boot and the refusal names its variable: blank or whitespace-only OAuth credentials, a bot
/// token carrying whitespace, a malformed trusted-proxy entry, a missing upload directory and a
/// relative asset directory.
#[test]
fn production_refuses_each_unusable_setting_by_name() {
    production_base()
        .validate()
        .expect("a complete production configuration boots");
    let cases: [(&str, BreakSetting); 7] = [
        ("DISCORD_CLIENT_ID", |cfg| cfg.discord_client_id = "".into()),
        ("DISCORD_CLIENT_SECRET", |cfg| {
            cfg.discord_client_secret = "\t  \n".into()
        }),
        ("DISCORD_REDIRECT_URL", |cfg| {
            cfg.discord_redirect_url.clear()
        }),
        ("DISCORD_BOT_TOKEN", |cfg| {
            cfg.discord_bot_token = "MTIzNDU2.Nzg5MA.abcdef\n".into()
        }),
        ("TRUSTED_PROXIES", |cfg| {
            cfg.trusted_proxies = vec!["127.0.0.1".into(), "10.0.0.5/8".into()]
        }),
        ("UPLOAD_DIR", |cfg| cfg.upload_dir.clear()),
        ("MAP_ASSETS_DIR", |cfg| {
            cfg.map_assets_dir = "assets/terrains".into()
        }),
    ];
    for (variable, break_setting) in cases {
        let mut cfg = production_base();
        break_setting(&mut cfg);
        let error = cfg
            .validate()
            .err()
            .unwrap_or_else(|| panic!("{variable}: the broken setting must be refused"));
        assert_eq!(refused_variable(error), variable);
    }
}
