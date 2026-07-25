//! T-181.35 — in-game identity linking. `#tbd link <code>`.
//!
//! ══ WHAT THIS CLOSES ═══════════════════════════════════════════════════════════════════════
//! `POST /api/v1/ingest/match-results` (T-181.13.1, `TBD_ResultsReporter`) resolves every player
//! with `SELECT discord_id FROM users WHERE arma_id = $1`
//! (`apps/website/api/src/handlers/telemetry.rs:238`). `users.arma_id` is written by exactly two
//! things: the dev seed, and `POST /api/v1/ingest/link-confirm`
//! (`apps/website/api/src/handlers/me.rs:160-205`, service-token tier, registered at
//! `apps/website/api/src/app.rs:39-40`) — the GAME SERVER confirming a link code. Until this file
//! existed the mod never called it, so in production nobody had an `arma_id`, the results POST
//! returned 200, the match rows were written, and attendance marking, the user-stat recompute and
//! the leaderboard refresh all silently did nothing. `TBD_ResultsReporter.LogIdentityCensus` prints
//! that no-op every round; this is the other half.
//!
//! The flow has three actors and this owns exactly one leg of it:
//!   1. a logged-in user hits `POST /api/v1/me/link` on the website (avatar menu -> "Link Arma
//!      Identity" -> Generate Link Code) and gets a 6-digit code, live for 10 minutes;
//!   2. they type `#tbd link <code>` in game -> **this file** POSTs `{code, arma_id,
//!      arma_character}` to `/api/v1/ingest/link-confirm` with the server's `X-Service-Token`;
//!   3. the backend consumes the code and sets `users.arma_id`.
//!
//! ══ THE IDENTITY IS NOT RESOLVED HERE, ON PURPOSE ══════════════════════════════════════════
//! The `arma_id` sent here MUST be byte-identical to the one `TBD_ResultsReporter` sends, or the
//! join matches nobody, forever, with no error anywhere — the backend cheerfully returns 200 at
//! both ends. So both halves call the ONE accessor, `TBD_PlayerIdentity.GetArmaId`, and neither
//! reimplements it. If the shape ever has to change it changes THERE, so the two halves move
//! together.
//!
//! That accessor deliberately returns EMPTY rather than falling back to a `player:<id>` seat lease
//! (which is what `TBD_SpawnManager.PlayerBindKey` correctly does for one-life bookkeeping). This
//! file honours that: **a player with no durable identity cannot link, and is told why.**
//! `users.arma_id` is UNIQUE, so writing a seat number or a name hash into it does not fail
//! loudly — it binds one Discord account to whoever occupies that seat/name next week, and blocks
//! every other account from ever claiming it. A refusal the player can read beats a permanent
//! mis-binding nobody notices.
//!
//! ══ WHY A SERIAL QUEUE ═════════════════════════════════════════════════════════════════════
//! `RestCallbackFunc` is `void f(RestCallback cb)` — the callback carries no user data, so a
//! response cannot be correlated back to a player unless exactly one request is outstanding.
//! Subclassing `RestCallback` and casting in the handler would avoid that, but inheriting from a
//! native `Managed` proto class is a RUNTIME property this lane cannot observe, and "compiles" is
//! not "works". One in flight at a time is provably correct with the primitives already shipped
//! (`TBD_MissionLoader`, `TBD_ResultsReporter` both hold a single static `RestCallback`), and
//! linking is a once-per-human action, so head-of-line blocking costs nothing real. The queue is
//! bounded and every entry has a watchdog, so it cannot wedge.
//!
//! ══ HONEST LIMITS ══════════════════════════════════════════════════════════════════════════
//! * **The code is typed in PUBLIC CHAT and this cannot prevent that.**
//!   `SCR_ChatComponent.OnNewMessage` is the RECEIVE-side display hook (vanilla forwards it
//!   straight to `SCR_ChatPanelManager`); by the time it runs on the server the message has
//!   already been distributed. Consuming it here suppresses nothing. The exposure window is the
//!   round trip on success (the backend sets `consumed_at` in the same transaction), but on ANY
//!   failure the code stays live and public — so every failure reply tells the player to mint a
//!   fresh one. The real fix is a UI field instead of a chat line; that is a screen, and screens
//!   are blocked on the Workbench rdb pass (see the program hub).
//! * **Compile-verified plus a real HTTP round trip against a capture endpoint** — see the slice
//!   report. Nothing here has been exercised by a live player on a dedicated server, because no
//!   gate on the fast lane connects a client.
//!
//! @route POST /api/v1/ingest/link-confirm (service-token tier; `X-Service-Token`)
//! @authority server — the identity lookup, the service token and the chat intercept are all
//!                     server-side; a client has no business resolving any of the three.

