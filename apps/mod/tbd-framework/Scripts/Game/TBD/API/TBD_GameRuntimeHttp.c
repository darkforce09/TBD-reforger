//! Shared transport for the routes that authenticate with this server's `mod_runtime` machine
//! credential (`Authorization: Bearer tbdm_...`): everything under `/api/v1/game-runtime/` and the
//! fleet command routes under `/api/v1/fleet-executor/`.
//!
//! Every consumer of those routes (the deployed mission and its artifact, the runtime session, the
//! event roster, deployment authorization, fleet commands, the deployable mission list and the
//! in-game deployment relay) sends through `Post` / `Get` with a TBD_GameRuntimeCall subclass and
//! receives exactly one TBD_GameRuntimeAnswer, so the header set, the timeout, the watchdog and the
//! reading of an answer exist once.
//!
//! An answer is matched to its call by the RestCallback the engine answers through. A call the
//! engine never reports is answered TRANSIENT by its watchdog; its callback is kept alive until it
//! answers late, because the script owns a callback's lifetime while its request is in flight.
//!
//! @authority server - the machine credential lives only in the authority's profile.

//! One game-runtime request in flight. A sender subclasses it with what it needs to settle the
//! answer; `OnAnswered` receives exactly one answer per sent call.
class TBD_GameRuntimeCall
{
	//! The engine's handle of the request, owned by TBD_GameRuntimeHttp.
	ref RestCallback m_Callback;
	int m_iTicket;

	//------------------------------------------------------------------------------------------------
	void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
	}
}

class TBD_GameRuntimeHttp
{
	static const string ROUTE_PREFIX = "/api/v1/game-runtime";

	//! Every issued credential starts with this (`tbdm_<credential id>_<64 hex>`).
	static const string CREDENTIAL_PREFIX = "tbdm_";

	//! Transport timeout of one request.
	static const int REQUEST_TIMEOUT_S = 15;

	//! A call with no callback by now is answered TRANSIENT. It sits safely beyond the transport
	//! timeout, so it only fires when the engine never reports the request at all.
	static const int WATCHDOG_MS = 25000;

	//! Callbacks of calls a watchdog answered, kept until the engine reports them. Bounded; the
	//! oldest is released first.
	protected static const int RETIRED_CALLBACKS_MAX = 16;
	protected static ref array<ref RestCallback> s_aRetiredCallbacks;

	//! Calls sent and not answered yet.
	protected static ref array<ref TBD_GameRuntimeCall> s_aCallsInFlight;
	protected static int s_iLastTicket;

	//------------------------------------------------------------------------------------------------
	// CONFIGURATION
	//------------------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! True when a backend URL and a usable machine credential are configured.
	static bool IsConfigured()
	{
		if (TBD_BackendConfig.GetBackendUrl().IsEmpty())
			return false;

		return IsCredentialUsable(TBD_BackendConfig.GetMachineCredential());
	}

	//------------------------------------------------------------------------------------------------
	//! The backend for a log line. Never prints the credential.
	static string DescribeBackend()
	{
		string url = TBD_BackendConfig.GetBackendUrl();
		if (url.IsEmpty())
			return "none";

		string credential = TBD_BackendConfig.GetMachineCredential();
		if (credential.IsEmpty())
			return url + " (no machine credential)";

		if (!IsCredentialUsable(credential))
			return url + " (machineCredential is not a tbdm_ credential)";

		return url;
	}

