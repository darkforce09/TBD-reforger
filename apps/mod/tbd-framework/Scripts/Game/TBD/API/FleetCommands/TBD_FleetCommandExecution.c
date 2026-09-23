//! One claimed fleet command, from its claim to its reported outcome. Every command follows the
//! same protocol with the platform:
//!   1. its arguments are checked again exactly as the platform validated them, and its
//!      preconditions on this server are tested (TBD_FleetCommandArguments); a command that fails
//!      here is reported failed and its effect never starts;
//!   2. `executing` is reported (`POST /api/v1/fleet-executor/commands/{command_id}/executing` with
//!      the fencing token) and must be admitted BEFORE the effect starts;
//!   3. the effect runs once (TBD_FleetPlayerActions, TBD_FleetLoadMissionAction);
//!   4. the outcome is reported (`.../{command_id}/result`): `succeeded` with the action's outcome,
//!      or `failed` with a reason of 1 to 512 bytes.
//! A report that gets no answer is sent again with backoff from 2 s to 30 s; the effect is never
//! repeated. A 409 (`STALE_FENCING_TOKEN` or any other) means the claim is no longer this
//! runtime's: the command is abandoned with no further effect and no further report. One 409 is
//! read differently: a result report refused because the command already stands `succeeded`. An
//! earlier attempt of this very report was recorded and its answer lost - once `executing` is
//! admitted under a claim, no other claim can finish the command - so what follows a recorded
//! success (the scenario restart of `load_mission`) still happens.
//! @authority server

//! Where a command stands in the protocol.
enum TBD_EFleetCommandStage
{
	CHECKING,
	REPORTING_EXECUTING,
	EFFECT,
	REPORTING_RESULT,
	DONE,
}

//! A claimed command and what its execution has produced so far.
class TBD_FleetCommand
{
	string m_sCommandId;
	string m_sAction;
	int m_iFencingToken;
	//! Every argument as text; null when the claim's arguments could not be read as text values.
	ref map<string, string> m_mArguments;
	//! The poller world that claimed the command (TBD_FleetCommandPoller).
	int m_iWorld;
	TBD_EFleetCommandStage m_eStage = TBD_EFleetCommandStage.CHECKING;
	//! The connected player a kick targets, resolved by the argument check.
	int m_iTargetPlayerId;

	bool m_bSucceeded;
	//! The `outcome` JSON object of a success.
	string m_sOutcome;
	string m_sFailureReason;
	//! A recorded success is followed by an in-process scenario restart (`load_mission`).
	bool m_bRestartAfterResult;
	int m_iReportFailures;

	//------------------------------------------------------------------------------------------------
	//! The argument `key` as sent, or empty.
	string Argument(string key)
	{
		string value;
		if (m_mArguments)
			m_mArguments.Find(key, value);

		return value;
	}

	//------------------------------------------------------------------------------------------------
	//! `key=value` pairs of every argument, for one log line.
	string DescribeArguments()
	{
		if (!m_mArguments)
			return "<unreadable>";

		string described;
		foreach (string key, string value : m_mArguments)
		{
			if (!described.IsEmpty())
				described += " ";

			described += string.Format("%1='%2'", key, value);
		}

		return described;
	}

	//------------------------------------------------------------------------------------------------
	string BuildResultBody()
	{
		if (m_bSucceeded)
		{
			string outcome = m_sOutcome;
			if (outcome.IsEmpty())
				outcome = "{}";

			return string.Format("{\"fencing_token\":%1,\"succeeded\":true,\"outcome\":%2}", m_iFencingToken, outcome);
		}

		return string.Format("{\"fencing_token\":%1,\"succeeded\":false,\"failure_reason\":\"%2\"}",
			m_iFencingToken, TBD_GameRuntimeHttp.JsonEscape(TBD_FleetCommandExecution.SafeReason(m_sFailureReason)));
	}
}

//! An `executing` or `result` report on its way to the platform.
class TBD_FleetReportCall : TBD_GameRuntimeCall
{
	ref TBD_FleetCommand m_Command;
	bool m_bResult;

	//------------------------------------------------------------------------------------------------
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_FleetCommandExecution.OnReportAnswered(this, answer);
	}
}

class TBD_FleetCommandExecution
{
	protected static const int REPORT_RETRY_BASE_MS = 2000;
	protected static const int REPORT_RETRY_CAP_MS = 30000;
	//! A failure reason is at most 512 bytes on the platform; kept below with room to spare.
	protected static const int FAILURE_REASON_MAX_BYTES = 500;

