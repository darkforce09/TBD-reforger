/**
 * @file TBD_GameRuntimeHttp.c
 * @brief Shared transport for every route that authenticates with this server's machine credential.
 *
 * Role: opens a context with `Authorization: Bearer tbdm_...`, sends `Post`/`Get`, and delivers
 * exactly one `TBD_GameRuntimeAnswer` per call, so the header set, the timeout, the watchdog and
 * the reading of an answer exist once.  Position: used by every consumer of
 * `/api/v1/game-runtime/` and `/api/v1/fleet-executor/` (deployed mission and artifact, runtime
 * session, event roster, deployment authorisation, fleet commands, deployable mission list,
 * deployment relay); reads `TBD_BackendConfig`.
 * State: the calls in flight, the ticket counter and the retired callbacks; server statics.
 * Invariants: an answer is matched to its call by the `RestCallback` the engine answers through;
 * a call the engine never reports is answered TRANSIENT by its watchdog, and its callback is kept
 * alive (bounded) until it answers late, because the script owns a callback while its request is
 * in flight; the credential is never logged.
 */

//! One game-runtime request in flight. A sender subclasses it with what it needs to settle the
//! answer; `OnAnswered` receives exactly one answer per sent call.
class TBD_GameRuntimeCall
{
	ref RestCallback m_Callback; //!< the engine's handle of the request, owned by `TBD_GameRuntimeHttp`
	int m_iTicket; //!< per-send ticket its watchdog is armed with

	//! Receive the one answer to this call; the base does nothing.
	//! @param answer the answer, from the engine or the watchdog
	void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
	}
}

//! Machine-credential HTTP transport: context, send, answer matching, watchdog, retry clock.
//! @authority server
class TBD_GameRuntimeHttp
{
	static const string ROUTE_PREFIX = "/api/v1/game-runtime"; //!< path prefix of the game-runtime routes
	static const string CREDENTIAL_PREFIX = "tbdm_"; //!< prefix of every issued credential (`tbdm_<credential id>_<64 hex>`)
	static const int REQUEST_TIMEOUT_S = 15; //!< transport timeout of one request, in seconds
	static const int WATCHDOG_MS = 25000; //!< delay after which an unreported call is answered TRANSIENT, in milliseconds; past the transport timeout
	protected static const int RETIRED_CALLBACKS_MAX = 16; //!< retired callbacks kept at most; the oldest is released first
	protected static ref array<ref RestCallback> s_aRetiredCallbacks; //!< callbacks of calls a watchdog answered, kept until the engine reports them
	protected static ref array<ref TBD_GameRuntimeCall> s_aCallsInFlight; //!< calls sent and not answered yet
	protected static int s_iLastTicket; //!< last ticket issued

	//! Whether a backend URL and a usable machine credential are configured.
	//! @return true when a call can be sent
	static bool IsConfigured()
	{
		if (TBD_BackendConfig.GetBackendUrl().IsEmpty())
			return false;

		return IsCredentialUsable(TBD_BackendConfig.GetMachineCredential());
	}

	//! The backend for a log line, with the machine credential's state; never prints the credential.
	//! @return `none`, the URL, or the URL with the reason the credential is unusable
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

	//! A credential this client can send. A value without the issued prefix is a placeholder (the
	//! shipped example config carries one) and counts as absent. `RestContext.SetHeaders` takes
	//! `Key,Value,Key,Value`, so a comma would split the header list; issued credentials contain
	//! neither a comma nor whitespace.
	//! @return true when `credential` can be sent
	protected static bool IsCredentialUsable(string credential)
	{
		if (!credential.StartsWith(CREDENTIAL_PREFIX))
			return false;

		if (credential.Contains(",") || credential.Contains(" "))
			return false;

		return true;
	}

	//! A context against the configured backend carrying the bearer credential, the JSON content
	//! type and the request timeout.
	//! @param failure why no context opened; empty on success
	//! @return the context, or null
	//! @route POST /api/v1/game-runtime/{path}
	//! @route GET /api/v1/game-runtime/{path}
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

	//! `POST path` with `body`; `call.OnAnswered` receives the answer later.
	//! @param failure why nothing was sent; empty on success
	//! @return false, with nothing sent, when no context can be opened
	//! @route POST /api/v1/game-runtime/{path}
	//! @authority server
	static bool Post(notnull TBD_GameRuntimeCall call, string path, string body, out string failure)
	{
		RestContext context = OpenContext(failure);
		if (!context)
			return false;

		Track(call);
		context.POST(call.m_Callback, path, body);
		return true;
	}

	//! `GET path`, answered like `Post`.
	//! @param failure why nothing was sent; empty on success
	//! @return false, with nothing sent, when no context can be opened
	//! @route GET /api/v1/game-runtime/{path}
	//! @authority server
	static bool Get(notnull TBD_GameRuntimeCall call, string path, out string failure)
	{
		RestContext context = OpenContext(failure);
		if (!context)
			return false;

		Track(call);
		context.GET(call.m_Callback, path);
		return true;
	}

	//! Give `call` a ticket and a fresh callback, record it in flight and arm its watchdog.
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

	//! Engine success callback of any call.
	protected static void OnCallSuccess(RestCallback callback)
	{
		Answer(callback, true);
	}

	//! Engine error callback of any call.
	protected static void OnCallError(RestCallback callback)
	{
		Answer(callback, false);
	}

	//! Deliver the engine's answer to the call that owns `callback`; a late answer to a call its
	//! watchdog already answered only releases the retired callback.
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

	//! Watchdog, armed for every call: answers TRANSIENT when the engine has not reported it.
	//! @param ticket the call's ticket; an answered call is not found
	protected static void OnCallWatchdog(int ticket)
	{
		TBD_GameRuntimeCall call = TakeCallWithTicket(ticket);
		if (!call)
			return;

		RetireCallback(call.m_Callback);
		call.m_Callback = null;
		call.OnAnswered(TBD_GameRuntimeAnswer.Unanswered(string.Format("no answer within %1 ms", WATCHDOG_MS)));
	}

	//! Remove and return the in-flight call that owns `callback`.
	//! @return the call, or null
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

	//! Remove and return the in-flight call with `ticket`.
	//! @return the call, or null
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

	//! Keep `callback` alive until the engine reports it, releasing the oldest past the bound.
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

	//! Release a retired callback the engine has now reported.
	protected static void ForgetRetiredCallback(RestCallback callback)
	{
		if (!s_aRetiredCallbacks || !callback)
			return;

		int index = s_aRetiredCallbacks.Find(callback);
		if (index >= 0)
			s_aRetiredCallbacks.RemoveOrdered(index);
	}

	//! Exponential backoff: `baseMs` after the first failure, doubling with every further one, never
	//! more than `capMs`.
	//! @return the delay, in milliseconds
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

	//! Milliseconds since the game started, the clock every retry schedule here is kept on.
	//! @return the tick count
	static int NowMs()
	{
		return System.GetTickCount();
	}

	//! True once `notBeforeMs` has been reached. Compared as a difference so the comparison stays
	//! correct when the millisecond counter wraps.
	//! @return true when due
	static bool IsDue(int notBeforeMs)
	{
		return NowMs() - notBeforeMs >= 0;
	}
}
