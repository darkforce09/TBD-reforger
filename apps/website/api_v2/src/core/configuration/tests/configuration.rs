use super::*;

use std::net::IpAddr;

use proxy_network::ProxyNet;

fn production_base() -> Config {
    let mut cfg = Config::for_tests("postgres://x/x", "jwt-secret");
    cfg.env = "production".into();
    cfg.discord_client_id = "client-id".into();
    cfg.discord_client_secret = "secret".into();
    cfg.discord_redirect_url = "https://example.com/callback".into();
    cfg
}

fn ip(s: &str) -> IpAddr {
    s.parse().expect("test address")
}

#[test]
fn production_rejects_blank_discord_client_id() {
    let mut cfg = production_base();
    cfg.discord_client_id.clear();
    match cfg.validate() {
        Err(ConfigError::Missing("DISCORD_CLIENT_ID")) => {}
        other => panic!("expected Missing(DISCORD_CLIENT_ID), got {other:?}"),
    }
}

/// Production must reject whitespace-only `DISCORD_CLIENT_ID`: an `is_empty()`-only guard lets
/// `" "` validate, and the misconfiguration then surfaces as `oauth_unconfigured` at first use
/// instead of a boot-time Missing.
#[test]
fn production_rejects_whitespace_only_discord_client_id() {
    let mut cfg = production_base();
    cfg.discord_client_id = " ".into();
    match cfg.validate() {
        Err(ConfigError::Missing("DISCORD_CLIENT_ID")) => {}
        other => panic!("expected Missing(DISCORD_CLIENT_ID), got {other:?}"),
    }
    // Tab / mixed whitespace are the same lie as a single space.
    cfg = production_base();
    cfg.discord_client_id = "\t  \n".into();
    match cfg.validate() {
        Err(ConfigError::Missing("DISCORD_CLIENT_ID")) => {}
        other => panic!("expected Missing(DISCORD_CLIENT_ID) for mixed ws, got {other:?}"),
    }
}

#[test]
fn production_rejects_blank_discord_client_secret() {
    let mut cfg = production_base();
    cfg.discord_client_secret.clear();
    match cfg.validate() {
        Err(ConfigError::Missing("DISCORD_CLIENT_SECRET")) => {}
        other => panic!("expected Missing(DISCORD_CLIENT_SECRET), got {other:?}"),
    }
}

/// Production must reject whitespace-only `DISCORD_CLIENT_SECRET` — the same disguise class as
/// the client id.
#[test]
fn production_rejects_whitespace_only_discord_client_secret() {
    let mut cfg = production_base();
    cfg.discord_client_secret = " ".into();
    match cfg.validate() {
        Err(ConfigError::Missing("DISCORD_CLIENT_SECRET")) => {}
        other => panic!("expected Missing(DISCORD_CLIENT_SECRET), got {other:?}"),
    }
    cfg = production_base();
    cfg.discord_client_secret = "\t  \n".into();
    match cfg.validate() {
        Err(ConfigError::Missing("DISCORD_CLIENT_SECRET")) => {}
        other => panic!("expected Missing(DISCORD_CLIENT_SECRET) for mixed ws, got {other:?}"),
    }
}

#[test]
fn production_rejects_blank_discord_redirect_url() {
    let mut cfg = production_base();
    cfg.discord_redirect_url.clear();
    match cfg.validate() {
        Err(ConfigError::Missing("DISCORD_REDIRECT_URL")) => {}
        other => panic!("expected Missing(DISCORD_REDIRECT_URL), got {other:?}"),
    }
}

/// Production must reject whitespace-only `DISCORD_REDIRECT_URL` — the same disguise class as
/// the client id.
#[test]
fn production_rejects_whitespace_only_discord_redirect_url() {
    let mut cfg = production_base();
    cfg.discord_redirect_url = " ".into();
    match cfg.validate() {
        Err(ConfigError::Missing("DISCORD_REDIRECT_URL")) => {}
        other => panic!("expected Missing(DISCORD_REDIRECT_URL), got {other:?}"),
    }
    cfg = production_base();
    cfg.discord_redirect_url = "\t  \n".into();
    match cfg.validate() {
        Err(ConfigError::Missing("DISCORD_REDIRECT_URL")) => {}
        other => panic!("expected Missing(DISCORD_REDIRECT_URL) for mixed ws, got {other:?}"),
    }
}

