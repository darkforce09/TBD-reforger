/**
 * @file TBD_BriefingRulesPage.c
 * @brief The Rules page: collapsible groups of numbered rules.
 *
 * Role: draws one section per rule group, numbered from 1 within each group.  Position: TBD_BriefingNav.CreatePage creates it; TBD_BriefingScreen builds it into the page column.
 * State: none beyond TBD_BriefingPage.  Invariants: reads TBD_BriefingCatalog only.
 */

//! `rules_panel`: two collapsible groups of numbered rules.
class TBD_BriefingRulesPage : TBD_BriefingPage
{
	//! @return the panel title
	override string Title() { return "Rules"; }
	//! @return the header icon key
	override string Icon()  { return "warning"; }

	//! Fill the page from the catalog's rule groups.
	//! @param content the scroll list column
	override void Fill(Widget content)
	{
		foreach (TBD_RuleGroup group : m_Catalog.GetRuleGroups())
		{
			TBD_SectionComponent section = TBD_SectionComponent.Mount(content, group.m_sTitle, m_iGround);
			if (!section)
				continue;

			section.SetIcon("assignment");
			section.SetExpanded(group.m_bOpen);
			int bodyGround = section.GetBodyGround();
			int number = 1;
			foreach (TBD_RuleInfo rule : group.m_aRules)
			{
				TBD_NumberedCardComponent card = TBD_NumberedCardComponent.Mount(section.GetBody(), number, rule.m_sTitle, bodyGround);
				if (card)
					card.SetBody(rule.m_sBody);
				number++;
			}
		}
	}
}
