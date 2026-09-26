/**
 * @file TBD_MissionFlowReport.c
 * @brief Puts the mission's `flow` block into force at load and announces the briefing length.
 *
 * Role: hands `flow.safeStartSeconds` to TBD_SafestartManager, logs one `[TBD][Flow]` line per
 * `flow` field naming the source of the value in force, and announces `flow.briefingSeconds`.
 * Position: called by TBD_LoadingGate once the mission is valid, and by TBD_FrameworkManager on
 * entering BRIEFING; reads TBD_MissionFlow and TBD_JipPolicy.
 * State: none.  Invariants: runs on the server only; an authored length the safe start refuses is
 * logged at ERROR and never clamped; the permitted-join label comes from TBD_JipPolicy.
 */

//! Load-time application and report of the `flow` block.
class TBD_MissionFlowReport
{
	//! Put every `flow` field into force and report each on its own line, authored or not. Runs once
	//! per mission load, before the stage machine leaves LOADING.
	//! @authority server
	static void Apply()
	{
		string source;

		int briefing = TBD_MissionFlow.ResolveSeconds(TBD_MissionFlow.RawBriefingSeconds(), source);
		ReportSeconds("briefingSeconds", briefing, source, "no briefing timer exists in this build");

		ApplySafeStartSeconds();

		int limit = TBD_MissionFlow.ResolveSeconds(TBD_MissionFlow.RawTimeLimitSeconds(), source);
		ReportSeconds("timeLimitSeconds", limit, source, "no time limit");

		ReportJip();
	}

	//! Log one duration field and the source of the value in force.
	//! @param field the `flow` key
	//! @param value the resolved seconds
	//! @param source a TBD_MissionFlow SRC_* label
	//! @param defaultLabel what is in force when the value is absent or invalid
	//! @authority server
	protected static void ReportSeconds(string field, int value, string source, string defaultLabel)
	{
		if (source == TBD_MissionFlow.SRC_INVALID)
		{
			TBD_Log.Error(TBD_MissionFlow.CH_FLOW, string.Format(
				"flow.%1 is NEGATIVE in the mission document (schema declares minimum 0) -- ignored, default in force: %2",
				field, defaultLabel));
			return;
		}

		if (source == TBD_MissionFlow.SRC_DEFAULT)
		{
			TBD_Log.Event(TBD_MissionFlow.CH_FLOW, string.Format(
				"flow.%1=<absent> (default: %2)", field, defaultLabel));
			return;
		}

		TBD_Log.Event(TBD_MissionFlow.CH_FLOW, string.Format("flow.%1=%2 (authored)", field, value));
	}

	//! Hand `flow.safeStartSeconds` to TBD_SafestartManager.AdminSetSeconds, the entry point
	//! `#tbd safestart <seconds>` drives, then log the manager's own StatusLine as read-back. A value
	//! outside the manager's range (including an authored `0`) is logged at ERROR with the manager's
	//! reason and the previous length stays in force; it is never clamped.
	//! @authority server
	protected static void ApplySafeStartSeconds()
	{
		string source;
		int seconds = TBD_MissionFlow.ResolveSeconds(TBD_MissionFlow.RawSafeStartSeconds(), source);

		TBD_SafestartManager safestart = TBD_SafestartManager.GetInstance();
		if (!safestart)
		{
			// Not an error: SetStage refuses SAFE_START on a world without the component.
			TBD_Log.Warn(TBD_MissionFlow.CH_FLOW,
				"flow.safeStartSeconds not applied -- TBD_SafestartManager is not on this game mode (SAFE_START is refused on this world anyway).");
			return;
		}

		string defaultLabel = string.Format("%1s (TBD_SafestartManager.DEFAULT_COUNTDOWN_SECONDS)",
			TBD_SafestartManager.DEFAULT_COUNTDOWN_SECONDS);

		if (source != TBD_MissionFlow.SRC_AUTHORED)
		{
			ReportSeconds("safeStartSeconds", seconds, source, defaultLabel);
			TBD_Log.Kv(TBD_MissionFlow.CH_FLOW, "safestart", safestart.StatusLine());
			return;
		}

		bool applied = false;
		string reply = safestart.AdminSetSeconds(seconds, applied);

		if (!applied)
		{
			// Appended in steps: one long format chain fails to compile ("Formula too complex").
			string refused = string.Format("flow.safeStartSeconds=%1 (authored) REFUSED by TBD_SafestartManager", seconds);
			refused += " -- " + reply;
			refused += " The authored length is NOT in force.";
			TBD_Log.Error(TBD_MissionFlow.CH_FLOW, refused);
			TBD_Log.Kv(TBD_MissionFlow.CH_FLOW, "safestart", safestart.StatusLine());
			return;
		}

		TBD_Log.Event(TBD_MissionFlow.CH_FLOW, string.Format("flow.safeStartSeconds=%1 (authored)", seconds));
		// Read-back: the manager quotes the length it now holds.
		TBD_Log.Kv(TBD_MissionFlow.CH_FLOW, "safestart", safestart.StatusLine());
	}

	//! Log the authored join policy, naming an unrecognised value at ERROR, and the stages it permits.
	//! @authority server
	protected static void ReportJip()
	{
		string raw = TBD_MissionFlow.RawJip();
		string permitted = TBD_JipPolicy.JoinsPermittedLabel();

		if (raw.IsEmpty())
		{
			TBD_Log.Event(TBD_MissionFlow.CH_FLOW, string.Format(
				"flow.jip=<absent> (default: %1) joins-permitted=%2",
				TBD_JipPolicy.Name(), permitted));
			return;
		}

		if (!TBD_JipPolicy.IsKnownString(raw))
		{
			TBD_Log.Error(TBD_MissionFlow.CH_FLOW, string.Format(
				"flow.jip='%1' is not a value this build understands -- falling back to '%2'. joins-permitted=%3",
				raw, TBD_JipPolicy.Name(), permitted));
			return;
		}

		TBD_Log.Event(TBD_MissionFlow.CH_FLOW, string.Format(
			"flow.jip=%1 (authored) joins-permitted=%2", raw, permitted));
	}

	//! Announce `flow.briefingSeconds` to the log and to every player. Advisory: BRIEFING has no
	//! timer, an admin advances it with `#tbd stage next`, because SAFE_START can be refused and a
	//! briefing ends when the side has finished planning.
	//! @authority server
	static void AnnounceBriefing()
	{
		if (TBD_Authority.IsClient())
			return;

		int seconds = TBD_MissionFlow.BriefingSeconds();
		if (seconds == TBD_MissionFlow.UNSET)
			return;

		if (seconds == 0)
		{
			TBD_Log.Kv(TBD_MissionFlow.CH_FLOW, "briefing",
				"authoredSeconds=0 -- the mission asks for no briefing pause; advance when ready");
			return;
		}

		string clock = TBD_ClockText.FormatClock(seconds);
		TBD_Log.Kv(TBD_MissionFlow.CH_FLOW, "briefing", string.Format(
			"authoredSeconds=%1 (%2) -- ADVISORY, no auto-advance; an admin advances with '#tbd stage next'",
			seconds, clock));

		string msg = "[TBD] BRIEFING -- the mission allows ";
		msg += clock;
		msg += " for orders. Read your side's brief now.";
		TBD_PlayerChat.Broadcast(TBD_MissionFlow.CH_FLOW, msg);
	}
}
