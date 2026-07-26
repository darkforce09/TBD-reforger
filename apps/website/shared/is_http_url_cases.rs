// ONE TABLE, TWO IMPLEMENTATIONS — the anti-drift pin for `is_http_url`. **T-405.**
//
// `is_http_url` exists twice on purpose:
//
//   * `apps/website/api/src/services/text.rs`          — the INGRESS guard (T-391). Rejects at
//                                                        the write boundary with a 400.
//   * `apps/website/frontend/src/url_guard.rs`         — the EGRESS guard (T-405). Refuses to
//                                                        emit a non-http(s) `href`.
//
// It is not one function in a shared crate because there is no crate both sides can honestly
// share — see the module header on `frontend/src/url_guard.rs` for the decision and the two
// candidates that were rejected. Two copies of a security predicate that can drift apart
// silently is the failure mode that would make the duplication worse than the disease, so the
// copies do not get to drift silently: this file is the single list of inputs and expected
// verdicts, and BOTH crates `include!` it into their test modules. Change one implementation
// and the OTHER crate's test suite goes red on the same commit.
//
// It is a `.rs` fragment holding real Rust string literals rather than a data file with an
// escape syntax, because a hand-rolled unescaper would be a THIRD thing that could drift — and
// half these cases are load-bearing precisely on their control characters. `rustc` unescapes
// them, identically, for both consumers, or neither compiles.
//
// ADDING A CASE: put it in the group it belongs to and give it a verdict. The `false` half is a
// security assertion — if an implementation starts answering `true` to one of those, that is a
// bypass, and the fix is the implementation, never this table. The `true` half is the
// documented, deliberately-narrow contract: the guard is a SCHEME allowlist and nothing more,
// so several entries below are hostile URLs that it accepts by design (see the "not a promise"
// section of the `is_http_url` doc comment). They are pinned here so that "the guard accepts
// `http://2130706433/x`" stays a decision somebody made rather than a thing somebody discovers.

