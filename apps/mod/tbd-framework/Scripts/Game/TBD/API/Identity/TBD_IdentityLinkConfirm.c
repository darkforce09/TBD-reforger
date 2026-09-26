/**
 * @file TBD_IdentityLinkConfirm.c
 * @brief The serial confirm queue behind `#tbd link <code>`: one POST in flight at a time.
 *
 * Role: sends each queued code to the backend's link-confirm route, maps every answer to a
 * player line, and drains the queue.  Position: fed by `TBD_IdentityLink.Submit`; posts through
 * `RestApi` with the server's `X-Service-Token`; replies through `TBD_PlayerChat`.
 * State: the bounded queue, the in-flight request, its `RestCallback` and the watchdog ticket;
 * server statics that outlive a world.  Invariants: at most one request is in flight, because
 * `RestCallbackFunc` carries no user data and a response is correlated to its player only by
 * being the one outstanding; every completion path pumps the next entry; a watchdog bounds a
 * callback that never fires; the link code is never echoed in chat or logs.
 */

//! Serial confirm queue: one link-confirm POST in flight, bounded backlog, watchdog per send.
//! @authority server
class TBD_IdentityLinkConfirm
{
	protected static const string CONFIRM_PATH = "/api/v1/ingest/link-confirm"; //!< backend route, service-token tier
	protected static const int REQUEST_TIMEOUT_S = 15; //!< transport timeout, in seconds
	protected static const int WATCHDOG_MS = 25000; //!< watchdog delay past the transport timeout, in milliseconds; bounds a callback that never fires
	static const int MAX_QUEUE = 16; //!< queued requests at most; one entry per human, so a chat flood cannot grow it

	protected static ref array<ref TBD_IdentityLinkPending> s_aQueue; //!< waiting requests, oldest first; created on first use
	protected static ref TBD_IdentityLinkPending s_InFlight; //!< the one request awaiting an answer, or null
	protected static ref RestCallback s_RestCallback; //!< callback of the in-flight POST, held so it outlives the call
	protected static int s_iTicket; //!< monotonic per-send ticket; a watchdog armed for an older ticket is discarded

	//! Queue `pending` and start it when nothing is in flight.
	//! @param pending a request validated by `TBD_IdentityLink`
	//! @authority server
	static void Enqueue(notnull TBD_IdentityLinkPending pending)
	{
		if (!s_aQueue)
			s_aQueue = new array<ref TBD_IdentityLinkPending>();

		s_aQueue.Insert(pending);
		Pump();
	}

	//! Whether the backlog holds `MAX_QUEUE` requests.
	//! @return true when a new request must be refused
	static bool IsQueueFull()
	{
		return s_aQueue && s_aQueue.Count() >= MAX_QUEUE;
	}

	//! Whether `playerId` already has a request in flight or queued; one per player at a time.
	//! @return true when a new request from that player must wait
	static bool HasOutstanding(int playerId)
	{
		if (s_InFlight && s_InFlight.playerId == playerId)
			return true;

		if (!s_aQueue)
			return false;

		foreach (TBD_IdentityLinkPending queued : s_aQueue)
		{
			if (queued && queued.playerId == playerId)
				return true;
		}

		return false;
	}

	//! Start the next request when nothing is in flight. Every completion path ends here, so the
	//! queue always drains. `SendConfirm` can complete synchronously (no backend, no `RestApi`, no
	//! context), which makes `Pump -> SendConfirm -> Finish -> Pump` recursive; each pass removes
	//! one entry and nothing enqueues inside the cycle, so the depth stays within `MAX_QUEUE`.
	protected static void Pump()
	{
		if (s_InFlight)
			return;

		if (!s_aQueue || s_aQueue.IsEmpty())
			return;

		s_InFlight = s_aQueue[0];
		// `array.Remove` takes an index, not a value.
		s_aQueue.Remove(0);

		SendConfirm();
	}

