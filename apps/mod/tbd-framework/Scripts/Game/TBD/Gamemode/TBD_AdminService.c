//! T-181.11.2 — the admin AUTHORITY. One server-side choke point for every power an admin
//! surface can trigger, and the only place those powers are reachable from.
//!
//! ── Why this class exists at all ────────────────────────────────────────────────────────────
//! The admin backend already worked through chat (`TBD_AdminCommands`). The menu is a **second
//! front-end onto the same operations**, not a second implementation of them — so the operations
//! moved here, once, and both surfaces call in. Two consequences that matter more than the tidiness:
//!
//!  1. **One permission gate.** `Execute` and `ForceStage` refuse before they do anything unless
//!     `IsAdmin(callerId)` says yes, resolved from `SCR_PlayerListedAdminManagerComponent` — the
//!     same vanilla admin list the chat path has always used. A future third surface cannot reach
//!     a power without passing it, because there is no other public function here that touches
//!     `TBD_SpawnManager` or `TBD_FrameworkManager`.
//!  2. **One audit trail.** Every attempt — allowed or refused — lands in `TBD_AdminAudit`, so the
//!     chat fallback and the screen write the same history instead of two partial ones.
//!
//! ── Trust boundary ──────────────────────────────────────────────────────────────────────────
//! `callerId` is **never** taken from the wire. The RPC entry points in `TBD_MissionBrowser.c`
//! pass `GetPlayerId()` of the replicated player controller the RPC arrived on, which the client
//! cannot forge. There is no API here that accepts "I am an admin" as an argument, and no
//! client-side flag anywhere is consulted — the screen's own `m_bAuthorised` is a rendering hint
//! derived FROM the server's answer, never an input to it.

//! The powers the admin surfaces expose. `NONE` is 0 so an unset int is inert.
enum TBD_EAdminAction
{
	NONE,
	//! One-life escape hatch — put a player who has SPENT their life back in the world.
	RESPAWN,
	//! Recovery for a player who still has their life but never got a body (stuck on loading).
	DEPLOY,
	//! Force the round's stage machine one step forward.
	STAGE_ADVANCE
}

//! @authority server — every function that mutates state refuses outright off the authority.
class TBD_AdminService
{
	//! "<playerId>|<surface>" -> a refusal has already taken a slot in the bounded audit ring for
	//! this pair. See NoteDeniedAccess.
	protected static ref map<string, bool> s_mDeniedSeen;

