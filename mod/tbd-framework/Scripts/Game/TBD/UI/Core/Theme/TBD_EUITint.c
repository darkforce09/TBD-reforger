/**
 * @file TBD_EUITint.c
 * @brief The tint vocabulary of chips, badges and tinted panels.
 *
 * Role: names the semantic tints a screen may pick for a chip, badge, faction row or panel.
 * Position: chosen by screens and components; turned into colour only by `TBD_UITintColours`.
 * State: none.
 * Invariants: a screen never maps a tint to a colour itself; SOLID is the one filled variant
 * (the selected card's tag pill).
 */

//! Tint vocabulary for chips, badges and tinted panels. Screens pick a tint; TBD_UITintColours
//! owns what it looks like. SOLID is the one filled variant (the selected card's tag pill).
enum TBD_EUITint
{
	NEUTRAL, //!< slate glass; the default
	PRIMARY, //!< the quiet blue
	SUCCESS, //!< emerald
	WARNING, //!< amber
	DANGER, //!< red
	TERTIARY, //!< orange tertiary container
	BLUFOR, //!< blue faction
	OPFOR, //!< red faction
	SOLID //!< filled blue with white ink
}