//! One queued confirm request. Everything needed to answer the player is captured at enqueue,
//! because none of it can be re-derived once they disconnect.
class TBD_IdentityLinkPending
{
	int    playerId;
	//! Stamped at enqueue, not re-resolved at send: the player typed the code while they were
	//! present, and a disconnect mid-flight must not turn a valid link into a dropped one.
	string armaId;
	string armaCharacter;
	string code;
}

class TBD_IdentityLink
{
	//! Greppable channel for this subsystem: `grep '\[TBD\]\[Link\]' console.log`.
	//! Kept local rather than added to `TBD_Log`'s vocabulary because `Core/TBD_Log.c` is outside
	//! this slice's ownership — same reason `TBD_ResultsReporter` keeps `CH_RESULTS` local.
	protected static const string CH_LINK = "Link";

	protected static const string CONFIRM_PATH = "/api/v1/ingest/link-confirm";

	//! Every reply the player sees starts with this, matching `TBD_AdminCommands.Reply`.
	protected static const string TAG = "TBD: ";

	//! Transport timeout, and a watchdog set safely beyond it. The watchdog exists because
	//! "the callback never fires" is an unobservable failure on this lane and a silent wedge is
	//! exactly the class of bug this program keeps finding.
	protected static const int REQUEST_TIMEOUT_S = 15;
	protected static const int WATCHDOG_MS       = 25000;

	//! Bounded so a chat flood cannot grow an unbounded queue. 16 is far past any real event:
	//! one entry per human, once ever.
	protected static const int MAX_QUEUE = 16;

	//! Purely a sanity bound so a pasted essay never reaches the wire. The backend is the only
	//! authority on whether a code is valid — it answers 404 for anything it does not recognise.
	protected static const int CODE_MAX_CHARS = 32;

	protected static ref array<ref TBD_IdentityLinkPending> s_aQueue;
	protected static ref TBD_IdentityLinkPending s_InFlight;
	protected static ref RestCallback s_RestCallback;

	//! Monotonic per-send ticket. The watchdog fires with the ticket it was armed for, so a late
	//! watchdog for a request that already completed is discarded instead of poisoning its
	//! successor.
	protected static int s_iTicket;