#[test]
fn development_allows_blank_discord() {
    // Config::for_tests + local APP_ENV=development must keep blank Discord
    // for dev-login; only production fails closed.
    let cfg = Config::for_tests("postgres://x/x", "jwt-secret");
    assert!(cfg.is_development());
    assert!(cfg.discord_client_id.is_empty());
    assert!(cfg.discord_client_secret.is_empty());
    assert!(cfg.discord_redirect_url.is_empty());
    cfg.validate().expect("development blank Discord must load");
}

#[test]
fn production_with_discord_creds_loads() {
    production_base()
        .validate()
        .expect("production with Discord client id+secret+redirect must load");
}

// ---- DISCORD_BOT_TOKEN ----------------------------------------------

/// Unset is legal but must NOT read as a usable token. The whole point of the accessor: `""`
/// can never escape as a `Bot ` header, it becomes a named error instead.
#[test]
fn unset_bot_token_loads_but_is_not_readable() {
    let cfg = production_base();
    assert!(cfg.discord_bot_token.is_empty());
    let cfg = cfg.validate().expect("unset bot token must not block boot");
    assert!(!cfg.discord_bot_configured());
    match cfg.require_discord_bot_token() {
        Err(ConfigError::Missing("DISCORD_BOT_TOKEN")) => {}
        other => panic!("expected Missing(DISCORD_BOT_TOKEN), got {other:?}"),
    }
}

/// `DISCORD_BOT_TOKEN=` in `.env` decodes to `Ok("")`, not absent — the live `.env` is exactly
/// this, so it must stay bootable in every env.
#[test]
fn empty_bot_token_is_unconfigured_in_development_too() {
    let cfg = Config::for_tests("postgres://x/x", "jwt-secret");
    assert!(cfg.is_development());
    let cfg = cfg
        .validate()
        .expect("development empty bot token must load");
    assert!(!cfg.discord_bot_configured());
}

/// A real token round-trips and is readable.
#[test]
fn configured_bot_token_is_readable() {
    let mut cfg = production_base();
    cfg.discord_bot_token = "MTIzNDU2.Nzg5MA.abcdefGHIJKL-_".into();
    let cfg = cfg.validate().expect("a well-formed bot token must load");
    assert!(cfg.discord_bot_configured());
    assert_eq!(
        cfg.require_discord_bot_token().expect("readable"),
        "MTIzNDU2.Nzg5MA.abcdefGHIJKL-_"
    );
}

/// Whitespace-only is the blank-credential lie applied to the bot token: non-empty to
/// `is_empty()`, unusable to Discord.
#[test]
fn whitespace_only_bot_token_is_rejected_at_boot() {
    for bad in [" ", "\t", "\n", "\t  \n"] {
        let mut cfg = production_base();
        cfg.discord_bot_token = bad.into();
        match cfg.validate() {
            Err(ConfigError::Malformed("DISCORD_BOT_TOKEN", _)) => {}
            other => panic!("expected Malformed(DISCORD_BOT_TOKEN) for {bad:?}, got {other:?}"),
        }
    }
}

/// The failure this actually prevents: a trailing newline from a copy-paste or a
/// secrets-manager read. `is_empty()` and `trim().is_empty()` BOTH pass it, so only a
/// whitespace-anywhere rule catches it — and uncaught it ships an invalid `Authorization`
/// header that Discord answers with 401.
#[test]
fn bot_token_with_surrounding_or_inner_whitespace_is_rejected() {
    for bad in [
        "MTIzNDU2.Nzg5MA.abcdef\n",
        " MTIzNDU2.Nzg5MA.abcdef",
        "MTIzNDU2.Nzg5MA.abcdef\r\n",
        "MTIzNDU2 .Nzg5MA.abcdef",
    ] {
        let mut cfg = production_base();
        cfg.discord_bot_token = bad.into();
        match cfg.validate() {
            Err(ConfigError::Malformed("DISCORD_BOT_TOKEN", _)) => {}
            other => panic!("expected Malformed(DISCORD_BOT_TOKEN) for {bad:?}, got {other:?}"),
        }
    }
}

