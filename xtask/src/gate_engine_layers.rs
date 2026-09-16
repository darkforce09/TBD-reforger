//! Engine-layer walls — [`ENGINE_SPLIT_PROGRAM.md`] §5 rules **1 and 2**, and only those two.
//!
//! ── WHAT THIS DEFENDS ────────────────────────────────────────────────────────────────────────
//!
//! Phase 1 of the engine split took `website-graphics-engine` apart and put the ~50k lines of map
//! domain (terrain, symbology, spatial, streaming) in `website-map-engine`, leaving behind a crate
//! that is *only* a renderer: device, pipelines, shaders, draw batching, text packing, the rAF
//! pump. The dependency arrow is one-way and non-negotiable:
//!
//! ```text
//! frontend ──► website-map-engine ──► website-graphics-engine
//! ```
//!
//! Nothing in the compiler enforces that. `website-graphics-engine` could add a dependency edge
//! back to `website-map-engine` tomorrow and everything would still build — the wall exists only
//! because someone drew it in a document. §5 of the program says so outright: *"One 99k crate rots
//! without enforcement."* Rules 1 and 2 are the two halves of that wall:
//!
//! | # | rule | what rots without it |
//! |---|------|----------------------|
//! | 1 | `apps/website/graphics-engine/**` may not import `website_map_engine` | the arrow turns into a cycle and the split is undone by accident |
//! | 2 | no `terrain` / `symbology` / `mission` / `orbat` / `arma` in a declared name there | "pure renderer" becomes a claim in a README rather than a property of the code |
//!
//! Rules 3–6 (`crate::frame` reach, `data/scenario` isolation, no DOM in map-engine, frontend may
//! not import graphics) police trees that phases 2 and 3 have not built yet. They land with those
//! phases; there is nothing for them to read today.
//!
//! ── WHY EXIT 2 EXISTS ────────────────────────────────────────────────────────────────────────
//!
//! A layer wall is exactly the kind of gate that gets renamed out from under itself: phase 2 moves
//! directories on purpose, and the first thing a move breaks is the path this gate scans. If a
//! missing root read as "no violations found", the wall would evaporate on the very commit that
//! most needs it, silently, with a green tick. So a root that is absent or unreadable is
//! [`NotRun`] and exits **2** — "I never looked" is a different operator action from "I looked and
//! it is dirty". Same reasoning, and the same [`tbd_gate::Verdict`] machinery, as
//! `sql_gates::report_did_not_run`.
//!
//! The second vacuity hole is subtler and is guarded separately: a root that *exists* and is
//! *empty*. `walk_files` would return `Ok(vec![])`, `grep_lines` would find nothing, and the gate
//! would report a clean wall over zero bytes of source. So the scanned file count is printed, and
//! zero Rust files is a hard FAIL rather than a pass.
//!
//! ── RULE 2 AND THE WORD "SUBMISSION" ─────────────────────────────────────────────────────────
//!
//! The naive form of rule 2 — grep the five nouns case-insensitively — cannot be used as a gate.
//! The words are ordinary English and ordinary graphics vocabulary: a doc comment may discuss what
//! the *map engine* hands over, a `.wgsl` path may be named after the feature it draws, and
//! "submission" contains "mission". A gate that fires on prose gets suppressed, and a suppressed
//! gate is not a gate.
//!
//! So rule 2 matches **declarations, not mentions**: a declaration keyword, whitespace, then an
//! identifier that contains the noun.
//!
//! ```text
//! \b(struct|enum|trait|type|fn|const|static|mod)\s+\w*(terrain|symbology|mission|orbat|arma)
//! ```
//!
//! case-insensitive, so `struct TerrainBlob`, `fn pack_mission`, `fn submit_mission` and
//! `mod symbology` all fail while `/// a submission arrives here`, `include_str!("terrain.wgsl")`
//! and any sentence naming the map engine all pass. `\w*` is what makes `submit_mission` a
//! violation and `submission` not one: the noun has to sit inside a name that a keyword just
//! introduced.
//!
//! What it deliberately does not see: enum **variants** and struct **fields**, which are bare
//! identifiers inside a block with no keyword in front of them. §5's prose names variants too, but
//! catching them needs block tracking rather than a line matcher, and phase 1D.1 already emptied
//! the one real population (`LaneRole`'s 48 named variants moved to `map-engine/overlay/lanes.rs`
//! and graphics keys on an opaque `LaneId`). The line-level rule is what phase 1F was scoped to.
//!
//! ── WHY `verify engine-layers` AND NOT `verify-engine-layers` ────────────────────────────────
//!
//! §5 writes the command as bare `cargo xtask verify-engine-layers`, which is not the shape this
//! CLI has: every sibling gate is `cargo xtask verify <name>` ([`crate::VerifyCmd`]), and a
//! top-level `verify-engine-layers` would be the only hyphenated verb on the root command. The
//! spec was describing the gate, not the argv. Both spellings work anyway — the `TASKS` alias row
//! is named `verify-engine-layers`, so `cargo xtask ci verify-engine-layers` resolves, the same
//! way `verify-no-node` aliases `verify no-node`.

