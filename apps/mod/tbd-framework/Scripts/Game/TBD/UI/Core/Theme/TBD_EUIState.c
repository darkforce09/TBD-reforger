/**
 * @file TBD_EUIState.c
 * @brief The semantic state shared by every TBD interactive surface.
 *
 * Role: names the states a list row or button can be in, independent of any screen's meaning.
 * Position: set by screens on rows and buttons; turned into colour only by `TBD_UIStateColours`.
 * State: none.
 * Invariants: screens map their own vocabulary onto these five states and never pick colours.
 */

//! Semantic state shared by every TBD interactive surface. The lobby maps its own vocabulary
//! onto these (free slot -> NORMAL, your slot -> ACTIVE, someone else's -> TAKEN, wrong side ->
//! LOCKED) so colour decisions stay in TBD_UIStateColours and never in a screen.
enum TBD_EUIState
{
	NORMAL, //!< idle and available; the default
	ACTIVE, //!< the viewer's own or the selected item
	TAKEN, //!< held by someone else
	LOCKED, //!< unavailable to the viewer
	DANGER //!< destructive or alerting
}