/// Development is not a hole: an unusable token is unusable everywhere. (Unlike the OAuth trio,
/// this rule is env-independent — a whitespace token is never a legitimate dev state.)
#[test]
fn whitespace_bot_token_is_rejected_in_development_too() {
    let mut cfg = Config::for_tests("postgres://x/x", "jwt-secret");
    cfg.discord_bot_token = " ".into();
    assert!(cfg.is_development());
    match cfg.validate() {
        Err(ConfigError::Malformed("DISCORD_BOT_TOKEN", _)) => {}
        other => panic!("expected Malformed(DISCORD_BOT_TOKEN) in dev, got {other:?}"),
    }
}

// ---- GAME_AGENT_SOCKET ----------------------------------------------

/// Unset is legal — there is no agent on a developer's box — but it must not read as a usable
/// path. `Path::new("")` connects to nothing and reports ENOENT, which at the call site is
/// indistinguishable from a dead agent.
#[test]
fn unset_agent_socket_loads_but_is_not_readable() {
    let cfg = production_base();
    assert!(cfg.game_agent_socket.is_empty());
    let cfg = cfg
        .validate()
        .expect("unset GAME_AGENT_SOCKET must not block boot");
    assert!(!cfg.game_agent_configured());
    match cfg.require_game_agent_socket() {
        Err(ConfigError::Missing("GAME_AGENT_SOCKET")) => {}
        other => panic!("expected Missing(GAME_AGENT_SOCKET), got {other:?}"),
    }
}

/// The real deployment value round-trips (`%t/tbd-reforger-agent.sock` expanded by systemd).
#[test]
fn configured_agent_socket_is_readable() {
    let mut cfg = production_base();
    cfg.game_agent_socket = "/run/user/1000/tbd-reforger-agent.sock".into();
    let cfg = cfg.validate().expect("an absolute socket path must load");
    assert!(cfg.game_agent_configured());
    assert_eq!(
        cfg.require_game_agent_socket().expect("readable"),
        Path::new("/run/user/1000/tbd-reforger-agent.sock")
    );
}

/// A trailing newline from a copy-paste or a secrets-manager read. Both `is_empty()` and
/// `is_absolute()` pass it, so only the trim rule catches it — and uncaught it becomes ENOENT,
/// reported to the operator as an unreachable game host rather than as a typo in their `.env`.
#[test]
fn agent_socket_with_surrounding_whitespace_is_rejected() {
    for bad in [
        "/run/user/1000/tbd-reforger-agent.sock\n",
        " /run/user/1000/tbd-reforger-agent.sock",
        "/run/user/1000/tbd-reforger-agent.sock\r\n",
        "\t/run/user/1000/tbd-reforger-agent.sock ",
    ] {
        let mut cfg = production_base();
        cfg.game_agent_socket = bad.into();
        match cfg.validate() {
            Err(ConfigError::Malformed("GAME_AGENT_SOCKET", _)) => {}
            other => panic!("expected Malformed(GAME_AGENT_SOCKET) for {bad:?}, got {other:?}"),
        }
    }
}

/// A relative path resolves against the API process's CWD — a systemd detail nobody chose.
/// Reject it at boot rather than connect somewhere unintended.
#[test]
fn relative_agent_socket_is_rejected() {
    for bad in [
        "tbd-reforger-agent.sock",
        "run/user/1000/tbd-reforger-agent.sock",
        "./agent.sock",
    ] {
        let mut cfg = production_base();
        cfg.game_agent_socket = bad.into();
        match cfg.validate() {
            Err(ConfigError::Malformed("GAME_AGENT_SOCKET", _)) => {}
            other => panic!("expected Malformed(GAME_AGENT_SOCKET) for {bad:?}, got {other:?}"),
        }
    }
}

/// Inner spaces are legal in a filesystem path and must NOT be swept up by the trim rule — that
/// would be a guard rejecting valid configuration, which is its own kind of lie.
#[test]
fn agent_socket_may_contain_inner_spaces() {
    let mut cfg = production_base();
    cfg.game_agent_socket = "/run/user/1000/tbd agent.sock".into();
    let cfg = cfg
        .validate()
        .expect("a path with an inner space is a legal path");
    assert!(cfg.game_agent_configured());
}

