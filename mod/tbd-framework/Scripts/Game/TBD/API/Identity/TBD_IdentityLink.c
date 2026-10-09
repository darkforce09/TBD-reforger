/**
 * @file TBD_IdentityLink.c
 * @brief The `#tbd link <code>` chat command that links a game identity to a TBD website account.
 *
 * Role: recognises the command, answers usage and status, and validates a code locally before
 * queueing it.  Position: called from the `TBD_AdminCommands` chat hook before the admin gate and
 * before `super.OnNewMessage`; armed by `TBD_MissionLoader.ParseMissionJson`; hands accepted codes
 * to `TBD_IdentityLinkConfirm`, which posts them to `/api/v1/ingest/link-confirm`.
 * State: none of its own; the queue lives in `TBD_IdentityLinkConfirm`.  Invariants: the
 * `arma_id` comes only from `TBD_PlayerIdentity.GetArmaId`, the accessor `TBD_ResultsReporter`
 * also uses, so both backend joins see identical bytes; a player without a durable identity is
 * refused with a reason and never linked under a seat number or a name hash, because
 * `users.arma_id` is UNIQUE; the code never reaches public chat.
 */

//! The `#tbd link` chat surface: usage, status and local validation before the confirm queue.
//! @authority server
class TBD_IdentityLink
{
	static const string CH_LINK = "Link"; //!< log channel: `grep '\[TBD\]\[Link\]' console.log`
	static const string TAG = "TBD: "; //!< prefix of every player reply, matching `TBD_AdminCommands.Reply`
	protected static const int CODE_MAX_CHARS = 32; //!< sanity bound on a typed code, in characters; the backend alone judges validity

	//! Say once, in the log, whether linking can work on this host. Called from
	//! `TBD_MissionLoader.ParseMissionJson` next to `TBD_ResultsReporter.Arm()`, a server-only path
	//! and the earliest moment a link means anything. Idempotent: statics outlive a world inside
	//! one process, so a second call is harmless.
	//! Does nothing on a client.
	//! @authority server
	static void Arm()
	{
		if (TBD_Authority.IsClient())
			return;

		TBD_Log.Kv(CH_LINK, "armed", string.Format("command='#tbd link <code>' backend=%1", TBD_BackendText.DescribeBackend()));

		if (!BackendConfigured())
		{
			// A legal state on a local or PIE host, logged at normal level so it neither alarms an
			// operator nor trips the world-boot error triage.
			TBD_Log.Event(CH_LINK,
				"no backend configured (backendUrl or machineCredential missing) - '#tbd link' will tell players so instead of failing silently. This is a legal state on a local host.");
		}
	}

	//! Consume-before-broadcast guard for the chat hook. Every peer, clients included, suppresses
	//! the echo of a link command; only the authority runs the link flow.
	//! @param chat the chat component the message arrived on
	//! @param msg the raw chat line
	//! @param senderId the typing player
	//! @param authority true on the server, which runs `TryHandleChat`
	//! @return true when `msg` is a `#tbd link` command and the caller must not call
	//! `super.OnNewMessage`; false for ordinary chat, including a bare code without the prefix
	static bool TryConsumeBeforeBroadcast(SCR_ChatComponent chat, string msg, int senderId, bool authority)
	{
		if (!IsLinkCommand(msg))
			return false;

		if (authority)
			TryHandleChat(chat, msg, senderId);

		return true;
	}

	//! Whether `msg` is `#tbd link` in any letter case; the prefix only, no further filtering.
	//! @return true for a link command
	protected static bool IsLinkCommand(string msg)
	{
		if (!msg.StartsWith("#tbd"))
			return false;

		array<string> parts = new array<string>();
		msg.Split(" ", parts, true);
		if (parts.Count() < 2)
			return false;

		string sub = string.Format("%1", parts[1]);
		sub.ToLower();
		return sub == "link";
	}

	//! Handle a `#tbd link ...` line: usage, `status`, or a code to submit. Every player needs to
	//! link, so this entry point sits in front of the admin gate in `TBD_AdminCommands`, never
	//! behind it. `chat` serves only the immediate reply; asynchronous replies resolve the
	//! player's chat component again through `TBD_PlayerChat`.
	//! @return true when the line was a link command and is fully handled
	//! @authority server
	static bool TryHandleChat(SCR_ChatComponent chat, string msg, int senderId)
	{
		array<string> parts = new array<string>();
		msg.Split(" ", parts, true);

		if (parts.Count() < 2)
			return false;

		// Case-folded so `#tbd Link` works. `ToLower()` mutates in place and returns a count, so it
		// runs as a statement on a `string.Format` copy that cannot reach back into `parts`. The
		// code itself is not folded: it is the backend's token.
		string sub = string.Format("%1", parts[1]);
		sub.ToLower();
		if (sub != "link")
			return false;

		string arg;
		if (parts.Count() > 2)
			arg = parts[2];

		if (arg.IsEmpty())
		{
			ReplyLines(chat, senderId, Usage());
			return true;
		}

		string argLower = string.Format("%1", arg);
		argLower.ToLower();
		if (argLower == "status")
		{
			ReplyLines(chat, senderId, StatusLines(senderId));
			return true;
		}

		Submit(chat, senderId, arg);
		return true;
	}

