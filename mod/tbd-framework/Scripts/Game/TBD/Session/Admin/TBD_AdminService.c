/**
 * @file TBD_AdminService.c
 * @brief The admin authority: the one permission gate and the menu powers behind it.
 *
 * Role: answers who is an admin and runs the respawn, deploy and stage powers for both admin
 * surfaces.  Position: the admin RPCs on SCR_PlayerController and TBD_AdminCommands call Execute and
 * ForceStage; TBD_AdminSubcommands and TBD_AdminSnapshotService use IsAdmin, Label and
 * NoteDeniedAccess; the powers drive TBD_SpawnManager and TBD_FrameworkManager and write
 * TBD_AdminAudit.
 * State: the per-(player, surface) refusal counts, static on the server.  Invariants: every power
 * refuses on a client and refuses a caller IsAdmin rejects before it touches anything; the caller id
 * is the player id of the controller the request arrived on, never a wire argument; every attempt,
 * refused or not, is audited; no other public function here reaches TBD_SpawnManager or
 * TBD_FrameworkManager.
 */

//! Server-side choke point for every admin power. The screen and the `#tbd` chat commands both
//! call in, so they share one gate and one audit trail.
//! @authority server
class TBD_AdminService
{
	protected static ref map<string, int> s_mDeniedCount; //!< "<playerId>|<surface>" -> refusals counted; see NoteDeniedAccess
	protected static const int MAX_DENIED_KEYS = 256; //!< refusal-count rows kept; the map is cleared when it grows past this

	//! Whether a player is on the vanilla listed-admin list, the one permission question.
	//! @param playerId the player to ask about
	//! @return false for a non-positive id, a missing admin manager, or a player not on the list
	//! @authority server
	static bool IsAdmin(int playerId)
	{
		if (playerId <= 0)
			return false;

		SCR_PlayerListedAdminManagerComponent admins = SCR_PlayerListedAdminManagerComponent.GetInstance();
		if (!admins)
			return false;

		return admins.IsPlayerOnAdminList(playerId);
	}

	//! How a player appears in the audit trail: `Name(id)`, since a name can be shared and an id
	//! alone cannot be read back later.
	//! @param playerId the player; a non-positive id is the server itself
	//! @return `server`, `Name(id)`, or `player(id)` when the player manager or name is missing
	static string Label(int playerId)
	{
		if (playerId <= 0)
			return "server";

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return string.Format("player(%1)", playerId);

		string name = players.GetPlayerName(playerId);
		if (name.IsEmpty())
			name = "player";

		return string.Format("%1(%2)", name, playerId);
	}

	//! Turn an int that arrived over the wire into an action. Enfusion assigns any int to an enum
	//! variable, so the wire boundary rejects values no member names.
	//! @param actionId the int the client sent
	//! @return the matching TBD_EAdminAction, or NONE for any other value
	static TBD_EAdminAction FromWire(int actionId)
	{
		if (actionId == TBD_EAdminAction.RESPAWN)
			return TBD_EAdminAction.RESPAWN;

		if (actionId == TBD_EAdminAction.DEPLOY)
			return TBD_EAdminAction.DEPLOY;

		if (actionId == TBD_EAdminAction.STAGE_ADVANCE)
			return TBD_EAdminAction.STAGE_ADVANCE;

		return TBD_EAdminAction.NONE;
	}

	//! The audit name of an action.
	//! @return `respawn`, `deploy`, `stage-advance`, or `none`
	static string ActionName(TBD_EAdminAction action)
	{
		if (action == TBD_EAdminAction.RESPAWN)
			return "respawn";

		if (action == TBD_EAdminAction.DEPLOY)
			return "deploy";

		if (action == TBD_EAdminAction.STAGE_ADVANCE)
			return "stage-advance";

		return "none";
	}

