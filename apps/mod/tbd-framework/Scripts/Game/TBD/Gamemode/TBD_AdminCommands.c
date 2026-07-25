//! Server-side admin chat command intercept. Listed admins drive the in-game
//! mission browser with `#tbd` chat commands:
//!   #tbd missions             — list available missions
//!   #tbd mission <n>          — load mission number n (reloads the world)
//!   #tbd backend <url> [tok]  — repoint the backend + refresh the list
//!   #tbd refresh              — refresh the mission list
//!   #tbd validate             — replay the mission validation findings (T-181.14)
//!
//! The custom menu (Phase D) calls the same TBD_FrameworkManager methods.
modded class SCR_ChatComponent
{
	//------------------------------------------------------------------------------------------------
	//! @authority server — admin chat commands are intercepted and executed on the server.
	override void OnNewMessage(string msg, int channelId, int senderId)
	{
		super.OnNewMessage(msg, channelId, senderId);

		// Authority only — commands execute on the server.
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!msg.StartsWith("#tbd"))
			return;

		SCR_PlayerListedAdminManagerComponent admins = SCR_PlayerListedAdminManagerComponent.GetInstance();
		if (!admins || !admins.IsPlayerOnAdminList(senderId))
		{
			TBD_AdminCommands.Reply(this, senderId, "TBD: admin only.");
			return;
		}

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
		{
			TBD_AdminCommands.Reply(this, senderId, "TBD: framework not ready.");
			return;
		}

		TBD_AdminCommands.Dispatch(this, fm, msg, senderId);
	}
}

//! Parses and executes #tbd admin commands, replying to the sending admin.
class TBD_AdminCommands
{
	//------------------------------------------------------------------------------------------------
	static void Dispatch(SCR_ChatComponent chat, TBD_FrameworkManager fm, string msg, int senderId)
	{
		array<string> parts = new array<string>();
		msg.Split(" ", parts, true);

		string sub;
		if (parts.Count() > 1)
			sub = parts[1];

		// T-181.14 — a mission rejected by TBD_MissionValidator is invisible from in-game: the
		// stage machine simply never leaves LOADING and nothing on screen says why. Lead with it
		// on every #tbd reply rather than waiting for an admin to think of asking.
		if (TBD_MissionValidator.HasRun() && !TBD_MissionValidator.Passed())
		{
			Reply(chat, senderId, string.Format("TBD: !! mission FAILED validation (%1 error(s)) — run '#tbd validate'.",
				TBD_MissionValidator.GetErrorCount()));
		}

		if (sub.IsEmpty() || sub == "missions" || sub == "list")
		{
			array<string> lines = fm.BuildMissionListText();
			foreach (string line : lines)
				Reply(chat, senderId, line);
			return;
		}

		if (sub == "refresh")
		{
			fm.RefreshMissionList();
			Reply(chat, senderId, "TBD: refreshing mission list…");
			return;
		}

		//! T-181.14 — replay the mission validation findings in game. Every problem from the
		//! last parse, errors first, so an admin can diagnose a rejected mission without SSHing
		//! to the server and reading console.log.
		if (sub == "validate")
		{
			array<string> lines = TBD_MissionValidator.BuildReportLines();
			foreach (string line : lines)
				Reply(chat, senderId, line);
			return;
		}

		if (sub == "mission")
		{
			if (parts.Count() < 3)
			{
				Reply(chat, senderId, "Usage: #tbd mission <number>");
				return;
			}
			Reply(chat, senderId, fm.SelectMissionByNumber(parts[2].ToInt()));
			return;
		}

		if (sub == "backend")
		{
			string url;
			string token;
			if (parts.Count() > 2)
				url = parts[2];
			if (parts.Count() > 3)
				token = parts[3];
			Reply(chat, senderId, fm.SetBackend(url, token));
			return;
		}

		//! T-181.11.1 — the one-life escape hatch. TBD events are one life, so a player who dies
		//! is out; this exists for GLITCH deaths only (fell through terrain, killed by a broken
		//! prop). Rematerializes a fresh dressed body on their own slot and writes an audit line.
		//! Permission is the vanilla admin list, already checked in OnNewMessage above.
		if (sub == "respawn")
		{
			if (parts.Count() < 3)
			{
				Reply(chat, senderId, "Usage: #tbd respawn <playerId>   (one-life glitch recovery)");
				return;
			}
			TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
			if (!sm)
			{
				Reply(chat, senderId, "TBD: spawn manager not ready.");
				return;
			}
			int target = parts[2].ToInt();
			if (target <= 0)
			{
				Reply(chat, senderId, "TBD: bad playerId '" + parts[2] + "'.");
				return;
			}
			TBD_EDeployResult r = sm.AdminRespawn(target, senderId.ToString());
			Reply(chat, senderId, string.Format("TBD: respawn player=%1 -> %2",
				target, typename.EnumToString(TBD_EDeployResult, r)));
			return;
		}

		//! Who has spent their life — the roster an admin needs before using `respawn`.
		if (sub == "dead")
		{
			TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
			if (!sm)
			{
				Reply(chat, senderId, "TBD: spawn manager not ready.");
				return;
			}
			array<int> ids = new array<int>();
			GetGame().GetPlayerManager().GetPlayers(ids);
			string line = "TBD dead:";
			foreach (int id : ids)
			{
				if (sm.IsPlayerDead(id))
					line += " " + id.ToString();
			}
			Reply(chat, senderId, line);
			return;
		}

		Reply(chat, senderId, "TBD: #tbd missions | mission <n> | backend <url> [token] | refresh | validate | respawn <playerId> | dead");
	}

	//------------------------------------------------------------------------------------------------
	//! Logs to the server console and sends a private chat message back to the admin.
	static void Reply(SCR_ChatComponent chat, int senderId, string text)
	{
		Print("[TBD][admin " + senderId + "] " + text);
		if (chat)
			chat.SendPrivateMessage(text, senderId);
	}
}
