//! The game runtime's side of the fleet command ledger. While this world holds a runtime session,
//! the next command for it is claimed every 5 s (`POST /api/v1/fleet-executor/commands/claim` with
//! `{runtime_session_id}`, mod_runtime credential) and handed to TBD_FleetCommandExecution. One
//! command at a time: nothing is claimed while a command is being executed or reported, so no effect
//! runs twice and none overlaps another. A claim answered with no body (204) means nothing is
//! claimable now; a claim refused because the session ended is handed to TBD_RuntimeSession; any
//! other failure is logged at most once a minute, and the next claim goes out on schedule.
//!
//! The poll runs from the game start to the game end of a framework world
//! (TBD_RuntimeSessionLifecycle); an answer to a claim of an earlier world is dropped, and its
//! command returns to the queue when its 30 s lease lapses.
//! @authority server

//! `POST .../claim` answer. Field names are the JSON keys. The arguments are read separately
//! (TBD_FleetCommandArgumentsStruct), so arguments that cannot be read hide neither the command id
//! nor the fencing token the command's failure is reported with.
class TBD_ClaimedFleetCommandStruct
{
	string command_id;
	string server_id;
	string action;
	int fencing_token;
	string lease_expires_at;
}

//! The claimed command's `arguments`, every value as text. The field name is the JSON key.
class TBD_FleetCommandArgumentsStruct
{
	ref map<string, string> arguments;
}

//! A claim on its way to the platform, with the world and session it was sent for.
class TBD_FleetClaimCall : TBD_GameRuntimeCall
{
	int m_iWorld;
	string m_sSessionId;

	//------------------------------------------------------------------------------------------------
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_FleetCommandPoller.OnClaimAnswered(this, answer);
	}
}

class TBD_FleetCommandPoller
{
	//! Greppable channel: `grep '\[TBD\]\[Fleet\]' console.log`.
	static const string CH_FLEET = "Fleet";
	static const string ROUTE_PREFIX = "/api/v1/fleet-executor/commands";

	protected static const int CLAIM_INTERVAL_MS = 5000;
	protected static const int FAILURE_LOG_INTERVAL_MS = 60000;

	//! Bumped by Start and Stop; a claim answered for another world is dropped.
	protected static int s_iWorld;
	protected static bool s_bRunning;
	protected static bool s_bClaimInFlight;
	protected static bool s_bFailureLogged;
	protected static int s_iFailuresSinceLog;
	protected static int s_iLastFailureLogMs;

