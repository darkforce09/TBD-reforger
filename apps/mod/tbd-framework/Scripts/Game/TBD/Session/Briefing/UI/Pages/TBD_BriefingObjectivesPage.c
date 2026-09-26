/**
 * @file TBD_BriefingObjectivesPage.c
 * @brief The Objectives page: time limit and numbered objective cards.
 *
 * Role: draws the directives caption, the time limit and one card per objective with its type,
 * capture time, retake rule and a Locate button.  Position: TBD_BriefingNav.CreatePage creates it; TBD_BriefingScreen builds it into the page column.
 * State: none beyond TBD_BriefingPage.  Invariants: reads TBD_BriefingCatalog only.
 */

//! `objectives_panel`: time-limit bar, numbered objective cards with a stat grid and Locate.
class TBD_BriefingObjectivesPage : TBD_BriefingPage
{
	//! @return the panel title
	override string Title() { return "Mission Objectives"; }
	//! @return the header icon key
	override string Icon()  { return "target"; }

	//! Fill the page from the catalog's capture summary, time limit and objectives.
	//! @param content the scroll list column
	override void Fill(Widget content)
	{
		TBD_Caption.Mount(content, "Directives", m_Catalog.GetCaptureSummary());
		TBD_KeyValueRowComponent limit = AddRow(content, "Time Limit", m_Catalog.GetTimeLimit(), m_iGround);
		if (limit)
			limit.SetIcon("timer", TBD_UITheme.MUTED_INK);

		TBD_Caption.Mount(content, "Objectives", string.Format("%1 total", m_Catalog.GetObjectives().Count()));
		foreach (TBD_ObjectiveInfo objective : m_Catalog.GetObjectives())
		{
			TBD_NumberedCardComponent card = TBD_NumberedCardComponent.Mount(content, objective.m_iIndex, objective.m_sTitle, m_iGround);
			if (!card)
				continue;

			card.SetChip(objective.m_sRoleLabel, TBD_EUITint.NEUTRAL);
			int cardGround = card.GetGround();

			array<ref TBD_KitEntry> stats = {};
			stats.Insert(new TBD_KitEntry("Type", objective.m_sType));
			stats.Insert(new TBD_KitEntry("Capture Time", objective.m_sCaptureTime));
			AddCellGrid(card.GetBodyDock(), stats, 2, cardGround, false);

			array<ref TBD_KitEntry> retake = {};
			retake.Insert(new TBD_KitEntry("Retake", objective.m_sRetake, 0, TBD_EUITint.SUCCESS));
			AddCellGrid(card.GetBodyDock(), retake, 1, cardGround, false);

			AddLocate(card.GetFooterDock(), objective.m_fX, objective.m_fZ);
		}
	}
}
