//! Server-side admin chat command intercept. Listed admins drive the in-game
//! mission browser with `#tbd` chat commands:
//!   #tbd missions             — list available missions
//!   #tbd mission <n>          — load mission number n (reloads the world)
//!   #tbd backend <url> [tok]  — repoint the backend + refresh the list
//!   #tbd refresh              — refresh the mission list
//!   #tbd validate             — replay the mission validation findings (T-181.14)
//!   #tbd dead                 — who has spent their life
//!   #tbd respawn <playerId>   — the one-life escape hatch (T-181.11.1)
//!   #tbd deploy  <playerId>   — put a live player with no body into the world (T-181.11.2)
//!   #tbd stage [next|<NAME>]  — force the stage machine (T-181.11.2)
//!   #tbd safestart [status|go|<seconds>] — warmup phase: is damage off, end it, set its length (T-181.17)
//!   #tbd audit                — replay the admin audit trail (T-181.11.2)
//!   #tbd menu                 — raise the admin screen on the caller's client (T-181.11.2)
//!
//! ── T-181.11.2 — chat is a FRONT-END, not a second implementation ───────────────────────────
//! The admin menu (`TBD_AdminScreen`) and this file are two doors into the same room. The powers
//! themselves moved to `TBD_AdminService`, which re-checks the admin list and writes the audit
//! trail, so both surfaces enforce identically and both feed one history. The reply strings here
//! keep their old `respawn player=N -> RESULT` shape so anything grepping the console still works.
//!
//! Chat remains the surface that WORKS TODAY: the menu preset cannot resolve until Workbench
//! regenerates the addon's `resourceDatabase.rdb` (see `TBD_AdminScreen`), and nothing in this
//! file depends on that.
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

		// One permission oracle for every admin surface — the vanilla listed-admin manager, asked
		// through TBD_AdminService so chat and the menu can never drift apart on who counts as an
		// admin.
		if (!TBD_AdminService.IsAdmin(senderId))
		{
			TBD_AdminService.NoteDeniedAccess(senderId, "#tbd chat command");
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
		//! T-181.11.2 — now routed through TBD_AdminService, so it re-checks the admin list and
		//! records the attempt in the same trail the admin screen shows.
		if (sub == "respawn")
		{
			if (parts.Count() < 3)
			{
				Reply(chat, senderId, "Usage: #tbd respawn <playerId>   (one-life glitch recovery)");
				return;
			}
			Reply(chat, senderId, RunAction(senderId, TBD_EAdminAction.RESPAWN, parts[2]));
			return;
		}

		//! T-181.11.2 — the other half of "spawn it if it breaks": a player who still has their
		//! life but never got a body. AdminRespawn refuses anyone who is not dead, so this is a
		//! different lever, not the same one under another name.
		if (sub == "deploy")
		{
			if (parts.Count() < 3)
			{
				Reply(chat, senderId, "Usage: #tbd deploy <playerId>   (live player stuck with no body)");
				return;
			}
			Reply(chat, senderId, RunAction(senderId, TBD_EAdminAction.DEPLOY, parts[2]));
			return;
		}

		//! T-181.11.2 — force the stage machine. The recovery for a round that cannot advance on
		//! its own, which is precisely what a rejected mission produces.
		if (sub == "stage")
		{
			string arg = "next";
			if (parts.Count() > 2)
				arg = parts[2];

			bool ok;
			Reply(chat, senderId, TBD_AdminService.ForceStage(senderId, arg, ok));
			return;
		}

		//! T-181.17 — the warmup phase. `status` answers "is damage actually off right now" in one
		//! command; `go` ends it early; a number sets/extends the countdown. Entering SAFE_START at
		//! all is still `#tbd stage` — this controls the phase, it does not start it.
		if (sub == "safestart")
		{
			string safestartArg = "status";
			if (parts.Count() > 2)
				safestartArg = parts[2];

			bool safestartOk;
			Reply(chat, senderId, TBD_AdminService.Safestart(senderId, safestartArg, safestartOk));
			return;
		}

		//! T-181.11.2 — who did what to whom, newest first. The same trail the admin screen
		//! renders, readable without the screen.
		if (sub == "audit")
		{
			array<string> lines = TBD_AdminAudit.BuildReportLines();
			foreach (string line : lines)
				Reply(chat, senderId, line);
			return;
		}

		//! T-181.11.2 — raise the admin screen on the caller's own client. The server pushes it
		//! over an owner-targeted RPC, so it needs no keybind bound and no client-side state.
		if (sub == "menu")
		{
			Reply(chat, senderId, OpenMenuFor(senderId));
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

		Reply(chat, senderId, "TBD: #tbd missions | mission <n> | backend <url> [token] | refresh | validate | dead | respawn <playerId> | deploy <playerId> | stage [next|<NAME>] | safestart [status|go|<seconds>] | audit | menu");
	}

	//------------------------------------------------------------------------------------------------
	//! Parse a playerId argument and run one admin power through the shared authority. The gate
	//! and the audit line both live in TBD_AdminService — this only turns text into an int.
	protected static string RunAction(int senderId, TBD_EAdminAction action, string targetArg)
	{
		int target = targetArg.ToInt();
		if (target <= 0)
			return "TBD: bad playerId '" + targetArg + "'.";

		bool ok;
		return TBD_AdminService.Execute(senderId, action, target, ok);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — push the admin screen onto the requesting admin's own client.
	protected static string OpenMenuFor(int senderId)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return "TBD: no player manager.";

		SCR_PlayerController controller = SCR_PlayerController.Cast(players.GetPlayerController(senderId));
		if (!controller)
			return "TBD: could not find your player controller.";

		controller.TBD_OpenAdminMenuOnOwner();
		TBD_AdminAudit.Record(string.Format("%1 opened the admin menu", TBD_AdminService.Label(senderId)), false);

		// Honest about the one thing that can stop it appearing, so an admin does not stare at an
		// unchanged screen wondering whether the command worked.
		return "TBD: opening the admin menu… (if nothing appears, the menu preset is not in resourceDatabase.rdb yet — chat commands still work).";
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
