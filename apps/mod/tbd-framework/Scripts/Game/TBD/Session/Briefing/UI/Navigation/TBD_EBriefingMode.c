/**
 * @file TBD_EBriefingMode.c
 * @brief The Briefing screen's modes, in primary navigation order.
 *
 * Role: names the four primary navigation items.  Position: TBD_BriefingPrimaryNav raises the index;
 * TBD_BriefingScreen.SetMode switches on it.
 * State: none.  Invariants: the member order is the item order of TBD_BriefingPrimaryNav.Build.
 */

//! One Briefing screen mode.
enum TBD_EBriefingMode
{
	MAP, //!< the map alone
	BRIEFING, //!< topic navigation and the selected page
	PLAYERS, //!< the players panel in WideDock
	MARKERS //!< the markers panel in CenterDock
}
