use super::*;
use website_offline_service_worker::offline_pack::terrain_pack;

const ORIGIN: &str = "https://tbd.example";

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|item| (*item).to_owned()).collect()
}

fn everon_manifest() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../assets_v2/terrains/everon/manifest.json"
    );
    std::fs::read_to_string(path).expect("committed Everon manifest")
}

#[test]
fn document_targets_keep_shell_files_and_the_icon_font_and_drop_everything_else() {
    let urls = strings(&[
        "https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@24,400,0,0",
        "/aegis-3e2e77f21a4f8f62.css",
        "/website-frontend-423f29d64f4d9707.js",
        "/website-frontend-423f29d64f4d9707_bg.wasm",
        "/manifest.webmanifest",
        "/website-frontend-423f29d64f4d9707.js#again",
        "/service_worker.js",
        "/offline_service_worker.js",
        "/api/v1/me",
        "https://cdn.example/lib.js",
    ]);
    let targets = document_targets(ORIGIN, &urls);
    let keys: Vec<&str> = targets.iter().map(|target| target.key.as_str()).collect();
    assert_eq!(
        keys,
        [
            "https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@24,400,0,0",
            "https://tbd.example/aegis-3e2e77f21a4f8f62.css",
            "https://tbd.example/website-frontend-423f29d64f4d9707.js",
            "https://tbd.example/website-frontend-423f29d64f4d9707_bg.wasm",
            "https://tbd.example/manifest.webmanifest",
        ]
    );
    assert_eq!(targets[0].class, RequestClass::IconFont);
    assert!(targets[1..]
        .iter()
        .all(|target| target.class == RequestClass::ShellAsset));
}

#[test]
fn catalog_targets_list_the_list_and_every_version_it_names() {
    let list = r#"{"data":[
        {"catalog_id":"vanilla-mortars","catalog_version":1,"title":"t","game_build":"1.4",
         "export_generation_id":"{0}","catalog_sha256":"ab","uploaded_at":"2026-09-28T00:00:00Z"},
        {"catalog_id":"vanilla-mortars","catalog_version":2,"title":"t","game_build":"1.4",
         "export_generation_id":"{0}","catalog_sha256":"cd","uploaded_at":"2026-09-28T00:00:00Z"}]}"#;
    let targets = catalog_targets(ORIGIN, list).expect("list");
    let described: Vec<(&str, RequestClass)> = targets
        .iter()
        .map(|target| (target.key.as_str(), target.class))
        .collect();
    assert_eq!(
        described,
        [
            (
                "https://tbd.example/api/v1/ballistics-catalogs",
                RequestClass::CatalogList
            ),
            (
                "https://tbd.example/api/v1/ballistics-catalogs/vanilla-mortars/versions/1",
                RequestClass::CatalogVersion
            ),
            (
                "https://tbd.example/api/v1/ballistics-catalogs/vanilla-mortars/versions/2",
                RequestClass::CatalogVersion
            ),
        ]
    );
    let empty = catalog_targets(ORIGIN, r#"{"data":[]}"#).expect("empty list");
    assert_eq!(empty.len(), 1);
}