/// `(input, expected_verdict)`. `true` = "a browser will FOLLOW this", `false` = "refuse it".
const IS_HTTP_URL_CASES: &[(&str, bool)] = &[
    // ── MUST REJECT: schemes that execute ────────────────────────────────────────────────
    // The literal T-391 defect: this is what ran from an `<a href>` on click.
    ("javascript:alert(1)", false),
    // Case. The reason this is an allowlist of two and not a `starts_with("javascript:")`.
    ("JaVaScRiPt:alert(1)", false),
    ("JAVASCRIPT:alert(1)", false),
    ("Javascript:alert(1)", false),
    // Other executing / content-bearing schemes.
    ("data:text/html,<script>alert(1)</script>", false),
    (
        "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
        false,
    ),
    ("vbscript:msgbox(1)", false),
    ("VBScript:msgbox(1)", false),
    ("blob:https://evil.com/1234", false),
    ("file:///etc/passwd", false),
    ("about:blank", false),
    // ── MUST REJECT: a hostile scheme carrying a REAL AUTHORITY ──────────────────────────
    // This group is the one that proves the scheme allowlist is load-bearing, and it was added
    // by perturbation: deleting the `matches!(scheme, "http" | "https")` clause outright — the
    // single most important line in the guard — left every other case in this table GREEN.
    // Everything above is rejected by the *host* clause instead, because `url` reports no host
    // for the scheme-with-no-authority forms (`javascript:alert(1)` has `host_str() == None`),
    // so the allowlist was never the thing under test.
    //
    // These have a host. `javascript://evil.com/%0aalert(1)` is not a contrivance — it is the
    // standard payload for exactly this mistake: `//evil.com/` is a JavaScript line comment, the
    // `%0a` ends it, and `alert(1)` runs. A guard that asked only "does it parse, and does it
    // have a host?" would wave it straight through. Measured: `url` gives it
    // scheme `javascript`, host `Some("evil.com")`.
    ("javascript://evil.com/%0aalert(1)", false),
    ("jAvAsCrIpT://evil.com/%0aalert(1)", false),
    ("javascript://comment%0aalert(1)", false),
    ("vbscript://evil.com/%0amsgbox(1)", false),
    ("data://evil.com/x", false),
    ("blob://evil.com/x", false),
    // The other WHATWG *special* schemes. They parse into perfectly well-formed URLs with real
    // hosts and are not `http`/`https`, so the allowlist is the only thing refusing them.
    ("ftp://evil.com/x", false),
    ("ws://evil.com/", false),
    ("wss://evil.com/", false),
    // A scheme nobody has heard of, which is the whole argument for an allowlist: this table
    // cannot enumerate the dangerous ones, so it does not try.
    ("foo://evil.com/", false),
    // ── MUST REJECT: the stored bytes and the parsed bytes must not be able to disagree ───
    // A browser strips leading/trailing C0-and-space from a URL attribute before parsing, so a
    // raw-string check sees "no scheme here" while the browser sees `javascript:`.
    (" javascript:alert(1)", false),
    ("\tjavascript:alert(1)", false),
    ("\njavascript:alert(1)", false),
    ("\r\n javascript:alert(1)", false),
    ("javascript:alert(1) ", false),
    ("  https://good.example/replay.json  ", false),
    // ...and browsers DELETE tab/CR/LF from anywhere INSIDE a URL, so every one of these
    // resolves to `javascript:alert(1)` no matter what the stored string looks like.
    ("java\tscript:alert(1)", false),
    ("java\nscript:alert(1)", false),
    ("java\rscript:alert(1)", false),
    ("jav\u{0}ascript:alert(1)", false),
    ("\u{0}javascript:alert(1)", false),
    ("https://good.example/\u{0}replay", false),
    ("https://good.example/re\tplay", false),
    // ── MUST REJECT: invisible-character spellings ───────────────────────────────────────
    // These are NOT ASCII control characters and are NOT stripped by `trim`, so they survive
    // to the parser — where they are simply illegal in a scheme, which is why they fail. The
    // point of pinning them is that both implementations must fail them for the same reason.
    ("\u{200b}javascript:alert(1)", false), // ZWSP
    ("java\u{200b}script:alert(1)", false), // ZWSP, interior
    ("\u{feff}javascript:alert(1)", false), // BOM
    ("java\u{feff}script:alert(1)", false), // BOM, interior
    ("\u{ad}javascript:alert(1)", false),   // soft hyphen
    ("java\u{ad}script:alert(1)", false),   // soft hyphen, interior
    // NBSP is Unicode `White_Space`, so unlike the three above it is caught one step earlier,
    // by the `trim` rule. Same verdict, different mechanism — both pinned.
    ("\u{a0}javascript:alert(1)", false),
    ("javascript:alert(1)\u{a0}", false),
    // Interior NBSP is neither trimmed nor legal in a scheme.
    ("java\u{a0}script:alert(1)", false),
    // ── MUST REJECT: encoded spellings of the scheme ─────────────────────────────────────
    // `%` cannot appear in a scheme, so none of these parse. A denylist that decoded first
    // would have to decide how many times to decode; an allowlist never has to ask.
    ("java%73cript:alert(1)", false),
    ("%6a%61%76%61%73%63%72%69%70%74:alert(1)", false),
    ("java&#115;cript:alert(1)", false),
    ("java\\u0073cript:alert(1)", false),
    // ── MUST REJECT: no scheme at all ────────────────────────────────────────────────────
    // Scheme-relative is the classic "well, it isn't javascript:" bypass. It is not a URL this
    // guard can vouch for, because the scheme it inherits is the page's.
    ("//evil.com", false),
    ("//evil.com/replay.json", false),
    ("\\\\evil.com\\share", false),
    ("/replays/local.json", false),
    ("replay.json", false),
    ("example.com/replay.json", false),
    // ── MUST REJECT: well-formed-looking but hostless ────────────────────────────────────
    // All three fail to parse at all — the WHATWG host parser reports "empty host" rather than
    // handing back a `Url` with an empty one, which is why the guard's host check is written as
    // "present and non-empty" instead of an unwrap.
    ("http://", false),
    ("https://", false),
    ("http://@", false),
    ("https://a@", false),
    ("", false),
    ("   ", false),
    // ── MUST ACCEPT: real replay links ───────────────────────────────────────────────────
    // This half is not decoration. A guard that 400s a legitimate link gets reverted by
    // whoever ships next, and the hole comes back — so the accept set is a test too.
    ("http://example.com", true),
    ("http://example.com/", true),
    ("https://aar.tbd/replays/abc.json", true),
    ("https://aar.tbd/replays?match=abc-123&format=json", true),
    ("https://aar.tbd:8443/replays/abc.json", true),
    ("http://192.168.1.10:8080/replay", true),
    ("https://aar.tbd/replays/abc.json#t=30", true),
    (
        "https://aar.tbd:8443/replays/abc.json?token=xyz&v=2#t=30",
        true,
    ),
    ("https://aar.tbd/replays/Operation%20Red%20Dawn.json", true),
    ("https://aar.tbd/a/b/c/d/e?x=1;y=2", true),
    ("https://xn--n3h.example/replay", true),
    // The parser lowercases the scheme, so an uppercase one is the same URL.
    ("HTTPS://AAR.TBD/replays/ABC.json", true),
    ("HtTp://example.com/x", true),
    // ── ACCEPTED BY DESIGN, AND HOSTILE ──────────────────────────────────────────────────
    // Every entry below is a URL an attacker would like. They pass because the guard checks
    // the SCHEME and the scheme only — it is not an SSRF guard, not a domain allowlist, and
    // has no opinion about path, query, fragment or userinfo. Pinned so the non-promises stay
    // documented behaviour rather than an unpleasant surprise, and so that a future attempt to
    // tighten them has to change this table on purpose. See T-391 / T-405.
    //
    // Not an SSRF guard: loopback, link-local and the cloud metadata endpoint are real
    // `http` URLs and pass. A caller that FETCHES a stored URL needs its own network check.
    ("http://127.0.0.1/", true),
    ("http://localhost:8080/admin", true),
    ("http://169.254.169.254/latest/meta-data/", true),
    ("http://[::1]/", true),
    // Decimal-integer host: 2130706433 == 127.0.0.1, spelled so a substring check misses it.
    ("http://2130706433/x", true),
    // Userinfo: the host is `evil.com`, not `expected.com`. A human skims this wrong; the
    // guard does not care either way, because a browser will still only FETCH it.
    ("https://a@evil.com", true),
    ("https://expected.com@evil.com/x", true),
    ("https://user:pass@aar.tbd/replay", true),
    // U+3002 IDEOGRAPHIC FULL STOP folds to `.` during IDNA, so the host really is `evil.com`.
    ("https://evil\u{3002}com/x", true),
    // WHATWG treats a backslash as a slash for special schemes, so this normalises to
    // `http://evil.com/`. It is still `http`, so it still passes.
    ("http:/\\evil.com", true),
    ("http:\\\\evil.com", true),
    // Fewer slashes than expected, same result: the "special authority slashes" states tolerate
    // any number of `/` and `\`, so all of these resolve to `http://example.com/`.
    ("http:/example.com", true),
    ("http:example.com", true),
    // MORE slashes than expected, and this one is a genuine trap. It reads as "hostless URL with
    // path /replay.json" and it is not: the special-authority-IGNORE-slashes state eats the third
    // slash too, so `replay.json` is parsed as the HOST. Measured, not assumed —
    // `Url::parse("https:///replay.json")` returns host `Some("replay.json")`, path `/`,
    // serialising to `https://replay.json/`.
    //
    // It is `true` here because it is genuinely an `https` URL and cannot execute, which is the
    // only question this guard answers. It is written down because a reviewer WILL read it as
    // hostless — the first draft of this table did, and the API guard is what corrected it. What
    // it costs is not XSS but destination: a link that looks local navigates to a third-party
    // host. Anything that cares where a URL POINTS, as opposed to what it DOES, needs its own
    // check; see the "not an SSRF guard" note above.
    ("https:///replay.json", true),
];