use std::path::Path;

use anyhow::Result;
use tbd_gate::scan::{self, Hit};
use tbd_gate::{Kind, NotRun, Pattern, Verdict, gate};

/// The pure crate. Rules 1 and 2 are both scoped to it and nothing else.
const CRATE_REL: &str = "apps/website/graphics-engine";

/// Rule 1's Rust spelling — what an `use`/path reference to the map engine looks like in source.
const MAP_ENGINE_PATH: &str = "website_map_engine";
/// Rule 1's Cargo spelling — what a dependency edge looks like in the manifest.
const MAP_ENGINE_PKG: &str = "website-map-engine";

/// Rule 2's declaration matcher. See the module docs for why it is anchored on a keyword.
const DECL_RE: &str =
    r"\b(struct|enum|trait|type|fn|const|static|mod)\s+\w*(terrain|symbology|mission|orbat|arma)";

const RULE1_HEAD: &str =
    "==> engine-layers rule 1 — apps/website/graphics-engine must not import website_map_engine";
const RULE2_HEAD: &str =
    "==> engine-layers rule 2 — no map noun in a declared name under apps/website/graphics-engine";

const RULE1_TAIL: &[&str] = &[
    "      The arrow runs map-engine -> graphics-engine and only that way. Compute it in",
    "      map-engine and hand the result over as a FramePacket / DrawBatch / TextRun",
    "      (ENGINE_SPLIT_PROGRAM.md §1 \"Dependency direction — non-negotiable\").",
];
const RULE2_TAIL: &[&str] = &[
    "      graphics-engine is a renderer and may name geometry and GPU handles only. A map",
    "      noun in a declared name means domain logic came back across the wall — move the",
    "      decision to map-engine and leave the packing here (ENGINE_SPLIT_PROGRAM.md §5 rule 2).",
];
const PROBE_FAIL: &[&str] = &[
    "FAIL: matcher self-probe returned no match over a subject it must match.",
    "      The search engine is broken. A check that cannot run is not a pass.",
];
const NOTHING_TAIL: &[&str] = &[
    "      An engine-layer check with no inputs is not a pass: either the crate moved and this",
    "      gate is scanning a husk, or the walk is broken. Both are red.",
];

pub fn verify_engine_layers(repo_root: &Path) -> Result<u8> {
    let (code, out) = run(repo_root);
    for line in out {
        println!("{line}");
    }
    Ok(code)
}

/// Push a fixed block. `""` is a deliberate blank line, which `str::lines` would swallow.
fn say(o: &mut Vec<String>, lines: &[&str]) {
    o.extend(lines.iter().map(|s| (*s).to_string()));
}

