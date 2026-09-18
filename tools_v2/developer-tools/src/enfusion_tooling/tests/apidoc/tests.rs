use super::*;

// Verbatim shape from the real annotated.html.
const ROW: &str = r#"<tr id="row_0_" class="even"><td class="entry"><span class="icona"><span class="icon">C</span></span><a class="el" href="interfaceAABGridMap.html" target="_self">AABGridMap</a></td><td class="desc">Represent a grid map </td></tr>"#;

#[test]
fn parses_index_rows() {
    let rows = parse_index(ROW);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "AABGridMap");
    assert_eq!(rows[0].1, "interfaceAABGridMap.html");
    assert_eq!(rows[0].2, "Represent a grid map");
}

#[test]
fn strips_entities() {
    assert_eq!(text_of("<b>a&#160;b</b>"), "a b");
}

#[test]
fn build_refuses_header_only_classes_tsv() {
    // T-537 Class-R: empty annotated.html must not overwrite committed vanilla_api_*.tsv.
    let dir = std::env::temp_dir().join(format!("t537-apidoc-empty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let src = dir.join("src");
    let out = dir.join("out");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("annotated.html"), "<html><body></body></html>").unwrap();
    // Seed a non-empty committed-shaped out so a silent overwrite would be observable.
    std::fs::create_dir_all(&out).unwrap();
    let classes_path = out.join("vanilla_api_classes.tsv");
    let members_path = out.join("vanilla_api_members.tsv");
    std::fs::write(&classes_path, "class\tdoc_page\nKeepMe\tkeep.html\n").unwrap();
    std::fs::write(&members_path, "class\tsignature\nKeepMe\tvoid Keep()\n").unwrap();
    let before_c = std::fs::read_to_string(&classes_path).unwrap();
    let before_m = std::fs::read_to_string(&members_path).unwrap();

    let err = build(&src, &out).expect_err("must refuse empty apidoc write");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing empty write") && msg.contains("zero classes"),
        "{msg}"
    );
    assert_eq!(
        std::fs::read_to_string(&classes_path).unwrap(),
        before_c,
        "classes TSV must be untouched on refuse"
    );
    assert_eq!(
        std::fs::read_to_string(&members_path).unwrap(),
        before_m,
        "members TSV must be untouched on refuse"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_refuses_header_only_members_tsv() {
    // Classes parse, but no member pages → members TSV would be header-only.
    let dir = std::env::temp_dir().join(format!("t537-apidoc-nomembers-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let src = dir.join("src");
    let out = dir.join("out");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("annotated.html"),
        r#"<tr id="row_0_" class="even"><td class="entry"><a class="el" href="interfaceAABGridMap.html" target="_self">AABGridMap</a></td><td class="desc">grid</td></tr>"#,
    )
    .unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let members_path = out.join("vanilla_api_members.tsv");
    std::fs::write(&members_path, "class\tsignature\nKeepMe\tvoid Keep()\n").unwrap();
    let before_m = std::fs::read_to_string(&members_path).unwrap();

    let err = build(&src, &out).expect_err("must refuse empty members write");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing empty write") && msg.contains("zero member"),
        "{msg}"
    );
    assert_eq!(
        std::fs::read_to_string(&members_path).unwrap(),
        before_m,
        "members TSV must be untouched on refuse"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
