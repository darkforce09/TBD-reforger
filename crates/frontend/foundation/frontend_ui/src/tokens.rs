//! The state classes the shared form controls and the Mission Creator's chrome both wear.
//!
//! **Role:** owns the two named state recipes the primitives need — the neutral hover fill and the
//! disabled dimming — so every interactive control speaks one state language instead of an ad-hoc
//! `hover:` or `opacity-` pair per call site.
//! **Position:** read by the slider, the dropdown and the search field in this folder, and
//! re-exported by the Mission Creator's chrome layout, which composes them with its own toggled
//! plate and menu recipes.
//! **Signals & state:** none; string constants.
//! **Invariants:** a hover is a solid fill and never a border, so it can never read as a toggled
//! control, whose cue is a top border. The disabled recipe cancels the hover fill and is composed
//! after it, so a dimmed control does not still light up under the pointer.

/// **Hover = solid fill.** The transient pointer-over state of a neutral interactive control (menu
/// bar buttons, icon buttons, tree rows, the form controls). A solid fill and never a border, so it
/// cannot be mistaken for a toggled control's plate-and-top-border cue; the fill is `bg-white/10`,
/// the glass-surface neutral.
///
/// Carries `transition-colors` so the fill eases in, and lifts the label to `text-on-surface` on
/// hover (the muted-to-bright idiom). Compose after a control's base and geometry classes:
/// `cn(&["… base …", HOVER_FILL])`.
pub const HOVER_FILL: &str = "transition-colors hover:bg-white/10 hover:text-on-surface";

/// **Disabled = dimmed glyph, and the tooltip still shows.** The dim half: the control keeps its
/// slot, greys out, and does not react to hover. The tooltip half is not a class but a pattern: the
/// `title=` stays on the control (or its wrapper) even while `disabled`, so a control that cannot
/// act still explains why.
///
/// `disabled:hover:bg-transparent` cancels [`HOVER_FILL`]'s fill so a dimmed control does not still
/// light up under the pointer. Compose it after `HOVER_FILL` so the `disabled:` variant wins.
pub const DISABLED_GLYPH: &str = "disabled:opacity-30 disabled:hover:bg-transparent";
