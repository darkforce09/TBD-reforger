/**
 * @file TBD_BriefingNav.c
 * @brief The topic navigation's item table, the page widths and the page factory.
 *
 * Role: lists the ten topics in three groups, gives each page its column width and creates its
 * page.  Position: TBD_BriefingTopicNav reads the items; TBD_BriefingScreen.ShowPage reads the width and creates the page.
 * State: none; pure functions.  Invariants: the item order matches TBD_EBriefingPage; widths are
 * the mockups' panel widths in pixels; the map behind never moves.
 */

//! Topic table and page factory of the Briefing screen.
class TBD_BriefingNav
{
	static const int WIDTH_MARKERS = 320; //!< markers panel width, pixels

	//! Append the ten topic items; Friendly Assets and Objectives carry a separator before them.
	//! @param outItems receives the items in TBD_EBriefingPage order
	static void TopicItems(notnull array<ref TBD_NavItemData> outItems)
	{
		outItems.Insert(new TBD_NavItemData("Frequencies", "radio"));
		outItems.Insert(new TBD_NavItemData("ORBAT", "groups"));
		outItems.Insert(Separated(new TBD_NavItemData("Friendly Assets", "directions_car")));
		outItems.Insert(new TBD_NavItemData("Friendly Uniforms", "checkroom"));
		outItems.Insert(new TBD_NavItemData("Enemy Assets", "directions_car"));
		outItems.Insert(new TBD_NavItemData("Enemy Uniforms", "checkroom"));
		outItems.Insert(Separated(new TBD_NavItemData("Objectives", "adjust")));
		outItems.Insert(new TBD_NavItemData("Rules", "warning"));
		outItems.Insert(new TBD_NavItemData("Background", "description"));
		outItems.Insert(new TBD_NavItemData("Parameters", "tune"));
	}

	//! Mark `item` to draw a separator above it.
	//! @return `item`
	protected static TBD_NavItemData Separated(TBD_NavItemData item)
	{
		item.m_bSeparatorBefore = true;
		return item;
	}

	//! The page column width; ORBAT holds the lobby roster and the kit inspector side by side.
	//! @param page the page
	//! @return the width in pixels; 448 for an unknown page
	static int PageWidth(TBD_EBriefingPage page)
	{
		switch (page)
		{
			case TBD_EBriefingPage.FREQUENCIES:       return 448;
			case TBD_EBriefingPage.ORBAT:             return 1200;
			case TBD_EBriefingPage.FRIENDLY_ASSETS:   return 576;
			case TBD_EBriefingPage.ENEMY_ASSETS:      return 576;
			case TBD_EBriefingPage.FRIENDLY_UNIFORMS: return 768;
			case TBD_EBriefingPage.ENEMY_UNIFORMS:    return 768;
			case TBD_EBriefingPage.OBJECTIVES:        return 440;
			case TBD_EBriefingPage.RULES:             return 480;
			case TBD_EBriefingPage.BACKGROUND:        return 440;
			case TBD_EBriefingPage.PARAMETERS:        return 440;
		}

		return 448;
	}

	//! Create the page object for `page`; the caller builds it.
	//! @param page the page
	//! @return a new page, or null for an unknown page
	static TBD_BriefingPage CreatePage(TBD_EBriefingPage page)
	{
		switch (page)
		{
			case TBD_EBriefingPage.FREQUENCIES:       return new TBD_BriefingFrequenciesPage();
			case TBD_EBriefingPage.ORBAT:             return new TBD_BriefingOrbatPage();
			case TBD_EBriefingPage.FRIENDLY_ASSETS:   return new TBD_BriefingAssetsPage(true);
			case TBD_EBriefingPage.ENEMY_ASSETS:      return new TBD_BriefingAssetsPage(false);
			case TBD_EBriefingPage.FRIENDLY_UNIFORMS: return new TBD_BriefingUniformsPage(true);
			case TBD_EBriefingPage.ENEMY_UNIFORMS:    return new TBD_BriefingUniformsPage(false);
			case TBD_EBriefingPage.OBJECTIVES:        return new TBD_BriefingObjectivesPage();
			case TBD_EBriefingPage.RULES:             return new TBD_BriefingRulesPage();
			case TBD_EBriefingPage.BACKGROUND:        return new TBD_BriefingBackgroundPage();
			case TBD_EBriefingPage.PARAMETERS:        return new TBD_BriefingParametersPage();
		}

		return null;
	}
}
