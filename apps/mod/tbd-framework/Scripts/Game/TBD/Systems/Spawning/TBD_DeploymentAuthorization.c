//! Platform authorization of deployments into event seats, and the player lives it opens.
//!
//! When the running mission is deployed for a platform event (the deployment names an `event_id`,
//! TBD_DeployedMission), TBD_SpawnManager asks `Check` before it puts a player into a slot. Which slots are event seats comes from the event roster's
//! slot table (TBD_RosterLoader), and until that table has loaded EVERY deployment is refused: the
//! gate fails closed rather than let anyone into a seat the platform has not ruled on. Once the
//! table has loaded, a slot it does not list is not an event seat and deploys without asking; a
//! slot it lists goes to the platform with a fresh `player_life_id` per spawn attempt
//! (TBD_DeploymentRequestQueue), and the deploy waits until the platform decides:
//!   * allowed - the life is remembered with its `occupancy_id` and the spawn manager continues the
//!     deploy (`TBD_SpawnManager.OnDeploymentAuthorized`);
//!   * denied - the player is told the platform's reason and the spawn manager gives the seat back,
//!     returning them to slot selection (`TBD_SpawnManager.OnDeploymentRefused`). A denial for
//!     `SLOT_NOT_IN_LOADED_MISSION` is the exception: the platform does not run this server's
//!     mission for the event, which is the server's problem, so the player keeps the seat.
//! A deployment that cannot be authorized at all (no slot table, no runtime session, no game
//! identity, a request the platform rejects) is refused with the reason and the seat kept, so the
//! player deploys again once the cause is fixed. Every refusal reaches the admin audit trail.
//! When a life ends - death, disconnect, a seat change, the round or the world ending - its end is
//! reported through TBD_DeploymentEndQueue.
//!
//! A deployment without an event never asks.
//! @authority server

//! TBD_DeploymentAuthorization.Check's answer to the spawn manager.
enum TBD_EDeploymentGate
{
	PROCEED, //!< Deploy now: no authorization applies, or this player's open life covers this slot.
	WAIT,    //!< The platform is deciding; its decision continues or refuses the deploy.
	REFUSED, //!< The deployment cannot be authorized now; the player has been told why.
}

//! A life the platform allowed: one player in one slot, from the allowed decision until it ends.
class TBD_OpenLife
{
	int m_iPlayerId;
	int m_iConnectionEpoch;
	string m_sArmaId;
	string m_sSlotUid;
	string m_sOrbatSlotId;
	string m_sPlayerLifeId;
	string m_sOccupancyId;
	//! The runtime session the life was opened in; its end is reported there.
	string m_sRuntimeSessionId;
}

class TBD_DeploymentAuthorization
{
	//! Greppable channel: `grep '\[TBD\]\[Deployment\]' console.log`.
	protected static const string CH_DEPLOYMENT = "Deployment";

	//! Every private message the player sees starts with this, like the other TBD replies.
	protected static const string TAG = "TBD: ";

	protected static const string ROSTER_NOT_LOADED = "the event roster has not loaded on this server yet, so event seats cannot be authorized - try again shortly.";
	protected static const string NOT_RUNNING_THAT_MISSION = "this server is not running the mission this seat belongs to on the platform, so the seat cannot be authorized - tell an admin. The seat stays yours.";

	//! Bound of s_mAuditedRefusals; the set starts over when full.
	protected static const int AUDITED_REFUSALS_MAX = 256;

	//! playerId -> the life the platform allowed that player.
	protected static ref map<int, ref TBD_OpenLife> s_mLives;
	//! Monotonic per process. Only the process that started a runtime session addresses it, so every
	//! player_life_id is unique within its session.
	protected static int s_iLifeCounter;
	//! Slot keys already reported as missing from the roster.
	protected static ref map<string, bool> s_mUnlistedSlotsReported;
	//! Refusals already in the admin audit trail this round, see `AuditRefusal`.
	protected static ref map<string, bool> s_mAuditedRefusals;