	//! POST the in-flight request with the server's `X-Service-Token` and arm its watchdog. A
	//! missing backend, `RestApi` or context finishes the request at once with a player line.
	//! Never blocks and never touches the stage machine.
	//! @route POST /api/v1/ingest/link-confirm
	//! @authority server
	protected static void SendConfirm()
	{
		if (!s_InFlight)
			return;

		string baseUrl = TBD_BackendConfig.GetBackendUrl();
		string token = TBD_BackendConfig.GetServerToken();
		if (baseUrl.IsEmpty() || token.IsEmpty())
		{
			// `#tbd backend <url>` repoints the config at runtime, so the check made at enqueue
			// is not trusted here.
			Finish(TBD_IdentityLink.TAG + "cannot link: this server lost its website configuration. Tell an admin. Your code was not used.",
				"no-backend-at-send");
			return;
		}

		RestApi rest = GetGame().GetRestApi();
		if (!rest)
		{
			Finish(TBD_IdentityLink.TAG + "cannot link: this server's HTTP layer is unavailable. Tell an admin. Your code was not used.",
				"no-restapi");
			return;
		}

		if (baseUrl.EndsWith("/"))
			baseUrl = baseUrl.Substring(0, baseUrl.Length() - 1);

		RestContext ctx = rest.GetContext(baseUrl);
		if (!ctx)
		{
			Finish(TBD_IdentityLink.TAG + "cannot link: could not open a connection to the website. Try again in a moment; your code was not used.",
				"no-restcontext");
			return;
		}

		s_RestCallback = new RestCallback();
		s_RestCallback.SetOnSuccess(OnConfirmSuccess);
		s_RestCallback.SetOnError(OnConfirmError);

		// Content-Type is required: the handler's Axum `Json<LinkConfirmRequest>` extractor rejects
		// a body without `application/json` with a 400 before the handler runs. Same
		// `X-Service-Token` tier as the results POST (`TBD_ResultsReporter`).
		ctx.SetHeaders(string.Format("X-Service-Token,%1,Content-Type,application/json,Accept,application/json", token));
		ctx.SetTimeout(REQUEST_TIMEOUT_S);

		string payload = BuildPayload(s_InFlight);

		s_iTicket++;
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
		{
			// `ScriptCallQueue.Remove` cancels by function, not by argument. The queue is serial,
			// so at most one watchdog is armed and this cancels exactly that one; the ticket
			// stops a surviving watchdog from firing against a later request.
			queue.Remove(OnWatchdog);
			queue.CallLater(OnWatchdog, WATCHDOG_MS, false, s_iTicket);
		}

		TBD_Log.Kv(TBD_IdentityLink.CH_LINK, "confirm", string.Format("player=%1 armaId=%2 url=%3%4 bytes=%5",
			s_InFlight.playerId, s_InFlight.armaId, baseUrl, CONFIRM_PATH, payload.Length()));

		ctx.POST(s_RestCallback, CONFIRM_PATH, payload);
	}

	//! The wire body, the backend's `LinkConfirmRequest`: `code`, `arma_id`, `arma_character`.
	//! Hand-built so the bytes are fixed by code, and assembled in steps because a long `+` chain
	//! fails with `Formula too complex`. Every key is always emitted: a missing `arma_character`
	//! is a decode failure (400) and an empty `code` or `arma_id` is rejected by the handler. An
	//! empty `arma_character` value is legal; nothing joins on it.
	//! @return the JSON body
	protected static string BuildPayload(notnull TBD_IdentityLinkPending pending)
	{
		string json = "{";
		json += string.Format("\"code\":\"%1\"", TBD_BackendText.JsonEscape(pending.code));
		json += string.Format(",\"arma_id\":\"%1\"", TBD_BackendText.JsonEscape(pending.armaId));
		json += string.Format(",\"arma_character\":\"%1\"", TBD_BackendText.JsonEscape(pending.armaCharacter));
		json += "}";
		return json;
	}

