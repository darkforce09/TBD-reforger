/**
 * @file TBD_ObjectiveText.c
 * @brief Player-facing objective board text: one line per objective, from one side's view.
 *
 * Role: renders an objective's title and status as a viewer on one side reads them.
 * Position: called by `TBD_ObjectiveHudPublisher` for the HUD rows and by
 * `TBD_ObjectivesComponent.BuildBoardForPlayer`; reads `TBD_Objective` and takes each usable
 * objective's status from its kind's `TBD_ObjectiveKindBehaviour.StatusText`.
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
	//! @param objective the objective
	//! @param viewerFaction the viewer's side from server-owned state; may be empty
	//! @return `inactive` for an inert objective, else its kind behaviour's status text
	static string StatusText(notnull TBD_Objective objective, string viewerFaction)
	{
		if (!objective.m_bUsable)
			return "inactive";

		TBD_ObjectiveKindBehaviour behaviour = TBD_ObjectiveKindBehaviour.For(objective.m_eKind);
		return behaviour.StatusText(objective, viewerFaction);
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
