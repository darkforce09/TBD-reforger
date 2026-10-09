/**
 * @file TBD_EBriefingPage.c
 * @brief The Briefing screen's ten topic pages, in topic navigation order.
 *
 * Role: names the topic pages.  Position: TBD_BriefingTopicNav raises the index; TBD_BriefingNav
 * maps it to a width and a page; TBD_BriefingScreen.ShowPage shows it.
 * State: none.  Invariants: the member order is the item order of TBD_BriefingNav.TopicItems.
 */

//! One Briefing topic page.
enum TBD_EBriefingPage
{
	FREQUENCIES, //!< radio nets
	ORBAT, //!< read-only roster and kit inspector
	FRIENDLY_ASSETS, //!< the reader's side's vehicles
	FRIENDLY_UNIFORMS, //!< the reader's side's uniforms
	ENEMY_ASSETS, //!< the other side's vehicles
	ENEMY_UNIFORMS, //!< the other side's uniforms
	OBJECTIVES, //!< time limit and objective cards
	RULES, //!< rule groups
	BACKGROUND, //!< lore paragraphs
	PARAMETERS //!< mission parameters
}
