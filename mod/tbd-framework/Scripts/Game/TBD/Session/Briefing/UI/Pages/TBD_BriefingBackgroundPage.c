/**
 * @file TBD_BriefingBackgroundPage.c
 * @brief The Background page: the lore, one inset paragraph each.
 *
 * Role: draws each lore paragraph in an inset text box.  Position: TBD_BriefingNav.CreatePage creates it; TBD_BriefingScreen builds it into the page column.
 * State: none beyond TBD_BriefingPage.  Invariants: reads TBD_BriefingCatalog only.
 */

//! `lore_panel`: the situation, one inset paragraph each.
class TBD_BriefingBackgroundPage : TBD_BriefingPage
{
	//! @return the panel title
	override string Title() { return "Background"; }
	//! @return the header icon key
	override string Icon()  { return "description"; }

	//! Fill the page from the catalog's lore paragraphs.
	//! @param content the scroll list column
	override void Fill(Widget content)
	{
		foreach (string paragraph : m_Catalog.GetLore())
		{
			Widget inset = TBD_UILayouts.CreateStretched(TBD_UILayouts.INSET_TEXT, content);
			if (!inset)
				continue;

			AlignableSlot.SetPadding(inset, 0, 0, 0, 10);
			Widget border = inset.FindAnyWidget("InsetBorder");
			Widget background = inset.FindAnyWidget("InsetBG");
			TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_ROW);
			TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_ROW - 1);
			TBD_UITheme.PaintOver(border, TBD_UITheme.KIT_CARD_BORDER, m_iGround);
			TBD_UITheme.PaintOver(background, TBD_UITheme.INSET_FILL, m_iGround);

			TextWidget body = TextWidget.Cast(inset.FindAnyWidget("Body"));
			TBD_UITheme.Write(body, paragraph);
			TBD_UITheme.Paint(body, TBD_UITheme.ON_SURFACE);
		}
	}
}
