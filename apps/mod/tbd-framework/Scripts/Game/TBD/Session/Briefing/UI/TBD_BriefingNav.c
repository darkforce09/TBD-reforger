//! Briefing rebuild (2026-09-14) — the two navigation strips' tables and the page factory.
//!
//! Primary nav (LeftDock, `primary_navigation_panel`) is `TBD_BriefingPrimaryNav` (its own panel);
//! `TBD_EBriefingMode` is its item order.
//! Topic nav (CenterDock, `briefing_navigation_panel`): ten topics in three groups, separators
//! between the groups. Page widths are the mockups' panel widths; the map behind never moves.
enum TBD_EBriefingMode
{
	MAP,
	BRIEFING,
	PLAYERS,
	MARKERS
}

enum TBD_EBriefingPage
{
	FREQUENCIES,
	ORBAT,
	FRIENDLY_ASSETS,
	FRIENDLY_UNIFORMS,
	ENEMY_ASSETS,
	ENEMY_UNIFORMS,
	OBJECTIVES,
	RULES,
	BACKGROUND,
	PARAMETERS
}

class TBD_BriefingNav
{
	static const int WIDTH_MARKERS = 320;

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected static TBD_NavItemData Separated(TBD_NavItemData item)
	{
		item.m_bSeparatorBefore = true;
		return item;
	}

	//------------------------------------------------------------------------------------------------
	//! Content dock width per page (mockup panel widths; ORBAT = lobby roster + kit inspector).
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

	//------------------------------------------------------------------------------------------------
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
