/**
 * @file TBD_EGameStage.c
 * @brief The seven stages of a TBD round, LOADING to DEBRIEF.
 *
 * Role: the one stage vocabulary every folder reads.  Position: owned by
 * TBD_FrameworkManager.SetStage; replicated as its m_Stage and read across API/, Session/,
 * Systems/ and UI/.  State: none.
 * Invariants: declaration order is the round order and the admin `#tbd stage next` order.
 */

//! Round stage, in round order.
enum TBD_EGameStage
{
	LOADING, //!< the mission loads and the roster and loadouts settle; the initial stage
	LOBBY, //!< players pick slots; the safe start shield is armed
	BRIEFING, //!< each side reads its brief and plans; shield armed
	SAFE_START, //!< the countdown to LIVE runs; shield armed
	LIVE, //!< the round is played; damage is on and the end rules are checked
	END, //!< the round is over; the END banner names the winner and reason
	DEBRIEF //!< the scoreboard is shown
}
