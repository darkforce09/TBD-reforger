/**
 * @file TBD_UIStateColours.c
 * @brief Colours of an interactive surface for each `TBD_EUIState`.
 *
 * Role: turns a row or button state, with its hover and selection, into background, title,
 * detail and accent tokens.
 * Position: called by `TBD_ListBoxRow`; returns `TBD_UITheme` tokens that the caller paints.
 * State: none; pure functions.
 * Invariants: hover beats selection beats idle, so the pointer always wins the readout; LOCKED
 * never lights; TRANSPARENT means "draw nothing".
 */

//! State-to-colour table for list rows and buttons.
class TBD_UIStateColours
{
	//! Background tint for an interactive surface (list row, button) given its semantic state.
	//! Hover beats selection beats idle -- immediate feedback is design law, so the pointer
	//! always wins the readout.
	static int StateBackground(TBD_EUIState state, bool hovered, bool selected)
	{
		if (state == TBD_EUIState.LOCKED)
			return TBD_UITheme.ROW_IDLE;

		if (hovered)
			return TBD_UITheme.ROW_HOVER;

		if (selected || state == TBD_EUIState.ACTIVE)
			return TBD_UITheme.ROW_SELECTED;

		return TBD_UITheme.ROW_IDLE;
	}

	//! Ink for the primary line of an interactive surface.
	static int StateTitle(TBD_EUIState state)
	{
		switch (state)
		{
			case TBD_EUIState.ACTIVE: return TBD_UITheme.PRIMARY;
			case TBD_EUIState.TAKEN:  return TBD_UITheme.ON_SURFACE_VARIANT;
			case TBD_EUIState.LOCKED: return TBD_UITheme.ROW_DISABLED_TEXT;
			case TBD_EUIState.DANGER: return TBD_UITheme.ERROR_ALERT;
		}

		return TBD_UITheme.ON_SURFACE;
	}

	//! Ink for the secondary/right-hand line.
	static int StateDetail(TBD_EUIState state)
	{
		switch (state)
		{
			case TBD_EUIState.ACTIVE: return TBD_UITheme.PRIMARY;
			case TBD_EUIState.LOCKED: return TBD_UITheme.ROW_DISABLED_TEXT;
			case TBD_EUIState.DANGER: return TBD_UITheme.ERROR_ALERT;
		}

		return TBD_UITheme.ON_SURFACE_VARIANT;
	}

	//! The 2px leading rail that marks the active row. TRANSPARENT means "draw nothing" --
	//! progressive disclosure, not a permanent cage of borders.
	static int StateAccent(TBD_EUIState state, bool selected)
	{
		if (state == TBD_EUIState.ACTIVE || selected)
			return TBD_UITheme.PRIMARY;

		if (state == TBD_EUIState.DANGER)
			return TBD_UITheme.ERROR;

		return TBD_UITheme.TRANSPARENT;
	}
}
