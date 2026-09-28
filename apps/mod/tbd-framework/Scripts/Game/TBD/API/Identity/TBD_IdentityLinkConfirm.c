/**
 * @file TBD_IdentityLinkConfirm.c
 * @brief The serial confirm queue behind `#tbd link <code>`: one POST in flight at a time.
 *
 * Role: sends each queued code to the backend's link-confirm route, maps every answer to a
 * player line, and drains the queue.  Position: fed by `TBD_IdentityLink.Submit`; posts through
 * `TBD_GameRuntimeHttp` with this server's machine credential as `Authorization: Bearer`; replies
 * through `TBD_PlayerChat`.
 * State: the bounded queue and the in-flight request; server statics that outlive a world.
 * Invariants: at most one request is in flight; a confirm is interactive, held in memory only and
 * never put on the durable telemetry queue; every completion path pumps the next entry; the
 * transport's watchdog answers a call the engine never reports, so every send completes; the link
 * code is never echoed in chat or logs.
 */

//! The one link-confirm request in flight; hands its answer to `TBD_IdentityLinkConfirm`.
class TBD_IdentityLinkConfirmCall : TBD_GameRuntimeCall
{
	ref TBD_IdentityLinkPending m_Pending; //!< the request this call sends

	//! Forward the answer to the confirm queue.
	//! @param answer the classified answer
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_IdentityLinkConfirm.OnAnswer(this, answer);
	}
}

//! Serial confirm queue: one link-confirm POST in flight, bounded backlog.
//! @authority server
class TBD_IdentityLinkConfirm
{
	protected static const string CONFIRM_PATH = "/api/v1/ingest/link-confirm"; //!< backend route, machine-credential tier
	static const int MAX_QUEUE = 16; //!< queued requests at most; one entry per human, so a chat flood cannot grow it

	protected static ref array<ref TBD_IdentityLinkPending> s_aQueue; //!< waiting requests, oldest first; created on first use
	protected static ref TBD_IdentityLinkPending s_InFlight; //!< the one request awaiting an answer, or null

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

	//! POST the in-flight request through `TBD_GameRuntimeHttp`. A request that cannot be sent
	//! (no backend, no credential, no `RestApi` or context) finishes at once with a player line.
	//! Never blocks and never touches the stage machine.
	//! @route POST /api/v1/ingest/link-confirm
	//! @authority server
	protected static void SendConfirm()
	{
		if (!s_InFlight)
			return;

		string payload = BuildPayload(s_InFlight);
		TBD_IdentityLinkConfirmCall call = new TBD_IdentityLinkConfirmCall();
		call.m_Pending = s_InFlight;

		TBD_Log.Kv(TBD_IdentityLink.CH_LINK, "confirm", string.Format("player=%1 armaId=%2 url=%3%4 bytes=%5",
			s_InFlight.playerId, s_InFlight.armaId, TBD_BackendConfig.GetBackendUrl(), CONFIRM_PATH, payload.Length()));

		string failure;
		if (TBD_GameRuntimeHttp.Post(call, CONFIRM_PATH, payload, failure))
			return;

		// `#tbd backend <url>` repoints the config at runtime, so the check made at enqueue is not
		// trusted here.
		Finish(TBD_IdentityLink.TAG + "cannot link: this server could not reach its website configuration. Tell an admin. Your code was not used.",
			"not-sent: " + failure);
	}

	//! The wire body, the backend's `LinkConfirmRequest`: `code`, `arma_id`, `arma_character`.
	//! Hand-built so the bytes are fixed by code, and assembled in steps because a long `+` chain
	//! fails with `Formula too complex`. Every key is always emitted: a missing `arma_character`
	//! is a decode failure (400) and an empty `code` or `arma_id` is rejected by the handler. An
	//! empty `arma_character` value is legal; nothing joins on it.
	//! @return the JSON body
	//! @contract arma-link.schema.json#/definitions/LinkConfirmRequest
	protected static string BuildPayload(notnull TBD_IdentityLinkPending pending)
	{
		string json = "{";
		json += string.Format("\"code\":\"%1\"", TBD_BackendText.JsonEscape(pending.code));
		json += string.Format(",\"arma_id\":\"%1\"", TBD_BackendText.JsonEscape(pending.armaId));
		json += string.Format(",\"arma_character\":\"%1\"", TBD_BackendText.JsonEscape(pending.armaCharacter));
		json += "}";
		return json;
	}

