use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn flatten_replaces_every_slash() {
    assert_eq!(
        flatten_rel(Path::new("data/sub/Nested.PAK")),
        "data_sub_Nested.PAK"
    );
    assert_eq!(flatten_rel(Path::new("core/Engine.pak")), "core_Engine.pak");
}

#[test]
fn is_pak_case_insensitive() {
    assert!(is_pak_iname(Path::new("x.pak")));
    assert!(is_pak_iname(Path::new("x.PAK")));
    assert!(is_pak_iname(Path::new("x.Pak")));
    assert!(!is_pak_iname(Path::new("x.txt")));
}

#[test]
fn clean_tree_links_flattened_names() {
    let base = tempfile_dir("clean");
    let game = base.join("game");
    let fake = base.join("fake");
    seed_game(&game, true);
    let code = run_with_paths(&game, &fake).unwrap();
    assert_eq!(code, 0);
    assert!(fake.join("addons/data").is_dir());
    assert_eq!(
        fs::read_link(fake.join("addons/core_Engine.pak")).unwrap(),
        game.join("addons/core/Engine.pak")
    );
    assert_eq!(
        fs::read_link(fake.join("addons/data_Base.pak")).unwrap(),
        game.join("addons/data/Base.pak")
    );
    assert_eq!(
        fs::read_link(fake.join("addons/data_sub_Nested.PAK")).unwrap(),
        game.join("addons/data/sub/Nested.PAK")
    );
    // non-pak ignored
    assert!(!fake.join("addons/data_readme.txt").exists());
}

#[test]
fn missing_addons_dir_exits_1() {
    let base = tempfile_dir("no-addons");
    let game = base.join("game");
    let fake = base.join("fake");
    fs::create_dir_all(&game).unwrap();
    let code = run_with_paths(&game, &fake).unwrap();
    assert_eq!(code, 1);
    assert!(!fake.exists(), "must not create FAKE on broken arm");
}

#[test]
fn addons_as_file_exits_1() {
    let base = tempfile_dir("addons-file");
    let game = base.join("game");
    let fake = base.join("fake");
    fs::create_dir_all(&game).unwrap();
    fs::write(game.join("addons"), b"not a dir\n").unwrap();
    let code = run_with_paths(&game, &fake).unwrap();
    assert_eq!(code, 1);
}

#[test]
fn empty_addons_links_zero() {
    let base = tempfile_dir("empty");
    let game = base.join("game");
    let fake = base.join("fake");
    fs::create_dir_all(game.join("addons")).unwrap();
    let code = run_with_paths(&game, &fake).unwrap();
    assert_eq!(code, 0);
    assert!(fake.join("addons/data").is_dir());
}

fn seed_game(game: &Path, with_paks: bool) {
    fs::create_dir_all(game.join("addons/data/sub")).unwrap();
    fs::create_dir_all(game.join("addons/core")).unwrap();
    if with_paks {
        fs::write(game.join("addons/data/Base.pak"), b"pak1\n").unwrap();
        fs::write(game.join("addons/core/Engine.pak"), b"pak2\n").unwrap();
        fs::write(game.join("addons/data/sub/Nested.PAK"), b"pak3\n").unwrap();
        fs::write(game.join("addons/data/readme.txt"), b"ignore\n").unwrap();
    }
    // make sure modes are ordinary files
    let _ = fs::set_permissions(
        game.join("addons/data/Base.pak"),
        fs::Permissions::from_mode(0o644),
    );
}

fn tempfile_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "t876-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}