	//! The one command being executed or reported, or null.
	protected static ref TBD_FleetCommand s_Current;

	//------------------------------------------------------------------------------------------------
	static bool IsIdle()
	{
		return !s_Current;
	}

	//------------------------------------------------------------------------------------------------
	static TBD_FleetCommand GetCurrent()
	{
		return s_Current;
	}

	//------------------------------------------------------------------------------------------------
	//! A command was claimed: check it, then report `executing` or its failure.
	static void Begin(notnull TBD_FleetCommand command)
	{
		s_Current = command;
		TBD_Log.Kv(TBD_FleetCommandPoller.CH_FLEET, "claimed", string.Format("command=%1 action=%2 fencing=%3 arguments: %4",
			command.m_sCommandId, command.m_sAction, command.m_iFencingToken, command.DescribeArguments()));

		string refusal = TBD_FleetCommandArguments.Check(command);
		if (!refusal.IsEmpty())
		{
			Fail(command, refusal);
			return;
		}

		command.m_eStage = TBD_EFleetCommandStage.REPORTING_EXECUTING;
		SendReport(command, false);
	}

	//------------------------------------------------------------------------------------------------
	//! The effect succeeded: report it with `outcome` (a JSON object).
	static void Succeed(notnull TBD_FleetCommand command, string outcome, bool restartAfterResult)
	{
		if (command != s_Current || command.m_eStage == TBD_EFleetCommandStage.REPORTING_RESULT)
			return;

		command.m_bSucceeded = true;
		command.m_sOutcome = outcome;
		command.m_bRestartAfterResult = restartAfterResult;
		command.m_eStage = TBD_EFleetCommandStage.REPORTING_RESULT;
		TBD_Log.Kv(TBD_FleetCommandPoller.CH_FLEET, "succeeded", string.Format("command=%1 action=%2 outcome=%3", command.m_sCommandId, command.m_sAction, outcome));
		SendReport(command, true);
	}

	//------------------------------------------------------------------------------------------------
	//! The command failed, before or during its effect: report it with `reason`.
	static void Fail(notnull TBD_FleetCommand command, string reason)
	{
		if (command != s_Current || command.m_eStage == TBD_EFleetCommandStage.REPORTING_RESULT)
			return;

		command.m_bSucceeded = false;
		command.m_sFailureReason = reason;
		command.m_eStage = TBD_EFleetCommandStage.REPORTING_RESULT;
		TBD_Log.Warn(TBD_FleetCommandPoller.CH_FLEET, string.Format("command=%1 action=%2 FAILED - %3", command.m_sCommandId, command.m_sAction, reason));
		SendReport(command, true);
	}

	//------------------------------------------------------------------------------------------------
	protected static void SendReport(notnull TBD_FleetCommand command, bool result)
	{
		string path = TBD_FleetCommandPoller.ROUTE_PREFIX + "/" + command.m_sCommandId;
		string body = string.Format("{\"fencing_token\":%1}", command.m_iFencingToken);
		if (result)
		{
			path += "/result";
			body = command.BuildResultBody();
		}
		else
		{
			path += "/executing";
		}

		TBD_FleetReportCall call = new TBD_FleetReportCall();
		call.m_Command = command;
		call.m_bResult = result;

		string failure;
		if (TBD_GameRuntimeHttp.Post(call, path, body, failure))
			return;

		RetryReport(command, result, "not sent: " + failure);
	}

