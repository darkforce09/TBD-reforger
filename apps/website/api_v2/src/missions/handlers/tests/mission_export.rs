//! Source and response pins for the `/compiled` boundary: the catalogued compile gate, and the
//! structured diagnostics that must ride alongside the bytes rather than be dropped.

use super::*;

const SRC: &str = include_str!("../mission_export.rs");

/// The production half of this handler file — everything before its sibling-test declaration.
fn production_half() -> &'static str {
    SRC.split("#[cfg(test)]")
        .next()
        .expect("mission_export.rs must declare a sibling test module")
}

/// live `/compiled` must load the Save phys catalog and compile through the catalogued
/// gate — the empty-catalog `flatten_to_mod_document` would let an over-capacity version ship.
/// RED: swap back to the no-arg flatten, or drop the catalog load.
#[test]
fn compiled_route_loads_cargo_phys_catalog() {
    let production = production_half();
    let start = production
        .find("pub async fn get_compiled_mission(")
        .expect("get_compiled_mission must exist");
    let after = &production[start..];
    // Everything from the route down to the private `unreadable_stored_payload` helper.
    let body = after
        .split("\nfn unreadable_stored_payload(")
        .next()
        .expect("get_compiled_mission must precede unreadable_stored_payload");
    assert!(
        body.contains("load_cargo_phys_catalog"),
        "/compiled must load registry phys into the catalog; got:\n{body}"
    );
    assert!(
        body.contains("flatten_to_mod_document_with_catalog("),
        "/compiled must call the catalogued compile gate; got:\n{body}"
    );
    // Isolate so a with_catalog import alone cannot false-green a no-arg call.
    let stripped = body.replace("flatten_to_mod_document_with_catalog", "");
    assert!(
        !stripped.contains("flatten_to_mod_document("),
        "/compiled must not call the empty-catalog no-arg flatten; got:\n{body}"
    );
}

/// `/compiled` must SURFACE the compile's structured findings, not drop them.
///
/// This is the one boundary where a dropped finding is invisible: everything the compile learned
/// would be thrown away or flattened into a pass/fail, and the caller is a game server that reads
/// no body on failure — so a dropped finding leaves literally no trace anywhere. The pin reads the
/// live handler body and asserts four things:
///
/// 1. the findings are lifted BEFORE `validated_compiled_body` consumes the document (the field
///    is `#[serde(skip)]`; after serialization they are unrecoverable);
/// 2. both headers are emitted (the "alongside the bytes" channel);
/// 3. each finding reaches the log, which this file documents as the only channel an operator
///    actually reads on this route; and
/// 4. a finding is NOT converted into a refusal — no `ApiError` is constructed from one.
///
/// RED: delete the `let diagnostics = doc.diagnostics.clone();` line, or the `tracing::warn!`.
#[test]
fn compiled_route_surfaces_the_structured_diagnostics() {
    let production = production_half();
    let start = production
        .find("pub async fn get_compiled_mission(")
        .expect("get_compiled_mission must exist");
    // Window must EXCLUDE the helper def — otherwise a route that no longer calls the helper
    // still greens on the definition sitting between get_compiled_mission and
    // unreadable_stored_payload.
    let body = production[start..]
        .split("\nfn compiled_diagnostics_response_headers(")
        .next()
        .expect("get_compiled_mission must precede compiled_diagnostics_response_headers");

    let lift = body
        .find("doc.diagnostics")
        .expect("/compiled must read the compile's findings off the document");
    let consume = body
        .find("validated_compiled_body(")
        .expect("/compiled must still hold the body to mission.schema.json");
    assert!(
        lift < consume,
        "the findings must be lifted BEFORE the document is serialized — `#[serde(skip)]` \
         means they cannot be recovered afterwards; got:\n{body}"
    );
    assert!(
        body.contains("compiled_diagnostics_response_headers(&diagnostics)"),
        "/compiled route body must call compiled_diagnostics_response_headers(&diagnostics); got:\n{body}"
    );
    let helper = production
        .split("fn compiled_diagnostics_response_headers(")
        .nth(1)
        .expect("compiled_diagnostics_response_headers must exist");
    assert!(
        helper.contains("COMPILE_DIAGNOSTICS_COUNT_HEADER")
            && helper.contains("COMPILE_DIAGNOSTICS_RULES_HEADER"),
        "/compiled must carry the findings alongside the bytes in both headers; got:\n{helper}"
    );
    assert!(
        body.contains("tracing::warn!"),
        "/compiled must log each finding — the mod discards the response body, so the log is \
         what an operator reads; got:\n{body}"
    );
    // A diagnostic is not a refusal: the findings loop must not mint an error.
    let loop_body = body
        .split("for f in &diagnostics {")
        .nth(1)
        .and_then(|t| t.split("\n    }").next())
        .expect("the per-finding loop must exist");
    assert!(
        !loop_body.contains("ApiError") && !loop_body.contains("return"),
        "a finding must not refuse the compile — the document is complete and valid; got:\n{loop_body}"
    );
}

/// a clean compile's *response* carries `x-compile-diagnostics-count: 0`.
///
/// The source pin above covers the route's shape (lift-before-serialize, both header constants,
/// warn!, no refusal). That is necessary but not sufficient: a source scan cannot prove the
/// assembled `Response` actually exposes the count header when the findings list is empty. This
/// builds the response the same way the route does — via [`compiled_diagnostics_response_headers`]
/// — and asserts the wire value.
///
/// RED: skip the count insert when `diagnostics.is_empty()`.
#[test]
fn compiled_clean_mission_response_carries_diagnostics_count_zero() {
    let headers = compiled_diagnostics_response_headers(&[]);
    let response = (headers, axum::body::Body::empty()).into_response();
    let count = response
        .headers()
        .get(COMPILE_DIAGNOSTICS_COUNT_HEADER)
        .and_then(|v| v.to_str().ok());
    assert_eq!(
        count,
        Some("0"),
        "clean /compiled must advertise x-compile-diagnostics-count: 0, not omit the header"
    );
    assert!(
        response
            .headers()
            .get(COMPILE_DIAGNOSTICS_RULES_HEADER)
            .is_none(),
        "rules header is omitted when nothing fired"
    );

    // Also pin the route, not only the helper. A bypass that inlines a HeaderMap omitting the
    // count on empty findings must RED here (and in the narrowed window above).
    let production = production_half();
    let start = production
        .find("pub async fn get_compiled_mission(")
        .expect("get_compiled_mission must exist");
    let route = production[start..]
        .split("\nfn compiled_diagnostics_response_headers(")
        .next()
        .expect("get_compiled_mission must precede compiled_diagnostics_response_headers");
    assert!(
        route.contains("compiled_diagnostics_response_headers(&diagnostics)"),
        "clean /compiled count pin requires the route to call the helper; got:\n{route}"
    );
}
