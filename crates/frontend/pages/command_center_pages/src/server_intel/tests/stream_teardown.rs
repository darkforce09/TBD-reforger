//! The page side of the status stream's teardown: leaving the route aborts the stream.

use frontend_test_support::class_r_scrub::live_code;

/// The panel must register the stream's abort under `on_cleanup`, so leaving the route tears the
/// stream down; the transport's own tests pin that the abort really aborts.
///
/// The needle is read on scrubbed code, with comments and string literals blanked and unreachable
/// items removed, so commenting out the live cleanup registration while leaving its text in a
/// comment fails rather than passes.
#[test]
fn server_intel_registers_the_stream_abort_on_cleanup() {
    let intel = crate::source_pins::server_intel_source();
    let intel_code = live_code(&intel);
    assert!(
        intel_code.contains("on_cleanup(frontend_transport::sse::abort_server_status_stream)")
            || intel_code.contains("on_cleanup(abort_server_status_stream)"),
        "ServerIntelInner must register abort_server_status_stream under on_cleanup \
         (live production line — comment-only string is not enough)"
    );
}
