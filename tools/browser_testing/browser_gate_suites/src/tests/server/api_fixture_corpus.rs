use super::*;
use crate::server::{ServeConfig, start_server};
use std::path::PathBuf;

/// A fresh directory under the system temporary directory, unique to this test process and tag.
fn scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tbd-gate-corpus-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn mortar_offline_corpus_file_name_joins_the_path_after_api_v1() {
    assert_eq!(
        corpus_file_name(&Method::GET, "/api/v1/ballistics-catalogs").as_deref(),
        Some("GET__ballistics-catalogs.json")
    );
    assert_eq!(
        corpus_file_name(
            &Method::GET,
            "/api/v1/ballistics-catalogs/vanilla_mortars/versions/1/"
        )
        .as_deref(),
        Some("GET__ballistics-catalogs__vanilla_mortars__versions__1.json")
    );
    assert_eq!(
        corpus_file_name(&Method::POST, "/api/v1/fire-missions").as_deref(),
        Some("POST__fire-missions.json")
    );
}

#[test]
fn mortar_offline_corpus_file_name_refuses_names_outside_the_corpus() {
    for path in [
        "/api/v2/ballistics-catalogs",
        "/map-assets/everon/manifest.json",
        "/api/v1/",
        "/api/v1/ballistics-catalogs/..%2F..%2Fsecret",
        "/api/v1/ballistics-catalogs/../../secret",
        "/api/v1/a b",
    ] {
        assert_eq!(corpus_file_name(&Method::GET, path), None, "{path}");
    }
}

#[tokio::test]
async fn mortar_offline_corpus_route_serves_recorded_reads_and_refuses_the_rest() {
    let corpus = scratch_dir("corpus");
    let dist = scratch_dir("dist");
    let recorded = br#"{"catalogs":[{"catalog_id":"x","catalog_version":1}]}"#;
    std::fs::write(corpus.join("GET__ballistics-catalogs.json"), recorded).unwrap();
    std::fs::write(dist.join("index.html"), b"<html>app</html>").unwrap();
    let server = start_server(
        ServeConfig {
            dir: dist.clone(),
            api_proxy: None,
            map_assets: None,
            api_fixture_corpus: Some(corpus.clone()),
        },
        0,
    )
    .await
    .unwrap();
    let origin = format!("http://127.0.0.1:{}", server.port);
    let client = crate::http_client::new_http_client();

    let hit = client
        .get(format!("{origin}/api/v1/ballistics-catalogs?fresh=1"))
        .send()
        .await
        .unwrap();
    assert_eq!(hit.status().as_u16(), 200);
    assert_eq!(
        hit.headers()["content-type"].to_str().unwrap(),
        "application/json"
    );
    assert_eq!(
        hit.headers()["cross-origin-embedder-policy"]
            .to_str()
            .unwrap(),
        "credentialless"
    );
    assert_eq!(hit.bytes().await.unwrap().as_ref(), recorded);

    let miss = client
        .get(format!("{origin}/api/v1/ballistics-catalogs/x/versions/1"))
        .send()
        .await
        .unwrap();
    assert_eq!(miss.status().as_u16(), 404);
    assert!(
        miss.text()
            .await
            .unwrap()
            .contains("GET__ballistics-catalogs__x__versions__1.json")
    );

    let post = client
        .post(format!("{origin}/api/v1/ballistics-catalogs"))
        .send()
        .await
        .unwrap();
    assert_eq!(post.status().as_u16(), 404);

    let app = client
        .get(format!("{origin}/tools/mortar"))
        .send()
        .await
        .unwrap();
    assert_eq!(app.status().as_u16(), 200);
    assert_eq!(app.text().await.unwrap(), "<html>app</html>");

    server.close().await;
    let _ = std::fs::remove_dir_all(&corpus);
    let _ = std::fs::remove_dir_all(&dist);
}

/// Starts a gate server over a fresh corpus holding the recorded catalog list and a fresh app;
/// returns the server and the two directories.
async fn corpus_server(tag: &str) -> (crate::server::RunningServer, PathBuf, PathBuf) {
    let corpus = scratch_dir(&format!("{tag}-corpus"));
    let dist = scratch_dir(&format!("{tag}-dist"));
    std::fs::write(
        corpus.join("GET__ballistics-catalogs.json"),
        br#"{"data":[]}"#,
    )
    .unwrap();
    std::fs::write(dist.join("index.html"), b"<html>app</html>").unwrap();
    let server = start_server(
        ServeConfig {
            dir: dist.clone(),
            api_proxy: None,
            map_assets: None,
            api_fixture_corpus: Some(corpus.clone()),
        },
        0,
    )
    .await
    .unwrap();
    (server, corpus, dist)
}

#[tokio::test]
async fn mortar_offline_corpus_route_answers_502_while_its_api_is_down_and_only_its_own() {
    let (down_server, down_corpus, down_dist) = corpus_server("down").await;
    let (up_server, up_corpus, up_dist) = corpus_server("up").await;
    let client = crate::http_client::new_http_client();
    let list = |port: u16| format!("http://127.0.0.1:{port}/api/v1/ballistics-catalogs");

    let before = client.get(list(down_server.port)).send().await.unwrap();
    assert_eq!(before.status().as_u16(), 200);
    assert!(
        before.headers().contains_key("date"),
        "a saved copy is dated by the Date header the server sends"
    );

    set_api_down(&down_corpus, true);
    assert!(is_api_down(&down_corpus));
    assert!(!is_api_down(&up_corpus));
    for path in [
        "/api/v1/ballistics-catalogs",
        "/api/v1/ballistics-catalogs/x/versions/1",
    ] {
        let down = client
            .get(format!("http://127.0.0.1:{}{path}", down_server.port))
            .send()
            .await
            .unwrap();
        assert_eq!(down.status().as_u16(), 502, "{path}");
        assert!(down.bytes().await.unwrap().is_empty(), "{path}");
    }
    let app = client
        .get(format!(
            "http://127.0.0.1:{}/tools/mortar",
            down_server.port
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(app.status().as_u16(), 200, "the proxy still serves the app");
    let other = client.get(list(up_server.port)).send().await.unwrap();
    assert_eq!(other.status().as_u16(), 200, "another corpus stays up");

    set_api_down(&down_corpus, false);
    assert!(!is_api_down(&down_corpus));
    let after = client.get(list(down_server.port)).send().await.unwrap();
    assert_eq!(after.status().as_u16(), 200);

    down_server.close().await;
    up_server.close().await;
    for dir in [down_corpus, down_dist, up_corpus, up_dist] {
        let _ = std::fs::remove_dir_all(dir);
    }
}
