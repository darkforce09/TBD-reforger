/**
 * @file TBD_ObjectiveText.c
 * @brief Player-facing objective board text: one line per objective, from one side's view.
 *
 * Role: renders an objective's title and status as a viewer on one side reads them.
 * Position: called by `TBD_ObjectiveHudPublisher` for the HUD rows and by
 * `TBD_ObjectivesComponent.BuildBoardForPlayer`; reads `TBD_Objective`.
 * State: none; pure functions.  Invariants: output is ASCII (`--`, never an arrow glyph); the
 * viewer's side always comes from server-owned slot state, never from a client; an unframed
 * objective renders exactly its label and status.
 */

//! Objective board and status text.
class TBD_ObjectiveText
{
	//! One board line: the viewer's title and the status in brackets.
	//! @param viewerFaction the viewer's side from server-owned state; may be empty
	//! @return `<title> [<status>]`
	static string BoardLine(notnull TBD_Objective objective, string viewerFaction)
	{
		string line = objective.TitleFor(viewerFaction);
		line += " [";
		line += StatusText(objective, viewerFaction);
		line += "]";
		return line;
	}

	//! The status half of a board line.
	//! @return `inactive` for an inert objective, else the kind's status text
	static string StatusText(notnull TBD_Objective objective, string viewerFaction)
	{
		if (!objective.m_bUsable)
			return "inactive";

		if (objective.m_eKind == TBD_EObjectiveKind.DESTROY)
			return DestroyStatusText(objective);

		if (objective.m_eKind == TBD_EObjectiveKind.HOLD_UNTIL)
			return HoldStatusText(objective);

		return CaptureStatusText(objective, viewerFaction);
	}

	//! DESTROY status: `DESTROYED`, or `intact <destroyed>/<required>`.
	protected static string DestroyStatusText(notnull TBD_Objective objective)
	{
		if (objective.m_bComplete)
			return "DESTROYED";

		string text = "intact ";
		text += objective.m_iTargetsDestroyed.ToString();
		text += "/";
		text += objective.RequiredKills().ToString();
		return text;
	}

	//! HOLD_UNTIL status: `HELD`, or `hold <n>s left`, with ` (PAUSED)` while the clock stands.
	protected static string HoldStatusText(notnull TBD_Objective objective)
	{
		if (objective.m_bComplete)
			return "HELD";

		// Rounded into an int first, so the text never shows float precision ("600.000000").
		int remaining = Math.Round(objective.HoldRemaining());

		string text = "hold ";
		text += remaining.ToString();
		text += "s left";
		if (objective.m_bHoldPaused)
			text += " (PAUSED)";

		return text;
	}

	//! CAPTURE status: `neutral`, `OURS` or `held by <side>`, then ` -- CONTESTED` or the
	//! partial progress percentage and the side banking it.
	protected static string CaptureStatusText(notnull TBD_Objective objective, string viewerFaction)
	{
		string text;

		if (objective.m_sOwner.IsEmpty())
		{
			text = "neutral";
		}
		else if (!viewerFaction.IsEmpty() && objective.m_sOwner == viewerFaction)
		{
			text = "OURS";
		}
		else
		{
			text = "held by ";
			text += objective.m_sOwner;
		}

		if (objective.m_bContested)
		{
			text += " -- CONTESTED";
			return text;
		}

		int percent = objective.ProgressPercent();
		if (percent > 0 && percent < 100)
		{
			text += " -- ";
			text += percent.ToString();
			text += "% ";
			text += objective.m_sProgressFaction;
		}

		return text;
	}

	//! The whole board as `factionKey` reads it: each objective's line, and under it, indented,
	//! that side's task text when the side was framed.
	//! @param board the prepared objectives; null or empty yields the no-objectives line
	//! @param factionKey the viewer's side from server-owned state
	//! @return the board lines, never empty
	static array<string> BoardForFaction(array<ref TBD_Objective> board, string factionKey)
	{
		array<string> lines = new array<string>();

		if (!board || board.IsEmpty())
		{
			lines.Insert("TBD: this mission has no objectives.");
			return lines;
		}

		foreach (TBD_Objective objective : board)
		{
			if (!objective)
				continue;

			lines.Insert(BoardLine(objective, factionKey));

			// The task body goes on its own indented continuation line.
			string task = objective.TaskTextFor(factionKey);
			if (task.IsEmpty())
				continue;

			string detail = "    ";
			detail += task;
			lines.Insert(detail);
		}

		return lines;
	}
}
