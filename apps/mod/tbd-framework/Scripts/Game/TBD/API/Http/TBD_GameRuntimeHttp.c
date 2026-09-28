/**
 * @file TBD_GameRuntimeHttp.c
 * @brief Shared transport for every route that authenticates with this server's machine credential.
 *
 * Role: opens a context with `Authorization: Bearer tbdm_...`, sends `Post`/`Get`, and delivers
 * exactly one `TBD_GameRuntimeAnswer` per call on the main thread, so the header set, the timeout,
 * the watchdog, the hand-over from the engine's callback thread and the reading of an answer exist
 * once.  Position: used by every consumer of `/api/v1/game-runtime/`, `/api/v1/fleet-executor/` and
 * `/api/v1/ingest/` (deployed mission and artifact, runtime session, event roster, deployment
 * authorisation, fleet commands, deployable mission list, deployment relay, match telemetry
 * delivery, identity link confirmation); callers pass the full path; reads `TBD_BackendConfig`;
 * each request's answer arrives in its `TBD_GameRuntimeRestCallback`.
 * State: the calls in flight, the retired callbacks, the ticket counter and whether the delivery
 * pass is on the call queue; server statics read and written on the main thread only.
 * Invariants: the engine reports a request on its REST callback thread, where
 * `TBD_GameRuntimeRestCallback` only records the answer in its own fields; every `OnAnswered` runs
 * on the main thread, from the delivery pass (every frame while a call is in flight) or from the
 * call's watchdog, never inside `Post` or `Get`, so no answer handler spawns, kicks, broadcasts,
 * restarts, chats or arms the call queue from another thread; `Post` and `Get` are called on the
 * main thread; a call the engine never reports is answered TRANSIENT by its watchdog and its
 * callback is kept (bounded) until the engine reports it, because the script owns a callback while
 * its request is in flight; the delivery pass leaves the call queue once nothing is in flight, and
 * it and the watchdogs live on the game's call queue, which outlives a world, so a call sent before
 * a scenario restart is still answered once and its sender's own world guard judges it; the
 * credential is never logged.
 */

//! One game-runtime request in flight. A sender subclasses it with what it needs to settle the
//! answer; `OnAnswered` receives exactly one answer per sent call, on the main thread.
class TBD_GameRuntimeCall
{
	ref TBD_GameRuntimeRestCallback m_Callback; //!< the engine's handle of the request and the answer it reports, owned by `TBD_GameRuntimeHttp`
	int m_iTicket; //!< per-send ticket its watchdog is armed with

	//! Receive the one answer to this call, on the main thread; the base does nothing.
	//! @param answer the answer, from the engine or the watchdog
	void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
	}
}

//! Machine-credential HTTP transport: context, send, main-thread delivery, watchdog, retry clock.
//! @authority server
class TBD_GameRuntimeHttp
{
	static const string ROUTE_PREFIX = "/api/v1/game-runtime"; //!< path prefix of the game-runtime routes; fleet-executor and ingest callers pass their own full paths
	static const string CREDENTIAL_PREFIX = "tbdm_"; //!< prefix of every issued credential (`tbdm_<credential id>_<64 hex>`)
	static const int REQUEST_TIMEOUT_S = 15; //!< transport timeout of one request, in seconds
	static const int WATCHDOG_MS = 25000; //!< delay after which an unreported call is answered TRANSIENT, in milliseconds; past the transport timeout
	protected static const int RETIRED_CALLBACKS_MAX = 16; //!< retired callbacks kept at most; reported ones are released first, then the oldest
	protected static ref array<ref TBD_GameRuntimeRestCallback> s_aRetiredCallbacks; //!< callbacks of calls a watchdog answered, kept until the engine reports them
	protected static ref array<ref TBD_GameRuntimeCall> s_aCallsInFlight; //!< calls sent and not answered yet, oldest first
	protected static int s_iLastTicket; //!< last ticket issued
	protected static bool s_bDeliveryArmed; //!< true while `DeliverReportedAnswers` repeats on the call queue

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
	//! @route POST /api/v1/fleet-executor/{path}
	//! @route POST /api/v1/ingest/{path}
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

	//! `POST path` with `body`; `call.OnAnswered` receives the answer later, on the main thread.
	//! Called on the main thread.
	//! @param path the full path, for example `/api/v1/ingest/matches`
	//! @param failure why nothing was sent; empty on success
	//! @return false, with nothing sent, when no context opens or no call queue can answer the call
	//! @route POST /api/v1/game-runtime/{path}
	//! @route POST /api/v1/fleet-executor/{path}
	//! @route POST /api/v1/ingest/{path}
	//! @authority server
	static bool Post(notnull TBD_GameRuntimeCall call, string path, string body, out string failure)
	{
		RestContext context = OpenContext(failure);
		if (!context)
			return false;

		if (!Track(call, failure))
			return false;

		context.POST(call.m_Callback, path, body);
		return true;
	}

	//! `GET path`, answered like `Post`. Called on the main thread.
	//! @param failure why nothing was sent; empty on success
	//! @return false, with nothing sent, when no context opens or no call queue can answer the call
	//! @route GET /api/v1/game-runtime/{path}
	//! @authority server
	static bool Get(notnull TBD_GameRuntimeCall call, string path, out string failure)
	{
		RestContext context = OpenContext(failure);
		if (!context)
			return false;

		if (!Track(call, failure))
			return false;

		context.GET(call.m_Callback, path);
		return true;
	}