/// Development is not a hole: an unusable path is unusable everywhere.
#[test]
fn bad_agent_socket_is_rejected_in_development_too() {
    let mut cfg = Config::for_tests("postgres://x/x", "jwt-secret");
    cfg.game_agent_socket = "relative.sock".into();
    assert!(cfg.is_development());
    match cfg.validate() {
        Err(ConfigError::Malformed("GAME_AGENT_SOCKET", _)) => {}
        other => panic!("expected Malformed(GAME_AGENT_SOCKET) in dev, got {other:?}"),
    }
}

// ---- TRUSTED_PROXIES at the Config boundary -------------------------

/// A bad entry is a **boot failure**, and the message names the entry. Silently dropping it
/// would leave an operator who typed one CIDR wrong believing per-client keying is live.
#[test]
fn a_malformed_trusted_proxy_entry_fails_validation() {
    let mut cfg = production_base();
    cfg.trusted_proxies = vec!["127.0.0.1".into(), "10.0.0.5/8".into()];
    match cfg.validate() {
        Err(ConfigError::MalformedEntry("TRUSTED_PROXIES", entry, _)) => {
            assert_eq!(entry, "10.0.0.5/8", "the error must name the bad entry");
        }
        other => panic!("expected MalformedEntry(TRUSTED_PROXIES), got {other:?}"),
    }
}

/// Unset stays legal and means trust-none — the shipped default.
#[test]
fn an_empty_trusted_proxy_list_is_valid_and_trusts_nothing() {
    let cfg = production_base().validate().expect("no proxies is legal");
    assert!(cfg.trusted_proxies.is_empty());
    assert_eq!(
        parse_trusted_proxies(&cfg.trusted_proxies).unwrap(),
        Vec::<ProxyNet>::new()
    );
}

/// **The value the deployment actually ships.** `docker-compose.staging.yml` sets
/// `TRUSTED_PROXIES: ${TRUSTED_PROXIES:-127.0.0.1/32}`, and that line is load-bearing in two
/// ways at once: it is the setting that switches per-client keying on, and it is parsed at
/// boot, so a value this module refuses would stop the staging API from starting.
///
/// Read out of the compose file rather than retyped, because a typed copy would agree with
/// itself while the deployment shipped something else.
#[test]
fn the_shipped_staging_default_parses_and_matches_the_loopback_proxy() {
    const COMPOSE: &str = include_str!("../../../../../docker-compose.staging.yml");
    let line = COMPOSE
        .lines()
        .find(|l| l.trim_start().starts_with("TRUSTED_PROXIES:"))
        .expect("docker-compose.staging.yml no longer sets TRUSTED_PROXIES");
    let shipped = line
        .split_once(":-")
        .and_then(|(_, rest)| rest.split_once('}'))
        .map(|(default, _)| default.trim())
        .expect("TRUSTED_PROXIES no longer has a `${VAR:-default}` default");
    let entries = split_csv(shipped);
    let nets = parse_trusted_proxies(&entries).unwrap_or_else(|(entry, why)| {
        panic!(
            "the staging compose default {shipped:?} does not parse — entry {entry:?}: {why}. \
             The API would refuse to boot on staging."
        )
    });
    assert!(
        nets.iter().any(|n| n.contains(ip("127.0.0.1"))),
        "the staging default {shipped:?} does not cover the loopback address Caddy proxies \
         from, so X-Forwarded-For would still be ignored there"
    );
    assert!(
        !nets.iter().any(|n| n.contains(ip("203.0.113.9"))),
        "the staging default {shipped:?} trusts a public address"
    );
}

/// The whole list parses, in order, and a good list validates.
#[test]
fn a_well_formed_list_parses_and_validates() {
    let mut cfg = production_base();
    cfg.trusted_proxies = vec!["127.0.0.1".into(), "10.0.0.0/8".into(), "::1".into()];
    let cfg = cfg.validate().expect("well-formed list must validate");
    let nets = parse_trusted_proxies(&cfg.trusted_proxies).expect("parse");
    assert_eq!(nets.len(), 3);
    assert!(nets[0].contains(ip("127.0.0.1")));
    assert!(nets[1].contains(ip("10.9.9.9")));
    assert!(nets[2].contains(ip("::1")));
    assert!(!nets.iter().any(|n| n.contains(ip("203.0.113.1"))));
}
