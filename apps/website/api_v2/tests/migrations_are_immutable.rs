//! Applied migrations are immutable, and this is where that is enforced.
//!
//! `sqlx` records `Sha384(<the whole file>)` in `_sqlx_migrations.checksum` when a migration runs
//! and compares it on every boot, so any edit to an applied file — a rewritten comment included —
//! stops every database that applied it:
//!
//! ```text
//! Error: migration 21 was previously applied but has been modified
//! ```
//!
//! `durable_rate_limit.rs` pins one migration's *statements* with a substring check; that guards
//! the DDL and not the hash a database actually compares, which is how a comment edit reached
//! production. This pins every file's hash, taken from the same `sqlx::migrate!` corpus the binary
//! embeds, so the edit fails here before it reaches a database.
//!
//! When this fails: an applied migration was edited. Revert it, or add a new migration. If the
//! change really is comments-only, update the pin below and run
//! `cargo xtask db repair-migration-checksum` on every database that applied it — development,
//! staging and production alike; the command proves the statements are unchanged before it writes.

/// `(version, lowercase hex SHA-384)` for every migration on disk, in version order. One row per
/// migration, so a failure points at one line and `sha384sum` output can be pasted in unchanged.
#[rustfmt::skip]
const PINNED: &[(i64, &str)] = &[
    (1, "9f28cb904916352aa3c1aeed314858ba87152e71d9fe0d178289718f7c5aacf44c9ed766118d9fe44b390ba8d5d6cb7a"),
    (2, "52fee759dea7af59ec4b2be0d8f562a229a779804da24301fcfdc2b555145ea9f360c9f4cb19cd02982a8ed6524a601f"),
    (3, "0edd0e727ef04ebade63875cb3ed5ef57f6f1a1b100c9e51063fb0616798953623599ee0421be48dbd2ae0ff57d72763"),
    (4, "184d061e1f2337733ffea0dd0d08d58f1a7d894c2831c6c1cb6d6b5c67d5813101a93f743ac20f8cd5690b8273e9ce57"),
    (5, "84fa5b7ba0a1554cef1bef650a24ffae8c68628d849a5c9515c64bd7a1ef554bb6a2249244ad44590e13403c8d5c8a05"),
    (6, "d9cb93918272e13514e429c07f3beb024ada4a10eb229cf148894c0693fd7213659611a1881ae50746a9da306b89b97b"),
    (7, "4d7cb4989c1e1fd7df995c2769d94a8e4f14929869752741a3ab494c2bec2b06a196c7d85a2192b2855fa7ff087213bb"),
    (8, "24afdf830eed8d5df1cbaf19c46e5bf8c587d755c136b1ec0ab29edf6fc1826842664ffde95ea6a37d26c5da0b78fea2"),
    (9, "17270a89965d4fe8959dda301041ca1b9fc291d2e32ffc43646560f965e1c4968567c99ed094ee6dd4b70723074ba34d"),
    (10, "5b6d1b3f73d3e626c2d41054fbfc5e4e5ce53d9e2568994d0fb419638a7339e8fb7bde1880d45b0c822b5d116d9901ad"),
    (11, "efb978a3cf066ed50a087d71fe952d941c9a56cb255a223825219e2b2dfa3dcbdba138674c98ba24f4aa55bb67c7f5b8"),
    (12, "993e75627de7ae8842c8394ebb99038bb8652d042ceca56c2c186b343356487c868029ab9448d57a4c39c415b318b453"),
    (13, "64c3a90c550f7a3e19eb4d67ffdea3e84314d4362c40eaa9d15c0e0302dbff0d60d94e2ce7ef6f407534f05fcd7eca3a"),
    (14, "d42dbf1af1bf0893cb996b249fcbe4301b7f43f7f70141f3f5e46a18970315ad6c4dc4d11c520dd2aac652f94abb2f63"),
    (15, "19ff15deb5e85ad78449b1a04342c01695fb8c66a87902191fa3f1b6ba32c2e8e9449976903acf3a162bced7f6142f23"),
    (16, "308e4c4236c92b20d6a3bcfa9ab10f6b51ebb13f6817a1ab5d7e0dc996dbd01ccfc56f28d0cf11eba464e1dacaa2fdc7"),
    (17, "70529950f9ba8aa63dc2bd1002098d9df15d285bad909e88723829b1f19c9858655f889ce648ac49e3606d78259c67e3"),
    (18, "7b7af554eff4aff66dcf6dcb1e40aad1de34639e510cd1800a14e6dc040bdd39df5ee2ea2a6d14f0faf027fec0b6b4bc"),
    (19, "5daa856eccb8eb3a8850fce5219f3a883641f326450496464622b708920f0cda543a59ba4aff2556ef2165dff994dc9f"),
    (20, "7f8a3142caa87076e3b3ea19b476f2caa514ba794d68566fdfa623fdca1a8676c05b3772d48b27d5f0102ce4b276db21"),
    (21, "58d6e78104dc86e8f8a69462b04c8ac7e31928579232bafabd30a99572c01fa0a50e6d91d90e17c6276f955ac9e517cb"),
    (25, "ac2d34997625013724a33fc17fb885b2467642fcaf85dc3b57422787e41bc6dad83f8bf853ccfd221388ef1ba2136416"),
];

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// The corpus the binary embeds and applies — the same `migrate!` call `core::database` makes.
fn embedded() -> sqlx::migrate::Migrator {
    sqlx::migrate!("./migrations")
}

#[test]
fn every_migration_on_disk_matches_its_pinned_checksum() {
    let migrator = embedded();
    let mut unpinned = Vec::new();
    for migration in migrator.migrations.iter() {
        match PINNED.iter().find(|(v, _)| *v == migration.version) {
            None => unpinned.push(migration.version),
            Some((_, pinned)) => assert_eq!(
                hex(&migration.checksum),
                *pinned,
                "migration {} ({}) was edited after it was applied.\n\
                 Every database that applied it will refuse to boot with\n\
                 `migration {} was previously applied but has been modified`.\n\
                 Revert the edit, or add a new migration. If the change is comments-only, update \
                 this pin and run `cargo xtask db repair-migration-checksum` on every database.",
                migration.version,
                migration.description,
                migration.version
            ),
        }
    }
    assert!(
        unpinned.is_empty(),
        "migration(s) {unpinned:?} have no pin here — add a (version, sha384) row for each so the \
         file is immutable from the moment it lands. `sha384sum migrations/<file>.sql` prints it."
    );
}

/// A pin without a file would let a deleted migration pass unnoticed; a database that applied it
/// would then carry a schema the tree no longer describes.
#[test]
fn every_pin_still_has_its_migration_on_disk() {
    let migrator = embedded();
    for (version, _) in PINNED {
        assert!(
            migrator.migrations.iter().any(|m| m.version == *version),
            "pinned migration {version} is no longer on disk — a migration is never deleted once \
             applied; add a reversing migration instead"
        );
    }
}
