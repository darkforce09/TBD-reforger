//! The arguments of a claimed fleet command, checked again exactly as the platform validates them
//! (each action accepts only its own keys; text is 1 to N bytes after trimming, with no control
//! character; ids are UUIDs; digests are 64 lowercase hex), and the command's preconditions on this
//! server. A command failing here is reported failed and its effect never starts:
//!   * `broadcast` {message}: message of 1 to 256 bytes;
//!   * `kick` {arma_id, runtime_session_id, reason?}: arma_id and reason of 1 to 128 bytes; the
//!     session must be the one this runtime holds, and a connected player must have that identity
//!     (TBD_PlayerIdentity.GetArmaId);
//!   * `load_mission` {deployment_id, artifact_id, artifact_sha256, runtime_session_id}: the session
//!     must be the one this runtime holds;
//!   * any other action is not a game-runtime action.
//! @authority server
class TBD_FleetCommandArguments
{
	protected static const string HEX_DIGITS = "0123456789abcdefABCDEF";

	//------------------------------------------------------------------------------------------------
	//! Why `command` must not run, or empty when it may.
	static string Check(notnull TBD_FleetCommand command)
	{
		if (!command.m_mArguments)
			return "the command's arguments cannot be read as text values";

		if (command.m_sAction == "broadcast")
			return CheckBroadcast(command.m_mArguments);

		if (command.m_sAction == "kick")
			return CheckKick(command);

		if (command.m_sAction == "load_mission")
			return CheckLoadMission(command.m_mArguments);

		return string.Format("'%1' is not a game-runtime action", command.m_sAction);
	}

	//------------------------------------------------------------------------------------------------
	//! The connected player whose game identity is `armaId`, or -1.
	static int FindConnectedPlayer(string armaId)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players || armaId.IsEmpty())
			return -1;

		array<int> ids = {};
		players.GetPlayers(ids);
		foreach (int playerId : ids)
		{
			if (TBD_PlayerIdentity.GetArmaId(playerId).Trim() == armaId)
				return playerId;
		}

		return -1;
	}

	//------------------------------------------------------------------------------------------------
	protected static string CheckBroadcast(notnull map<string, string> arguments)
	{
		array<string> accepted = {"message"};
		string refusal = OnlyKeys(arguments, accepted, "broadcast");
		if (!refusal.IsEmpty())
			return refusal;

		if (!IsBoundedText(arguments, "message", 256))
			return "message must contain 1 to 256 bytes without control characters";

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	protected static string CheckKick(notnull TBD_FleetCommand command)
	{
		map<string, string> arguments = command.m_mArguments;
		array<string> accepted = {"arma_id", "runtime_session_id", "reason"};
		string refusal = OnlyKeys(arguments, accepted, "kick");
		if (!refusal.IsEmpty())
			return refusal;

		if (!IsBoundedText(arguments, "arma_id", 128))
			return "arma_id must contain 1 to 128 bytes without control characters";

		refusal = CheckOwnSession(arguments);
		if (!refusal.IsEmpty())
			return refusal;

		if (arguments.Contains("reason") && !IsBoundedText(arguments, "reason", 128))
			return "reason must contain 1 to 128 bytes without control characters";

		int playerId = FindConnectedPlayer(command.Argument("arma_id").Trim());
		if (playerId < 0)
			return "the player is not connected";

		command.m_iTargetPlayerId = playerId;
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	protected static string CheckLoadMission(notnull map<string, string> arguments)
	{
		array<string> accepted = {"deployment_id", "artifact_id", "artifact_sha256", "runtime_session_id"};
		string refusal = OnlyKeys(arguments, accepted, "load_mission");
		if (!refusal.IsEmpty())
			return refusal;

		string value;
		if (!arguments.Find("deployment_id", value) || !IsUuid(value))
			return "deployment_id must be a UUID";

		if (!arguments.Find("artifact_id", value) || !IsUuid(value))
			return "artifact_id must be a UUID";

		if (!arguments.Find("artifact_sha256", value) || !TBD_Sha256.IsHexDigest(value))
			return "artifact_sha256 must be 64 lowercase hex characters";

		return CheckOwnSession(arguments);
	}

	//------------------------------------------------------------------------------------------------
	//! The command must be issued against the runtime session this runtime holds.
	protected static string CheckOwnSession(notnull map<string, string> arguments)
	{
		string named;
		if (!arguments.Find("runtime_session_id", named) || !IsUuid(named))
			return "runtime_session_id must be a UUID";

		string held = TBD_RuntimeSession.GetSessionId();
		if (named != held)
		{
			if (held.IsEmpty())
				held = "none";

			return string.Format("the command is issued against runtime session %1, and this runtime holds %2", named, held);
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Empty when every key of `arguments` is in `accepted`, else the refusal naming the first other.
	protected static string OnlyKeys(notnull map<string, string> arguments, notnull array<string> accepted, string action)
	{
		for (int i = 0; i < arguments.Count(); i++)
		{
			string key = arguments.GetKey(i);
			if (accepted.Find(key) < 0)
				return string.Format("argument %1 is not accepted by %2", key, action);
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! The argument `key` is present and, trimmed, 1 to `maxBytes` bytes with no control character:
	//! no byte below 32, no 127, and no UTF-8 encoded C1 control (0xC2 0x80..0x9F).
	protected static bool IsBoundedText(notnull map<string, string> arguments, string key, int maxBytes)
	{
		string value;
		if (!arguments.Find(key, value))
			return false;

		string text = value.Trim();
		int length = text.Length();
		if (length < 1 || length > maxBytes)
			return false;

		for (int i = 0; i < length; i++)
		{
			int code = text.ToAscii(i) & 255;
			if (code < 32 || code == 127)
				return false;

			if (code == 194 && i + 1 < length)
			{
				int next = text.ToAscii(i + 1) & 255;
				if (next >= 128 && next <= 159)
					return false;
			}
		}

		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! 8-4-4-4-12 hex digits.
	static bool IsUuid(string text)
	{
		if (text.Length() != 36)
			return false;

		for (int i = 0; i < 36; i++)
		{
			string character = text.Get(i);
			if (i == 8 || i == 13 || i == 18 || i == 23)
			{
				if (character != "-")
					return false;

				continue;
			}

			if (!HEX_DIGITS.Contains(character))
				return false;
		}

		return true;
	}
}
