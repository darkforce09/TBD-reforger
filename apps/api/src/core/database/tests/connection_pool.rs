use super::*;

fn lookup_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |key| {
        pairs
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| (*v).to_string())
    }
}

fn tuned() -> DbPoolConfig {
    DbPoolConfig {
        max_connections: 3,
        idle_timeout_secs: 7,
        max_lifetime_secs: 11,
        acquire_timeout_secs: 13,
    }
}

/// Pins the defaults to the literals the API runs with when nothing is set — a changed default
/// is a changed production pool AND a changed `db test-it` profile.
#[test]
fn pool_config_defaults_are_the_shipped_literals() {
    let unset = DbPoolConfig::from_lookup(|_| None).expect("all unset parses");
    assert_eq!(unset, DbPoolConfig::default());
    assert_eq!(
        unset,
        DbPoolConfig {
            max_connections: 25,
            idle_timeout_secs: 300,
            max_lifetime_secs: 1800,
            acquire_timeout_secs: 30,
        }
    );
}

#[test]
fn pool_config_reads_each_override() {
    let cfg = DbPoolConfig::from_lookup(lookup_of(&[
        (DB_POOL_MAX_CONNECTIONS_ENV, "3"),
        (DB_POOL_IDLE_TIMEOUT_ENV, " 7 "),
        (DB_POOL_MAX_LIFETIME_ENV, "11"),
        (DB_POOL_ACQUIRE_TIMEOUT_ENV, "13\n"),
    ]))
    .expect("valid overrides parse");
    assert_eq!(cfg, tuned());
}

#[test]
fn pool_config_blank_means_default() {
    let cfg = DbPoolConfig::from_lookup(lookup_of(&[
        (DB_POOL_MAX_CONNECTIONS_ENV, ""),
        (DB_POOL_ACQUIRE_TIMEOUT_ENV, "  "),
    ]))
    .expect("blank = unset");
    assert_eq!(cfg, DbPoolConfig::default());
}

#[test]
fn pool_config_rejects_each_malformed_value_naming_the_variable() {
    for (key, bad) in [
        (DB_POOL_MAX_CONNECTIONS_ENV, "abc"),
        (DB_POOL_MAX_CONNECTIONS_ENV, "0"),
        (DB_POOL_MAX_CONNECTIONS_ENV, "-1"),
        (DB_POOL_MAX_CONNECTIONS_ENV, "2.5"),
        (DB_POOL_IDLE_TIMEOUT_ENV, "5m"),
        (DB_POOL_MAX_LIFETIME_ENV, "-30"),
        (DB_POOL_ACQUIRE_TIMEOUT_ENV, "thirty"),
    ] {
        let err = DbPoolConfig::from_lookup(lookup_of(&[(key, bad)]))
            .expect_err(&format!("{key}={bad:?} must be refused"));
        assert!(
            matches!(&err, ConfigError::MalformedValue(k, v, _) if *k == key && v == bad),
            "{key}={bad:?}: {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains(key), "{msg:?} must name {key}");
        assert!(
            msg.contains(&format!("{bad:?}")),
            "{msg:?} must quote {bad:?}"
        );
    }
}

#[test]
fn pool_options_carry_every_knob() {
    let opts = pool_options(&tuned());
    assert_eq!(opts.get_max_connections(), 3);
    assert_eq!(opts.get_idle_timeout(), Some(Duration::from_secs(7)));
    assert_eq!(opts.get_max_lifetime(), Some(Duration::from_secs(11)));
    assert_eq!(opts.get_acquire_timeout(), Duration::from_secs(13));
}

#[test]
fn pool_options_default_pins_the_shipped_literals() {
    let opts = pool_options(&DbPoolConfig::default());
    assert_eq!(opts.get_max_connections(), 25);
    assert_eq!(opts.get_idle_timeout(), Some(Duration::from_secs(5 * 60)));
    assert_eq!(opts.get_max_lifetime(), Some(Duration::from_secs(30 * 60)));
    assert_eq!(opts.get_acquire_timeout(), Duration::from_secs(30));
}