	//! Success callback. A 200, 201 or status-less answer links the player
	//! (`{"linked":true,"discord_id":...,"arma_id":...,"arma_character":...}`); any other status
	//! goes to the same `HandleFailure` as `OnConfirmError`, because which callback carries a 4xx
	//! is an engine choice.
	//! @authority server
	protected static void OnConfirmSuccess(RestCallback cb)
	{
		if (!s_InFlight)
			return;

		string body = cb.GetData();

		HttpCode code = cb.GetHttpCode();
		if (code != HttpCode.HTTP_CODE_200 && code != HttpCode.HTTP_CODE_201 && code != HttpCode.HTTP_CODE_NULL)
		{
			HandleFailure(code, cb.GetRestResult(), body);
			return;
		}

		string ok = TBD_IdentityLink.TAG + "linked. Your game identity is now attached to your TBD account -- attendance and stats count from your next round.";
		TBD_Log.Kv(TBD_IdentityLink.CH_LINK, "linked", string.Format("player=%1 armaId=%2 response=%3",
			s_InFlight.playerId, s_InFlight.armaId, body));
		Finish(ok, "ok");
	}

	//! Error callback: an HTTP error status, or no status at all (transport failure).
	//! @authority server
	protected static void OnConfirmError(RestCallback cb)
	{
		if (!s_InFlight)
			return;

		HandleFailure(cb.GetHttpCode(), cb.GetRestResult(), cb.GetData());
	}

	//! Turn the backend's answer into player lines, keyed on the HTTP status alone: 404 wrong,
	//! used or expired code; 409 identity linked to another account; 400 malformed request from
	//! this server; 401/403 service token rejected; no status, the website was never reached;
	//! anything else, a website error. Never matches response text: wording changes, and on a
	//! transport failure `GetData()` returns the request body.
	//! @param code the HTTP status
	//! @param result the transport result, logged when there is no status
	//! @param body the response body, logged
	protected static void HandleFailure(HttpCode code, ERestResult result, string body)
	{
		string reason = typename.EnumToString(HttpCode, code);
		string player;

		if (code == HttpCode.HTTP_CODE_404)
		{
			player = TBD_IdentityLink.TAG + "that code is not valid -- wrong, already used, or expired (codes last 10 minutes).";
			ReplyAsync(player);
			ReplyAsync(TBD_IdentityLink.TAG + "generate a fresh one on the website (avatar menu -> 'Link Arma Identity') and type it here. You are NOT linked.");
			FinishQuiet(reason, body);
			return;
		}

		if (code == HttpCode.HTTP_CODE_409)
		{
			player = TBD_IdentityLink.TAG + "this game identity is already linked to a DIFFERENT TBD account, and one identity can only belong to one account.";
			ReplyAsync(player);
			ReplyAsync(TBD_IdentityLink.TAG + "if that other account is yours, log into it on the website and press 'Unlink Arma ID' first. If it is not, contact an admin. You are NOT linked.");
			FinishQuiet(reason, body);
			return;
		}

		if (code == HttpCode.HTTP_CODE_401 || code == HttpCode.HTTP_CODE_403)
		{
			player = TBD_IdentityLink.TAG + "this game server is not authorised to talk to the website -- nothing you can do. Tell an admin (the server's service token is being rejected). You are NOT linked.";
			ReplyAsync(player);
			ReplyAsync(NewCodeAdvice());
			FinishQuiet(reason, body);
			return;
		}

		if (code == HttpCode.HTTP_CODE_400)
		{
			// A 400 leaves `identity_link_codes` untouched, so the code stays live. The request
			// is not re-sent: the body cannot change, so a retry would loop against the website.
			// The player decides whether to type again.
			player = TBD_IdentityLink.TAG + "the website rejected this server's request as malformed. That is a bug on our side, not your code -- tell an admin. You are NOT linked.";
			ReplyAsync(player);
			ReplyAsync(NewCodeAdvice());
			FinishQuiet(reason, body);
			return;
		}

		if (code == HttpCode.HTTP_CODE_NULL)
		{
			// No HTTP status: DNS, connection refused, TLS, or the request never left. `ERestResult`
			// says which, in the log. The body is dropped: with no response, `GetData()` returns
			// the request, which carries the player's link code.
			player = TBD_IdentityLink.TAG + "could not reach the website (network). Your code was not used -- try again in a moment.";
			ReplyAsync(player);
			FinishQuiet(reason + "/" + typename.EnumToString(ERestResult, result), "(no response)");
			return;
		}

		player = TBD_IdentityLink.TAG + "the website returned an error (" + reason + "). Try again shortly; if it keeps happening, tell an admin. You are NOT linked.";
		ReplyAsync(player);
		ReplyAsync(NewCodeAdvice());
		FinishQuiet(reason + "/" + typename.EnumToString(ERestResult, result), body);
	}