	//! Give `call` a ticket and a fresh callback, record it in flight, arm its watchdog and make sure
	//! the delivery pass runs. Called by `Post` and `Get` on the main thread, the only place the
	//! transport arms the call queue.
	//! @param failure why the call cannot be answered; empty on success
	//! @return false, with nothing recorded, when the game has no call queue to answer it from
	protected static bool Track(notnull TBD_GameRuntimeCall call, out string failure)
	{
		failure = string.Empty;
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
		{
			failure = "call queue unavailable";
			return false;
		}

		if (!s_aCallsInFlight)
			s_aCallsInFlight = new array<ref TBD_GameRuntimeCall>();

		s_iLastTicket++;
		call.m_iTicket = s_iLastTicket;
		call.m_Callback = TBD_GameRuntimeRestCallback.Create();
		s_aCallsInFlight.Insert(call);

		// Never cancelled (`Remove` cancels by function, i.e. every call's watchdog): a watchdog whose
		// call has been answered finds nothing and returns.
		queue.CallLater(OnCallWatchdog, WATCHDOG_MS, false, call.m_iTicket);

		if (!s_bDeliveryArmed)
		{
			queue.CallLater(DeliverReportedAnswers, 0, true);
			s_bDeliveryArmed = true;
		}

		return true;
	}

	//! The delivery pass, repeating on the main thread every frame while a call is in flight: it
	//! releases the retired callbacks the engine has reported, answers each call whose callback the
	//! engine has reported, oldest first, and leaves the call queue once nothing is in flight. One
	//! call is taken per step, so an answer handler that fails leaves the others in flight for the
	//! next pass; the steps are bounded by the calls in flight when the pass starts.
	//! @authority server
	protected static void DeliverReportedAnswers()
	{
		ForgetReportedRetiredCallbacks();

		int steps = 0;
		if (s_aCallsInFlight)
			steps = s_aCallsInFlight.Count();

		for (int step = 0; step < steps; step++)
		{
			TBD_GameRuntimeCall call = TakeFirstReportedCall();
			if (!call)
				break;

			call.OnAnswered(AnswerReportedBy(call.m_Callback));
		}

		if (s_aCallsInFlight && s_aCallsInFlight.Count() > 0)
			return;

		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(DeliverReportedAnswers);

		s_bDeliveryArmed = false;
	}

	//! Watchdog, armed for every call: a call still in flight is answered with what its callback
	//! reported since the last delivery pass, else TRANSIENT, and its unreported callback is retired.
	//! The TRANSIENT detail names the engine's own transport result, so a request the engine finished
	//! without reaching the capture handlers reads differently in the log from one still open.
	//! @param ticket the call's ticket; an answered call is not found
	//! @authority server
	protected static void OnCallWatchdog(int ticket)
	{
		TBD_GameRuntimeCall call = TakeCallWithTicket(ticket);
		if (!call)
			return;

		TBD_GameRuntimeRestCallback callback = call.m_Callback;
		if (callback && callback.m_bReported)
		{
			call.OnAnswered(AnswerReportedBy(callback));
			return;
		}

		string engineResult = "none";
		if (callback)
			engineResult = typename.EnumToString(ERestResult, callback.GetRestResult());

		RetireCallback(callback);
		call.m_Callback = null;
		call.OnAnswered(TBD_GameRuntimeAnswer.Unanswered(string.Format("no answer within %1 ms (engine rest=%2)", WATCHDOG_MS, engineResult)));
	}

	//! The classified answer the engine reported through `callback`.
	//! @param callback a callback whose `m_bReported` is true
	//! @return the answer
	protected static TBD_GameRuntimeAnswer AnswerReportedBy(notnull TBD_GameRuntimeRestCallback callback)
	{
		return TBD_GameRuntimeAnswer.Reported(callback.m_eReportedCode, callback.m_eReportedRestResult,
			callback.m_sReportedBody, callback.m_bReportedOnSuccess);
	}

	//! Remove and return the oldest in-flight call whose callback the engine has reported.
	//! @return the call, or null when none has been reported
	protected static TBD_GameRuntimeCall TakeFirstReportedCall()
	{
		if (!s_aCallsInFlight)
			return null;

		for (int i = 0; i < s_aCallsInFlight.Count(); i++)
		{
			TBD_GameRuntimeCall call = s_aCallsInFlight[i];
			if (call && call.m_Callback && call.m_Callback.m_bReported)
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

	//! Keep `callback` alive until the engine reports it; past the bound, the reported ones are
	//! released first, then the oldest.
	protected static void RetireCallback(TBD_GameRuntimeRestCallback callback)
	{
		if (!callback)
			return;

		if (!s_aRetiredCallbacks)
			s_aRetiredCallbacks = new array<ref TBD_GameRuntimeRestCallback>();

		ForgetReportedRetiredCallbacks();
		if (s_aRetiredCallbacks.Count() >= RETIRED_CALLBACKS_MAX)
			s_aRetiredCallbacks.RemoveOrdered(0);

		s_aRetiredCallbacks.Insert(callback);
	}

	//! Release every retired callback the engine has reported since it was retired.
	protected static void ForgetReportedRetiredCallbacks()
	{
		if (!s_aRetiredCallbacks)
			return;

		for (int i = s_aRetiredCallbacks.Count() - 1; i >= 0; i--)
		{
			TBD_GameRuntimeRestCallback retired = s_aRetiredCallbacks[i];
			if (!retired || retired.m_bReported)
				s_aRetiredCallbacks.RemoveOrdered(i);
		}
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