	//! The answer to a confirm call. A 200, 201 or status-less success links the player
	//! (`{"linked":true,"discord_id":...,"arma_id":...,"arma_character":...}`); anything else goes to
	//! `HandleFailure`. An answer to a call other than the one in flight is ignored.
	//! @param call the answered call
	//! @param answer the classified answer
	//! @authority server
	static void OnAnswer(notnull TBD_IdentityLinkConfirmCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		if (!s_InFlight || call.m_Pending != s_InFlight)
			return;

		if (answer.m_eOutcome != TBD_EGameRuntimeOutcome.SUCCESS)
		{
			HandleFailure(answer);
			return;
		}

		string ok = TBD_IdentityLink.TAG + "linked. Your game identity is now attached to your TBD account -- attendance and stats count from your next round.";
		TBD_Log.Kv(TBD_IdentityLink.CH_LINK, "linked", string.Format("player=%1 armaId=%2 response=%3",
			s_InFlight.playerId, s_InFlight.armaId, answer.m_sBody));
		Finish(ok, "ok");
	}

	//! Turn the backend's answer into player lines, keyed on the HTTP status alone: 404 wrong,
	//! used or expired code; 409 identity linked to another account; 400 malformed request from
	//! this server; 401/403 machine credential rejected; no status, the website was never reached
	//! or did not answer in time; anything else, a website error. Never matches response text:
	//! wording changes. The answer carries a body only when a status arrived with it, so the link
	//! code inside the request is never logged.
	//! @param answer the non-success answer
	protected static void HandleFailure(notnull TBD_GameRuntimeAnswer answer)
	{
		HttpCode code = answer.m_eCode;
		string body = answer.m_sBody;
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
			player = TBD_IdentityLink.TAG + "this game server is not authorised to talk to the website -- nothing you can do. Tell an admin (the server's machine credential is being rejected). You are NOT linked.";
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
			// No HTTP status: DNS, connection refused, TLS, the request never left, or no answer
			// within the transport's watchdog. The detail names which, in the log.
			player = TBD_IdentityLink.TAG + "could not reach the website (network). Your code was not used -- try again in a moment.";
			ReplyAsync(player);
			FinishQuiet(answer.m_sDetail, "(no response)");
			return;
		}

		player = TBD_IdentityLink.TAG + "the website returned an error (" + reason + "). Try again shortly; if it keeps happening, tell an admin. You are NOT linked.";
		ReplyAsync(player);
		ReplyAsync(NewCodeAdvice());
		FinishQuiet(answer.m_sDetail, body);
	}

	//! The advice line sent after a failure, when the website did not consume the code; a
	//! successful link consumes it (`consumed_at`).
	//! @return the line
	protected static string NewCodeAdvice()
	{
		return TBD_IdentityLink.TAG + "your code was not consumed -- generate a NEW one before retrying.";
	}

	//! Complete the in-flight request: send `playerLine` when not empty, log the outcome, drain the
	//! queue.
	protected static void Finish(string playerLine, string outcome)
	{
		if (!playerLine.IsEmpty())
			ReplyAsync(playerLine);

		FinishQuiet(outcome, string.Empty);
	}

	//! Complete without a player line: log any outcome other than `ok` with `body`, clear the
	//! in-flight request and pump the next.
	protected static void FinishQuiet(string outcome, string body)
	{
		if (s_InFlight && outcome != "ok")
		{
			TBD_Log.Warn(TBD_IdentityLink.CH_LINK, string.Format("not linked player=%1 outcome=%2 response='%3'",
				s_InFlight.playerId, outcome, body));
		}

		s_InFlight = null;

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