#[test]
fn catalog_targets_refuse_a_malformed_list_or_an_id_that_leaves_the_version_route() {
    assert!(catalog_targets(ORIGIN, "not json").is_err());
    assert!(catalog_targets(ORIGIN, r#"{"items":[]}"#).is_err());
    let escaping = r#"{"data":[{"catalog_id":"a/b","catalog_version":1}]}"#;
    assert!(catalog_targets(ORIGIN, escaping).is_err());
}

#[test]
fn icon_font_file_targets_read_every_gstatic_url_once() {
    let css = "@font-face { font-family: 'Material Symbols Outlined'; \
        src: url(https://fonts.gstatic.com/s/materialsymbolsoutlined/v1/a.woff2) format('woff2'); }\n\
        @font-face { src: url('https://fonts.gstatic.com/s/materialsymbolsoutlined/v1/b.woff2'); }\n\
        @font-face { src: url(\"https://fonts.gstatic.com/s/materialsymbolsoutlined/v1/a.woff2\"); }\n\
        .x { background: url(https://evil.example/tracker.png); }\n\
        .y { background: url(data:image/png;base64,AAAA); }";
    let targets = icon_font_file_targets(ORIGIN, css);
    let keys: Vec<&str> = targets.iter().map(|target| target.key.as_str()).collect();
    assert_eq!(
        keys,
        [
            "https://fonts.gstatic.com/s/materialsymbolsoutlined/v1/a.woff2",
            "https://fonts.gstatic.com/s/materialsymbolsoutlined/v1/b.woff2",
        ]
    );
    assert!(targets
        .iter()
        .all(|target| target.class == RequestClass::IconFont));
    assert!(icon_font_file_targets(ORIGIN, "url(unterminated").is_empty());
}

#[test]
fn terrain_targets_cover_the_whole_everon_pack_as_map_assets() {
    let index = r#"{"schemaVersion":1,"terrainId":"everon","tiles":[
        {"z":0,"x":0,"y":0,"bytes":100},{"z":1,"x":1,"y":0,"bytes":200},
        {"z":2,"x":0,"y":3,"bytes":300},{"z":3,"x":0,"y":0,"bytes":1},
        {"z":4,"x":0,"y":0,"bytes":1},{"z":5,"x":0,"y":0,"bytes":1},{"z":6,"x":0,"y":0,"bytes":1}]}"#;
    let pack = terrain_pack(MAP_ASSETS_ROOT, &everon_manifest(), Some(index)).expect("pack");
    assert!(pack.is_complete());
    let targets = terrain_targets(ORIGIN, &pack);
    assert_eq!(targets.len(), pack.entries.len());
    assert!(targets
        .iter()
        .all(|target| target.class == RequestClass::MapAsset));
    assert_eq!(
        targets[0].key,
        "https://tbd.example/map-assets/everon/manifest.json"
    );
    assert!(targets.iter().any(|target| target.key
        == "https://tbd.example/map-assets/everon/satellite/everon-sat.tbd-sat"
        && target.expected_bytes == Some(152_713_114)));
    assert!(targets.iter().any(|target| target.key
        == "https://tbd.example/map-assets/everon/tiles/map/2/0/3.webp"
        && target.expected_bytes == Some(300)));
    assert!(!targets
        .iter()
        .any(|target| target.key.contains("/tiles/satellite/")));
    assert_eq!(declared_bytes(&targets), 152_713_114 + 100 + 200 + 300 + 4);
}

#[test]
fn target_for_refuses_passthrough_urls() {
    assert_eq!(
        target_for(
            ORIGIN,
            "/map-assets/everon/tiles/satellite/0/0/0.webp",
            None
        ),
        None
    );
    assert_eq!(target_for(ORIGIN, "/api/v1/events", None), None);
    assert_eq!(target_for(ORIGIN, "https://other.example/x.js", None), None);
    assert_eq!(target_for("not an origin", "/x.js", None), None);
    assert_eq!(
        target_for(ORIGIN, "/map-assets/everon/dem/everon.png", Some(5)),
        Some(OfflineTarget {
            key: "https://tbd.example/map-assets/everon/dem/everon.png".into(),
            class: RequestClass::MapAsset,
            expected_bytes: Some(5),
        })
    );
}

#[test]
fn the_catalog_list_and_version_targets_are_the_keys_catalog_targets_writes() {
    let list = catalog_list_target(ORIGIN).expect("the list target");
    assert_eq!(list.class, RequestClass::CatalogList);
    assert_eq!(list.key, format!("{ORIGIN}/api/v1/ballistics-catalogs"));
    let version = catalog_version_target(ORIGIN, "vanilla_mortars", 2).expect("a version");
    assert_eq!(version.class, RequestClass::CatalogVersion);
    assert_eq!(
        version.key,
        format!("{ORIGIN}/api/v1/ballistics-catalogs/vanilla_mortars/versions/2")
    );
    let written = catalog_targets(
        ORIGIN,
        r#"{"data":[{"catalog_id":"vanilla_mortars","catalog_version":2}]}"#,
    )
    .expect("a readable list");
    assert_eq!(written, vec![list, version]);
    assert_eq!(catalog_version_target(ORIGIN, "../fire-missions", 1), None);
}