	//! The usage lines, with the whole flow, so a player never has to ask where the code comes from.
	//! @return the lines to send
	protected static array<string> Usage()
	{
		array<string> lines = new array<string>();
		lines.Insert(TAG + "usage: #tbd link <code>   (also: #tbd link status)");
		lines.Insert(TAG + "1. on the website, open the avatar menu -> 'Link Arma Identity' -> Generate Link Code");
		lines.Insert(TAG + "2. type that 6-digit code here within 10 minutes. It links this game identity to your TBD account so attendance and stats count.");
		lines.Insert(TAG + "NOTE: this command is private -- other players do not see the code. If a link fails, generate a NEW code before retrying.");
		return lines;
	}

	//! What this host can do, without touching the backend: whether an identity resolves, whether
	//! it is durable, and whether a backend is configured. No machine-credential route answers "is this
	//! arma id linked"; `GET /me/link/status` answers for a signed-in browser.
	//! @return the lines to send
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
			lines.Insert(TAG + "website: " + TBD_BackendText.DescribeBackend());
		else
			lines.Insert(TAG + "website: NOT CONFIGURED on this server - linking is unavailable here.");

		return lines;
	}

	//! Validate everything that can be validated locally, then queue one confirm request. Every
	//! refusal carries a reason the player can act on. Whether the code itself is good is the
	//! backend's call (404 for wrong, used or expired).
	protected static void Submit(SCR_ChatComponent chat, int playerId, string rawCode)
	{
		// `Trim()` returns a new string, unlike `Replace`/`ToLower`, which mutate in place, so this
		// is an assignment; a bare `code.Trim();` does nothing.
		string code = rawCode.Trim();

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
			// Vanilla's `00bbbddd-` synthesized id is a hash of the display name, issued only off a
			// dedicated server. In `users.arma_id` (UNIQUE) it binds the account to everyone who
			// uses that name and locks every other account out of it.
			ReplyLine(chat, playerId, TAG + "cannot link: this host gives you a name-derived identity, not a real one, so a link made here would break the moment you rename - and would block anyone else with your name.");
			ReplyLine(chat, playerId, TAG + "link from a DEDICATED TBD server instead. Your code was not used; generate a new one anyway.");
			TBD_Log.Warn(CH_LINK, string.Format(
				"refused player=%1 reason=synthetic-identity id=%2 (listen/hosted host - vanilla name hash). Run events on a dedicated server.", playerId, armaId));
			return;
		}

		if (!BackendConfigured())
		{
			ReplyLine(chat, playerId, TAG + "cannot link: this server is not connected to the TBD website, so it cannot confirm your code. Tell an admin. Your code was not used.");
			TBD_Log.Event(CH_LINK, string.Format(
				"refused player=%1 reason=no-backend (backendUrl or machineCredential missing). Legal state on a local host.", playerId));
			return;
		}

		if (TBD_IdentityLinkConfirm.HasOutstanding(playerId))
		{
			ReplyLine(chat, playerId, TAG + "already checking your last code - wait for the answer before sending another.");
			return;
		}

		if (TBD_IdentityLinkConfirm.IsQueueFull())
		{
			ReplyLine(chat, playerId, TAG + "too many link requests queued right now - try again in a minute.");
			TBD_Log.Warn(CH_LINK, string.Format("queue full (%1) - dropped request from player=%2", TBD_IdentityLinkConfirm.MAX_QUEUE, playerId));
			return;
		}

		TBD_IdentityLinkPending pending = new TBD_IdentityLinkPending();
		pending.playerId = playerId;
		pending.armaId = armaId;
		pending.armaCharacter = PlayerName(playerId);
		pending.code = code;

		ReplyLine(chat, playerId, TAG + "checking that code with the website...");
		TBD_IdentityLinkConfirm.Enqueue(pending);
	}

	//! Write a player reply to the log as `[TBD][Link <playerId>] <text>`.
	static void LogReply(int playerId, string text)
	{
		Print("[TBD][" + CH_LINK + " " + playerId + "] " + text);
	}

	//! Immediate reply on the component the message arrived on, the synchronous path where that
	//! component is known to be live. Logs the line; a null `chat` sends nothing.
	protected static void ReplyLine(SCR_ChatComponent chat, int playerId, string text)
	{
		LogReply(playerId, text);
		if (chat)
			chat.SendPrivateMessage(text, playerId);
	}

	//! `ReplyLine` for each of `lines`, in order.
	protected static void ReplyLines(SCR_ChatComponent chat, int playerId, notnull array<string> lines)
	{
		foreach (string line : lines)
			ReplyLine(chat, playerId, line);
	}

	//! Whether a backend URL and a usable machine credential are both set, the pair
	//! `TBD_GameRuntimeHttp` sends a confirm with. A missing `TBD_BackendConfig` is a legal state on
	//! a local or PIE host; its getters return empty, so this is a value test.
	//! @return true when a confirm could be sent
	protected static bool BackendConfigured()
	{
		return TBD_GameRuntimeHttp.IsConfigured();
	}

	//! Display name, for `users.arma_character`. Cosmetic on the backend (nothing joins on it),
	//! so an empty one is acceptable; the column is NOT NULL and takes `''`.
	//! @return the name, or empty without a player manager
	protected static string PlayerName(int playerId)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return string.Empty;

		return players.GetPlayerName(playerId);
	}
}
