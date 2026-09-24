//! Role: the rules 3 and 1 pins — proof that this renderer is still damage-driven, and that
//! nothing in the frame path allocates per frame.
//! Position: `frame/tests` in the map engine.
//! Signals & state: the source text of `frame/{lifecycle,encode,engine}.rs`, read at compile
//! time. Nothing here touches a GPU.
//! Invariants: `render()` refuses an undamaged frame; every lane mutation marks damage; the
//! packet reads the engine's persistent batch list rather than a per-frame rebuild; and the
//! packet's two lookup tables are engine fields that are cleared and refilled, never rebuilt.
//!
//! ── WHY A SOURCE PIN AND NOT A BEHAVIOURAL TEST ──────────────────────────────────────────
//!
//! `documentation_v2/standards/engine_boundary_rules.md` §2C rule 3 is the one rule of the four
//! whose breach is invisible:
//!
//! > If packet building becomes "walk the world, rebuild every batch", you have turned a
//! > damage-driven renderer into an immediate-mode one. That is the whole design gone, not a
//! > few percent.
//!
//! Every test in this crate would still pass on that day. The picture would be identical. Only
//! `__editorBench` would move, and only if somebody read it. The state machine itself is
//! covered — `website-graphics-engine`'s `frame/tests/damage_tests.rs` exercises
//! mark / `begin_frame` / `after_submit` / continuous directly. What no test covered until now
//! is that this crate still *consults* it: `RenderDamage` could be reduced to a field nobody
//! reads without one assertion firing.
//!
//! `RenderEngine` is `#[cfg(all(target_arch = "wasm32", feature = "render"))]` and every one of
//! its methods needs a `wgpu::Device`, so a native test cannot construct one and a wasm test
//! cannot get a GPU in CI. Scrubbing the source is what is left, and it is the idiom this crate
//! already uses for exactly this class of proof (`overlay/tests/tests/draw_order_t808_*`).
//!
//! It is a PIN, not a description: it fails if the wiring is removed, and it fails just as
//! loudly if a function is renamed out from under it, because [`body`] panics on a signature it
//! cannot find. A check that silently stops looking is not a check.

/// The three files that decide whether a frame is submitted and what it draws.
const SRC: &str = concat!(
    include_str!("../lifecycle.rs"),
    "\n",
    include_str!("../encode.rs"),
    "\n",
    include_str!("../engine.rs"),
);

