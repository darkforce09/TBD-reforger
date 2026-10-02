use super::*;

#[test]
fn package_name_reads_only_the_package_table() {
    let dir = std::env::temp_dir().join(format!("t853-pkg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"website-api\"\nedition = \"2024\"\n\n[dependencies]\nname = \"wrong\"\n",
    )
    .unwrap();
    assert_eq!(
        package_name(&dir.display().to_string()),
        Some("website-api".into())
    );
    let _ = std::fs::remove_dir_all(&dir);
}