	//------------------------------------------------------------------------------------------------
	//! A credential this client can send. A value without the issued prefix is a placeholder (the
	//! shipped example config carries one) and counts as absent. `RestContext.SetHeaders` takes
	//! `Key,Value,Key,Value`, so a comma would split the header list; issued credentials contain
	//! neither a comma nor whitespace.
	protected static bool IsCredentialUsable(string credential)
	{
		if (!credential.StartsWith(CREDENTIAL_PREFIX))
			return false;

		if (credential.Contains(",") || credential.Contains(" "))
			return false;

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! A context against the configured backend carrying the bearer credential, the JSON content
	//! type and the request timeout, or null with `failure` saying why.
	protected static RestContext OpenContext(out string failure)
	{
		failure = string.Empty;

		if (!IsConfigured())
		{
			failure = "backend URL or machine credential not configured";
			return null;
		}

		RestApi rest = GetGame().GetRestApi();
		if (!rest)
		{
			failure = "RestApi unavailable";
			return null;
		}

		string baseUrl = TBD_BackendConfig.GetBackendUrl();
		if (baseUrl.EndsWith("/"))
			baseUrl = baseUrl.Substring(0, baseUrl.Length() - 1);

		RestContext context = rest.GetContext(baseUrl);
		if (!context)
		{
			failure = "RestContext unavailable for " + baseUrl;
			return null;
		}

		// Content-Type is required: the backend's JSON extractors reject a body without it before
		// the handler runs.
		string headers = string.Format("Authorization,Bearer %1,Content-Type,application/json,Accept,application/json",
			TBD_BackendConfig.GetMachineCredential());
		context.SetHeaders(headers);
		context.SetTimeout(REQUEST_TIMEOUT_S);
		return context;
	}

	//------------------------------------------------------------------------------------------------
	// SENDING
	//------------------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! `POST path` with `body`. False, with `failure` saying why and nothing sent, when no context can
	//! be opened; otherwise `call.OnAnswered` receives the answer later.
	static bool Post(notnull TBD_GameRuntimeCall call, string path, string body, out string failure)
	{
		RestContext context = OpenContext(failure);
		if (!context)
			return false;

		Track(call);
		context.POST(call.m_Callback, path, body);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! `GET path`, answered like `Post`.
	static bool Get(notnull TBD_GameRuntimeCall call, string path, out string failure)
	{
		RestContext context = OpenContext(failure);
		if (!context)
			return false;

		Track(call);
		context.GET(call.m_Callback, path);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static void Track(notnull TBD_GameRuntimeCall call)
	{
		if (!s_aCallsInFlight)
			s_aCallsInFlight = new array<ref TBD_GameRuntimeCall>();

		s_iLastTicket++;
		call.m_iTicket = s_iLastTicket;
		call.m_Callback = new RestCallback();
		call.m_Callback.SetOnSuccess(OnCallSuccess);
		call.m_Callback.SetOnError(OnCallError);
		s_aCallsInFlight.Insert(call);

		// Never cancelled (`Remove` cancels by function, i.e. every call's watchdog): a watchdog whose
		// call has been answered finds nothing and returns.
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.CallLater(OnCallWatchdog, WATCHDOG_MS, false, call.m_iTicket);
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnCallSuccess(RestCallback callback)
	{
		Answer(callback, true);
	}

	//------------------------------------------------------------------------------------------------
	protected static void OnCallError(RestCallback callback)
	{
		Answer(callback, false);
	}

	//------------------------------------------------------------------------------------------------
	protected static void Answer(RestCallback callback, bool arrivedOnSuccess)
	{
		TBD_GameRuntimeCall call = TakeCallAnsweredBy(callback);
		if (!call)
		{
			// The late answer of a call its watchdog already answered.
			ForgetRetiredCallback(callback);
			return;
		}

		call.OnAnswered(TBD_GameRuntimeAnswer.Read(callback, arrivedOnSuccess));
	}

	//------------------------------------------------------------------------------------------------
	//! Reached for every call; answers the ones the engine has not reported by now.
	protected static void OnCallWatchdog(int ticket)
	{
		TBD_GameRuntimeCall call = TakeCallWithTicket(ticket);
		if (!call)
			return;

		RetireCallback(call.m_Callback);
		call.m_Callback = null;
		call.OnAnswered(TBD_GameRuntimeAnswer.Unanswered(string.Format("no answer within %1 ms", WATCHDOG_MS)));
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_GameRuntimeCall TakeCallAnsweredBy(RestCallback callback)
	{
		if (!s_aCallsInFlight || !callback)
			return null;

		for (int i = 0; i < s_aCallsInFlight.Count(); i++)
		{
			TBD_GameRuntimeCall call = s_aCallsInFlight[i];
			if (call && call.m_Callback == callback)
			{
				s_aCallsInFlight.RemoveOrdered(i);
				return call;
			}
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_GameRuntimeCall TakeCallWithTicket(int ticket)
	{
		if (!s_aCallsInFlight)
			return null;

		for (int i = 0; i < s_aCallsInFlight.Count(); i++)
		{
			TBD_GameRuntimeCall call = s_aCallsInFlight[i];
			if (call && call.m_iTicket == ticket)
			{
				s_aCallsInFlight.RemoveOrdered(i);
				return call;
			}
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	protected static void RetireCallback(RestCallback callback)
	{
		if (!callback)
			return;

		if (!s_aRetiredCallbacks)
			s_aRetiredCallbacks = new array<ref RestCallback>();

		if (s_aRetiredCallbacks.Count() >= RETIRED_CALLBACKS_MAX)
			s_aRetiredCallbacks.RemoveOrdered(0);

		s_aRetiredCallbacks.Insert(callback);
	}

	//------------------------------------------------------------------------------------------------
	protected static void ForgetRetiredCallback(RestCallback callback)
	{
		if (!s_aRetiredCallbacks || !callback)
			return;

		int index = s_aRetiredCallbacks.Find(callback);
		if (index >= 0)
			s_aRetiredCallbacks.RemoveOrdered(index);
	}

	//------------------------------------------------------------------------------------------------
	// HELPERS
	//------------------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Exponential backoff: `baseMs` after the first failure, doubling with every further one, never
	//! more than `capMs`.
	static int BackoffMs(int failures, int baseMs, int capMs)
	{
		int delay = baseMs;
		for (int i = 1; i < failures; i++)
		{
			if (delay >= capMs / 2)
				return capMs;

			delay = delay * 2;
		}

		if (delay > capMs)
			return capMs;

		return delay;
	}

	//------------------------------------------------------------------------------------------------
	//! Milliseconds since the game started, the clock every retry schedule here is kept on.
	static int NowMs()
	{
		return System.GetTickCount();
	}

	//------------------------------------------------------------------------------------------------
	//! True once `notBeforeMs` has been reached. Compared as a difference so the comparison stays
	//! correct when the millisecond counter wraps.
	static bool IsDue(int notBeforeMs)
	{
		return NowMs() - notBeforeMs >= 0;
	}

	//------------------------------------------------------------------------------------------------
	//! Make a string safe inside a JSON double-quoted scalar. `Replace` mutates in place and returns
	//! a count, so the copy comes from `string.Format` and every call is a statement. Backslash goes
	//! first so the quotes' escapes are not escaped again; control characters become spaces.
	static string JsonEscape(string value)
	{
		string escaped = string.Format("%1", value);
		escaped.Replace("\\", "\\\\");
		escaped.Replace("\"", "\\\"");
		escaped.Replace("\n", " ");
		escaped.Replace("\r", " ");
		escaped.Replace("\t", " ");
		return escaped;
	}
}