/// The text of the item `sig` introduces, brace-balanced.
///
/// Panics when `sig` is absent or ambiguous. Both are the point: a rename that moves the
/// damage wiring somewhere this pin cannot see must fail the suite, not pass it.
fn body(sig: &str) -> String {
    let start = SRC
        .find(sig)
        .unwrap_or_else(|| panic!("rule 3: frame/ has no `{sig}` — the pin is looking at a husk"));
    assert!(
        !SRC[start + sig.len()..].contains(sig),
        "rule 3: `{sig}` is not unique — the extractor would pin the wrong body"
    );
    let after = &SRC[start..];
    let brace = after.find('{').expect("body");
    let mut depth = 0usize;
    let mut end = brace;
    for (i, ch) in after[brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = brace + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    after[..end].to_string()
}

/// ANTI-VACUITY. `include_str!` of a file that has been emptied, or of one whose contents moved
/// elsewhere, would leave every assertion below trivially true over a few hundred bytes of doc
/// comment. Establish that the subject is real before asserting anything about it.
#[test]
fn the_pin_is_reading_the_real_frame_module() {
    assert!(
        SRC.len() > 20_000,
        "rule 3: frame/{{lifecycle,encode,engine}}.rs came to {} bytes — too small to be the \
         engine. Either the files moved or the pin is scrubbing a stub.",
        SRC.len()
    );
    for sig in [
        "pub fn render(&mut self)",
        "fn upsert_lane",
        "fn remove_lane",
        "fn encode_main_pass",
    ] {
        // `body` panics on absent or ambiguous; calling it IS the assertion.
        let b = body(sig);
        assert!(b.len() > 40, "rule 3: `{sig}` has an empty body:\n{b}");
    }
}

/// RULE 3, FIRST HALF — an undamaged frame costs nothing.
///
/// The early-out has to come before `present::acquire`: acquiring a swapchain texture and then
/// declining to draw into it is not a skipped frame, it is a skipped *draw*, and it still pays
/// the acquire. So the order of the two is asserted, not just their presence.
#[test]
fn render_refuses_to_submit_an_undamaged_frame() {
    let b = body("pub fn render(&mut self)");

    let gate = b
        .find("if !self.damage.begin_frame().submit {")
        .expect("rule 3: render() must open by asking RenderDamage whether to submit at all");
    let acquire = b
        .find("present::acquire")
        .expect("rule 3: render() must still acquire through present::acquire");
    assert!(
        gate < acquire,
        "rule 3: the damage gate must precede the swapchain acquire — an acquired-then-\
         discarded frame is not a skipped frame:\n{b}"
    );
    assert!(
        b[gate..acquire].contains("return Ok(());"),
        "rule 3: the damage gate must RETURN, not merely branch:\n{b}"
    );
    assert!(
        b.contains("self.damage.after_submit();"),
        "rule 3: a submitted frame must clear the damage flag, or every frame stays dirty and \
         the renderer is immediate-mode with extra steps:\n{b}"
    );
    assert!(
        acquire < b.find("self.damage.after_submit();").expect("after_submit"),
        "rule 3: after_submit() must run AFTER the submit, never before it:\n{b}"
    );
}

/// RULE 3, SECOND HALF — every mutation of the draw list marks damage.
///
/// `remove_lane`'s mark is conditional on purpose: `packet::remove` reports whether anything
/// was actually dropped, and dropping nothing changes no pixel. That conditional is the shape
/// being pinned — an unconditional mark there would make every clear-a-lane call redraw.
#[test]
fn every_lane_mutation_marks_the_frame_damaged() {
    let up = body("fn upsert_lane");
    assert!(
        up.contains("packet::upsert(&mut self.batches, batch);"),
        "rule 3: upsert_lane must write through the persistent batch list:\n{up}"
    );
    assert!(
        up.contains("self.damage.mark();"),
        "rule 3: upsert_lane changes what would be drawn and must mark damage:\n{up}"
    );

    let rm = body("fn remove_lane");
    assert!(
        rm.contains("if packet::remove(&mut self.batches, lane) {")
            && rm.contains("self.damage.mark();"),
        "rule 3: remove_lane must mark damage exactly when a batch was really dropped:\n{rm}"
    );
}

/// RULE 3, THE ONE THAT COSTS FRAMES — the packet points at the engine's own batch list.
///
/// `batches: &self.batches` is the whole of rule 1's real case as well: `FramePacket` borrows,
/// so the vec that scales with scene complexity is a field that keeps its capacity across
/// frames and is refilled only by `upsert_lane` / `remove_lane`. A local `let batches = ...`
/// built here would be a per-frame rebuild of the entire scene, which is precisely the
/// immediate-mode collapse rule 3 names.
#[test]
fn the_packet_reads_the_persistent_batch_list() {
    assert!(
        SRC.contains("pub(crate) batches: Vec<DrawBatch>"),
        "rule 3: RenderEngine must own the batch list as a field"
    );

    let enc = body("fn encode_main_pass");
    assert!(
        enc.contains("batches: &self.batches,"),
        "rule 3: the frame packet must BORROW the engine's persistent batch list:\n{enc}"
    );
    for rebuild in ["batches.clear()", "let batches", "let mut batches"] {
        assert!(
            !enc.contains(rebuild),
            "rule 3: encode_main_pass must not build a batch list — it has one. Found \
             `{rebuild}`:\n{enc}"
        );
    }
}

/// RULE 1, THE PART THAT IS NOT `batches` — the two lookup tables the packet indexes.
///
/// `pipeline_table` and `bind_group_table` were rebuilt every frame from Phase 1 until 2C §R1:
/// a `Vec::with_capacity(9)` and a `vec![None; 100]`, plus roughly twenty `Arc` bumps, at
/// 60 Hz. Both are fixed-size and scene-independent, so this was never the expensive half of
/// rule 1 — `the_packet_reads_the_persistent_batch_list` above covers that one. It is pinned
/// because `frame/encode.rs` is the module every later packet-building belt will copy, and the
/// canonical rule-1 violation living inside the canonical rule-1 module is how the rule stops
/// meaning anything.
///
/// `indirect` is deliberately absent from this pin. `IndirectDraw<'a>` borrows two
/// `wgpu::Buffer` handles out of `self.icon_cull`, so a `RenderEngine` field of that type would
/// make the struct self-referential — it stays a local because safe Rust leaves no alternative,
/// and asserting otherwise here would be asserting something false.
#[test]
fn the_packets_lookup_tables_are_refilled_not_rebuilt() {
    for field in [
        "pub(crate) frame_pipelines: Vec<wgpu::RenderPipeline>",
        "pub(crate) frame_bind_groups: Vec<Option<wgpu::BindGroup>>",
    ] {
        assert!(
            SRC.contains(field),
            "rule 1: the packet's lookup tables must be engine fields, not per-frame locals. \
             Missing `{field}`"
        );
    }

    let enc = body("fn encode_main_pass");
    for borrow in [
        "pipelines: &self.frame_pipelines,",
        "bind_groups: &self.frame_bind_groups,",
    ] {
        assert!(
            enc.contains(borrow),
            "rule 1: the packet must borrow the persistent table, not a local. Missing \
             `{borrow}`:\n{enc}"
        );
    }
    for rebuild in [
        "let pipelines =",
        "let bind_groups =",
        "vec![None;",
        "Vec::with_capacity",
    ] {
        assert!(
            !enc.contains(rebuild),
            "rule 1: encode_main_pass must not build a lookup table per frame. Found \
             `{rebuild}`:\n{enc}"
        );
    }
}
