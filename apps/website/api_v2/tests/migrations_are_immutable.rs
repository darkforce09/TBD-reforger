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
    (27, "3c5d4fc0f139e6f5c03e345df886583f7d9bde97a868f0269593a7e4e1360e4ce9599b8c6ed2e86f74223a8273a6a6c8"),
    (28, "351998cfccf58f627628eec5aa2a249e2d7c3f825ac281526607d15493a2162d301fc9040282f5f0cf0a76a434fcd195"),
    (29, "881d5c8681cb1a036cc83ecd5e75e782a99e86f6252f0aa57a33b6958fbfc4f461cb72c212b6202ec93f28dd14e8d4c6"),
    (30, "cd26c7b9aecfac69c0b6ad4c6ae7341c5a99626221f98a60012accd2eb7517dfdd81947cf0c0825629713f11f5a770ea"),
    (31, "120e870cb93198e8e3361c39be26a4cff621930dc65cd35c076c693898fe54d1d90f725986ab0f57a5f0a664ff4b8c4a"),
    (32, "ed24b391d913d367d73b65361237474a4e4a0fa9724ae6f29a3a1c0e4065fa80b2ad66e72f1ddde37c576f188b438a24"),
    (33, "ac1b26fe88e5281ae7059caa73327269151276bbc314a6b551a1bf8526ed6638f3d25d8c35c892142bfd0c69eceb157e"),
    (34, "1e313efb2d0df84957c89b53a39a4da66add4a7764209f6308a73b3dd81acadf6e74d7a7c987512923e3b318ebb06c84"),
    (35, "0e259295f7f4f36f3c365e93d465c8dcb6b63e9a75e109114febc776cdba5e79559b1e128cee01007ea61a21364c32f3"),
    (36, "d376d684d5d03243a8f2e44ddfefe91d6f348e125fba92a6c8185f92d266dff5aebc798a664a423b09ead1149fe0b138"),
    (37, "5782f99f230e57d01e1d101da1494de109628108c60f4fb41a7a01acf1af958860e765f95e28852afb2bb4eae7f683d8"),
    (38, "9607ca0d7a1903cb13c4fb6fc7184a7a52c8da45c7e4104b3e73668f9b3f4c413859d4b70013518ed87a302ecaaf4119"),
    (39, "32cc62adf66670ba3a6c5629a0717cfde829efac49d798f5679baf893cd5fe4e3c6ce2139885f387dce9b65557a1a9b5"),
    (40, "f896941d1f2d4493dfd787e175c6be23a7de4f9496e8ec12b359976b597a26425bfcb8d6a305a50defa24a8d529e8e80"),
    (41, "57a21a76fb106c43e98ca26a22ab6ac441e664fb69934a18960d20901cea9dc2d79c57f116a6a71c36e2b23573620618"),
    (42, "087f3518dc9c16fe276198b354ae523cc8cc1da5b4ca24a72656fcf6bd38857a5b7ec642a9cc5693ab0aef65dec7ada8"),
    (43, "ef6984806d1e4b5840b295a21ddc09f921b519964da14010ce2c193590ff8f8f6aab6bcf72157ca75784adc4f2a150b7"),
    (44, "2798300bf9f4c02d6e45b86cc38b99aca0a66f850923f9dccb0c41c1bad49ac6f377d74cd993bc4a1a41d6c44efc8ae8"),
    (45, "2c7fb519e03b734f54b603a2e5de492241799d5d7e864eb64b9af0e9e5789aee6474a38c444284f50a50d5d5828368e5"),
    (46, "c528b536e81605051190e241b8333ddac57302ca50cbcb22f3db26018ff3fb93ddf7e3ede2cf23db2eadf9197dba015b"),
    (47, "5adc4486abc9fc6e8b4bcb2b00d53b935a016898bb56413abf83302c076cf663c4453bfef346c5d237c030f9cc4fb755"),
    (48, "dd11db3ab4f6d4f9685bfeb2f9016bb3dd3983c30c353b84d0a458db2c1c7f3b1a345b227c5d26c6ae02e2ac0b8e87f3"),
    (49, "3fbf1b1a3474e51f2d81eea2625cd7d531392c6f2addc79aa9fb25abdb037a1a640cf05069f63f70f1f3ad2c8082f7db"),
    (50, "6a23656366b0a004dee6093d6283b50dc24b1c839c1573804760c8928433e2d0ec43e5df846277f826d826280e32529d"),
    (51, "798c5e829fd6e2f7fc276d1c9689e5e032eaadfc81e56143a2db2044659e8d5b40d6fe45a955c2582b5f2ffb0a5cbb27"),
    (52, "da165cd16fc461c76c72bedfd4bd998fcb8cc2c6eb305ac8ad90c010c83d36600bdc86569a240dbd8d96438499fcd9da"),
    (53, "56a25b1dd2303b10cfdf12e6868098e5655b28e57f8bab73d088d5e79b6eae332e8653ab9180d59b9dc7a5ef1a4fcae6"),
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

/// Retired versions remain retired; newly pinned migrations append after the accepted head.
fn validate_versions(versions: &[i64]) -> Result<(), &'static str> {
    const HISTORICAL_HEAD: i64 = 27;
    if versions.is_empty() || !versions.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err("migration versions must be nonempty, unique and strictly increasing");
    }
    if versions.iter().any(|version| (22..25).contains(version)) {
        return Err("migration versions 0022, 0023 and 0024 are retired and cannot be filled");
    }
    let historical: Vec<_> = (1..=21).chain(25..=HISTORICAL_HEAD).collect();
    if versions
        .iter()
        .copied()
        .filter(|v| *v <= HISTORICAL_HEAD)
        .collect::<Vec<_>>()
        != historical
    {
        return Err("historical migration versions must be preserved");
    }
    Ok(())
}

#[test]
fn migration_versions_preserve_the_historical_gap_and_append_after_the_head() {
    let versions: Vec<_> = embedded().migrations.iter().map(|m| m.version).collect();
    validate_versions(&versions).unwrap();
    validate_versions(&PINNED.iter().map(|(v, _)| *v).collect::<Vec<_>>()).unwrap();
}

#[test]
fn migration_version_guard_rejects_gap_fill_duplicates_and_reordering() {
    let historical: Vec<_> = (1..=21).chain(25..=27).collect();
    assert!(validate_versions(&historical).is_ok());
    for retired in 22..25 {
        let mut invalid = historical.clone();
        invalid.push(retired);
        invalid.sort_unstable();
        assert!(validate_versions(&invalid).is_err());
    }
    let mut duplicate = historical.clone();
    duplicate.push(27);
    assert!(validate_versions(&duplicate).is_err());
    let mut reordered = historical.clone();
    reordered.extend([29, 28]);
    assert!(validate_versions(&reordered).is_err());
    let mut appended = historical;
    appended.extend([28, 29]);
    assert!(validate_versions(&appended).is_ok());
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