/// `path:line:text` — [`Hit::rendered`]'s shape, but repo-relative.
///
/// `walk_files` is handed absolute roots, so `rendered()` would print this machine's checkout
/// prefix. Every path in this gate's output names a file a reader has to go and edit, and the
/// relative form is the one that pastes into an editor and into a test assertion unchanged.
fn rel(repo_root: &Path, hit: &Hit) -> String {
    let p = hit.path.strip_prefix(repo_root).unwrap_or(&hit.path);
    format!("{}:{}:{}", p.display(), hit.line_no, hit.line)
}

/// Build output is not source.
///
/// A stray `apps/website/graphics-engine/src/target-container/` (536 MB of wasm artifacts, one
/// generated `thiserror` `private.rs` among them) exists in this checkout today, and a gate that
/// reported on a dependency's generated code would be reporting on code nobody in this repo wrote.
/// `.gitignore` covers `/target/` and `target-*/` anywhere in the tree, so a pruned directory can
/// never hold a tracked file — the prune removes noise, not coverage. Only components *below*
/// `repo_root` are considered, or a checkout that happened to live under `~/target-x` would prune
/// the entire repository and the gate would go vacuously green.
fn is_source(repo_root: &Path, path: &Path) -> bool {
    let rel = path.strip_prefix(repo_root).unwrap_or(path);
    !rel.components().any(|c| {
        let n = c.as_os_str().to_string_lossy();
        n == "target" || n.starts_with("target-")
    })
}

fn refuse(o: &mut Vec<String>, what: &str, cause: NotRun) -> (u8, Vec<String>) {
    o.push(Verdict::did_not_run(what, Kind::Ban, cause).to_string());
    say(o, &["", "ENGINE-LAYERS: FAIL (did not run)"]);
    // 2, not 1: "the wall is breached" and "I never read the crate" are different operator
    // actions, and phase 2 will move these paths on purpose.
    (2, std::mem::take(o))
}

