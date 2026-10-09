/**
 * @file TBD_ObjectiveProgression.c
 * @brief Advances every usable objective by one 1 Hz tick through its kind behaviour.
 *
 * Role: the per-tick advancement loop: hands each usable objective to its kind's
 * `TBD_ObjectiveKindBehaviour.Advance` and collects the completion lines the behaviours return for
 * chat.  Position: owned by `TBD_ObjectivesComponent`, which samples presence, calls `Advance` for
 * each usable objective while the stage is LIVE, then delivers `GetCompletionLines`; the capture,
 * hold and destroy rules live in the behaviours under `Gamemode/Objectives/Types/`.
 * State: this tick's completion lines, owned by the component on the server.  Invariants: one
 * behaviour call per usable objective per tick, in registry order; a behaviour's empty return
 * queues nothing; only CAPTURED, DESTROYED and HELD reach chat.
 */

//! Per-tick objective advancement for one objectives component.
class TBD_ObjectiveProgression : Managed
{
	protected ref array<string> m_aCompletionLines; //!< this tick's completion lines (captured, destroyed, held); progress goes to the HUD

	//! Allocate the completion line buffer.
	void TBD_ObjectiveProgression()
	{
		m_aCompletionLines = new array<string>();
	}

	//! Start a tick: drop the previous tick's completion lines.
	void BeginTick()
	{
		m_aCompletionLines.Clear();
	}

	//! This tick's completion lines, in the order the objectives completed.
	array<string> GetCompletionLines()
	{
		return m_aCompletionLines;
	}

	//! Advance one usable objective by one tick through its kind behaviour, and queue the
	//! completion line the behaviour returns on the tick the objective completes.
	//! @param objective a usable objective
	//! @authority server
	void Advance(notnull TBD_Objective objective)
	{
		TBD_ObjectiveKindBehaviour behaviour = TBD_ObjectiveKindBehaviour.For(objective.m_eKind);
		string line = behaviour.Advance(objective);
		if (!line.IsEmpty())
			m_aCompletionLines.Insert(line);
	}
}
