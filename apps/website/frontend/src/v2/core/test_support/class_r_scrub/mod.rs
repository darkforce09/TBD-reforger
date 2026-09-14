//! A source scrubber for the guard tests that assert things about this crate's own source.
//!
//! **Role:** reduces a Rust file to the text a shipping build would actually compile and run —
//! comments gone, dead `cfg` items gone, unprovable blocks gone, everything after an
//! unconditional jump gone — so that a test asserting "this file calls X" cannot be satisfied by a
//! mention of X in a comment, a string literal, or code no build reaches.
//! **Position:** test-only support. Guard tests across the crate call [`live_source`],
//! [`live_code`], [`only_item`] and [`only_body`] instead of reading raw source themselves.
//! **Signals & state:** none. Every entry point is a pure function of the text handed to it.
//! **Invariants:** an undecided condition fails closed — the block is removed, not kept — because
//! a wrongly removed block turns a guard red and loud while a wrongly kept one stays silent over
//! code the build never runs. [`only_item`] and [`only_body`] refuse ambiguity: zero matches and
//! two matches are both errors, since no textual search can tell which of two definitions ships.

mod cfg;
mod consts;
mod expr;
mod lexer;
mod scrub;

#[allow(unused_imports)]
pub(crate) use cfg::{cfg_eval, cfg_eval_wasm, mentions_cfg_family, resolve_wasm_cfg};
#[allow(unused_imports)]
pub(crate) use consts::Consts;
#[allow(unused_imports)]
pub(crate) use expr::eval_bool;
#[allow(unused_imports)]
pub(crate) use lexer::is_ident_char;
#[allow(unused_imports)]
pub(crate) use scrub::{live_code, live_source, only_body, only_item};

#[cfg(test)]
#[path = "../tests/class_r_scrub.rs"]
mod tests;
