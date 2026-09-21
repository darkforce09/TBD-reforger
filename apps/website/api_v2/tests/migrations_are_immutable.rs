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
    (1, "2efc1849458eaf37ca6346914cc764dee5e49b8f9e28177014e3f9aede1451f5e86de9d1939a5e2fb5a87941388cdbe9"),
    (2, "878c880563bbe36e9bc45fe2782d41150bc90e39140d7a877b6477990ba5609695f77c943e568efd689c19c66013f683"),
    (3, "84861fba54eb4ae01a341d37170d0fb2327f189df8e09d289cabffa56a7b348026108a134ffb82e4892e4601895476f0"),
    (4, "dbde42bf7a1cbb9716179b0bda298cd108555cb30f13c6f5047e78b1a5ef6222a64b17df04cc1fa24b5581f942a81943"),
    (5, "7e7dda21089c4156a790cf551b70fd06f17ccc16b50a63d4ca8dd6a15509ab9f8a0fea187fca93bbccb76696d4dde344"),
    (6, "2daf682fa507e011137e7a07b38abd87b54ab98ca80c8d9832f6d5a0e3bed85e5f0798f2bfb4ad9d1e5f0cdecfa2282a"),
    (7, "a739770cf2a14df333484edb50f53cddb1dab47a2c250b05f07e8d76b236c51151349e95e97dfc08cc4c4c7548b531fa"),
    (8, "04bb5924c64be6bec5bde53c6ff79d8d9d5de2cda86091dd0abcfdfd6523909f9c2b32a7e1d368ae844ca85a4db5484e"),
    (9, "bba78e5628d87bd026cad051715bb8560211b724e482093cdb93b4238df56d7808422e0520ed7887371123781a49b52f"),
    (10, "261b676aae469d2e337ba6f816689d10c063874b6bed307791fbce854a322328803bbdb165a4026923e5bc3f27fb0ac4"),
    (11, "f8fa53fafb3bd20534414fda717478d186890cdb0c4d5154d2c9be3e37c8a9f72de6fc840d8ad88a39a977c3b177c44a"),
    (12, "69fb888895f22e4027a84fc786c0663a41a403caa1bcd1bb5eb107ac836765e310113cd69fe2b8d9c52bab54acd56c65"),
    (13, "7b3027da72f47219c197c7344a11e7cd114a8665804768340c8df771ef239fb8aa3f7db27b843252a4148ca61270aef4"),
    (14, "f0e53575932c6fe18c534bc1e80e73c1364b33228d0a251d1f93f1bbe88d1eeab7127abcf8567a72c97f3dc9e7866520"),
    (15, "1f62e952186b7060e6f3021fd642f66df3d179af675369c2d44979e82c00f583b12ce1e85445371c442fbd9cb56cdb07"),
    (16, "873b4f03f8d23fd59f3f8252b27d866309b748d6af360812ad0804abc0f66bf43ccbb4adbb1e48959371611277fb85dd"),
    (17, "6efd63015670479f7ea756a0988ab9c74cef4c1df065bde5b7363868bf49051638dc8c42a832aeec81f957d7e67df3d7"),
    (18, "ffbf15441a95b8581b9ad58100c77886eadb11d8b783baeb4af8fe8428ccc7c27ce9f2b10cbbaf941ed6949bfa473a86"),
    (19, "1a069dca4dcc5dda42d379050c2c73e2544699f5fe62456349267b0661034b1398cac107a0eafd7d8d1b671497b8b416"),
    (20, "0d9b43509fc63d231832994382f424c30e9ec4ae3e793e2b2a011994fe626bf607369dc4c4e99d72eea98033e5faf25c"),
    (21, "61baf02c73f66e5548d668c0d04f26f6fe14f82b782280a5cd400e6f5f649518660567baf05526114139c09e191cdf9f"),
    (25, "5df1cef5f5ced7cdef9875b188a851c051cf6f099933e6e9f9294b02f39c7a93375e505532f2d162ed7c122f2e5741ef"),
    (26, "a2f2b516f5058e70bcc43d8648ed8879459bce6360e4548eadde26c43da1b72e200bb09023559f6e3c22055cc3bffbcd"),
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
