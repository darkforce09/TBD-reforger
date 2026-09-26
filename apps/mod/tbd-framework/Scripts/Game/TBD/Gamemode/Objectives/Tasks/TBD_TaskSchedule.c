/**
 * @file TBD_TaskSchedule.c
 * @brief The mission clock tasks are scheduled on, and each task's `schedule` window.
 *
 * Role: measures mission seconds since the stage went LIVE, binds a task's authored window,
 * opens it once, fails a task still assigned when it closes, and says whether a task is being
 * evaluated.  Position: called by `TBD_TaskStateMachine` from its 1 Hz tick; reads
 * `TBD_FrameworkManager` for the stage.
 * State: the latched LIVE start time and the last measured elapsed seconds, for the current
 * world, cleared by `TBD_TaskStateMachine.Clear`.  Invariants: the clock reads 0 outside LIVE and
 * restarts on the next LIVE entry; an untimed task is always evaluated; a timed task is evaluated
 * only inside [startAfterS, startAfterS + windowS).
 */

//! Mission clock and task schedule windows.
class TBD_TaskSchedule
{
	protected static bool s_bLiveClockLatched; //!< the LIVE start time is latched
	protected static float s_fLiveStartMs; //!< world time of the LIVE entry, milliseconds
	protected static int s_iMissionElapsedS; //!< the last measured mission seconds since LIVE

	//! Reset the clock.
	static void Clear()
	{
		s_bLiveClockLatched = false;
		s_fLiveStartMs = 0;
		s_iMissionElapsedS = 0;
	}

	//! The mission seconds measured by the last `ElapsedS` call, for log lines.
	static int LastElapsedS()
	{
		return s_iMissionElapsedS;
	}

	//! Bind the authored window onto `task` when both keys are present, `startAfterS` >= 0 and
	//! `windowS` > 0; otherwise the task stays untimed. Presence is the ABSENT sentinel, because
	//! the nested `schedule` is always allocated.
	static void Bind(notnull TBD_Task task, notnull TBD_TaskStruct raw)
	{
		if (!raw.schedule)
			return;

		int startAfterS = raw.schedule.startAfterS;
		int windowS = raw.schedule.windowS;
		if (startAfterS == TBD_TaskScheduleStruct.ABSENT)
			return;

		if (windowS == TBD_TaskScheduleStruct.ABSENT)
			return;

		if (startAfterS < 0)
			return;

		if (windowS <= 0)
			return;

		task.m_bHasSchedule = true;
		task.m_iStartAfterS = startAfterS;
		task.m_iWindowS = windowS;
	}

	//! Mission seconds since the stage went LIVE; latches the LIVE start on the first call.
	//! World time is in milliseconds.
	//! @return the elapsed whole seconds; 0 outside LIVE or without a world
	static int ElapsedS()
	{
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
		{
			s_iMissionElapsedS = 0;
			return 0;
		}

		if (fm.GetStage() != TBD_EGameStage.LIVE)
		{
			s_bLiveClockLatched = false;
			s_iMissionElapsedS = 0;
			return 0;
		}

		if (!GetGame() || !GetGame().GetWorld())
		{
			s_iMissionElapsedS = 0;
			return 0;
		}

		float now = GetGame().GetWorld().GetWorldTime();
		if (!s_bLiveClockLatched)
		{
			s_bLiveClockLatched = true;
			s_fLiveStartMs = now;
		}

		int elapsed = (now - s_fLiveStartMs) / 1000;
		if (elapsed < 0)
			elapsed = 0;

		s_iMissionElapsedS = elapsed;
		return elapsed;
	}

	//! Open each timed task's window once, logging `id=<n> t=<s> -> assigned`, and fail a task
	//! still assigned once its window has closed. Does nothing outside LIVE.
	//! @param tasks the prepared tasks
	//! @param t the mission seconds from `ElapsedS`
	static void Evaluate(array<ref TBD_Task> tasks, int t)
	{
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		if (fm.GetStage() != TBD_EGameStage.LIVE)
			return;

		foreach (TBD_Task task : tasks)
		{
			if (!task)
				continue;

			if (!task.m_bHasSchedule)
				continue;

			if (task.m_eState != TBD_ETaskState.ASSIGNED)
				continue;

			if (t < task.m_iStartAfterS)
				continue;

			if (!task.m_bWindowOpened)
			{
				task.m_bWindowOpened = true;
				TBD_Log.Event(TBD_TaskStateMachine.CH, string.Format(
					"id=%1 t=%2 -> assigned",
					task.m_sId,
					t));
			}

			int endT = task.m_iStartAfterS + task.m_iWindowS;
			if (t >= endT)
				TBD_TaskStateMachine.TryTransition(task, TBD_ETaskState.FAILED);
		}
	}

	//! Whether `task` is evaluated at mission second `t`.
	static bool IsEvaluating(notnull TBD_Task task, int t)
	{
		if (!task.m_bHasSchedule)
			return true;

		if (!task.m_bWindowOpened)
			return false;

		int endT = task.m_iStartAfterS + task.m_iWindowS;
		if (t >= endT)
			return false;

		return true;
	}
}
