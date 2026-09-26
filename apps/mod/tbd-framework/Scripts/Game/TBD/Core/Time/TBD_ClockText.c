/**
 * @file TBD_ClockText.c
 * @brief Countdown clock text and the second marks at which players are told the time left.
 *
 * Role: formats `m:ss` clocks and names the countdown milestones for chat and pop-ups.
 * Position: called by the safestart manager and the round clock of the framework manager.
 * State: none.  Invariants: `FormatClock` never returns a negative clock; the popup milestones
 * are a superset of the countdown chat milestones.
 */

//! Stateless countdown text and milestone predicates.
class TBD_ClockText
{
	//! `4:30`, `0:09`: minutes unpadded, seconds always two digits.
	//! @param seconds the time left; zero or less formats as `0:00`
	//! @return the clock text
	static string FormatClock(int seconds)
	{
		if (seconds <= 0)
			return "0:00";

		int minutes = seconds / 60;
		int rest = seconds % 60;
		return string.Format("%1:%2", minutes, rest.ToString(2));
	}

	//! The safestart chat ladder: 10 and 5 minutes, then 2 and 1 minutes, 30 s and 10 s. Sparse,
	//! because chat is durable and becomes noise fastest.
	//! @return true when `seconds` is on the ladder
	static bool IsCountdownChatMilestone(int seconds)
	{
		if (seconds == 600)
			return true;
		if (seconds == 300)
			return true;
		if (seconds == 120)
			return true;
		if (seconds == 60)
			return true;
		if (seconds == 30)
			return true;
		if (seconds == 10)
			return true;
		return false;
	}

	//! The safestart pop-up ladder: every chat milestone plus 4 and 3 minutes, 15 s, and each of
	//! the last five seconds. Denser, because a pop-up is transient.
	//! @return true when `seconds` is on the ladder
	static bool IsCountdownPopupMilestone(int seconds)
	{
		if (IsCountdownChatMilestone(seconds))
			return true;
		if (seconds == 240)
			return true;
		if (seconds == 180)
			return true;
		if (seconds == 15)
			return true;
		if (seconds > 0 && seconds <= 5)
			return true;
		return false;
	}

	//! The round clock chat ladder: 30 and 15 minutes, then the countdown chat ladder from 10
	//! minutes down.
	//! @return true when `seconds` is on the ladder
	static bool IsRoundClockMilestone(int seconds)
	{
		if (seconds == 1800)
			return true;
		if (seconds == 900)
			return true;
		return IsCountdownChatMilestone(seconds);
	}
}