	//! Run one menu power on behalf of `callerId`.
	//! @param callerId the requesting player, taken from the controller the request arrived on
	//! @param action the power to run
	//! @param targetId the player the power acts on; unused by STAGE_ADVANCE
	//! @param ok true only when the power achieved what it set out to do
	//! @return the line to show the admin; a refusal on a client or for a non-admin
	//! @authority server
	static string Execute(int callerId, TBD_EAdminAction action, int targetId, out bool ok)
	{
		ok = false;

		// Authority only -- every power below mutates server-owned state (lives, bodies, the stage
		// machine). A client build reaching here would half-run them locally and desync.
		if (TBD_Authority.IsClient())
			return "TBD: admin actions execute on the server only.";

		if (!IsAdmin(callerId))
		{
			NoteDeniedAccess(callerId, string.Format("action '%1'", ActionName(action)));
			return "TBD: refused -- you are not a listed server admin.";
		}

		bool done = false;
		string message = "TBD: unknown admin action.";

		if (action == TBD_EAdminAction.RESPAWN)
			message = Respawn(callerId, targetId, done);
		else if (action == TBD_EAdminAction.DEPLOY)
			message = Deploy(callerId, targetId, done);
		else if (action == TBD_EAdminAction.STAGE_ADVANCE)
			message = AdvanceStage(callerId, "next", done);

		ok = done;
		return message;
	}

	//! `#tbd stage next` or `#tbd stage <NAME>`: the named stage form the menu's single button does
	//! not offer, behind the same gate as Execute.
	//! @param callerId the sender's player id
	//! @param arg `next` or a stage name
	//! @param ok true when the stage changed
	//! @return the line to show the admin
	//! @authority server
	static string ForceStage(int callerId, string arg, out bool ok)
	{
		ok = false;

		// Authority only -- the stage machine is server-owned; `m_Stage` replicates outward and a
		// client writing it would be overwritten on the next BumpMe anyway.
		if (TBD_Authority.IsClient())
			return "TBD: admin actions execute on the server only.";

		if (!IsAdmin(callerId))
		{
			NoteDeniedAccess(callerId, "action 'stage'");
			return "TBD: refused -- you are not a listed server admin.";
		}

		bool done = false;
		string message = AdvanceStage(callerId, arg, done);
		ok = done;
		return message;
	}

	//! Record that a non-admin touched an admin surface. The caller of a refusal path controls its
	//! rate, so three bounds apply:
	//!   1. TBD_AdminAudit.RecordUnauthorised holds these to its own few ring slots, so real admin
	//!      actions keep the rest of the ring.
	//!   2. The console gets the first attempt per (player, surface) and then only the 10th, 100th,
	//!      1000th and so on, each with the running count.
	//!   3. The count map is cleared when it grows past MAX_DENIED_KEYS.
	//! @param playerId the refused player
	//! @param surface what they touched, as it reads in the audit line
	//! @authority server
	static void NoteDeniedAccess(int playerId, string surface)
	{
		string text = string.Format("REFUSED %1 by %2 -- not on the server admin list", surface, Label(playerId));

		if (!s_mDeniedCount)
			s_mDeniedCount = new map<string, int>();

		if (s_mDeniedCount.Count() > MAX_DENIED_KEYS)
			s_mDeniedCount.Clear();

		string key = string.Format("%1|%2", playerId, surface);

		int seen = 0;
		s_mDeniedCount.Find(key, seen);
		seen++;
		s_mDeniedCount.Set(key, seen);

		if (seen == 1)
		{
			TBD_AdminAudit.RecordUnauthorised(text);
			return;
		}

		if (IsRepeatMilestone(seen))
			TBD_AdminAudit.Note(string.Format("%1 (x%2)", text, seen));
	}

	//! Whether a repeat count is an exact power of ten from 10 up. Divides down, so it terminates in
	//! log10(count) steps for every input and cannot overflow.
	//! @param count the refusal count for one (player, surface)
	//! @return true for 10, 100, 1000, ...
	protected static bool IsRepeatMilestone(int count)
	{
		if (count < 10)
			return false;

		int n = count;
		while (n % 10 == 0)
		{
			n = n / 10;
		}

		return n == 1;
	}

