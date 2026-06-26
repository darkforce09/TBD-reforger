//! Server-side admin chat command intercept. Listed admins drive the in-game
//! mission browser with `#tbd` chat commands:
//!   #tbd missions             — list available missions
//!   #tbd mission <n>          — load mission number n (reloads the world)
//!   #tbd backend <url> [tok]  — repoint the backend + refresh the list
//!   #tbd refresh              — refresh the mission list
//!
//! The custom menu (Phase D) calls the same TBD_FrameworkManager methods.
modded class SCR_ChatComponent
{
	//------------------------------------------------------------------------------------------------
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

		Reply(chat, senderId, "TBD: #tbd missions | mission <n> | backend <url> [token] | refresh");
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