	//------------------------------------------------------------------------------------------------
	//! THE permission question. Server-side, resolved from the vanilla listed-admin manager.
	//!
	//! Fails closed on every uncertainty: a non-positive id (no such player), a missing manager
	//! (nobody is an admin yet) and a player absent from the list all return false.
	static bool IsAdmin(int playerId)
	{
		if (playerId <= 0)
			return false;

		SCR_PlayerListedAdminManagerComponent admins = SCR_PlayerListedAdminManagerComponent.GetInstance();
		if (!admins)
			return false;

		return admins.IsPlayerOnAdminList(playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! `Hicks(3)` — how a player appears in the audit trail. Name AND id, because a name can be
	//! shared (see the identity note in TBD_MOD_DESIGN.md §2) and an id cannot be read back later.
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

	//------------------------------------------------------------------------------------------------
	//! Turn an int that arrived over the wire into an action, or `NONE`.
	//!
	//! Enfusion will happily assign any int to an enum-typed variable, so without this a client
	//! could hand the switch a value no case covers. Everything downstream already fails closed on
	//! an unrecognised action, but an admin surface should reject a malformed request at the wire
	//! rather than rely on a default branch several frames later still being a refusal.
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

	//------------------------------------------------------------------------------------------------
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

	// ── The gate ────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Run one admin power on behalf of `callerId`. Returns the line to show the admin; `ok` is
	//! true only when the operation actually achieved what it set out to do.
	//!
	//! @authority server — refuses on a client build outright, so a modified client that somehow
	//! reached this function bounces off it instead of half-running it locally.
	static string Execute(int callerId, TBD_EAdminAction action, int targetId, out bool ok)
	{
		ok = false;

		// Authority only — every power below mutates server-owned state (lives, bodies, the stage
		// machine). A client build reaching here would half-run them locally and desync.
		if (RplSession.Mode() == RplMode.Client)
			return "TBD: admin actions execute on the server only.";

		// ── THE PERMISSION GATE. Nothing below runs without passing it. ──
		if (!IsAdmin(callerId))
		{
			NoteDeniedAccess(callerId, string.Format("action '%1'", ActionName(action)));
			return "TBD: refused — you are not a listed server admin.";
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

	//------------------------------------------------------------------------------------------------
	//! `#tbd stage next` / `#tbd stage LOBBY`. Public because chat needs the named form, which the
	//! menu's single STAGE_ADVANCE button does not expose; gated identically, so this is a second
	//! door into the same locked room rather than a way around the lock.
	//! @authority server
	static string ForceStage(int callerId, string arg, out bool ok)
	{
		ok = false;

		// Authority only — the stage machine is server-owned; `m_Stage` replicates outward and a
		// client writing it would be overwritten on the next BumpMe anyway.
		if (RplSession.Mode() == RplMode.Client)
			return "TBD: admin actions execute on the server only.";

		if (!IsAdmin(callerId))
		{
			NoteDeniedAccess(callerId, "action 'stage'");
			return "TBD: refused — you are not a listed server admin.";
		}

		bool done = false;
		string message = AdvanceStage(callerId, arg, done);
		ok = done;
		return message;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.17 — `#tbd safestart [status|go|<seconds>]`. Public for the same reason `ForceStage`
	//! is: chat needs an argument form the menu's single-button actions cannot express. Gated and
	//! audited identically, so it is another door into the same locked room, not a way around it.
	//!
	//! `status` is deliberately readable by any admin without changing anything — during a live
	//! event "is damage actually off right now" is a question that has to be answerable in one
	//! command, not inferred from the server console.
	//! @authority server
	static string Safestart(int callerId, string arg, out bool ok)
	{
		ok = false;

		// Authority only — the countdown and every damage mutation are server-owned; a client
		// build reaching here would half-run them locally and protect nobody.
		if (RplSession.Mode() == RplMode.Client)
			return "TBD: admin actions execute on the server only.";

		if (!IsAdmin(callerId))
		{
			NoteDeniedAccess(callerId, "action 'safestart'");
			return "TBD: refused — you are not a listed server admin.";
		}

		TBD_SafestartManager safestart = TBD_SafestartManager.GetInstance();
		if (!safestart)
			return "TBD: safestart manager not on this game mode — SAFE_START cannot be enforced here.";

		string request = arg;
		if (request.IsEmpty())
			request = "status";

		if (request == "status")
		{
			ok = true;
			return safestart.StatusLine();
		}

		if (request == "go")
		{
			if (!safestart.IsArmed())
				return "TBD: safestart is not running — nothing to end.";

			safestart.GoLive(string.Format("admin %1", Label(callerId)));
			ok = true;
			TBD_AdminAudit.Record(string.Format("%1 ended safestart early", Label(callerId)), false);
			return "TBD: safestart ended — weapons live.";
		}

		int seconds = request.ToInt();
		if (seconds <= 0)
			return "Usage: #tbd safestart [status|go|<seconds>]";

		bool applied = false;
		string reply = safestart.AdminSetSeconds(seconds, applied);
		ok = applied;
		TBD_AdminAudit.Record(string.Format("%1 set safestart length to %2s -> %3",
			Label(callerId), seconds, applied), !applied);
		return reply;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.32 — `#tbd identity [status|override <phrase>|enforce]`.
	//!
	//! ONE LIFE is a promise about a PERSON, and a dedicated server with no backend identity has no
	//! concept of a person — it hands out `player:<id>`, a lease on a NUMBER. `TBD_SpawnManager`
	//! therefore refuses SAFE_START/LIVE on such a host. That is right for an event and wrong for a
	//! legitimate test session, so this is the documented way out.
	//!
	//! ── Why the override is shaped like this ────────────────────────────────────────────────────
	//! It is a WAIVER, not a setting, and everything about the shape follows from that:
	//!   * `status` changes nothing and is readable by any admin — during an event "can this host
	//!     even enforce one life" has to be answerable in one command, exactly like `#tbd safestart
	//!     status`.
	//!   * `override` demands an exact phrase (`TBD_SpawnManager.IDENTITY_OVERRIDE_PHRASE`) as a
	//!     separate argument. A yes/no flag, or a `--force`, is something an admin can fat-finger
	//!     while trying to do something else; a literal sentence naming the consequence is not.
	//!   * Both outcomes — signed AND refused — hit `TBD_AdminAudit`. A waiver nobody can prove was
	//!     signed is not a waiver, and a refused attempt is exactly the thing a post-event dispute
	//!     needs to be able to see.
	//!   * `enforce` re-arms it, with no phrase required. Putting a safety rail BACK never needs
	//!     ceremony.
	//! The waiver only unblocks the STAGE GATE. It does not touch ONE LIFE itself, the one-life
	//! boundary in `DeployPlayerInternal`, or the mode-3 mark handling in `OnPlayerDisconnected` —
	//! it buys permission to start a round that the host cannot enforce, and says so every time it
	//! lets a stage through.
	//! @authority server
	static string Identity(int callerId, string arg, string confirm, out bool ok)
	{
		ok = false;

		// Authority only — the waiver and the census are server-owned, and off the authority
		// vanilla's GetPlayerIdentityId returns NULL_UUID for everybody, so a client build would
		// read a census that is pure noise.
		if (RplSession.Mode() == RplMode.Client)
			return "TBD: admin actions execute on the server only.";

		if (!IsAdmin(callerId))
		{
			NoteDeniedAccess(callerId, "action 'identity'");
			return "TBD: refused — you are not a listed server admin.";
		}

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return "TBD: spawn manager not on this game mode — ONE LIFE is not enforced here at all (see the roll-call).";

		string request = arg;
		if (request.IsEmpty())
			request = "status";

		if (request == "status")
		{
			ok = true;
			return spawn.IdentityStatusLine();
		}

		if (request == "enforce")
		{
			spawn.RequireDurableIdentity(Label(callerId));
			ok = true;
			TBD_AdminAudit.Record(string.Format("%1 re-armed ONE LIFE identity enforcement", Label(callerId)), false);
			return "TBD: identity enforcement re-armed — SAFE_START/LIVE are refused again while any connected player is on a NUMERIC key.";
		}

		if (request == "override")
		{
			if (!spawn.AcceptNonDurableIdentity(Label(callerId), confirm))
			{
				TBD_AdminAudit.Record(string.Format("%1 identity override REFUSED — wrong or missing confirmation phrase",
					Label(callerId)), true);
				return string.Format("TBD: refused. This waives ONE LIFE on a host that cannot enforce it, so it needs the phrase verbatim: '#tbd identity override %1'.",
					TBD_SpawnManager.IDENTITY_OVERRIDE_PHRASE);
			}

			ok = true;
			TBD_AdminAudit.Record(string.Format("%1 WAIVED ONE LIFE enforcement (no durable player identity on this host)",
				Label(callerId)), false);
			return "TBD: ONE LIFE enforcement WAIVED. SAFE_START/LIVE may now be entered, deaths will NOT survive a reconnect, and every stage this lets through says so in the log. '#tbd identity enforce' undoes it.";
		}

		return string.Format("Usage: #tbd identity [status|override %1|enforce]", TBD_SpawnManager.IDENTITY_OVERRIDE_PHRASE);
	}

	//------------------------------------------------------------------------------------------------
	//! Someone who is not an admin touched an admin surface. Records it — once per player per
	//! surface in the bounded on-screen trail, every single time in the console.
	//!
	//! The split exists because the ring is small and the refusal paths are the ones an attacker
	//! controls: an unauthorised client can poll the snapshot every 3 s or macro `#tbd` forever,
	//! and if each attempt took a ring slot it could flush the real actions out of the admin's
	//! view. The console keeps the complete record; the screen keeps the first of each kind.
	//! @authority server
	static void NoteDeniedAccess(int playerId, string surface)
	{
		string text = string.Format("REFUSED %1 by %2 — not on the server admin list", surface, Label(playerId));

		if (!s_mDeniedSeen)
			s_mDeniedSeen = new map<string, bool>();

		string key = string.Format("%1|%2", playerId, surface);
		if (s_mDeniedSeen.Contains(key))
		{
			TBD_AdminAudit.Note(text);
			return;
		}

		s_mDeniedSeen.Set(key, true);
		TBD_AdminAudit.Record(text, true);
	}

	// ── The powers. Protected: the gate above is the only way in. ───────────────────────────

	//------------------------------------------------------------------------------------------------
	//! The headline action, and the reason this whole screen exists.
	//!
	//! Under ONE LIFE a death is terminal by design (TBD_MOD_DESIGN.md §2). This is the single
	//! sanctioned exception, for a player killed by the engine rather than by the enemy. It does
	//! NOT invent a new spawn path: `TBD_SpawnManager.AdminRespawn` is the same authority-side
	//! function `#tbd respawn` has always called, and it is the only caller allowed to pass the
	//! one-life override (which lives on a `protected` overload precisely so nothing else can).
	protected static string Respawn(int callerId, int targetId, out bool ok)
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
			return string.Format("TBD: respawn player=%1 -> %2 — back in the world, life restored.", targetId, outcome);

		if (result == TBD_EDeployResult.RETRY)
			return string.Format("TBD: respawn player=%1 -> RETRY queued — they stay dead until a body lands.", targetId);

		return string.Format("TBD: respawn player=%1 -> %2 — they are STILL dead, run it again.", targetId, outcome);
	}

	//------------------------------------------------------------------------------------------------
	//! The other half of "spawn shit if it breaks": a player who still HAS their life but never
	//! got a body — stuck on the loading screen because a deploy failed or was never requested.
	//!
	//! Deliberately a different action from Respawn, and deliberately refused for a dead player.
	//! `AdminRespawn` refuses anyone who is not dead, and `DeployPlayerEx` refuses anyone who is
	//! (it carries no override) — so mapping one button onto both would silently do nothing half
	//! the time. Two honest actions beat one that lies about what it can do.
	protected static string Deploy(int callerId, int targetId, out bool ok)
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
			return string.Format("TBD: deploy player=%1 refused — their life is spent. Use Respawn.", targetId);
		}

		TBD_EDeployResult result = spawn.DeployPlayerEx(targetId);
		string outcome = typename.EnumToString(TBD_EDeployResult, result);
		ok = (result == TBD_EDeployResult.DEPLOYED);

		TBD_AdminAudit.Record(string.Format("%1 deploy %2 -> %3",
			Label(callerId), Label(targetId), outcome), !ok);

		if (ok)
			return string.Format("TBD: deploy player=%1 -> %2.", targetId, outcome);

		return string.Format("TBD: deploy player=%1 -> %2 — not in the world.", targetId, outcome);
	}

	//------------------------------------------------------------------------------------------------
	//! Force the round forward. Exposed because the stage machine is exactly the thing that can
	//! strand an event: a mission rejected by the validator never leaves LOADING, and nothing
	//! in-game says so. The admin is the only recovery, so the recovery has to be reachable.
	//!
	//! Drives `TBD_FrameworkManager.HandleAdminStageCommand` — the existing authority-side entry
	//! point — and reads the stage either side of it rather than duplicating the transition rules,
	//! which is also how "it refused" is detected without that function reporting anything.
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

		// `ToUpper()` mutates IN PLACE and returns a COUNT (measured landmine) — so this is two
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

			// T-181.17 — a transition can now be refused for a REASON rather than only for being
			// unparseable (SAFE_START with no enforcement behind it). Carry that reason to the
			// admin; "not a stage" would send them hunting a typo that is not there.
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
