use super::blueprint_source_tests::synth_pak;
use super::*;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

struct GameDirectory(PathBuf);

impl GameDirectory {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("pak-policy-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(path.join("addons")).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, bytes: &[u8]) {
        fs::write(self.0.join("addons").join(name), bytes).unwrap();
    }
    fn blueprint(&self) -> anyhow::Result<PakSet> {
        PakSet::from_dir(&self.0.join("addons"))
    }
    fn world(&self) -> PakVfs {
        PakVfs::open(&self.0).unwrap()
    }
}

impl Drop for GameDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Locate the record of a single top-level file in the synthetic directory.
fn record(bytes: &[u8], name: &str) -> usize {
    bytes
        .windows(name.len())
        .rposition(|part| part == name.as_bytes())
        .unwrap()
        + name.len()
}

#[test]
fn lookup_and_duplicate_policies_remain_distinct() {
    let game = GameDirectory::new("lookup");
    game.write(
        "b.pak",
        &synth_pak(&[("Prefabs/Door.et", b"second", false)]),
    );
    game.write(
        "a.PAK",
        &synth_pak(&[
            ("Prefabs/Door.et", b"first", false),
            ("prefabs/door.et", b"lowercase", false),
        ]),
    );
    let blueprint = game.blueprint().unwrap();
    let world = game.world();
    assert_eq!(blueprint.read("/PREFABS\\Door.et").unwrap(), b"first");
    assert_eq!(world.read_file("/Prefabs//Door.et/").unwrap(), b"first");
    assert_eq!(world.read_file("prefabs/door.et").unwrap(), b"lowercase");
    assert!(!world.exists("PREFABS/Door.et"));
    assert!(!blueprint.exists("Prefabs//Door.et/"));
    let mut paths = world.all_file_paths();
    paths.sort();
    assert_eq!(paths, ["Prefabs/Door.et", "prefabs/door.et"]);
}

#[test]
fn raw_deflate_is_world_only_and_metadata_is_preserved() {
    let payload = b"raw deflate payload for game scripts";
    let mut compressor =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
    compressor.write_all(payload).unwrap();
    let compressed = compressor.finish().unwrap();
    let mut bytes = synth_pak(&[("script.c", &compressed, false)]);
    let at = record(&bytes, "script.c");
    bytes[at + 8..at + 12].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes[at + 12..at + 18].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
    bytes[at + 18] = 1;
    let game = GameDirectory::new("raw");
    game.write("scripts.pak", &bytes);
    let world = game.world();
    assert_eq!(world.read_file("script.c").unwrap(), payload);
    assert_eq!(
        world.read_raw("script.c").unwrap(),
        (compressed, payload.len() as u32)
    );
    assert_eq!(world.entry_data_start("script.c"), Some(31));
    assert_eq!(
        world.entry_method("script.c"),
        Some(([1, 2, 3, 4, 5, 6], true))
    );
    assert!(game.blueprint().unwrap().read("script.c").is_err());
}

#[test]
fn decompressed_length_checks_follow_the_consumer_policy() {
    let mut bytes = synth_pak(&[("entry", b"payload", true)]);
    let at = record(&bytes, "entry");
    bytes[at + 8..at + 12].copy_from_slice(&99u32.to_le_bytes());
    let game = GameDirectory::new("length");
    game.write("data.pak", &bytes);
    assert_eq!(game.world().read_file("entry").unwrap(), b"payload");
    assert!(
        game.blueprint()
            .unwrap()
            .read("entry")
            .unwrap_err()
            .to_string()
            .contains("directory says 99")
    );
}

#[test]
fn malformed_archives_fail_blueprint_and_are_skipped_by_world() {
    let game = GameDirectory::new("malformed");
    game.write("a.pak", b"FORM");
    game.write("b.pak", &synth_pak(&[("valid", b"yes", false)]));
    assert!(game.blueprint().is_err());
    assert_eq!(game.world().read_file("valid").unwrap(), b"yes");
    let bytes = synth_pak(&[("file", b"payload", true)]);
    for end in 0..bytes.len() {
        assert!(
            parse_pak_bytes(&bytes[..end]).is_err(),
            "truncation at {end}"
        );
    }
}

#[test]
fn payload_truncation_and_missing_loose_fallback_are_errors() {
    let game = GameDirectory::new("truncated");
    game.write("data.pak", &synth_pak(&[("entry", b"data", false)]));
    let blueprint = game.blueprint().unwrap();
    let world = game.world();
    fs::write(game.0.join("addons/data.pak"), b"FORM").unwrap();
    assert!(blueprint.read("entry").is_err());
    assert!(world.read_file("entry").is_err());
    let loose = game.0.join("loose");
    fs::create_dir(&loose).unwrap();
    fs::write(loose.join("entry"), b"fallback").unwrap();
    fs::write(loose.join("unique"), b"loose").unwrap();
    let layered = LayeredSource {
        layers: vec![Box::new(blueprint), Box::new(DirSource { root: loose })],
    };
    // Corruption in the chosen layer must not silently select another asset.
    assert!(layered.read("entry").is_err());
    assert_eq!(layered.read("unique").unwrap(), b"loose");
    assert!(layered.read("missing").is_err());
}
