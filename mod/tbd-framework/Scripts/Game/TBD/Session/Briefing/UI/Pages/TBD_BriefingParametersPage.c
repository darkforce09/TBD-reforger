/**
 * @file TBD_BriefingParametersPage.c
 * @brief The Parameters page: icon, label and mono value rows.
 *
 * Role: draws one key-value row per mission parameter.  Position: TBD_BriefingNav.CreatePage creates it; TBD_BriefingScreen builds it into the page column.
 * State: none beyond TBD_BriefingPage.  Invariants: reads TBD_BriefingCatalog only.
 */

//! `parameters_panel`: icon - label - mono value rows.
class TBD_BriefingParametersPage : TBD_BriefingPage
{
	//! @return the panel title
	override string Title() { return "Parameters"; }
	//! @return the header icon key
	override string Icon()  { return "tune"; }

	//! Fill the page from the catalog's parameters.
	//! @param content the scroll list column
	override void Fill(Widget content)
	{
		foreach (TBD_ParamInfo param : m_Catalog.GetParams())
		{
			TBD_KeyValueRowComponent row = AddRow(content, param.m_sLabel, param.m_sValue, m_iGround, param.m_eTint);
			if (row)
				row.SetIcon(param.m_sIcon, TBD_UITheme.MUTED_INK);
		}
	}
}
