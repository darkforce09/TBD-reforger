//! Briefing pass (2026-09-14) — the small uppercase mono label a list section starts with
//! ("LONG RANGE (LR) COMMAND", "INVENTORY"), with an optional trailing note ("4 Available").
//! `TBD_Caption.layout`: `Caption`, `Trailing`. No handler; this helper writes and paints it.
class TBD_Caption
{
	//------------------------------------------------------------------------------------------------
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
