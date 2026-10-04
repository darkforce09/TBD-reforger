use super::*;

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn build_id_comes_from_the_first_app_bundle_hash() {
    let urls = strings(&[
        "https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined",
        "/aegis-3e2e77f21a4f8f62.css",
        "/frontend-423f29d64f4d9707.js",
        "/frontend-ffffffffffffffff_bg.wasm",
    ]);
    let build = build_id_from_asset_urls(&urls);
    assert_eq!(build.as_str(), "423f29d64f4d9707");
    assert_eq!(
        build.script_url(),
        "/service_worker.js?build=423f29d64f4d9707"
    );
}

#[test]
fn build_id_reads_the_wasm_module_and_absolute_urls() {
    let urls = strings(&["https://tbd.example/frontend-0a1b_bg.wasm?x=1"]);
    assert_eq!(build_id_from_asset_urls(&urls).as_str(), "0a1b");
}

#[test]
fn build_id_without_a_well_formed_bundle_hash_is_unversioned() {
    for urls in [
        strings(&[]),
        strings(&["/frontend-.js"]),
        strings(&["/frontend-not_hex.js"]),
        strings(&["/frontend-423f.css"]),
        strings(&["/other-423f29d64f4d9707.js"]),
    ] {
        assert_eq!(
            build_id_from_asset_urls(&urls).as_str(),
            BuildId::UNVERSIONED,
            "{urls:?}"
        );
    }
}