/// The gate proper, writing into a sink so the tests assert on exact bytes instead of scraping
/// stdout — the [`crate::gate_route_tags`] shape.
fn run(repo_root: &Path) -> (u8, Vec<String>) {
    let mut o: Vec<String> = Vec::new();

    // Probe the matcher over a subject whose answer is known, BEFORE it decides anything. The
    // engine is compiled in, so "tool absent" is unreachable — but "the matcher works" is still a
    // claim, and `probe_str` returning a `Result` forces the dead arm to be written down.
    let decl = match Pattern::regex(DECL_RE).and_then(Pattern::case_insensitive) {
        Ok(p) => p,
        Err(e) => {
            let cause = NotRun::ToolError {
                tool: "regex".into(),
                status: 2,
                stderr: e.to_string(),
            };
            return refuse(&mut o, "engine-layers rule 2 pattern", cause);
        }
    };
    match gate::probe_str(&decl, "pub struct TerrainBlob;") {
        Ok(true) => {}
        Ok(false) => {
            say(&mut o, PROBE_FAIL);
            return (1, o);
        }
        Err(cause) => return refuse(&mut o, "engine-layers self-probe", cause),
    }

    let crate_dir = repo_root.join(CRATE_REL);
    let manifest = crate_dir.join("Cargo.toml");
    let src = crate_dir.join("src");

    let root = repo_root.to_path_buf();
    let sources = match scan::walk_files(&[&src], move |p| {
        is_source(&root, p) && p.extension().is_some_and(|e| e == "rs")
    }) {
        Ok(f) => f,
        Err(cause) => return refuse(&mut o, "engine-layers could not walk the crate", cause),
    };
    // The manifest is walked rather than read so a missing one is the same `NotRun` as a missing
    // src/ — one refusal shape for the whole gate, not two.
    let manifest_files = match scan::walk_files(&[&manifest], |_| true) {
        Ok(f) => f,
        Err(cause) => return refuse(&mut o, "engine-layers could not read the manifest", cause),
    };

    let scanned = format!(
        "  scanned {} .rs file(s) + {CRATE_REL}/Cargo.toml",
        sources.len()
    );
    if sources.is_empty() {
        o.push(format!(
            "FAIL: engine-layers walked 0 .rs file(s) under {CRATE_REL}/src — refusing a vacuous pass."
        ));
        say(&mut o, NOTHING_TAIL);
        o.push("ENGINE-LAYERS: FAIL (no inputs)".to_string());
        return (1, o);
    }

    // ── rule 1 ───────────────────────────────────────────────────────────────────────────────
    o.push(RULE1_HEAD.to_string());
    let path_use = Pattern::literal(MAP_ENGINE_PATH);
    let mut breaches: Vec<String> = Vec::new();
    match scan::grep_lines(&path_use, &sources) {
        Ok(hits) => breaches.extend(hits.iter().map(|h| rel(repo_root, h))),
        Err(cause) => return refuse(&mut o, "engine-layers rule 1 scan", cause),
    }
    // The manifest arm closes the one hole the source arm has: `[dependencies] r = { package =
    // "website-map-engine" }` renames the crate, and every `use r::…` then spells something this
    // gate has never heard of. A `#` line is a comment — naming the other crate in prose is not a
    // dependency edge.
    match scan::grep_lines(&Pattern::literal(MAP_ENGINE_PKG), &manifest_files) {
        Ok(hits) => breaches.extend(
            hits.iter()
                .filter(|h| !h.line.trim_start().starts_with('#'))
                .map(|h| rel(repo_root, h)),
        ),
        Err(cause) => return refuse(&mut o, "engine-layers rule 1 manifest scan", cause),
    }
    if breaches.is_empty() {
        o.push("  OK (none)".to_string());
    } else {
        o.push("FAIL: graphics-engine reaches back into the map engine:".to_string());
        o.extend(breaches.iter().map(|b| format!("  {b}")));
        say(&mut o, RULE1_TAIL);
    }

    // ── rule 2 ───────────────────────────────────────────────────────────────────────────────
    o.push(RULE2_HEAD.to_string());
    let nouns: Vec<String> = match scan::grep_lines(&decl, &sources) {
        Ok(hits) => hits.iter().map(|h| rel(repo_root, h)).collect(),
        Err(cause) => return refuse(&mut o, "engine-layers rule 2 scan", cause),
    };
    if nouns.is_empty() {
        o.push("  OK (none)".to_string());
    } else {
        o.push("FAIL: map nouns declared inside the pure renderer:".to_string());
        o.extend(nouns.iter().map(|n| format!("  {n}")));
        say(&mut o, RULE2_TAIL);
    }

    o.push(scanned);
    if breaches.is_empty() && nouns.is_empty() {
        o.push("ENGINE-LAYERS: PASS".to_string());
        return (0, o);
    }
    o.push(format!(
        "ENGINE-LAYERS: FAIL — {} wall breach(es), {} map-noun declaration(s)",
        breaches.len(),
        nouns.len()
    ));
    (1, o)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A clean two-file renderer. Every banned word appears here as PROSE or as an asset path,
    /// which is the false-positive case rule 2 has to survive to be usable.
    const CLEAN: &str = r#"
//! The draw belt. A submission from the map engine arrives as a DrawBatch; this file never
//! decides which symbol to draw, only how to pack it.
const SHADER: &str = include_str!("shaders/terrain.wgsl");
pub struct DrawBatch {
    pub lane: u32,
}
pub fn pack_batch(b: &DrawBatch) -> u32 {
    b.lane
}
"#;
    const MANIFEST: &str =
        "[package]\nname = \"website-graphics-engine\"\n\n[dependencies]\nbytemuck = \"1\"\n";

    struct Repo(PathBuf);
    impl Repo {
        fn new(name: &str) -> Repo {
            let mut p = std::env::temp_dir();
            p.push(format!("tbd-el-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(p.join("apps/website/graphics-engine/src/draw")).unwrap();
            let r = Repo(p);
            r.src("draw/mod.rs", CLEAN);
            r.manifest(MANIFEST);
            r
        }
        fn src(&self, rel: &str, body: &str) {
            let p = self.0.join("apps/website/graphics-engine/src").join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        fn manifest(&self, body: &str) {
            let p = self.0.join("apps/website/graphics-engine/Cargo.toml");
            std::fs::write(p, body).unwrap();
        }
        /// Run; assert the exit code and every expected line; hand back the joined output.
        fn expect(&self, code: u8, want: &[&str]) -> String {
            let (got, out) = super::run(&self.0);
            let all = out.join("\n");
            assert_eq!(got, code, "{all}");
            for w in want {
                assert!(all.contains(w), "missing {w:?} in:\n{all}");
            }
            all
        }
    }
    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// THE GREEN ARM — and with it the two false positives that would make this gate unusable:
    /// the word "submission" in a doc comment, and a `.wgsl` asset path named after a map feature.
    #[test]
    fn a_pure_renderer_passes_and_prose_does_not_trip_it() {
        let all = Repo::new("clean").expect(
            0,
            &[
                RULE1_HEAD,
                RULE2_HEAD,
                "  OK (none)",
                "  scanned 1 .rs file(s) + apps/website/graphics-engine/Cargo.toml",
                "ENGINE-LAYERS: PASS",
            ],
        );
        assert!(!all.contains("FAIL"), "{all}");
    }

    /// RULE 1, RED — an import of the map engine, reported with its exact line.
    #[test]
    fn importing_the_map_engine_fails() {
        let r = Repo::new("rule1");
        r.src("draw/bad.rs", "use website_map_engine::x;\n");
        r.expect(
            1,
            &[
                "FAIL: graphics-engine reaches back into the map engine:",
                "  apps/website/graphics-engine/src/draw/bad.rs:1:use website_map_engine::x;",
                RULE1_TAIL[0],
                "ENGINE-LAYERS: FAIL — 1 wall breach(es), 0 map-noun declaration(s)",
            ],
        );
    }

    /// RULE 1, the manifest arm: a dependency edge is a breach even with no `use` anywhere, and a
    /// `#` comment naming the other crate is not.
    #[test]
    fn a_dependency_edge_is_a_breach_and_a_comment_is_not() {
        let r = Repo::new("rule1-manifest");
        r.manifest(&format!(
            "{MANIFEST}me = {{ package = \"website-map-engine\" }}\n"
        ));
        r.expect(
            1,
            &[
                "  apps/website/graphics-engine/Cargo.toml:6:me = { package = \"website-map-engine\" }",
                "ENGINE-LAYERS: FAIL — 1 wall breach(es), 0 map-noun declaration(s)",
            ],
        );
        r.manifest(&format!(
            "{MANIFEST}# website-map-engine depends on us, never the reverse\n"
        ));
        r.expect(0, &["ENGINE-LAYERS: PASS"]);
    }

    /// RULE 2, RED — a type name and a fn name, each reported with its exact line.
    #[test]
    fn map_nouns_in_declared_names_fail() {
        let r = Repo::new("rule2");
        r.src(
            "draw/bad.rs",
            "pub struct TerrainBlob;\npub fn pack_mission() {}\n",
        );
        r.expect(
            1,
            &[
                "FAIL: map nouns declared inside the pure renderer:",
                "  apps/website/graphics-engine/src/draw/bad.rs:1:pub struct TerrainBlob;",
                "  apps/website/graphics-engine/src/draw/bad.rs:2:pub fn pack_mission() {}",
                RULE2_TAIL[0],
                "ENGINE-LAYERS: FAIL — 0 wall breach(es), 2 map-noun declaration(s)",
            ],
        );
    }

    /// The whole shape of rule 2, one line at a time. `submit_mission` is the pair that proves
    /// `\w*` is doing the work: the same eight letters pass as a word and fail as a name.
    #[test]
    fn rule_two_matches_declarations_and_not_mentions() {
        let p = Pattern::regex(DECL_RE)
            .and_then(Pattern::case_insensitive)
            .unwrap();
        for bad in [
            "pub struct TerrainBlob;",
            "enum SymbologyKind { A }",
            "trait MissionSink {}",
            "type ArmaId = u32;",
            "pub fn submit_mission() {}",
            "const ORBAT_SLOTS: u8 = 4;",
            "static TERRAIN_LOD: u8 = 2;",
            "mod symbology;",
            "    pub(crate) fn pack_mission_x(v: u32) -> u32 { v }",
        ] {
            assert!(p.is_match(bad), "should fail the gate: {bad}");
        }
        for ok in [
            "/// a submission from the map engine arrives here",
            "const SHADER: &str = include_str!(\"shaders/terrain.wgsl\");",
            "// symbology decides which symbol; we only pack it",
            "let mission = 1;",
            "pub struct DrawBatch;",
            "fn pack_icon_instance(v: u32) -> u32 { v }",
            "// the type for terrain lives one crate over",
        ] {
            assert!(!p.is_match(ok), "should pass the gate: {ok}");
        }
    }

    /// THE ANTI-VACUITY CASES. Neither "the crate is gone" nor "the crate is empty" may read as a
    /// clean wall — a moved directory is exactly what phase 2 does on purpose.
    #[test]
    fn inputs_that_were_never_read_do_not_pass() {
        let (code, out) = super::run(Path::new("/nonexistent/tbd-engine-layers/repo"));
        let all = out.join("\n");
        assert_eq!(code, 2, "a check that never ran must not exit 0:\n{all}");
        assert!(all.contains(CRATE_REL) && !all.contains("PASS"), "{all}");
        assert!(all.contains("ENGINE-LAYERS: FAIL (did not run)"), "{all}");

        // src/ present but empty of Rust: `walk_files` returns Ok(vec![]) and every grep finds
        // nothing, which is the shape that would report a clean wall over zero bytes of source.
        let r = Repo::new("empty");
        std::fs::remove_file(r.0.join("apps/website/graphics-engine/src/draw/mod.rs")).unwrap();
        r.expect(
            1,
            &[
                "FAIL: engine-layers walked 0 .rs file(s)",
                NOTHING_TAIL[0],
                "ENGINE-LAYERS: FAIL (no inputs)",
            ],
        );
    }

    /// A missing manifest is `did not run`, not "no dependency edge found".
    #[test]
    fn a_missing_manifest_does_not_read_as_clean() {
        let r = Repo::new("no-manifest");
        std::fs::remove_file(r.0.join("apps/website/graphics-engine/Cargo.toml")).unwrap();
        let (code, out) = super::run(&r.0);
        let all = out.join("\n");
        assert_eq!(code, 2, "{all}");
        assert!(all.contains("Cargo.toml"), "{all}");
    }

    /// Build output is pruned, and the prune is scoped below the repo root — a checkout living
    /// under a `target-*` path must not prune itself into a vacuous pass.
    #[test]
    fn build_output_is_pruned_but_only_below_the_root() {
        let r = Repo::new("prune");
        r.src(
            "target-container/debug/build/dep/out/private.rs",
            "pub struct TerrainBlob;\n",
        );
        r.expect(0, &["ENGINE-LAYERS: PASS", "  scanned 1 .rs file(s)"]);

        let root = Path::new("/home/x/target-container/checkout");
        assert!(
            is_source(
                root,
                &root.join("apps/website/graphics-engine/src/draw/mod.rs")
            ),
            "the prune must not see the root's own path components"
        );
        assert!(!is_source(
            root,
            &root.join("src/target-gate-frontend/x.rs")
        ));
        assert!(!is_source(root, &root.join("target/debug/x.rs")));
    }
}