	//! The advice line sent after a failure, when the website did not consume the code; a
	//! successful link consumes it (`consumed_at`).
	//! @return the line
	protected static string NewCodeAdvice()
	{
		return TBD_IdentityLink.TAG + "your code was not consumed -- generate a NEW one before retrying.";
	}

	//! Watchdog: finishes the in-flight request as unreachable when neither callback fired.
	//! @param ticket the send ticket it was armed for; a stale ticket does nothing
	protected static void OnWatchdog(int ticket)
	{
		if (!s_InFlight)
			return;

		if (ticket != s_iTicket)
			return;

		TBD_Log.Warn(TBD_IdentityLink.CH_LINK, string.Format(
			"no response after %1 ms for player=%2 -- treating as unreachable. If this repeats, the REST callback is not firing.",
			WATCHDOG_MS, s_InFlight.playerId));

		Finish(TBD_IdentityLink.TAG + "the website did not answer in time. Your code was not used -- try again in a moment.", "watchdog-timeout");
	}

	//! Complete the in-flight request: send `playerLine` when not empty, log the outcome, drain the
	//! queue.
	protected static void Finish(string playerLine, string outcome)
	{
		if (!playerLine.IsEmpty())
			ReplyAsync(playerLine);

		FinishQuiet(outcome, string.Empty);
	}

	//! Complete without a player line: log any outcome other than `ok` with `body`, cancel the
	//! watchdog, clear the in-flight request and pump the next.
	protected static void FinishQuiet(string outcome, string body)
	{
		if (s_InFlight && outcome != "ok")
		{
			TBD_Log.Warn(TBD_IdentityLink.CH_LINK, string.Format("not linked player=%1 outcome=%2 response='%3'",
				s_InFlight.playerId, outcome, body));
		}

		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(OnWatchdog);

		s_InFlight = null;
		s_RestCallback = null;

		Pump();
	}

	//! Reply to the in-flight request's player from a callback that may fire seconds later. A
	//! dedicated server recycles player ids, so the current identity is compared with the one
	//! stamped at enqueue; when they differ the player is gone and the line is only logged.
	//! @authority server
	protected static void ReplyAsync(string text)
	{
		if (!s_InFlight)
			return;

		string nowId = TBD_PlayerIdentity.GetArmaId(s_InFlight.playerId);
		if (nowId != s_InFlight.armaId)
		{
			TBD_Log.Event(TBD_IdentityLink.CH_LINK, string.Format(
				"player=%1 left before the answer arrived (id now '%2', was '%3') -- reply not delivered: %4",
				s_InFlight.playerId, nowId, s_InFlight.armaId, text));
			return;
		}

		TBD_IdentityLink.LogReply(s_InFlight.playerId, text);
		TBD_PlayerChat.Tell(s_InFlight.playerId, text);
	}
}