	//------------------------------------------------------------------------------------------------
	//! Called by TBD_FleetReportCall with the platform's answer to a report.
	static void OnReportAnswered(notnull TBD_FleetReportCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_FleetCommand command = call.m_Command;
		if (!command || command != s_Current)
			return;

		TBD_EFleetCommandStage expected = TBD_EFleetCommandStage.REPORTING_EXECUTING;
		if (call.m_bResult)
			expected = TBD_EFleetCommandStage.REPORTING_RESULT;

		if (command.m_eStage != expected)
			return;

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.SUCCESS)
		{
			command.m_iReportFailures = 0;
			if (call.m_bResult)
			{
				Finish(command);
				return;
			}

			command.m_eStage = TBD_EFleetCommandStage.EFFECT;
			StartEffect(command);
			return;
		}

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.TRANSIENT)
		{
			RetryReport(command, call.m_bResult, answer.m_sDetail);
			return;
		}

		if (call.m_bResult && command.m_bSucceeded && answer.m_Refusal && answer.m_Refusal.state == "succeeded")
		{
			TBD_Log.Kv(TBD_FleetCommandPoller.CH_FLEET, "result-already-recorded", string.Format("command=%1 - the platform recorded this success from an earlier attempt whose answer was lost",
				command.m_sCommandId));
			Finish(command);
			return;
		}

		string which = "executing";
		if (call.m_bResult)
			which = "result";

		Abandon(command, string.Format("the platform refused the %1 report (%2)", which, answer.m_sDetail));
	}

	//------------------------------------------------------------------------------------------------
	//! Retry a report that got no answer. The effect it reports is not repeated.
	protected static void RetryReport(notnull TBD_FleetCommand command, bool result, string detail)
	{
		command.m_iReportFailures++;
		int delay = TBD_GameRuntimeHttp.BackoffMs(command.m_iReportFailures, REPORT_RETRY_BASE_MS, REPORT_RETRY_CAP_MS);

		string which = "executing";
		if (result)
			which = "result";

		TBD_Log.Warn(TBD_FleetCommandPoller.CH_FLEET, string.Format("%1 report of command=%2 not admitted (%3) - attempt %4, sending it again in %5 ms",
			which, command.m_sCommandId, detail, command.m_iReportFailures, delay));

		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.CallLater(ResendReport, delay, false, command.m_sCommandId, result);
	}

	//------------------------------------------------------------------------------------------------
	protected static void ResendReport(string commandId, bool result)
	{
		if (!s_Current || s_Current.m_sCommandId != commandId)
			return;

		SendReport(s_Current, result);
	}

	//------------------------------------------------------------------------------------------------
	protected static void StartEffect(notnull TBD_FleetCommand command)
	{
		TBD_Log.Kv(TBD_FleetCommandPoller.CH_FLEET, "executing", string.Format("command=%1 action=%2 - the platform admitted the effect", command.m_sCommandId, command.m_sAction));

		if (command.m_sAction == "broadcast")
			TBD_FleetPlayerActions.Broadcast(command);
		else if (command.m_sAction == "kick")
			TBD_FleetPlayerActions.Kick(command);
		else if (command.m_sAction == "load_mission")
			TBD_FleetLoadMissionAction.Start(command);
		else
			Fail(command, string.Format("'%1' is not a game-runtime action", command.m_sAction));
	}

	//------------------------------------------------------------------------------------------------
	//! The outcome is recorded: the command is done, and what follows a success runs.
	protected static void Finish(notnull TBD_FleetCommand command)
	{
		command.m_eStage = TBD_EFleetCommandStage.DONE;
		s_Current = null;
		TBD_Log.Kv(TBD_FleetCommandPoller.CH_FLEET, "reported", string.Format("command=%1 action=%2 succeeded=%3", command.m_sCommandId, command.m_sAction, command.m_bSucceeded));

		if (command.m_bSucceeded && command.m_bRestartAfterResult)
			TBD_FleetLoadMissionAction.RestartScenario(command);
	}

	//------------------------------------------------------------------------------------------------
	//! The claim is no longer this runtime's: nothing more is done or reported for the command.
	protected static void Abandon(notnull TBD_FleetCommand command, string why)
	{
		command.m_eStage = TBD_EFleetCommandStage.DONE;
		s_Current = null;
		TBD_Log.Error(TBD_FleetCommandPoller.CH_FLEET, string.Format("command=%1 action=%2 ABANDONED - %3. No further effect and no further report from this runtime.",
			command.m_sCommandId, command.m_sAction, why));
	}

	//------------------------------------------------------------------------------------------------
	//! `text` as a failure reason the platform accepts: printable ASCII, trimmed, 1 to
	//! FAILURE_REASON_MAX_BYTES bytes. Other bytes become '?', so a cut can never split a character.
	static string SafeReason(string text)
	{
		string safe;
		int length = text.Length();
		if (length > FAILURE_REASON_MAX_BYTES)
			length = FAILURE_REASON_MAX_BYTES;

		for (int i = 0; i < length; i++)
		{
			int code = text.ToAscii(i) & 255;
			if (code >= 32 && code <= 126)
				safe += text.Get(i);
			else
				safe += "?";
		}

		safe = safe.Trim();
		if (safe.IsEmpty())
			return "no reason recorded";

		return safe;
	}
}
