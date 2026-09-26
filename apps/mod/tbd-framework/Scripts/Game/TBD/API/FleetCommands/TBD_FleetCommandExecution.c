/**
 * @file TBD_FleetCommandExecution.c
 * @brief One claimed fleet command, from its claim to its reported outcome.
 *
 * Role: runs the protocol every command follows: check arguments and preconditions
 * (`TBD_FleetCommandArguments`; a failure is reported and the effect never starts), report
 * `executing` with the fencing token and wait for admission, run the effect once
 * (`TBD_FleetPlayerActions`, `TBD_FleetLoadMissionAction`), then report the result: `succeeded`
 * with the action's outcome, or `failed` with a reason of 1 to 512 bytes.  Position: begun by
 * `TBD_FleetCommandPoller` for each claim; posts to `/api/v1/fleet-executor/commands/{id}/...`
 * through `TBD_GameRuntimeHttp`.
 * State: the one command being executed or reported; a server static.  Invariants: the effect
 * never starts before `executing` is admitted and never repeats; an unanswered report is re-sent
 * with backoff from 2 s to 30 s; a 409 means the claim is not this runtime's any more, so the
 * command is abandoned with no further effect or report, except a result refused because the
 * command already stands `succeeded`: an earlier attempt was recorded and its answer lost, so
 * what follows a recorded success (the scenario restart of `load_mission`) still happens.
 */

//! An `executing` or `result` report on its way to the platform.
class TBD_FleetReportCall : TBD_GameRuntimeCall
{
	ref TBD_FleetCommand m_Command; //!< the reported command
	bool m_bResult; //!< a `result` report; false for `executing`

	//! Hand the answer to `TBD_FleetCommandExecution.OnReportAnswered`.
	//! @authority server
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_FleetCommandExecution.OnReportAnswered(this, answer);
	}
}

//! The fleet command protocol driver: check, `executing`, effect, `result`.
//! @authority server
class TBD_FleetCommandExecution
{
	protected static const int REPORT_RETRY_BASE_MS = 2000; //!< first report retry delay, in milliseconds
	protected static const int REPORT_RETRY_CAP_MS = 30000; //!< report retry delay ceiling, in milliseconds
	protected static const int FAILURE_REASON_MAX_BYTES = 500; //!< failure reason cap, in bytes; the platform takes at most 512

	protected static ref TBD_FleetCommand s_Current; //!< the one command being executed or reported, or null

	//! Whether no command is being executed or reported.
	//! @return true when a new claim may begin
	static bool IsIdle()
	{
		return !s_Current;
	}

	//! The command being executed or reported.
	//! @return the command, or null
	static TBD_FleetCommand GetCurrent()
	{
		return s_Current;
	}

	//! A command was claimed: check it, then report `executing` or its failure.
	//! @authority server
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

	//! The effect succeeded: report it with `outcome`. Ignored for another command or one already
	//! reporting its result.
	//! @param outcome the `outcome` JSON object
	//! @param restartAfterResult restart the scenario once the success is recorded
	//! @authority server
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

	//! The command failed, before or during its effect: report it with `reason`. Ignored for
	//! another command or one already reporting its result.
	//! @authority server
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

	//! POST the `executing` report, or the `result` report when `result` is true; a report that
	//! cannot be sent is retried.
	//! @route POST /api/v1/fleet-executor/commands/{id}/executing
	//! @route POST /api/v1/fleet-executor/commands/{id}/result
	//! @authority server
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

	//! Called by `TBD_FleetReportCall` with the platform's answer to a report: admission starts the
	//! effect or finishes the command, a transient failure retries, a result already recorded as
	//! `succeeded` finishes, and any other refusal abandons. Answers for another command or stage
	//! are ignored.
	//! @authority server
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

	//! Retry a report that got no answer, with backoff. The effect it reports is not repeated.
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

	//! Send the report again when `commandId` is still the current command.
	protected static void ResendReport(string commandId, bool result)
	{
		if (!s_Current || s_Current.m_sCommandId != commandId)
			return;

		SendReport(s_Current, result);
	}

	//! Run the admitted command's effect by action; an unknown action fails.
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

	//! The outcome is recorded: the command is done, and what follows a success runs.
	protected static void Finish(notnull TBD_FleetCommand command)
	{
		command.m_eStage = TBD_EFleetCommandStage.DONE;
		s_Current = null;
		TBD_Log.Kv(TBD_FleetCommandPoller.CH_FLEET, "reported", string.Format("command=%1 action=%2 succeeded=%3", command.m_sCommandId, command.m_sAction, command.m_bSucceeded));

		if (command.m_bSucceeded && command.m_bRestartAfterResult)
			TBD_FleetLoadMissionAction.RestartScenario(command);
	}

	//! The claim is not this runtime's any more: nothing more is done or reported for the command.
	protected static void Abandon(notnull TBD_FleetCommand command, string why)
	{
		command.m_eStage = TBD_EFleetCommandStage.DONE;
		s_Current = null;
		TBD_Log.Error(TBD_FleetCommandPoller.CH_FLEET, string.Format("command=%1 action=%2 ABANDONED - %3. No further effect and no further report from this runtime.",
			command.m_sCommandId, command.m_sAction, why));
	}

	//! `text` as a failure reason the platform accepts: printable ASCII, trimmed, 1 to
	//! FAILURE_REASON_MAX_BYTES bytes. Other bytes become '?', so a cut can never split a character.
	//! @return the reason, or `no reason recorded` when nothing printable remains
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
