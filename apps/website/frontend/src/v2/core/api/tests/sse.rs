//! The stream must tear itself down when the page goes away, on both sides of the seam.

use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};

/// The stream must abort when the page goes away, and a comment saying so is not enough.
///
/// The whole invariant lives inside the browser-only body: an abort controller, the request
/// wired to its signal, and the handle parked in a thread-local. There is no runtime signature
/// a native harness could observe without standing up a fake browser, and a test that asserts
/// against its own mock of `fetch` proves nothing about the real one — so this reads the source
/// instead.
///
/// The positive assertions run on scrubbed code, with comments and string literals blanked and
/// unreachable items removed, so a needle only counts when it is a call the build can reach.
/// Every needle below also appears in this file's own prose, which is exactly why reading the
/// raw text would not do: the function could be deleted outright and the paragraph describing
/// it would keep the assertions green. The negative assertions deliberately do read the raw
/// file, because what they ban is stale prose, and prose is what the scrubber removes.
#[test]
fn class_r_sse_abort_teardown_exists() {
    let src = crate::v2::core::test_support::pins::sse_source();
    let src: &str = &src;
    const INTEL: &str = include_str!("../../../../pages/public/server_intel.rs");
    let production = live_code(src);
    // The page-side needle is scrubbed too, so commenting out the live cleanup registration
    // while leaving its text in a comment fails rather than passes.
    let intel_code = live_code(INTEL);

    assert_eq!(
        super::SSE_ABORT_CLEANUP_FN,
        "abort_server_status_stream",
        "cleanup fn name pin drifted from the const"
    );
    // Native no-op call — proves the Send+Sync seam is reachable outside wasm.
    super::abort_server_status_stream();

    // Scoped to the one function that owns the transport. A whole-file `contains` is satisfied
    // by a match ANYWHERE, including in a second, dead copy of the fetch — `only_body` refuses
    // two definitions rather than silently reading the first.
    let stream = only_body(&production, "pub fn stream_server_status(");
    assert!(
        stream.contains("AbortController"),
        "stream_server_status must create an AbortController for the fetch — on a live path, \
         not in the paragraph that explains why it needs one"
    );
    assert!(
        stream.contains("init.set_signal(Some(&signal))"),
        "RequestInit must wire the AbortController signal via init.set_signal(Some(&signal)); \
         an AbortController the fetch never receives aborts nothing"
    );
    assert!(
        stream.contains("SSE_ABORT.with("),
        "the controller must be parked in the thread_local (the !Send workaround), or \
         route-leave has nothing to take"
    );
    assert!(
        stream.contains("abort_server_status_stream"),
        "a re-subscribe must abort the prior controller first, or a remount leaks a stream"
    );

    let abort = only_body(&production, "pub fn abort_server_status_stream");
    assert!(
        abort.contains("SSE_ABORT.with(") && abort.contains(".abort"),
        "the zero-capture entry point must actually take the parked controller and abort it"
    );

    // The negatives read the raw production text on purpose: what they ban is stale
    // documentation, and documentation is the first thing the scrubber removes. The tests live in
    // their own file, so the production text carries none of this test's own assertion strings.
    let prose = src;
    assert!(
        !prose.contains("NOT torn down on SPA nav"),
        "documentation claiming the stream leaks on navigation must not survive the fix"
    );
    assert!(
        !prose.contains("navigation leaks at most one"),
        "documentation claiming the stream leaks on navigation must not survive the fix"
    );
    assert!(
        intel_code.contains("on_cleanup(crate::v2::core::api::sse::abort_server_status_stream)")
            || intel_code.contains("on_cleanup(abort_server_status_stream)"),
        "ServerIntelInner must register abort_server_status_stream under on_cleanup \
         (live production line — comment-only string is not enough)"
    );
}

/// **The pin above, pinned.** Each attack is applied to a copy of this file's own source and
/// the scrubbed result must no longer satisfy the needle — so a future edit that weakens the
/// scrubbing shows up here rather than as a quiet green.
///
/// This is the cheap generic form of the calibration `mission_title_prefer` gets for free by
/// executing the code: prove the instrument can still say NO.
#[test]
fn the_teardown_pin_rejects_every_dead_code_wrapper() {
    let needle = "init.set_signal(Some(&signal))";
    let attacks: [(&str, String); 12] = [
        (
            "if true == false",
            format!("if true == false {{ {needle}; }}"),
        ),
        ("loop { break; … }", format!("loop {{ break; {needle}; }}")),
        (
            "#[cfg(any())]",
            format!("#[cfg(any())] fn d() {{ {needle}; }}"),
        ),
        ("while false", format!("while false {{ {needle}; }}")),
        ("if !true", format!("if !true {{ {needle}; }}")),
        ("if 1 > 2", format!("if 1 > 2 {{ {needle}; }}")),
        (
            "if std::hint::black_box(false)",
            format!("if std::hint::black_box(false) {{ {needle}; }}"),
        ),
        (
            "const C: bool = false; if C",
            format!("const C: bool = false;\nfn d {{ if C {{ {needle}; }} }}"),
        ),
        ("return; above", format!("fn d() {{ return; {needle}; }}")),
        (
            "#[cfg(any())] mod shadow",
            format!("#[cfg(any())] mod shadow {{ fn d() {{ {needle}; }} }}"),
        ),
        (
            "match guard",
            format!("match  {{ _ if false => {{ {needle}; }} _ => {{}} }}"),
        ),
        ("comment", format!("// {needle}")),
    ];
    for (label, body) in attacks {
        let forged = format!("fn stream_server_status() {{\n    {body}\n}}\n#[cfg(test)]\n");
        assert!(
            !live_code(&forged).contains(needle),
            "{label}: the signal-wiring needle survived scrubbing, so this pin would report a \
             live abort wire over code the build never runs"
        );
    }
    // The honest wiring still reads as present, or the assertions above pin nothing.
    let live = format!("fn stream_server_status() {{\n    {needle};\n}}\n#[cfg(test)]\n");
    assert!(live_code(&live).contains(needle));
}