	//! Respawn a player whose one life is spent, and reword the reply while the TBD platform decides
	//! on their seat. Clears the player's last deploy result first, so the reading after the respawn
	//! is this attempt's; the audit line keeps the plain result name.
	//! @param callerId the admin
	//! @param targetId the player to respawn
	//! @param ok true when the player is back in the world
	//! @return the line to show the admin; AUTHORIZING and UNAUTHORIZED get their own sentences
	//! @authority server
	protected static string Respawn(int callerId, int targetId, out bool ok)
	{
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (spawn)
			spawn.ForgetDeployResult(targetId);

		string message = RespawnThroughSpawnManager(callerId, targetId, ok);
		if (!spawn)
			return message;

		TBD_EDeployResult result = spawn.LastDeployResult(targetId);
		if (result == TBD_EDeployResult.AUTHORIZING)
			return string.Format("TBD: respawn player=%1 -> AUTHORIZING - the TBD platform is deciding on their seat; they stay dead until it allows the new life.", targetId);

		if (result == TBD_EDeployResult.UNAUTHORIZED)
			return string.Format("TBD: respawn player=%1 -> UNAUTHORIZED - their seat cannot be authorized right now (see the audit trail); they stay dead.", targetId);

		return message;
	}

	//! The one sanctioned exception to ONE LIFE, for a player killed by the engine rather than the
	//! enemy. Runs TBD_SpawnManager.AdminRespawn, the only caller of the one-life override, and
	//! audits the result.
	//! @param callerId the admin
	//! @param targetId the player to respawn; non-positive means none selected
	//! @param ok true when the result is DEPLOYED
	//! @return the reply for DEPLOYED, a queued RETRY, or any other result
	//! @authority server
	protected static string RespawnThroughSpawnManager(int callerId, int targetId, out bool ok)
	{
		ok = false;

		if (targetId <= 0)
			return "TBD: no player selected.";

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
		{
			TBD_AdminAudit.Record(string.Format("%1 respawn %2 -> spawn manager not ready",
				Label(callerId), Label(targetId)), true);
			return "TBD: spawn manager not ready.";
		}

		TBD_EDeployResult result = spawn.AdminRespawn(targetId, Label(callerId));
		string outcome = typename.EnumToString(TBD_EDeployResult, result);
		ok = (result == TBD_EDeployResult.DEPLOYED);

		TBD_AdminAudit.Record(string.Format("%1 respawn %2 -> %3",
			Label(callerId), Label(targetId), outcome), !ok);

		if (ok)
			return string.Format("TBD: respawn player=%1 -> %2 -- back in the world, life restored.", targetId, outcome);

		if (result == TBD_EDeployResult.RETRY)
			return string.Format("TBD: respawn player=%1 -> RETRY queued -- they stay dead until a body lands.", targetId);

		return string.Format("TBD: respawn player=%1 -> %2 -- they are STILL dead, run it again.", targetId, outcome);
	}

	//! Deploy a live player who has no body, and reword the reply while the TBD platform decides on
	//! their seat. Clears the player's last deploy result first, so the reading after the deploy is
	//! this attempt's; the audit line keeps the plain result name.
	//! @param callerId the admin
	//! @param targetId the player to deploy
	//! @param ok true when the player is in the world
	//! @return the line to show the admin; AUTHORIZING and UNAUTHORIZED get their own sentences
	//! @authority server
	protected static string Deploy(int callerId, int targetId, out bool ok)
	{
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (spawn)
			spawn.ForgetDeployResult(targetId);

		string message = DeployThroughSpawnManager(callerId, targetId, ok);
		if (!spawn)
			return message;

		TBD_EDeployResult result = spawn.LastDeployResult(targetId);
		if (result == TBD_EDeployResult.AUTHORIZING)
			return string.Format("TBD: deploy player=%1 -> AUTHORIZING - the TBD platform is deciding on their seat; they deploy once it allows it.", targetId);

		if (result == TBD_EDeployResult.UNAUTHORIZED)
			return string.Format("TBD: deploy player=%1 -> UNAUTHORIZED - their seat cannot be authorized right now (see the audit trail); they were told why and keep the seat.", targetId);

		return message;
	}

