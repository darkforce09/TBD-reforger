//! Source pin on the boot overlay's terrain segment: the DEM download is streamed and measured
//! against the length its response announces, so the boot bar moves with the bytes.

/// Source pin on the terrain segment. The DEM is the single biggest thing the boot fetches and
/// the pre-T-628 path pulled it with a plain `fetch_bytes`, which yields one 71.9 MB step at the
/// very end — indistinguishable from a stall for the whole download.
#[test]
fn the_terrain_dem_is_streamed_against_its_content_length() {
    use frontend_test_support::class_r_scrub::{live_code, only_body};
    let src = live_code(
        &[
            frontend_test_support::repository_root::repository_text(
                env!("CARGO_MANIFEST_DIR"),
                "crates/streaming/map_streaming_host/src/lib.rs",
            ),
            "\n",
            frontend_test_support::repository_root::repository_text(
                env!("CARGO_MANIFEST_DIR"),
                "crates/streaming/map_streaming_host/src/queries.rs",
            ),
            "\n",
            frontend_test_support::repository_root::repository_text(
                env!("CARGO_MANIFEST_DIR"),
                "crates/streaming/map_streaming_host/src/map_host.rs",
            ),
            "\n",
            frontend_test_support::repository_root::repository_text(
                env!("CARGO_MANIFEST_DIR"),
                "crates/streaming/map_streaming_host/src/view_preferences.rs",
            ),
            "\n",
            frontend_test_support::repository_root::repository_text(
                env!("CARGO_MANIFEST_DIR"),
                "crates/streaming/map_streaming_host/src/viewport.rs",
            ),
            "\n",
            frontend_test_support::repository_root::repository_text(
                env!("CARGO_MANIFEST_DIR"),
                "crates/streaming/map_streaming_host/src/bootstrap.rs",
            ),
            "\n",
            frontend_test_support::repository_root::repository_text(
                env!("CARGO_MANIFEST_DIR"),
                "crates/streaming/map_streaming_host/src/terrain_load.rs",
            ),
        ]
        .concat(),
    );
    let body = only_body(&src, "async fn load_dem_and_hillshade(");
    assert!(
        body.contains("fetch_bytes_streamed(") && body.contains("BootSeg::Terrain"),
        "the DEM must be fetched through the measured, streamed helper — a whole-body GET has \
         nothing to report until it is already finished"
    );
    assert!(
        !body.contains("fetch_bytes(&format!"),
        "the unmeasured whole-body GET must not come back"
    );

    // The adapter that turns the fetch's byte counts into boot events for one segment.
    let adapter = only_body(&src, "fn boot_byte_progress<");
    assert!(
        adapter.contains("BootEvent::Budget(segment, progress.total"),
        "the budget must be the length this response announced, not a constant and not a guess"
    );
    assert!(
        adapter.contains("BootEvent::Done(") && adapter.contains("progress.received"),
        "progress must be the bytes the body reader handed over — nothing else in the adapter \
         is allowed to be the numerator"
    );

    let fetch = live_code(frontend_test_support::repository_root::repository_text(
        env!("CARGO_MANIFEST_DIR"),
        "crates/foundation/browser_platform/src/fetch.rs",
    ));
    let streamed = only_body(&fetch, "pub async fn fetch_bytes_streamed(");
    assert!(
        streamed.contains("open_streamed_body(url)") && streamed.contains(".read_to_end("),
        "the streamed GET must read its body through the measured reader"
    );
    let opened = only_body(&fetch, "pub async fn open_streamed_body(");
    // `live_code` blanks string literals, so the header NAME cannot be the needle — the shape
    // that survives is "a header off this response, parsed as a number, becomes the length",
    // which is the property that matters anyway.
    assert!(
        opened.contains(".headers()") && opened.contains("parse::<u64>()"),
        "the announced length must be a header read off this response"
    );
    let reading = only_body(&fetch, "pub async fn read_to_end(");
    assert!(
        reading.contains("progress(ByteProgress { received: 0, total })"),
        "the announced length must be reported before the first byte"
    );
    assert!(
        reading.contains("reader.read()")
            && reading.contains("received += u64::from(array.length())")
            && reading.contains("progress(ByteProgress { received, total })"),
        "progress must be the bytes that came out of the body reader — nothing else in this \
         function is allowed to be the numerator"
    );
    for needle in ["Date::now", "set_timeout", "performance"] {
        assert!(
            ![streamed, opened, reading, adapter]
                .iter()
                .any(|b| b.contains(needle)),
            "`{needle}` in the streaming fetch or its adapter would be a bar moving on a clock: \
             the one defect this whole slice is aimed at"
        );
    }
}