	//------------------------------------------------------------------------------------------------
	//! Claim from now on, while the world holds a runtime session.
	static void Start()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		s_iWorld++;
		s_bRunning = true;
		s_bClaimInFlight = false;
		s_bFailureLogged = false;
		s_iFailuresSinceLog = 0;
		TBD_Log.Event(CH_FLEET, string.Format("claiming fleet commands every %1 s while this world holds a runtime session", CLAIM_INTERVAL_MS / 1000));
		ScheduleClaim();
	}

	//------------------------------------------------------------------------------------------------
	//! Claim nothing more in this world. A command already claimed finishes its reports.
	static void Stop()
	{
		s_bRunning = false;
		s_iWorld++;

		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Claim);
	}

	//------------------------------------------------------------------------------------------------
	static int GetWorld()
	{
		return s_iWorld;
	}

	//------------------------------------------------------------------------------------------------
	//! True while `world` is the world claiming now.
	static bool IsCurrentWorld(int world)
	{
		return s_bRunning && world == s_iWorld;
	}

	//------------------------------------------------------------------------------------------------
	protected static void ScheduleClaim()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
			return;

		queue.Remove(Claim);
		queue.CallLater(Claim, CLAIM_INTERVAL_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	protected static void Claim()
	{
		if (!s_bRunning)
			return;

		ScheduleClaim();

		if (s_bClaimInFlight || !TBD_FleetCommandExecution.IsIdle())
			return;

		string sessionId = TBD_RuntimeSession.GetSessionId();
		if (sessionId.IsEmpty())
			return;

		TBD_FleetClaimCall call = new TBD_FleetClaimCall();
		call.m_iWorld = s_iWorld;
		call.m_sSessionId = sessionId;
		string body = string.Format("{\"runtime_session_id\":\"%1\"}", TBD_GameRuntimeHttp.JsonEscape(sessionId));

		// Marked before sending, so an answer can never find the claim unmarked.
		s_bClaimInFlight = true;
		string failure;
		if (TBD_GameRuntimeHttp.Post(call, ROUTE_PREFIX + "/claim", body, failure))
			return;

		s_bClaimInFlight = false;
		NoteFailure("not sent: " + failure);
	}

	//------------------------------------------------------------------------------------------------
	//! Called by TBD_FleetClaimCall with the platform's answer.
	static void OnClaimAnswered(notnull TBD_FleetClaimCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		if (call.m_iWorld != s_iWorld)
			return;

		s_bClaimInFlight = false;

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.SUCCESS)
		{
			NoteAnswered();
			if (answer.m_sBody.Trim().IsEmpty())
				return;

			string problem;
			TBD_FleetCommand command = ParseClaim(answer.m_sBody, problem);
			if (!command)
			{
				NoteFailure("the claimed command is unreadable: " + problem);
				return;
			}

			TBD_FleetCommandExecution.Begin(command);
			return;
		}

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.REFUSED && answer.m_Refusal.code == "RUNTIME_SESSION_ENDED")
		{
			TBD_RuntimeSession.ReportSessionEnded(call.m_sSessionId, answer.m_Refusal.end_reason);
			return;
		}

		NoteFailure(answer.m_sDetail);
	}

	//------------------------------------------------------------------------------------------------
	//! The command in a claim answer, or null with `problem` saying why it cannot be read. Arguments
	//! that cannot be read as text leave `m_mArguments` null; the argument check refuses them.
	protected static TBD_FleetCommand ParseClaim(string body, out string problem)
	{
		problem = string.Empty;
		JsonLoadContext context = new JsonLoadContext();
		TBD_ClaimedFleetCommandStruct claimed = new TBD_ClaimedFleetCommandStruct();
		if (!context.LoadFromString(body) || !context.ReadValue("", claimed))
		{
			problem = TBD_GameRuntimeAnswer.LoggableBody(body);
			return null;
		}

		if (claimed.command_id.IsEmpty() || claimed.fencing_token < 1)
		{
			problem = "no command_id or fencing_token in " + TBD_GameRuntimeAnswer.LoggableBody(body);
			return null;
		}

		TBD_FleetCommand command = new TBD_FleetCommand();
		command.m_sCommandId = claimed.command_id;
		command.m_sAction = claimed.action;
		command.m_iFencingToken = claimed.fencing_token;
		command.m_iWorld = s_iWorld;

		JsonLoadContext argumentsContext = new JsonLoadContext();
		TBD_FleetCommandArgumentsStruct arguments = new TBD_FleetCommandArgumentsStruct();
		if (argumentsContext.LoadFromString(body) && argumentsContext.ReadValue("", arguments) && arguments.arguments)
			command.m_mArguments = arguments.arguments;

		return command;
	}

	//------------------------------------------------------------------------------------------------
	protected static void NoteAnswered()
	{
		if (s_bFailureLogged)
			TBD_Log.Event(CH_FLEET, "fleet command claims are answered again");

		s_bFailureLogged = false;
		s_iFailuresSinceLog = 0;
	}

	//------------------------------------------------------------------------------------------------
	//! A claim failed: logged when it is the first since claims were last answered, then at most once
	//! a minute with the count since.
	protected static void NoteFailure(string detail)
	{
		s_iFailuresSinceLog++;
		int now = TBD_GameRuntimeHttp.NowMs();
		if (s_bFailureLogged && now - s_iLastFailureLogMs < FAILURE_LOG_INTERVAL_MS)
			return;

		TBD_Log.Warn(CH_FLEET, string.Format("fleet command claim failed (%1) - %2 failure(s) since the last report; claiming again every %3 s",
			detail, s_iFailuresSinceLog, CLAIM_INTERVAL_MS / 1000));
		s_bFailureLogged = true;
		s_iLastFailureLogMs = now;
		s_iFailuresSinceLog = 0;
	}
}
