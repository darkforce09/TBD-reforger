/**
 * @file TBD_BriefingFrequenciesPage.c
 * @brief The Frequencies page: the long-range command net and the short-range squad nets.
 *
 * Role: draws one row per radio net with its frequency chip and auxiliary channels.  Position: TBD_BriefingNav.CreatePage creates it; TBD_BriefingScreen builds it into the page column.
 * State: none beyond TBD_BriefingPage.  Invariants: long-range nets come first in gold; the reader's
 * own squad net is highlighted.
 */

//! `frequencies_panel`: the LR command net (gold) and the SR squad nets; the reader's own squad
//! net is highlighted.
class TBD_BriefingFrequenciesPage : TBD_BriefingPage
{
	//! @return the panel title
	override string Title() { return "Radio Frequencies Net"; }
	//! @return the header icon key
	override string Icon()  { return "cell_tower"; }

	//! Fill the page: the long-range nets, then the short-range nets with their count.
	//! @param content the scroll list column
	override void Fill(Widget content)
	{
		array<ref TBD_NetInfo> nets = m_Catalog.GetNets();

		TBD_Caption.Mount(content, "Long Range (LR) Command");
		foreach (TBD_NetInfo lr : nets)
		{
			if (lr.m_bLongRange)
				AddNet(content, lr);
		}

		int squadNets;
		foreach (TBD_NetInfo count : nets)
		{
			if (!count.m_bLongRange)
				squadNets++;
		}

		TBD_Caption.Mount(content, "Short Range (SR) Squad Nets", string.Format("%1 nets", squadNets));
		foreach (TBD_NetInfo sr : nets)
		{
			if (!sr.m_bLongRange)
				AddNet(content, sr);
		}
	}

	//! Add one net row, tinted gold for long range and highlighted for the reader's own net.
	//! @param content the column to add to
	//! @param net the net
	protected void AddNet(Widget content, TBD_NetInfo net)
	{
		Widget row = TBD_UILayouts.CreateStretched(TBD_UILayouts.BRIEFING_FREQ_ROW, content);
		if (!row)
			return;

		Widget border = row.FindAnyWidget("Border");
		Widget background = row.FindAnyWidget("Background");
		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_ROW - 1);

		int borderTone = TBD_UITheme.KIT_CARD_BORDER;
		TBD_EUITint chipTint = TBD_EUITint.NEUTRAL;
		int nameInk = TBD_UITheme.ON_SURFACE;
		if (net.m_bLongRange)
		{
			chipTint = TBD_EUITint.WARNING;
			nameInk = TBD_UITheme.BRIGHT_INK;
		}
		else if (net.m_bOwn)
		{
			chipTint = TBD_EUITint.PRIMARY;
			nameInk = TBD_UITheme.BRIGHT_INK;
			borderTone = TBD_UITheme.ROW_SELECTED_BORDER;
		}

		TBD_UITheme.PaintOver(border, borderTone, m_iGround);
		TBD_UITheme.PaintOver(background, TBD_UITheme.KIT_CARD_FILL, m_iGround);
		int rowGround = TBD_UITheme.Over(TBD_UITheme.KIT_CARD_FILL, m_iGround);

		TextWidget name = TextWidget.Cast(row.FindAnyWidget("Name"));
		TBD_UITheme.Write(name, net.Title());
		TBD_UITheme.Paint(name, nameInk);

		TBD_ChipComponent chip = TBD_ChipComponent.Mount(row.FindAnyWidget("FreqChipDock"), net.FreqText(), chipTint, rowGround);
		if (chip)
			chip.SetUppercase(false);

		TBD_UITheme.PaintOver(row.FindAnyWidget("AuxRule"), TBD_UITheme.KIT_CARD_BORDER, rowGround);
		TextWidget auxLabel = TextWidget.Cast(row.FindAnyWidget("AuxLabel"));
		TextWidget auxValue = TextWidget.Cast(row.FindAnyWidget("AuxValue"));
		if (net.m_bLongRange)
			TBD_UITheme.Write(auxLabel, "Auxiliary Fallback:");
		else
			TBD_UITheme.Write(auxLabel, "Aux Channels:");
		TBD_UITheme.Write(auxValue, net.AuxText());
		TBD_UITheme.Paint(auxLabel, TBD_UITheme.DIM_INK);
		TBD_UITheme.Paint(auxValue, TBD_UITheme.MUTED_INK);
	}
}
