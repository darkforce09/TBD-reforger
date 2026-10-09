/**
 * @file TBD_MatchEventRecorder.c
 * @brief Buffers the detailed events of the LIVE round and flushes them to the telemetry queue.
 *
 * Role: decides whether an event is recorded, stamps its mission time and capture time, keeps it
 * in capture order, and hands batches to `TBD_MatchEventBatch` with sequences reserved from
 * `TBD_TelemetryQueue`.  Position: `BeginRound` and `EndRound` are called by `TBD_ResultsReporter`
 * as it registers and reports a round; `Capture` by `TBD_MatchEventCapture`; `Tick` every
 * `TICK_MS` and `Clear` at game start by `TBD_RuntimeHeartbeat`.
 * State: the recorded round's source match id, its LIVE tick and the unsent events; server
 * statics.  Invariants: events are recorded only while a registered round is LIVE; sequences are
 * reserved at flush in buffer order, which is capture order, so `sequence` is the capture order
 * and `event_id` is the sequence as a decimal string; a batch holds at most `BATCH_MAX` events;
 * nothing buffered is dropped by a round change, which flushes first.
 */

//! One captured event before it is numbered.
class TBD_MatchEventRecord
{
	string m_sKind; //!< JSON `kind`, one of the `TBD_MatchEventWire.KIND_` names
	string m_sPayload; //!< JSON `payload`, the complete payload object
	int m_iMissionTimeMs; //!< JSON `mission_time_ms`: milliseconds since the round went LIVE
	string m_sOccurredAtUtc; //!< JSON `occurred_at`: capture time, RFC 3339 UTC
}

//! Recorder of the LIVE round's detailed events.
//! @authority server
class TBD_MatchEventRecorder
{
	static const string CH_EVENTS = "MatchEvents"; //!< log channel: `grep '\[TBD\]\[MatchEvents\]' console.log`
	static const int TICK_MS = 10000; //!< flush period in milliseconds, a whole multiple of the heartbeat beat
	static const int BATCH_MAX = 100; //!< events per queued batch at most

	protected static string s_sSourceMatchId; //!< the recorded round's source match id; empty when no round records
	protected static int s_iLiveTick; //!< `System.GetTickCount` when the recorded round began
	protected static ref array<ref TBD_MatchEventRecord> s_aPending; //!< captured, unsent events in capture order

	//! Start recording the round `TBD_MatchRegistration` just registered: flush what an earlier
	//! round left, clear the tally and start the mission clock.
	//! @authority server
	static void BeginRound()
	{
		if (TBD_Authority.IsClient())
			return;

		Flush();
		s_sSourceMatchId = TBD_MatchRegistration.GetSourceMatchId();
		s_iLiveTick = System.GetTickCount();
		TBD_MatchTelemetryTally.Clear();
		TBD_Log.Kv(CH_EVENTS, "recording", string.Format("source='%1'", s_sSourceMatchId));
	}

	//! Stop recording and flush every buffered event of the round; the tally stays readable
	//! until the next round begins.
	//! @authority server
	static void EndRound()
	{
		if (TBD_Authority.IsClient())
			return;

		Flush();
		if (!s_sSourceMatchId.IsEmpty())
			TBD_Log.Kv(CH_EVENTS, "stopped", string.Format("source='%1' last_sequence=%2", s_sSourceMatchId,
				TBD_TelemetryQueue.LastEventSequence(s_sSourceMatchId)));

		s_sSourceMatchId = string.Empty;
	}

	//! Flush what the previous world left and forget its round and tally; statics outlive a world
	//! inside one process. Called on every machine at game start.
	static void Clear()
	{
		EndRound();
		s_sSourceMatchId = string.Empty;
		s_aPending = null;
		TBD_MatchTelemetryTally.Clear();
	}

	//! Whether an event happening now is recorded: on the authority, with a registered round, in
	//! a framework world whose stage is LIVE.
	//! @return true while recording
	//! @authority server
	static bool IsRecording()
	{
		if (s_sSourceMatchId.IsEmpty() || TBD_Authority.IsClient())
			return false;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		return fm && fm.GetStage() == TBD_EGameStage.LIVE;
	}

	//! Record one event of `kind` now; does nothing unless `IsRecording`. A full batch flushes at
	//! once.
	//! @param kind one of the `TBD_MatchEventWire.KIND_` names
	//! @param payload the kind's complete payload object
	//! @authority server
	static void Capture(string kind, string payload)
	{
		if (!IsRecording())
			return;

		TBD_MatchEventRecord record = new TBD_MatchEventRecord();
		record.m_sKind = kind;
		record.m_sPayload = payload;
		record.m_iMissionTimeMs = Math.Max(0, System.GetTickCount() - s_iLiveTick);
		record.m_sOccurredAtUtc = TBD_BackendText.UtcNowIso8601();

		if (!s_aPending)
			s_aPending = new array<ref TBD_MatchEventRecord>();

		s_aPending.Insert(record);
		if (s_aPending.Count() >= BATCH_MAX)
			Flush();
	}

	//! The heartbeat's flush: queue every buffered event.
	//! @authority server
	static void Tick()
	{
		Flush();
	}

	//! Number and queue every buffered event in batches of at most `BATCH_MAX`, in capture order.
	//! A batch the queue refuses is logged by `TBD_MatchEventBatch.Enqueue` and its sequences stay
	//! reserved, so a later batch never reuses them.
	//! @authority server
	protected static void Flush()
	{
		if (!s_aPending || s_aPending.IsEmpty() || s_sSourceMatchId.IsEmpty())
			return;

		int total = s_aPending.Count();
		int first = TBD_TelemetryQueue.ReserveEventSequences(s_sSourceMatchId, total);
		array<string> batch = {};
		for (int i = 0; i < total; i++)
		{
			TBD_MatchEventRecord record = s_aPending[i];
			batch.Insert(TBD_MatchEventWire.BuildEvent(first + i, record.m_sKind, record.m_iMissionTimeMs,
				record.m_sOccurredAtUtc, record.m_sPayload));
			if (batch.Count() == BATCH_MAX || i == total - 1)
			{
				TBD_MatchEventBatch.Enqueue(s_sSourceMatchId, batch);
				batch = {};
			}
		}

		s_aPending.Clear();
		TBD_Log.Kv(CH_EVENTS, "flushed", string.Format("source='%1' events=%2 sequences=%3..%4", s_sSourceMatchId, total,
			first, first + total - 1));
	}
}
