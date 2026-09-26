/**
 * @file TBD_RuntimeHeartbeat.c
 * @brief One game-mode loop that clears and ticks the mission runtimes in a fixed order.
 *
 * Role: clears the mission runtimes when a game mode starts and ticks them from one self-re-arming
 * CallLater loop, each at its own period.  Position: armed by the modded
 * `SCR_BaseGameMode.OnGameStart` in this file inside a framework world; calls the static `Clear`
 * and `Tick` of `TBD_TaskStateMachine`, `TBD_TaskHud`, `TBD_WinConditionEvaluator`,
 * `TBD_GroupState`, `TBD_WaypointRuntime`, `TBD_AudioEmitter`, `TBD_WeatherRuntime`,
 * `TBD_DynamicSpawner` and `TBD_TriggerRuntime`.
 * State: the armed flag and beat counter on the live game mode instance.  Invariants: at most one
 * loop per game mode instance; a loop stops re-arming once another game mode is the live one;
 * the server ticks in `SERVER_ORDER` and a remote client in `CLIENT_ORDER`; every runtime period
 * is a whole multiple of `BEAT_MS`.
 */

//! The ordered clear and tick lists of the mission runtimes the game-mode heartbeat drives.
//!
//! The order is explicit and fixed: `ClearRuntimes`, `TickServer` and `TickClient` call the
//! runtimes one by one, and `SERVER_ORDER`/`CLIENT_ORDER` name the same sequence for the arm-time
//! log line. On a beat where it is due, the win-condition evaluator ticks first, so the win rule
//! reads the state the previous beat left.
class TBD_RuntimeHeartbeat
{
	static const string CH = "Heartbeat"; //!< Log channel; lines read `[TBD][Heartbeat] ...`.
	static const int BEAT_MS = 1000; //!< Loop period in milliseconds; each runtime period is a multiple.
	static const string SERVER_ORDER = "WinCondition/2,Task,GroupState,Waypoint,Audio,Weather,DynamicSpawner,Trigger"; //!< TickServer order; /2 = every second beat
	static const string CLIENT_ORDER = "TaskHud"; //!< Remote-client tick order; matches TickClient.

	//! Clears the per-world statics of every runtime, in the fixed start order. Statics outlive a
	//! world inside one process, so this runs at the start of each world on every machine, framework
	//! world or not. Never fails.
	static void ClearRuntimes()
	{
		TBD_TaskStateMachine.Clear();
		TBD_TaskHud.Clear();
		TBD_WinConditionEvaluator.Clear();
		TBD_GroupState.Clear();
		TBD_WaypointRuntime.Clear();
		TBD_AudioEmitter.Clear();
		TBD_WeatherRuntime.Clear();
		TBD_DynamicSpawner.Clear();
		TBD_TriggerRuntime.Clear();
	}

	//! Writes the arm-time line naming this machine's side and its tick order, for example
	//! `[TBD][Heartbeat] armed side=server beatMs=1000 order=WinCondition/2,Task,...`.
	//! @authority server - on a remote client the line names the client side and order.
	static void LogArmed()
	{
		string side = "server";
		string order = SERVER_ORDER;
		if (TBD_Authority.IsClient())
		{
			side = "client";
			order = CLIENT_ORDER;
		}

		TBD_Log.Kv(CH, "armed", string.Format("side=%1 beatMs=%2 order=%3", side, BEAT_MS, order));
	}

	//! Runs one beat: the server list on the authority, the client list on a remote client.
	//! `beat` counts from 1 at the first beat after arming. Never fails.
	//! @authority server - a remote client runs only `TickClient`.
	static void Beat(int beat)
	{
		if (TBD_Authority.IsClient())
		{
			TickClient(beat);
			return;
		}

		TickServer(beat);
	}

	//! True when a runtime with period `periodMs` ticks on beat number `beat`.
	static bool IsDue(int beat, int periodMs)
	{
		return (beat * BEAT_MS) % periodMs == 0;
	}

	//! Ticks the server runtimes due on `beat`, in `SERVER_ORDER`. The win-condition evaluator
	//! stops ticking once it has ended the round. Never fails.
	//! @authority server
	protected static void TickServer(int beat)
	{
		if (IsDue(beat, TBD_WinConditionEvaluator.TICK_MS) && !TBD_WinConditionEvaluator.HasEnded())
			TBD_WinConditionEvaluator.Tick();

		if (IsDue(beat, TBD_TaskStateMachine.TICK_MS))
			TBD_TaskStateMachine.Tick();

		if (IsDue(beat, TBD_GroupState.TICK_MS))
			TBD_GroupState.Tick();

		if (IsDue(beat, TBD_WaypointRuntime.TICK_MS))
			TBD_WaypointRuntime.Tick();

		if (IsDue(beat, TBD_AudioEmitter.TICK_MS))
			TBD_AudioEmitter.Tick();

		if (IsDue(beat, TBD_WeatherRuntime.TICK_MS))
			TBD_WeatherRuntime.Tick();

		if (IsDue(beat, TBD_DynamicSpawner.TICK_MS))
			TBD_DynamicSpawner.Tick();

		if (IsDue(beat, TBD_TriggerRuntime.TICK_MS))
			TBD_TriggerRuntime.Tick();
	}

	//! Ticks the remote-client runtimes due on `beat`, in `CLIENT_ORDER`: the task HUD asks the
	//! server for its task snapshot at the task period. Never fails.
	//! @authority client
	protected static void TickClient(int beat)
	{
		if (IsDue(beat, TBD_TaskStateMachine.TICK_MS))
			TBD_TaskHud.RequestLocal();
	}
}

//! Arms the runtime heartbeat on the live game mode of a framework world.
modded class SCR_BaseGameMode
{
	protected bool m_bTBD_HeartbeatArmed; //!< Set once the loop is scheduled on this instance; default false.
	protected int m_iTBD_HeartbeatBeat; //!< Beats run since arming; default 0.

	//! Calls super, clears the runtimes, then in a framework world arms one heartbeat loop per game
	//! mode instance and logs its order. A second call on the same instance arms nothing, so the
	//! runtimes never tick twice per beat. Never fails.
	protected override void OnGameStart()
	{
		super.OnGameStart();

		TBD_RuntimeHeartbeat.ClearRuntimes();

		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		if (m_bTBD_HeartbeatArmed)
			return;

		m_bTBD_HeartbeatArmed = true;
		TBD_RuntimeHeartbeat.LogArmed();
		GetGame().GetCallqueue().CallLater(TBD_HeartbeatBeat, TBD_RuntimeHeartbeat.BEAT_MS, false);
	}

	//! Runs one beat, then re-arms while this is still the live game mode. A one-shot re-armed each
	//! beat, because `ScriptCallQueue.Remove` cancels by function and a game mode has no teardown
	//! hook: a stale timer from a replaced world runs once more, sees another game mode, and stops.
	void TBD_HeartbeatBeat()
	{
		if (GetGame().GetGameMode() != this)
			return;

		m_iTBD_HeartbeatBeat++;
		TBD_RuntimeHeartbeat.Beat(m_iTBD_HeartbeatBeat);
		GetGame().GetCallqueue().CallLater(TBD_HeartbeatBeat, TBD_RuntimeHeartbeat.BEAT_MS, false);
	}
}