	//------------------------------------------------------------------------------------
	// LIFECYCLE
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Reset per-world state and say once, in the log, whether linking can work on this host.
	//! Called from `TBD_MissionLoader.ParseMissionJson` next to `TBD_ResultsReporter.Arm()` — a
	//! server-only path (`TBD_FrameworkManager.OnPostInit` returns early for `RplMode.Client`
	//! before `BeginLoad`), and the earliest moment a link could mean anything.
	//!
	//! Idempotent: statics outlive a world inside one process, so the REST-then-profile fallback
	//! calling this twice must be harmless.
	static void Arm()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!s_aQueue)
			s_aQueue = new array<ref TBD_IdentityLinkPending>();

		TBD_Log.Kv(CH_LINK, "armed", string.Format("command='#tbd link <code>' backend=%1", DescribeBackend()));

		if (!BackendConfigured())
		{
			// LEGAL STATE, not an error: a local/PIE host has no backend config at all. NORMAL so
			// it neither alarms an operator nor trips world-boot.sh's fail-closed error triage.
			TBD_Log.Event(CH_LINK,
				"no backend configured (backendUrl/serverToken empty) - '#tbd link' will tell players so instead of failing silently. This is a legal state on a local host.");
		}
	}

	//------------------------------------------------------------------------------------
	// CHAT SURFACE
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! The `#tbd link …` chat surface. Returns TRUE when the message was a link command and has
	//! been fully handled, so the caller must not fall through to anything else.
	//!
	//! ── THIS IS NOT ADMIN-GATED, AND THAT IS THE POINT ──────────────────────────────────────
	//! `TBD_AdminCommands` rejects a non-admin before it dispatches. Every player needs to link,
	//! so this has to be reached BEFORE that gate. It is a separate entry point rather than a
	//! branch inside `Dispatch` for exactly that reason: nothing about it should be able to drift
	//! behind the admin check later.
	//!
	//! It takes the `SCR_ChatComponent` the message arrived on only for the immediate reply.
	//! Asynchronous replies (seconds later, from the REST callback) re-resolve the component from
	//! the player's controller, the same way `TBD_ObjectivesComponent.Tell` and
	//! `TBD_PlayAreaComponent` do, because the original component may be gone by then.
	//!
	//! @authority server — callers must already have established authority.
	static bool TryHandleChat(SCR_ChatComponent chat, string msg, int senderId)
	{
		array<string> parts = new array<string>();
		msg.Split(" ", parts, true);

		if (parts.Count() < 2)
			return false;

		if (parts[1] != "link")
			return false;

		string arg;
		if (parts.Count() > 2)
			arg = parts[2];

		if (arg.IsEmpty())
		{
			ReplyLines(chat, senderId, Usage());
			return true;
		}

		if (arg == "status")
		{
			ReplyLines(chat, senderId, StatusLines(senderId));
			return true;
		}

		Submit(chat, senderId, arg);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Usage, with the whole flow in it. A player who types `#tbd link` should not have to ask
	//! anyone where the code comes from.
	protected static array<string> Usage()
	{
		array<string> lines = new array<string>();
		lines.Insert(TAG + "usage: #tbd link <code>   (also: #tbd link status)");
		lines.Insert(TAG + "1. on the website, open the avatar menu -> 'Link Arma Identity' -> Generate Link Code");
		lines.Insert(TAG + "2. type that 6-digit code here within 10 minutes. It links this game identity to your TBD account so attendance and stats count.");
		lines.Insert(TAG + "NOTE: whatever you type here is visible to other players. If a link fails, generate a NEW code before retrying.");
		return lines;
	}

	//------------------------------------------------------------------------------------------------
	//! What this host can currently do, without touching the backend. There is no service-token
	//! endpoint that answers "is this arma id linked" (`GET /me/link/status` is JWT-tier and
	//! answers for the CALLER, who is a browser), so this reports only what the server knows:
	//! whether an identity resolves, whether it is durable, and whether a backend is configured.
	protected static array<string> StatusLines(int playerId)
	{
		array<string> lines = new array<string>();

		string armaId = TBD_PlayerIdentity.GetArmaId(playerId);
		if (armaId.IsEmpty())
		{
			lines.Insert(TAG + "identity: NONE - this server issued you no backend identity, so linking is impossible right now.");
		}
		else if (!TBD_PlayerIdentity.IsDurable(armaId))
		{
			lines.Insert(TAG + "identity: NOT DURABLE (name-derived). Linking is refused - see '#tbd link <code>' for why.");
		}
		else
		{
			lines.Insert(TAG + "identity: ok (" + armaId + ")");
		}

		if (BackendConfigured())
			lines.Insert(TAG + "website: " + DescribeBackend());
		else
			lines.Insert(TAG + "website: NOT CONFIGURED on this server - linking is unavailable here.");

		return lines;
	}

	//------------------------------------------------------------------------------------
	// SUBMIT
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Validate everything that can be validated locally, then queue one confirm request.
	//!
	//! Everything refused here is refused with a reason the player can act on. The one thing this
	//! deliberately does NOT judge is whether the code itself is good — that is the backend's
	//! call, and it answers 404 for wrong/used/expired.
	protected static void Submit(SCR_ChatComponent chat, int playerId, string rawCode)
	{
		string code = rawCode;
		code.Trim();

		if (code.IsEmpty())
		{
			ReplyLines(chat, playerId, Usage());
			return;
		}

		if (code.Length() > CODE_MAX_CHARS)
		{
			ReplyLine(chat, playerId, TAG + "that does not look like a link code (too long). It is the 6 digits the website showed you.");
			return;
		}

		// ── The identity gate. See the file header: EMPTY is a refusal, never a substitution. ──
		string armaId = TBD_PlayerIdentity.GetArmaId(playerId);
		if (armaId.IsEmpty())
		{
			ReplyLine(chat, playerId, TAG + "cannot link: this server issued you no durable game identity, so there is nothing to attach to your account.");
			ReplyLine(chat, playerId, TAG + "that is a SERVER problem, not yours - tell an admin the backend identity service is not configured. Your code was not used; it is still valid.");
			TBD_Log.Warn(CH_LINK, string.Format(
				"refused player=%1 reason=no-identity (TBD_PlayerIdentity.GetArmaId returned empty - misconfigured dedicated server, or the player is mid-teardown)", playerId));
			return;
		}

		if (!TBD_PlayerIdentity.IsDurable(armaId))
		{
			// Vanilla's `00bbbddd-` synthesized id is a hash of the DISPLAY NAME, and it only
			// happens off a dedicated server. Writing it to `users.arma_id` (UNIQUE) would bind
			// this account to everyone who ever uses that name and lock every other account out
			// of it. Refusing is not caution, it is the only correct answer.
			ReplyLine(chat, playerId, TAG + "cannot link: this host gives you a name-derived identity, not a real one, so a link made here would break the moment you rename - and would block anyone else with your name.");
			ReplyLine(chat, playerId, TAG + "link from a DEDICATED TBD server instead. Your code was not used; generate a new one anyway, since it was visible in chat.");
			TBD_Log.Warn(CH_LINK, string.Format(
				"refused player=%1 reason=synthetic-identity id=%2 (listen/hosted host - vanilla name hash). Run events on a dedicated server.", playerId, armaId));
			return;
		}

		if (!BackendConfigured())
		{
			ReplyLine(chat, playerId, TAG + "cannot link: this server is not connected to the TBD website, so it cannot confirm your code. Tell an admin. Your code was not used.");
			TBD_Log.Event(CH_LINK, string.Format(
				"refused player=%1 reason=no-backend (backendUrl/serverToken empty). Legal state on a local host.", playerId));
			return;
		}

		if (!s_aQueue)
			s_aQueue = new array<ref TBD_IdentityLinkPending>();

		if (HasOutstanding(playerId))
		{
			ReplyLine(chat, playerId, TAG + "already checking your last code - wait for the answer before sending another.");
			return;
		}

		if (s_aQueue.Count() >= MAX_QUEUE)
		{
			ReplyLine(chat, playerId, TAG + "too many link requests queued right now - try again in a minute.");
			TBD_Log.Warn(CH_LINK, string.Format("queue full (%1) - dropped request from player=%2", MAX_QUEUE, playerId));
			return;
		}

		TBD_IdentityLinkPending pending = new TBD_IdentityLinkPending();
		pending.playerId = playerId;
		pending.armaId = armaId;
		pending.armaCharacter = PlayerName(playerId);
		pending.code = code;
		s_aQueue.Insert(pending);

		ReplyLine(chat, playerId, TAG + "checking that code with the website…");
		Pump();
	}

	//------------------------------------------------------------------------------------------------
	//! One request per player at a time — in flight or queued.
	protected static bool HasOutstanding(int playerId)
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

	//------------------------------------------------------------------------------------
	// SEND
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Start the next request if nothing is outstanding. Every completion path ends by calling
	//! this, so the queue always drains.
	protected static void Pump()
	{
		if (s_InFlight)
			return;

		if (!s_aQueue || s_aQueue.IsEmpty())
			return;

		s_InFlight = s_aQueue[0];
		// Enforce Script removes BY INDEX (measured landmine) — `Remove(value)` is not this API.
		s_aQueue.Remove(0);

		SendConfirm();
	}

	//------------------------------------------------------------------------------------------------
	//! One POST. Never throws, never blocks, never touches the stage machine.
	protected static void SendConfirm()
	{
		if (!s_InFlight)
			return;

		string baseUrl = TBD_BackendConfig.GetBackendUrl();
		string token = TBD_BackendConfig.GetServerToken();
		if (baseUrl.IsEmpty() || token.IsEmpty())
		{
			// The config can be repointed at runtime (`#tbd backend <url>`), so re-check rather
			// than trust the check made at enqueue time.
			Finish(TAG + "cannot link: this server lost its website configuration. Tell an admin. Your code was not used.",
				"no-backend-at-send");
			return;
		}

		RestApi rest = GetGame().GetRestApi();
		if (!rest)
		{
			Finish(TAG + "cannot link: this server's HTTP layer is unavailable. Tell an admin. Your code was not used.",
				"no-restapi");
			return;
		}

		if (baseUrl.EndsWith("/"))
			baseUrl = baseUrl.Substring(0, baseUrl.Length() - 1);

		RestContext ctx = rest.GetContext(baseUrl);
		if (!ctx)
		{
			Finish(TAG + "cannot link: could not open a connection to the website. Try again in a moment; your code was not used.",
				"no-restcontext");
			return;
		}

		s_RestCallback = new RestCallback();
		s_RestCallback.SetOnSuccess(OnConfirmSuccess);
		s_RestCallback.SetOnError(OnConfirmError);

		// Content-Type is NOT optional. The handler takes an Axum `Json<LinkConfirmRequest>`
		// extractor, which rejects a body without `application/json` BEFORE the handler runs — a
		// perfectly valid payload comes back 400 "code and arma_id required" and you hunt a bug
		// that is not there. Same `X-Service-Token` tier the mission fetch and the results POST
		// already use (`TBD_MissionLoader`, `TBD_ResultsReporter`).
		ctx.SetHeaders(string.Format("X-Service-Token,%1,Content-Type,application/json,Accept,application/json", token));
		ctx.SetTimeout(REQUEST_TIMEOUT_S);

		string payload = BuildPayload(s_InFlight);

		s_iTicket++;
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
		{
			// `ScriptCallQueue.Remove` cancels BY FUNCTION, not by argument (measured landmine).
			// That is exactly right here and only because the queue is SERIAL: there is never
			// more than one armed watchdog, so cancelling "all of them" cancels precisely this
			// one. The ticket is the second belt — a watchdog that somehow survives cancellation
			// still cannot fire against a later request.
			queue.Remove(OnWatchdog);
			queue.CallLater(OnWatchdog, WATCHDOG_MS, false, s_iTicket);
		}

		TBD_Log.Kv(CH_LINK, "confirm", string.Format("player=%1 armaId=%2 url=%3%4 bytes=%5",
			s_InFlight.playerId, s_InFlight.armaId, baseUrl, CONFIRM_PATH, payload.Length()));

		ctx.POST(s_RestCallback, CONFIRM_PATH, payload);
	}

	//------------------------------------------------------------------------------------------------
	//! The wire body. Hand-built rather than via a save context so the exact bytes are determined
	//! by code that compiles, not by a serializer whose output shape is a runtime property this
	//! lane cannot observe — the same reasoning `TBD_ResultsReporter.BuildPayload` records.
	//!
	//! Assembled in steps, never one long `+` chain: a 9-field chain is a measured
	//! `Formula too complex`, whose SECOND diagnostic is a misleading `Incompatible parameter`.
	//!
	//! Field names are the backend's `LinkConfirmRequest` (`handlers/me.rs`): `code`, `arma_id`,
	//! `arma_character` — all `#[serde(default)]`, all snake_case.
	protected static string BuildPayload(notnull TBD_IdentityLinkPending pending)
	{
		string json = "{";
		json += string.Format("\"code\":\"%1\"", JsonEscape(pending.code));
		json += string.Format(",\"arma_id\":\"%1\"", JsonEscape(pending.armaId));
		json += string.Format(",\"arma_character\":\"%1\"", JsonEscape(pending.armaCharacter));
		json += "}";
		return json;
	}

	//------------------------------------------------------------------------------------
	// RESPONSE
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! 2xx. The backend answers `{"linked":true,"discord_id":…,"arma_id":…,"arma_character":…}`.
	//!
	//! Whether a 404/409 arrives here or on `OnConfirmError` is an engine-internal choice this
	//! lane cannot observe (does "success" mean 2xx, or does it mean the transport worked?). Both
	//! entry points therefore funnel any non-2xx into the SAME `HandleFailure`, so the player gets
	//! the right message under either answer instead of a wrong one under half of them.
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

		string ok = TAG + "linked. Your game identity is now attached to your TBD account — attendance and stats count from your next round.";
		TBD_Log.Kv(CH_LINK, "linked", string.Format("player=%1 armaId=%2 response=%3",
			s_InFlight.playerId, s_InFlight.armaId, body));
		Finish(ok, "ok");
	}

	//------------------------------------------------------------------------------------------------
	//! Everything else: an HTTP error status, or no status at all (transport failure).
	protected static void OnConfirmError(RestCallback cb)
	{
		if (!s_InFlight)
			return;

		HandleFailure(cb.GetHttpCode(), cb.GetRestResult(), cb.GetData());
	}

	//------------------------------------------------------------------------------------------------
	//! Turn the backend's real answers into something a player can act on.
	//!
	//! The statuses are the handler's own, read from `handlers/me.rs`:
	//!   404 `invalid or expired code`             — wrong / already used / older than 10 minutes
	//!   409 `arma id already linked to another account`
	//!   400 `code and arma_id required`           — our bug, not theirs
	//!   401/403                                   — the SERVER's `X-Service-Token` is rejected
	//!   5xx                                       — the website broke
	//!   no code at all                            — never reached the website
	//!
	//! Distinguishing them needs the HTTP STATUS, and `RestCallback.GetHttpCode()` is what gives
	//! it. Compile-proved (the failing control in the same run was
	//! `RestCallback.GetHttpCodeThatDoesNotExist` -> `Undefined function`) and then RUNTIME-proved:
	//! a real boot drove six requests at a capture endpoint mirroring the backend's rules and got
	//! `HTTP_CODE_200 / 409 / 404 / 401 / 400 / NULL` back, each landing on its own branch.
	//!
	//! **Never match on the response TEXT.** Two reasons, and the second is measured:
	//!   1. it breaks the first time someone rewords a backend error string;
	//!   2. on a transport failure `GetData()` returns the REQUEST body, not a response — measured
	//!      on that same run, where a dead-port POST came back carrying our own payload verbatim.
	//!      A body-text matcher would have been reading its own request.
	protected static void HandleFailure(HttpCode code, ERestResult result, string body)
	{
		string reason = typename.EnumToString(HttpCode, code);
		string player;

		if (code == HttpCode.HTTP_CODE_404)
		{
			player = TAG + "that code is not valid — wrong, already used, or expired (codes last 10 minutes).";
			ReplyAsync(player);
			ReplyAsync(TAG + "generate a fresh one on the website (avatar menu -> 'Link Arma Identity') and type it here. You are NOT linked.");
			FinishQuiet(reason, body);
			return;
		}

		if (code == HttpCode.HTTP_CODE_409)
		{
			player = TAG + "this game identity is already linked to a DIFFERENT TBD account, and one identity can only belong to one account.";
			ReplyAsync(player);
			ReplyAsync(TAG + "if that other account is yours, log into it on the website and press 'Unlink Arma ID' first. If it is not, contact an admin. You are NOT linked.");
			FinishQuiet(reason, body);
			return;
		}

		if (code == HttpCode.HTTP_CODE_401 || code == HttpCode.HTTP_CODE_403)
		{
			player = TAG + "this game server is not authorised to talk to the website — nothing you can do. Tell an admin (the server's service token is being rejected). You are NOT linked.";
			ReplyAsync(player);
			ReplyAsync(NewCodeAdvice());
			FinishQuiet(reason, body);
			return;
		}

		if (code == HttpCode.HTTP_CODE_400)
		{
			player = TAG + "the website rejected this server's request as malformed. That is a bug on our side, not your code — tell an admin. You are NOT linked.";
			ReplyAsync(player);
			ReplyAsync(NewCodeAdvice());
			FinishQuiet(reason, body);
			return;
		}

		if (code == HttpCode.HTTP_CODE_NULL)
		{
			// No HTTP status at all: DNS, connection refused, TLS, or the request never left.
			// `ERestResult` says which, and it goes in the log rather than at the player.
			//
			// The body is deliberately DROPPED here. Measured on the live run: with no response to
			// return, `GetData()` hands back the REQUEST — so logging it would print the player's
			// link code into console.log while claiming it was the website's answer. Neither half
			// of that is acceptable.
			player = TAG + "could not reach the website (network). Your code was not used — try again in a moment.";
			ReplyAsync(player);
			FinishQuiet(reason + "/" + typename.EnumToString(ERestResult, result), "(no response)");
			return;
		}

		player = TAG + "the website returned an error (" + reason + "). Try again shortly; if it keeps happening, tell an admin. You are NOT linked.";
		ReplyAsync(player);
		ReplyAsync(NewCodeAdvice());
		FinishQuiet(reason + "/" + typename.EnumToString(ERestResult, result), body);
	}

	//------------------------------------------------------------------------------------------------
	//! Said after every failure, and only after a failure. On success the backend has already set
	//! `consumed_at`, so the code is dead the moment it works; on failure it is still live AND it
	//! was typed where everyone could read it.
	protected static string NewCodeAdvice()
	{
		return TAG + "your code was typed in public chat and is still usable — generate a NEW one before retrying.";
	}

	//------------------------------------------------------------------------------------------------
	//! The watchdog. Reaches a request only if neither callback ever fired.
	//!
	//! Arity IS compile-checked for `CallLater` callbacks (measured: `Not enough parameters in
	//! callback`), which is the one reason this signature can be trusted without a live run.
	protected static void OnWatchdog(int ticket)
	{
		if (!s_InFlight)
			return;

		if (ticket != s_iTicket)
			return;

		TBD_Log.Warn(CH_LINK, string.Format(
			"no response after %1 ms for player=%2 — treating as unreachable. If this repeats, the REST callback is not firing.",
			WATCHDOG_MS, s_InFlight.playerId));

		Finish(TAG + "the website did not answer in time. Your code was not used — try again in a moment.", "watchdog-timeout");
	}

	//------------------------------------------------------------------------------------
	// COMPLETION
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Complete the in-flight request: tell the player one line, log the outcome, drain the queue.
	protected static void Finish(string playerLine, string outcome)
	{
		if (!playerLine.IsEmpty())
			ReplyAsync(playerLine);

		FinishQuiet(outcome, string.Empty);
	}

	//------------------------------------------------------------------------------------------------
	//! Complete without adding a player line (the caller already sent its own, possibly several).
	protected static void FinishQuiet(string outcome, string body)
	{
		if (s_InFlight && outcome != "ok")
		{
			TBD_Log.Warn(CH_LINK, string.Format("not linked player=%1 outcome=%2 response='%3'",
				s_InFlight.playerId, outcome, body));
		}

		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(OnWatchdog);

		s_InFlight = null;
		s_RestCallback = null;

		Pump();
	}

	//------------------------------------------------------------------------------------
	// REPLY
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Reply to the in-flight request's player, from a callback that may fire seconds after the
	//! command was typed.
	//!
	//! The player id is re-checked against the identity stamped at enqueue before anything is
	//! sent. A dedicated server RECYCLES connection ids, so a raw id held across an async gap can
	//! address a different human (measured: the same hazard made a deferred respawn deploy a
	//! fresh joiner into a dead player's slot). Comparing the current identity to the stamped one
	//! is the epoch check, using data this file already holds — if they differ, the original
	//! player is gone and the line is logged instead of sent to a stranger.
	protected static void ReplyAsync(string text)
	{
		if (!s_InFlight)
			return;

		string nowId = TBD_PlayerIdentity.GetArmaId(s_InFlight.playerId);
		if (nowId != s_InFlight.armaId)
		{
			TBD_Log.Event(CH_LINK, string.Format(
				"player=%1 left before the answer arrived (id now '%2', was '%3') — reply not delivered: %4",
				s_InFlight.playerId, nowId, s_InFlight.armaId, text));
			return;
		}

		Tell(s_InFlight.playerId, text);
	}

	//------------------------------------------------------------------------------------------------
	//! Server -> one client, over the channel this codebase already uses for per-player replies
	//! (`TBD_AdminCommands.Reply`, `TBD_ObjectivesComponent.Tell`, `TBD_PlayAreaComponent`).
	protected static void Tell(int playerId, string text)
	{
		Print("[TBD][" + CH_LINK + " " + playerId + "] " + text);

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		PlayerController pc = players.GetPlayerController(playerId);
		if (!pc)
			return;

		SCR_ChatComponent chat = SCR_ChatComponent.Cast(pc.FindComponent(SCR_ChatComponent));
		if (!chat)
			return;

		chat.SendPrivateMessage(text, playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! Immediate reply on the component the message arrived on — the synchronous path, where that
	//! component is known-good and re-resolving it would be pointless.
	protected static void ReplyLine(SCR_ChatComponent chat, int playerId, string text)
	{
		Print("[TBD][" + CH_LINK + " " + playerId + "] " + text);
		if (chat)
			chat.SendPrivateMessage(text, playerId);
	}

	//------------------------------------------------------------------------------------------------
	protected static void ReplyLines(SCR_ChatComponent chat, int playerId, notnull array<string> lines)
	{
		foreach (string line : lines)
			ReplyLine(chat, playerId, line);
	}

	//------------------------------------------------------------------------------------
	// HELPERS
	//------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! `TBD_BackendConfig` may be absent entirely — a LEGAL state on a local/PIE host, never an
	//! error. Its getters already return empty rather than null-deref, so this is a value test.
	protected static bool BackendConfigured()
	{
		return !TBD_BackendConfig.GetBackendUrl().IsEmpty() && !TBD_BackendConfig.GetServerToken().IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	//! The backend URL for a log line, never the token.
	protected static string DescribeBackend()
	{
		string url = TBD_BackendConfig.GetBackendUrl();
		if (url.IsEmpty())
			return "none";

		if (TBD_BackendConfig.GetServerToken().IsEmpty())
			return url + " (NO TOKEN)";

		return url;
	}

	//------------------------------------------------------------------------------------------------
	//! Display name, for `users.arma_character`. Cosmetic on the backend (it is not joined on),
	//! so an empty one is acceptable — the column is NOT NULL and takes `''`.
	protected static string PlayerName(int playerId)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return string.Empty;

		return players.GetPlayerName(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! `Replace` MUTATES IN PLACE and returns a COUNT (measured), so this is a sequence of
	//! statements and never `s = s.Replace(...)`. `string.Format("%1", value)` first, to get a
	//! copy the caller's string does not alias.
	protected static string JsonEscape(string value)
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
