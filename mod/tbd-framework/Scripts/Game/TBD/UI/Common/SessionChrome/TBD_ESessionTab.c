/**
 * @file TBD_ESessionTab.c
 * @brief The three pre-game screens in top-bar order.
 *
 * Role: names the tabs of `TBD_SessionTopBar`; the value doubles as the tab strip index.
 * Position: reported by `TBD_SessionTopBar.GetOnTabSelected()`; mapped to a menu preset by
 * `TBD_DockScreen`; returned by each dock screen's `GetSessionTab`.
 * State: none.
 * Invariants: member order equals the order of the top bar's tabs.
 */

//! The pre-game screen a top-bar tab opens.
enum TBD_ESessionTab
{
	SCENARIO_BROWSER, //!< tab 0: the mission selector
	LOBBY, //!< tab 1: the lobby
	BRIEFING //!< tab 2: the briefing
}
