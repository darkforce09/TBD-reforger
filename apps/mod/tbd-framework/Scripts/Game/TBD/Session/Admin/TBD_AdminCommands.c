/**
 * @file TBD_AdminCommands.c
 * @brief The `#tbd` admin chat commands: the chat hook and the dispatcher behind it.
 *
 * Role: intercepts `#tbd` chat on the server, gates it on the admin list and runs the subcommand.
 * Position: vanilla SCR_ChatComponent.OnNewMessage feeds it; it calls TBD_AdminService,
 * TBD_AdminSubcommands, TBD_AdminAudit, TBD_DeployableMissionList, TBD_MissionDeploymentRelay,
 * TBD_MissionValidator, TBD_BackendConfig and TBD_IdentityLink; replies go to the sender by private
 * chat and to the server console.
 * State: none.  Invariants: `#tbd link` is consumed before vanilla broadcasts it; every other
 * command runs only on the server and only for a sender TBD_AdminService.IsAdmin accepts, and a
 * refused sender is noted; reply lines keep the `respawn player=N -> RESULT` shape.
 *
 * Commands:
 *   #tbd missions             list the missions the platform lets this server deploy
 *   #tbd mission <n>          ask the platform to deploy mission n
 *   #tbd backend <url>        repoint the backend URL, refresh the list
 *   #tbd refresh              refresh the mission list
 *   #tbd validate             replay the mission validation findings
 *   #tbd dead                 who has spent their life
 *   #tbd respawn <playerId>   the one-life escape hatch
 *   #tbd deploy <playerId>    put a live player with no body into the world
 *   #tbd stage [next|<NAME>]  force the stage machine
 *   #tbd safestart [status|go|<seconds>]              the warm-up: read it, end it, set its length
 *   #tbd identity [status|override <phrase>|enforce]  whether this host can enforce ONE LIFE
 *   #tbd audit                replay the admin audit trail
 *   #tbd menu                 raise the admin screen on the caller's client
 */

//! Chat hook for the `#tbd` admin commands.
modded class SCR_ChatComponent
{
	//! Consume `#tbd link`, pass everything else to vanilla, then on the server run a `#tbd` command
	//! for a listed admin or refuse and note the attempt.
	//! @param msg the chat text
	//! @param channelId the chat channel
	//! @param senderId the sender's player id
	//! @authority server
	override void OnNewMessage(string msg, int channelId, int senderId)
	{
		// Consume `#tbd link ...` before vanilla distributes. `super.OnNewMessage` is
		// the broadcast/display path (vanilla forwards it to `SCR_ChatPanelManager`). TRUE from
		// this guard means: do not call super; the code must never reach public chat. Authority
		// POSTs the code; every peer suppresses the echo. A bare token without this prefix is
		// ordinary chat -- we do not filter beyond the command.
		if (TBD_IdentityLink.TryConsumeBeforeBroadcast(this, msg, senderId, TBD_Authority.IsServer()))
			return;

		super.OnNewMessage(msg, channelId, senderId);

		// Authority only -- commands execute on the server.
		if (TBD_Authority.IsClient())
			return;

		if (!msg.StartsWith("#tbd"))
			return;

		// One permission oracle for every admin surface -- the vanilla listed-admin manager, asked
		// through TBD_AdminService so chat and the menu can never drift apart on who counts as an
		// admin.
		if (!TBD_AdminService.IsAdmin(senderId))
		{
			TBD_AdminService.NoteDeniedAccess(senderId, "#tbd chat command");
			TBD_AdminCommands.Reply(this, senderId, "TBD: admin only.");
			return;
		}

		if (!TBD_FrameworkManager.GetInstance())
		{
			TBD_AdminCommands.Reply(this, senderId, "TBD: framework not ready.");
			return;
		}

		TBD_AdminCommands.Dispatch(this, msg, senderId);
	}
}

//! Parses and runs `#tbd` admin commands, replying to the sending admin.
class TBD_AdminCommands
{
	//! Run one `#tbd` command for a sender already accepted as an admin. Leads with a warning line
	//! when the loaded mission failed validation; an unknown subcommand gets the command list.
	//! @param chat the chat component that received the message
	//! @param msg the full chat text
	//! @param senderId the sending admin's player id
	//! @authority server
	static void Dispatch(SCR_ChatComponent chat, string msg, int senderId)
	{
		array<string> parts = new array<string>();
		msg.Split(" ", parts, true);

		string sub;
		if (parts.Count() > 1)
			sub = parts[1];

		// A mission TBD_MissionValidator rejected never leaves LOADING and nothing on screen says
		// why, so every #tbd reply leads with it.
		if (TBD_MissionValidator.HasRun() && !TBD_MissionValidator.Passed())
		{
			Reply(chat, senderId, string.Format("TBD: !! mission FAILED validation (%1 error(s)) -- run '#tbd validate'.",
				TBD_MissionValidator.GetErrorCount()));
		}

		if (sub.IsEmpty() || sub == "missions" || sub == "list")
		{
			array<string> lines = TBD_DeployableMissionList.BuildListLines();
			foreach (string line : lines)
				Reply(chat, senderId, line);
			return;
		}

		if (sub == "refresh")
		{
			TBD_DeployableMissionList.Refresh();
			Reply(chat, senderId, "TBD: refreshing the mission list...");
			return;
		}

		// Every problem from the last parse, errors first, so a rejected mission can be diagnosed
		// in game.
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
			Reply(chat, senderId, TBD_MissionDeploymentRelay.RequestByNumber(senderId, parts[2].ToInt()));
			return;
		}

