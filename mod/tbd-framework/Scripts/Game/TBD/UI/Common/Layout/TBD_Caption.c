/**
 * @file TBD_Caption.c
 * @brief The small uppercase mono label a list section starts with, with an optional trailing note.
 *
 * Role: creates `TBD_Caption.layout` ("LONG RANGE (LR) COMMAND", "INVENTORY"; trailing
 * "4 Available") and writes and paints it; the layout has no handler.
 * Position: called by the briefing pages and the briefing markers panel.
 * State: none; a static helper.
 * Invariants: widget contract `Caption`, `Trailing`; the caption is written upper case and an
 * empty trailing note is hidden; the root is stretched to the parent's width.
 */

//! Static mount helper for `TBD_Caption.layout`.
class TBD_Caption
{
	//! Create a caption under `parent`, write `text` upper case and the trailing note, paint both.
	//! @return the caption root, or null when the layout cannot be created
	static Widget Mount(Widget parent, string text, string trailing = "")
	{
		Widget w = TBD_UILayouts.CreateStretched(TBD_UILayouts.CAPTION, parent);
		if (!w)
			return null;

		string upper = text;
		upper.ToUpper();
		TextWidget caption = TextWidget.Cast(w.FindAnyWidget("Caption"));
		TextWidget note = TextWidget.Cast(w.FindAnyWidget("Trailing"));
		TBD_UITheme.Write(caption, upper);
		TBD_UITheme.Write(note, trailing);
		TBD_UITheme.Show(note, !trailing.IsEmpty());
		TBD_UITheme.Paint(caption, TBD_UITheme.MUTED_INK);
		TBD_UITheme.Paint(note, TBD_UITheme.DIM_INK);
		return w;
	}
}
