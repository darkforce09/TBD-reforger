use super::*;

#[test]
fn demangles_doxygen_names() {
    assert_eq!(
        demangle("_s_c_r___base_game_mode_8c_source.html").as_deref(),
        Some("SCR_BaseGameMode.c")
    );
    assert_eq!(
        demangle("_chimera_menu_base_8c_source.html").as_deref(),
        Some("ChimeraMenuBase.c")
    );
}

#[test]
fn parses_source_lines_and_drops_numbers() {
    let html = r#"<div class="line"><a id="l00154" name="l00154"></a><span class="lineno">  154</span>    <span class="keyword">protected</span> void Foo()</div>
<div class="line"><span class="lineno">  155</span>    {</div>"#;
    let lines = parse_page(html);
    assert_eq!(lines.len(), 2);
    assert!(
        lines[0].contains("protected void Foo()"),
        "got {:?}",
        lines[0]
    );
    assert!(
        !lines[0].contains("154"),
        "line number leaked: {:?}",
        lines[0]
    );
    assert_eq!(lines[1].trim(), "{");
}