		if (sub == "backend")
		{
			string url;
			if (parts.Count() > 2)
				url = parts[2];
			Reply(chat, senderId, SetBackend(url));
			return;
		}

		// The one-life escape hatch, for glitch deaths only (fell through terrain, killed by a
		// broken prop): a fresh dressed body on the player's own slot, through TBD_AdminService.
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

		// A player who still has their life but never got a body. AdminRespawn refuses anyone who
		// is not dead, so this is a separate lever.
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

		// Force the stage machine: the recovery for a round that cannot advance on its own.
		if (sub == "stage")
		{
			string arg = "next";
			if (parts.Count() > 2)
				arg = parts[2];

			bool ok;
			Reply(chat, senderId, TBD_AdminService.ForceStage(senderId, arg, ok));
			return;
		}

		// The warm-up: `status` reads it, `go` ends it early, a number sets the countdown. Entering
		// SAFE_START is `#tbd stage`; this controls the phase, it does not start it.
		if (sub == "safestart")
		{
			string safestartArg = "status";
			if (parts.Count() > 2)
				safestartArg = parts[2];

			bool safestartOk;
			Reply(chat, senderId, TBD_AdminSubcommands.Safestart(senderId, safestartArg, safestartOk));
			return;
		}

		// Whether this host can enforce ONE LIFE, and the waiver if it cannot; the phrase is a
		// separate argument (see TBD_AdminSubcommands.Identity).
		if (sub == "identity")
		{
			string identityArg = "status";
			if (parts.Count() > 2)
				identityArg = parts[2];

			string identityConfirm;
			if (parts.Count() > 3)
				identityConfirm = parts[3];

			bool identityOk;
			Reply(chat, senderId, TBD_AdminSubcommands.Identity(senderId, identityArg, identityConfirm, identityOk));
			return;
		}

		// Who did what to whom, newest first: the trail the admin screen renders.
		if (sub == "audit")
		{
			array<string> lines = TBD_AdminAudit.BuildReportLines();
			foreach (string line : lines)
				Reply(chat, senderId, line);
			return;
		}

		// Raise the admin screen on the caller's own client over an owner-targeted RPC, so it needs
		// no key bound and no client-side state.
		if (sub == "menu")
		{
			Reply(chat, senderId, OpenMenuFor(senderId));
			return;
		}

		// Who has spent their life: the roster an admin needs before using `respawn`.
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

		Reply(chat, senderId, "TBD: #tbd missions | mission <n> | backend <url> | refresh | validate | dead | respawn <playerId> | deploy <playerId> | stage [next|<NAME>] | safestart [status|go|<seconds>] | identity [status|override <phrase>|enforce] | audit | menu");
	}

	//! Repoint the backend URL, keeping the machine credential, then refresh the mission list.
	//! @param url the backend base URL; empty returns the usage line
	//! @return the reply line
	protected static string SetBackend(string url)
	{
		if (url.IsEmpty())
			return "Usage: #tbd backend <url>";

		if (!TBD_BackendConfig.SetBackend(url))
			return "TBD: failed to set backend.";

		TBD_DeployableMissionList.Refresh();
		return string.Format("TBD: backend set to %1 - refreshing the mission list...", url);
	}

	//! Parse a playerId argument and run one admin power through TBD_AdminService.Execute, which owns
	//! the gate and the audit line.
	//! @param senderId the admin
	//! @param action the power
	//! @param targetArg the player id as typed
	//! @return the reply line; a non-positive id is refused before Execute
	protected static string RunAction(int senderId, TBD_EAdminAction action, string targetArg)
	{
		int target = targetArg.ToInt();
		if (target <= 0)
			return "TBD: bad playerId '" + targetArg + "'.";

		bool ok;
		return TBD_AdminService.Execute(senderId, action, target, ok);
	}

	//! Push the admin screen onto the requesting admin's own client and audit it.
	//! @param senderId the admin
	//! @return the reply line, or why the controller could not be found
	//! @authority server
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
		return "TBD: opening the admin menu... (if nothing appears, the menu preset is not in resourceDatabase.rdb yet -- chat commands still work).";
	}

	//! Log a reply to the server console and send it to the admin by private chat.
	//! @param chat the chat component to send through; null logs only
	//! @param senderId the admin
	//! @param text the reply
	static void Reply(SCR_ChatComponent chat, int senderId, string text)
	{
		Print("[TBD][admin " + senderId + "] " + text);
		if (chat)
			chat.SendPrivateMessage(text, senderId);
	}
}