	//------------------------------------------------------------------------------------------------
	// THE GATE
	//------------------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! True when the running mission is deployed for a platform event, so its event seats need
	//! authorization.
	static bool AppliesToThisMission()
	{
		return !TBD_DeployedMission.GetEventId().IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	//! May this player deploy into `slot` now? `connectionEpoch` is TBD_SpawnManager's stamp of the
	//! player's current connection; a decision is honoured only for that same connection.
	static TBD_EDeploymentGate Check(int playerId, int connectionEpoch, notnull TBD_MissionSlotStruct slot)
	{
		if (!AppliesToThisMission())
			return TBD_EDeploymentGate.PROCEED;

		if (!TBD_RosterLoader.IsSlotTableLoaded())
		{
			Refuse(playerId, slot, ROSTER_NOT_LOADED);
			return TBD_EDeploymentGate.REFUSED;
		}

		string armaId = TBD_PlayerIdentity.GetArmaId(playerId);
		string eventMissionId;
		string orbatSlotId;
		if (!TBD_RosterLoader.ResolveSlotBinding(slot.uid, armaId, eventMissionId, orbatSlotId))
		{
			ReportUnlistedSlot(slot);
			return TBD_EDeploymentGate.PROCEED;
		}

		TBD_OpenLife life = FindLife(playerId);
		if (life)
		{
			if (life.m_iConnectionEpoch == connectionEpoch && life.m_sArmaId == armaId && life.m_sOrbatSlotId == orbatSlotId)
				return TBD_EDeploymentGate.PROCEED;

			// A life in another seat, or of an earlier connection on this number, ends before the
			// next one is asked for.
			EndLife(playerId, "replaced by a deployment into another seat");
		}

		if (armaId.IsEmpty())
		{
			Refuse(playerId, slot, "this server could not read your game identity, so the platform cannot authorize your seat. Tell an admin.");
			return TBD_EDeploymentGate.REFUSED;
		}

		if (!TBD_RuntimeSession.CanHoldSession())
		{
			Refuse(playerId, slot, "this server holds no platform runtime session, so event seats cannot be authorized. Tell an admin.");
			return TBD_EDeploymentGate.REFUSED;
		}

		TBD_DeploymentRequest waiting = TBD_DeploymentRequestQueue.FindWaitingFor(playerId);
		if (waiting)
		{
			if (waiting.m_iConnectionEpoch == connectionEpoch && waiting.m_sArmaId == armaId && waiting.m_sOrbatSlotId == orbatSlotId)
				return TBD_EDeploymentGate.WAIT;

			TBD_DeploymentRequestQueue.Abandon(waiting);
		}

		TBD_DeploymentRequest request = new TBD_DeploymentRequest();
		request.m_iPlayerId = playerId;
		request.m_iConnectionEpoch = connectionEpoch;
		request.m_sArmaId = armaId;
		request.m_sSlotUid = slot.uid;
		request.m_sEventMissionId = eventMissionId;
		request.m_sOrbatSlotId = orbatSlotId;
		request.m_sPlayerLifeId = NewPlayerLifeId(playerId);

		TBD_Log.Kv(CH_DEPLOYMENT, "authorization-requested", string.Format("player=%1 slot=%2 orbatSlot=%3 eventMission=%4 life=%5",
			playerId, slot.uid, orbatSlotId, eventMissionId, request.m_sPlayerLifeId));

		TBD_DeploymentRequestQueue.Enqueue(request);
		return TBD_EDeploymentGate.WAIT;
	}

	//------------------------------------------------------------------------------------------------
	// DECISIONS (TBD_DeploymentRequestQueue)
	//------------------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! The platform allowed `request` and opened the life `occupancyId` in the request's session.
	static void OnAllowed(notnull TBD_DeploymentRequest request, string occupancyId, string authorizedBy)
	{
		if (request.m_bAbandoned || !IsCurrentConnection(request))
		{
			TBD_Log.Kv(CH_DEPLOYMENT, "allowed-unclaimed", string.Format("player=%1 life=%2 occupancy=%3 - nobody waits for this life any more; ending it",
				request.m_iPlayerId, request.m_sPlayerLifeId, occupancyId));
			TBD_DeploymentEndQueue.Enqueue(request.m_sRuntimeSessionId, occupancyId, request.m_sArmaId, request.m_sOrbatSlotId,
				"allowed after nobody waited for it");
			return;
		}

		if (FindLife(request.m_iPlayerId))
			EndLife(request.m_iPlayerId, "replaced by a newer allowed life");

		TBD_OpenLife life = new TBD_OpenLife();
		life.m_iPlayerId = request.m_iPlayerId;
		life.m_iConnectionEpoch = request.m_iConnectionEpoch;
		life.m_sArmaId = request.m_sArmaId;
		life.m_sSlotUid = request.m_sSlotUid;
		life.m_sOrbatSlotId = request.m_sOrbatSlotId;
		life.m_sPlayerLifeId = request.m_sPlayerLifeId;
		life.m_sOccupancyId = occupancyId;
		life.m_sRuntimeSessionId = request.m_sRuntimeSessionId;

		if (!s_mLives)
			s_mLives = new map<int, ref TBD_OpenLife>();

		s_mLives.Set(life.m_iPlayerId, life);

		TBD_Log.Kv(CH_DEPLOYMENT, "deployment-allowed", string.Format("player=%1 slot=%2 occupancy=%3 authorizedBy=%4 life=%5",
			life.m_iPlayerId, life.m_sSlotUid, occupancyId, authorizedBy, life.m_sPlayerLifeId));

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (spawn)
			spawn.OnDeploymentAuthorized(life.m_iPlayerId, life.m_iConnectionEpoch);
	}

	//------------------------------------------------------------------------------------------------
	//! The platform denied `request`. Returns true when the request is to be asked again: the
	//! platform still holds an open life of this player that this server does not track (its end
	//! report was lost or dropped), and that life is ended first.
	static bool OnDenied(notnull TBD_DeploymentRequest request, string reason, string message, string blockingOccupancyId)
	{
		if (request.m_bAbandoned)
		{
			TBD_Log.Kv(CH_DEPLOYMENT, "denied-unclaimed", string.Format("player=%1 life=%2 reason=%3",
				request.m_iPlayerId, request.m_sPlayerLifeId, reason));
			return false;
		}

		if (reason == "PLAYER_ALREADY_DEPLOYED" && !blockingOccupancyId.IsEmpty() && !request.m_bUntrackedLifeEnded
			&& !IsTrackedOccupancy(blockingOccupancyId))
		{
			request.m_bUntrackedLifeEnded = true;
			TBD_Log.Warn(CH_DEPLOYMENT, string.Format("player=%1 still holds occupancy=%2 on the platform, which this server does not track - ending it, then asking again with life %3",
				request.m_iPlayerId, blockingOccupancyId, request.m_sPlayerLifeId));
			TBD_DeploymentEndQueue.Enqueue(request.m_sRuntimeSessionId, blockingOccupancyId, request.m_sArmaId, string.Empty,
				"untracked life blocked a new deployment");
			return true;
		}

		if (!IsCurrentConnection(request))
			return false;

		TBD_Log.Warn(CH_DEPLOYMENT, string.Format("deployment-denied player=%1 slot=%2 reason=%3 message='%4' life=%5",
			request.m_iPlayerId, request.m_sSlotUid, reason, message, request.m_sPlayerLifeId));

		// The platform does not run this server's mission for the event: the server's fault, not the
		// player's, so the seat stays theirs.
		if (reason == "SLOT_NOT_IN_LOADED_MISSION")
		{
			AuditServerRefusal(NOT_RUNNING_THAT_MISSION, request.m_iPlayerId, request.m_sSlotUid);
			TBD_PlayerChat.Tell(request.m_iPlayerId, TAG + "deployment refused - " + NOT_RUNNING_THAT_MISSION);

			TBD_SpawnManager seatKeeper = TBD_SpawnManager.GetInstance();
			if (seatKeeper)
				seatKeeper.OnDeploymentRefused(request.m_iPlayerId, request.m_iConnectionEpoch, false);

			return false;
		}

		// One trail entry per player, seat and reason: a player retrying the same seat adds nothing.
		AuditRefusal(string.Format("denied:%1:%2:%3", request.m_sArmaId, request.m_sSlotUid, reason),
			string.Format("DEPLOYMENT: %1 seat %2 denied by the TBD platform - %3: %4", PlayerLabel(request.m_iPlayerId), request.m_sSlotUid, reason, message));

		TBD_PlayerChat.Tell(request.m_iPlayerId, TAG + "deployment refused - " + message + ".");
		TBD_PlayerChat.Tell(request.m_iPlayerId, HintFor(reason));

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (spawn)
			spawn.OnDeploymentRefused(request.m_iPlayerId, request.m_iConnectionEpoch, true);

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! The platform will not decide `request` (it rejected the request itself, or this server holds
	//! no session): the waiting player is refused with `sentence` and keeps the seat.
	static void OnUndecided(notnull TBD_DeploymentRequest request, string sentence)
	{
		if (request.m_bAbandoned || !IsCurrentConnection(request))
			return;

		TBD_Log.Warn(CH_DEPLOYMENT, string.Format("deployment-undecided player=%1 slot=%2 life=%3 - %4",
			request.m_iPlayerId, request.m_sSlotUid, request.m_sPlayerLifeId, sentence));

		AuditServerRefusal(sentence, request.m_iPlayerId, request.m_sSlotUid);
		TBD_PlayerChat.Tell(request.m_iPlayerId, TAG + "deployment refused - " + sentence);

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (spawn)
			spawn.OnDeploymentRefused(request.m_iPlayerId, request.m_iConnectionEpoch, false);
	}

	//------------------------------------------------------------------------------------------------
	// LIVES
	//------------------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! The player's life is over (death, disconnect, a seat given back): its end is reported, and a
	//! request the player still waits on is abandoned.
	static void EndLife(int playerId, string reason)
	{
		TBD_DeploymentRequestQueue.AbandonFor(playerId);
		CloseLife(playerId, reason);
	}

	//------------------------------------------------------------------------------------------------
	//! The player took another seat: a life or a waiting request in the previous seat ends, while one
	//! for the new seat, asked for as the claim deployed the player, stays.
	static void OnSeatChanged(int playerId, string previousSlotUid)
	{
		TBD_DeploymentRequest waiting = TBD_DeploymentRequestQueue.FindWaitingFor(playerId);
		if (waiting && waiting.m_sSlotUid == previousSlotUid)
			TBD_DeploymentRequestQueue.Abandon(waiting);

		TBD_OpenLife life = FindLife(playerId);
		if (life && life.m_sSlotUid == previousSlotUid)
			CloseLife(playerId, "changed seat");
	}

	//------------------------------------------------------------------------------------------------
	//! Forget the player's open life and report its end.
	protected static void CloseLife(int playerId, string reason)
	{
		TBD_OpenLife life = FindLife(playerId);
		if (!life)
			return;

		s_mLives.Remove(playerId);

		TBD_Log.Kv(CH_DEPLOYMENT, "life-ended", string.Format("player=%1 slot=%2 occupancy=%3 reason='%4'",
			playerId, life.m_sSlotUid, life.m_sOccupancyId, reason));

		TBD_DeploymentEndQueue.Enqueue(life.m_sRuntimeSessionId, life.m_sOccupancyId, life.m_sArmaId, life.m_sOrbatSlotId, reason);
	}

	//------------------------------------------------------------------------------------------------
	//! The round or the world is over: every life ends, no request is waited on any more, and the
	//! next round's refusals reach the audit trail afresh.
	static void EndAllLives(string reason)
	{
		TBD_DeploymentRequestQueue.AbandonAll();
		s_mAuditedRefusals = null;

		if (!s_mLives || s_mLives.Count() == 0)
			return;

		array<int> players = {};
		for (int i = 0; i < s_mLives.Count(); i++)
			players.Insert(s_mLives.GetKey(i));

		foreach (int playerId : players)
			EndLife(playerId, reason);
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_OpenLife FindLife(int playerId)
	{
		if (!s_mLives)
			return null;

		TBD_OpenLife life;
		s_mLives.Find(playerId, life);
		return life;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool IsTrackedOccupancy(string occupancyId)
	{
		if (!s_mLives)
			return false;

		for (int i = 0; i < s_mLives.Count(); i++)
		{
			TBD_OpenLife life = s_mLives.GetElement(i);
			if (life && life.m_sOccupancyId == occupancyId)
				return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Is the player the request was made for still on the connection it was made on? A recycled
	//! player id is a different person.
	protected static bool IsCurrentConnection(notnull TBD_DeploymentRequest request)
	{
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return false;

		return spawn.IsConnectionCurrent(request.m_iPlayerId, request.m_iConnectionEpoch);
	}

	//------------------------------------------------------------------------------------------------
	protected static string NewPlayerLifeId(int playerId)
	{
		s_iLifeCounter++;
		return string.Format("life-%1-player-%2", s_iLifeCounter, playerId);
	}

	//------------------------------------------------------------------------------------------------
	// MESSAGES AND THE AUDIT TRAIL
	//------------------------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Say once per slot that the loaded roster does not list it: it is not an event seat.
	protected static void ReportUnlistedSlot(notnull TBD_MissionSlotStruct slot)
	{
		if (!s_mUnlistedSlotsReported)
			s_mUnlistedSlotsReported = new map<string, bool>();

		string key = slot.Key();
		if (s_mUnlistedSlotsReported.Contains(key))
			return;

		s_mUnlistedSlotsReported.Set(key, true);
		TBD_Log.Warn(CH_DEPLOYMENT, string.Format("slot %1 (uid '%2') is not an event seat in the loaded roster - deployments into it need no platform authorization",
			key, slot.uid));
	}

	//------------------------------------------------------------------------------------------------
	//! A deployment this server cannot have authorized: logged, told to the player, recorded.
	protected static void Refuse(int playerId, notnull TBD_MissionSlotStruct slot, string sentence)
	{
		TBD_Log.Warn(CH_DEPLOYMENT, string.Format("deployment-refused player=%1 slot=%2 - %3", playerId, slot.Key(), sentence));
		AuditServerRefusal(sentence, playerId, slot.Key());
		TBD_PlayerChat.Tell(playerId, TAG + "deployment refused - " + sentence);
	}

	//------------------------------------------------------------------------------------------------
	//! A refusal whose cause is this server's state, not the player: one trail entry per cause, naming
	//! the first player it refused.
	protected static void AuditServerRefusal(string sentence, int playerId, string slotKey)
	{
		AuditRefusal("cause:" + sentence, string.Format("DEPLOYMENT: event seats refused - %1 (first refused: %2, seat %3)",
			sentence, PlayerLabel(playerId), slotKey));
	}

	//------------------------------------------------------------------------------------------------
	//! Record a refusal in the admin audit trail (TBD_AdminAudit), which the admin screen and
	//! `#tbd audit` show. The first refusal under `key` in a round takes a slot in the bounded trail;
	//! a repeat is already in the console through the caller's own log line, so players retrying
	//! cannot push the admins' own actions out of the trail.
	protected static void AuditRefusal(string key, string text)
	{
		if (!s_mAuditedRefusals || s_mAuditedRefusals.Count() >= AUDITED_REFUSALS_MAX)
			s_mAuditedRefusals = new map<string, bool>();

		if (s_mAuditedRefusals.Contains(key))
			return;

		s_mAuditedRefusals.Set(key, true);
		TBD_AdminAudit.Record(text, true);
	}

	//------------------------------------------------------------------------------------------------
	//! `name(id)` for the audit trail.
	protected static string PlayerLabel(int playerId)
	{
		string name;
		PlayerManager players = GetGame().GetPlayerManager();
		if (players)
			name = players.GetPlayerName(playerId);

		return string.Format("%1(%2)", name, playerId);
	}

	//------------------------------------------------------------------------------------------------
	//! What the player can do about a denial.
	protected static string HintFor(string reason)
	{
		if (reason == "IDENTITY_NOT_LINKED")
			return TAG + "link this game identity to your TBD account first - type '#tbd link' in chat for the steps.";

		if (reason == "MEMBERSHIP_VERIFICATION_REQUIRED")
			return TAG + "your Discord membership is not verified yet - try this seat again shortly.";

		if (reason == "ACCOUNT_UNAVAILABLE")
			return TAG + "your TBD account cannot deploy - contact an admin.";

		return TAG + "pick another seat: pause menu -> Change slot.";
	}
}