	//! Put a player who still has their life but no body into the world. Refused for a dead player:
	//! AdminRespawn refuses the living and DeployPlayerEx refuses the dead, so the two are separate
	//! actions. Runs TBD_SpawnManager.DeployPlayerEx and audits the result.
	//! @param callerId the admin
	//! @param targetId the player to deploy; non-positive means none selected
	//! @param ok true when the result is DEPLOYED
	//! @return the reply for the result, or the refusal for a spent life
	//! @authority server
	protected static string DeployThroughSpawnManager(int callerId, int targetId, out bool ok)
	{
		ok = false;

		if (targetId <= 0)
			return "TBD: no player selected.";

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
		{
			TBD_AdminAudit.Record(string.Format("%1 deploy %2 -> spawn manager not ready",
				Label(callerId), Label(targetId)), true);
			return "TBD: spawn manager not ready.";
		}

		if (spawn.IsPlayerDead(targetId))
		{
			TBD_AdminAudit.Record(string.Format("%1 deploy %2 -> REFUSED, life already spent",
				Label(callerId), Label(targetId)), true);
			return string.Format("TBD: deploy player=%1 refused -- their life is spent. Use Respawn.", targetId);
		}

		TBD_EDeployResult result = spawn.DeployPlayerEx(targetId);
		string outcome = typename.EnumToString(TBD_EDeployResult, result);
		ok = (result == TBD_EDeployResult.DEPLOYED);

		TBD_AdminAudit.Record(string.Format("%1 deploy %2 -> %3",
			Label(callerId), Label(targetId), outcome), !ok);

		if (ok)
			return string.Format("TBD: deploy player=%1 -> %2.", targetId, outcome);

		return string.Format("TBD: deploy player=%1 -> %2 -- not in the world.", targetId, outcome);
	}

	//! Force the round forward, the recovery for a round that cannot advance on its own (a mission the
	//! validator rejected never leaves LOADING). Drives TBD_FrameworkManager.HandleAdminStageCommand
	//! and compares the stage either side of it to detect a refusal.
	//! @param callerId the admin
	//! @param arg `next` (also empty) or a stage name, upper-cased before use
	//! @param ok true when the stage changed
	//! @return the transition, or why the stage is unchanged
	//! @authority server
	protected static string AdvanceStage(int callerId, string arg, out bool ok)
	{
		ok = false;

		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (!framework)
		{
			TBD_AdminAudit.Record(string.Format("%1 stage '%2' -> framework not ready", Label(callerId), arg), true);
			return "TBD: framework not ready.";
		}

		string request = arg;
		if (request.IsEmpty())
			request = "next";

		// `ToUpper()` mutates IN PLACE and returns a COUNT (an engine behaviour) -- so this is two
		// statements, and `next` is compared before the uppercase so `#tbd stage next` still works.
		if (request != "next")
			request.ToUpper();

		TBD_EGameStage before = framework.GetStage();
		framework.HandleAdminStageCommand(request);
		TBD_EGameStage after = framework.GetStage();

		string fromName = typename.EnumToString(TBD_EGameStage, before);
		string toName = typename.EnumToString(TBD_EGameStage, after);

		if (before == after)
		{
			TBD_AdminAudit.Record(string.Format("%1 stage '%2' -> REFUSED, still %3",
				Label(callerId), request, fromName), true);

			// A transition can be refused for a reason (SAFE_START with no enforcement behind it) as
			// well as for being unparseable; the reason goes to the admin when there is one.
			string why = framework.GetLastStageRefusal();
			if (!why.IsEmpty())
				return string.Format("TBD: stage unchanged (%1). %2", fromName, why);

			return string.Format("TBD: stage unchanged (%1). '%2' is not a stage, or the round is already at the last one.",
				fromName, request);
		}

		ok = true;
		TBD_AdminAudit.Record(string.Format("%1 forced stage %2 -> %3", Label(callerId), fromName, toName), false);
		return string.Format("TBD: stage %1 -> %2 (forced).", fromName, toName);
	}
}
