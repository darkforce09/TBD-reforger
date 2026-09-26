/**
 * @file TBD_FleetCommand.c
 * @brief One claimed fleet command and what its execution has produced so far.
 *
 * Role: carries the claim (id, action, fencing token, arguments) and the execution record (stage,
 * outcome or failure reason, report failures), and builds the result report body.
 * Position: built by `TBD_FleetCommandPoller` from a claim; driven by `TBD_FleetCommandExecution`
 * and the actions it starts.
 * State: the command's own fields, owned by the one execution in progress on the server.
 * Invariants: a result body carries the claim's fencing token; a failure reason goes through
 * `TBD_FleetCommandExecution.SafeReason`, so it is printable ASCII of 1 to 500 bytes.
 */

//! Where a command stands in the protocol.
enum TBD_EFleetCommandStage
{
	CHECKING, //!< arguments and preconditions are being checked
	REPORTING_EXECUTING, //!< the `executing` report awaits admission
	EFFECT, //!< the action's effect runs
	REPORTING_RESULT, //!< the `result` report awaits admission
	DONE, //!< recorded or abandoned; nothing more happens
}

//! A claimed command and what its execution has produced so far.
class TBD_FleetCommand
{
	string m_sCommandId; //!< JSON key `command_id`
	string m_sAction; //!< JSON key `action`: `broadcast`, `kick` or `load_mission`
	int m_iFencingToken; //!< JSON key `fencing_token`, sent with every report
	ref map<string, string> m_mArguments; //!< every argument as text; null when the claim's arguments are not text values
	int m_iWorld; //!< the poller world that claimed the command (`TBD_FleetCommandPoller`)
	TBD_EFleetCommandStage m_eStage = TBD_EFleetCommandStage.CHECKING; //!< where the command stands; default CHECKING
	int m_iTargetPlayerId; //!< the connected player a kick targets, resolved by the argument check
	bool m_bSucceeded; //!< JSON key `succeeded` of the result report
	string m_sOutcome; //!< JSON key `outcome`: the JSON object of a success
	string m_sFailureReason; //!< JSON key `failure_reason` of a failure
	bool m_bRestartAfterResult; //!< a recorded success is followed by an in-process scenario restart (`load_mission`)
	int m_iReportFailures; //!< consecutive unanswered reports; drives the backoff

	//! The argument `key` as sent.
	//! @return the value, or empty
	string Argument(string key)
	{
		string value;
		if (m_mArguments)
			m_mArguments.Find(key, value);

		return value;
	}

	//! Every argument for one log line.
	//! @return `key='value'` pairs, or `<unreadable>`
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

	//! The `result` report body: the fencing token with `succeeded` and `outcome`, or with
	//! `failure_reason`.
	//! @return the JSON body
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
			m_iFencingToken, TBD_BackendText.JsonEscape(TBD_FleetCommandExecution.SafeReason(m_sFailureReason)));
	}
}
