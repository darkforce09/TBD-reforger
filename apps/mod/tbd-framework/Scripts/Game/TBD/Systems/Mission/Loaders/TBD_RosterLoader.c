//! The event roster, `GET /api/v1/game-runtime/events/{eventId}/roster` (wire version 2): which
//! player is seated where, and which platform ORBAT slot every compiled mission slot stands for.
//! The event is the one the running mission is deployed for (TBD_DeployedMission.GetEventId); a
//! deployment without an event has no roster, and slots are assigned round-robin.
//!
//! One response feeds two lookups with two lifetimes:
//!   * SEATING, `armaId -> slotUid` from `assignments`: the seat a player reserved on the website,
//!     which TBD_SpawnManager seats them in (`GetSlotForIdentity`). Seating settles once - on the
//!     first answer, or at the stage machine's roster deadline (`ForceSettle`) - and stays as
//!     settled, so slot assignment is a function of settled state. A failed fetch settles it empty
//!     (round-robin seating).
//!   * the SLOT TABLE, `slotUid -> (orbatSlotId, eventMissionId)` from `slots`: what a deployment
//!     request names for ANY compiled slot, reserved or open (`ResolveSlotBinding`). Until it has
//!     loaded, TBD_DeploymentAuthorization refuses every deployment into an event seat, so the
//!     fetch is repeated until it loads: after a failure without an answer (network, 5xx, watchdog)
//!     with exponential backoff from 2 s to 60 s; after a refusal (401, 403, 404, a wire version
//!     other than 2, an unreadable body, a roster of another mission) every 60 s, with an ERROR
//!     once per distinct refusal. Each repeat re-reads the backend config, so a credential
//!     corrected in the profile and a server binding corrected on the platform take effect without
//!     a restart.
//!
//! The platform lists the slots of the deployment it runs for this server, and names that
//! deployment's mission in `missionId` (empty when it runs no mission of this event). A roster
//! counts only when `missionId` is the mission this world runs: a roster of another deployment
//! would authorize seats of a mission that is not loaded. A matching roster that lists no slot is
//! loaded too: that deployment has no seats.
//!
//! The keys are camelCase on this wire because `JsonLoadContext` binds JSON keys to field names by
//! exact match and silently ignores a key no field declares. `armaId` is the raw engine identity
//! that `TBD_PlayerIdentity.GetArmaId` puts on every wire and `users.arma_id` is written with;
//! `slotUid` is the compiled slot's `uid`, which `TBD_MissionLoader.GetSlotById` resolves.
//!
//! The roster covers every mission attached to the event. When one slot uid is listed under more
//! than one event mission (the same mission attached twice), the first listing wins: the platform
//! lists event missions by start time and seats a player registered on two of them in the earliest,
//! so both lookups agree. A player's own reservation always names its own event mission.
//!
//! Authenticated with this server's `mod_runtime` machine credential (TBD_GameRuntimeHttp); the
//! platform answers 403 when the event is not bound to this server. Statics outlive a world inside
//! one process, so every world starts from `Reset` and an answer to an earlier world's fetch is
//! dropped.
//! @authority server

//! One seated player. Field names are the JSON keys.
class TBD_RosterAssignmentStruct
{
	string armaId;
	string slotUid;
	string orbatSlotId;
	string eventMissionId;
}

//! One compiled slot and the platform ids a deployment into it names. Field names are the JSON keys.
class TBD_RosterSlotStruct
{
	string eventMissionId;
	string slotUid;
	string orbatSlotId;
}

//! The roster response. Field names are the JSON keys.
class TBD_RosterResponseStruct
{
	int version;
	string eventId;
	//! The catalog mission of the deployment the platform runs for this server, or empty when that
	//! deployment runs no mission of this event.
	string missionId;
	ref array<ref TBD_RosterAssignmentStruct> assignments;
	ref array<ref TBD_RosterSlotStruct> slots;
}

//! The roster fetch on its way to the platform, with the world it was sent for.
class TBD_RosterFetchCall : TBD_GameRuntimeCall
{
	int m_iGeneration;

	//------------------------------------------------------------------------------------------------
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_RosterLoader.OnFetchAnswered(this, answer);
	}
}

class TBD_RosterLoader
{
	//! Greppable channel: `grep '\[TBD\]\[Roster\]' console.log`.
	protected static const string CH_ROSTER = "Roster";

	//! `%1` = the event the running mission is deployed for.
	protected static const string ROSTER_PATH = "/api/v1/game-runtime/events/%1/roster";

	//! The only roster wire this client reads.
	protected static const int WIRE_VERSION = 2;

	protected static const int RETRY_BASE_MS = 2000;
	protected static const int RETRY_CAP_MS = 60000;

	//! armaId -> that player's reserved seat.
	protected static ref map<string, ref TBD_RosterAssignmentStruct> s_Assignments;
	//! slotUid -> that slot's platform ids.
	protected static ref map<string, ref TBD_RosterSlotStruct> s_Slots;
	//! Seating has settled; the stage machine waits on this.
	protected static bool s_Loaded;
	//! How seating settled ("loaded"/"no-event"/"unconfigured"/"failed"/"timeout"), for the
	//! `[TBD][Spawn] roster settled=...` line.
	protected static string s_SettleReason = "pending";
	//! A version-2 roster has been read: s_Slots lists every event seat, possibly none.
	protected static bool s_bSlotTableLoaded;
	protected static bool s_bFetchInFlight;
	//! Consecutive fetches that got no answer, for the backoff.
	protected static int s_iUnansweredFetches;
	//! Refusals already reported at ERROR.
	protected static ref map<string, bool> s_mReportedRefusals;
	//! Bumped by Reset; a fetch answered for an earlier world is dropped.
	protected static int s_iGeneration;

	//------------------------------------------------------------------------------------------------
	static bool IsLoaded()
	{
		return s_Loaded;
	}

	//------------------------------------------------------------------------------------------------
	static string GetSettleReason()
	{
		return s_SettleReason;
	}

	//------------------------------------------------------------------------------------------------
	static int GetAssignmentCount()
	{
		if (!s_Assignments)
			return 0;
		return s_Assignments.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! True once a version-2 roster has been read, whether or not it lists any slot.
	static bool IsSlotTableLoaded()
	{
		return s_bSlotTableLoaded;
	}

	//------------------------------------------------------------------------------------------------
	//! The stage machine's roster deadline passed with the fetch in flight: seating settles empty
	//! (round-robin). The slot table keeps loading.
	static void ForceSettle()
	{
		if (s_Loaded)
			return;

		SettleSeating("timeout");
		TBD_Log.Warn(CH_ROSTER, "settle deadline hit with the fetch in flight - round-robin slots only; the slot table keeps loading, and deployments into event seats are refused until it has.");
	}

	//------------------------------------------------------------------------------------------------
	//! The compiled slot uid this player reserved, or empty.
	static string GetSlotForIdentity(string identityId)
	{
		if (!s_Loaded || !s_Assignments || identityId.IsEmpty())
			return string.Empty;

		TBD_RosterAssignmentStruct assignment;
		if (s_Assignments.Find(identityId, assignment) && assignment)
			return assignment.slotUid;

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! The platform ids a deployment of player `armaId` into the compiled slot `slotUid` names: the
	//! player's own reservation when it is this slot, else the slot's roster listing. False when the
	//! slot table has not loaded or does not list the slot.
	static bool ResolveSlotBinding(string slotUid, string armaId, out string eventMissionId, out string orbatSlotId)
	{
		eventMissionId = string.Empty;
		orbatSlotId = string.Empty;

		if (!s_bSlotTableLoaded || slotUid.IsEmpty())
			return false;

		TBD_RosterAssignmentStruct assignment;
		if (s_Assignments && !armaId.IsEmpty() && s_Assignments.Find(armaId, assignment) && assignment && assignment.slotUid == slotUid)
		{
			eventMissionId = assignment.eventMissionId;
			orbatSlotId = assignment.orbatSlotId;
			return true;
		}

		TBD_RosterSlotStruct slot;
		if (s_Slots && s_Slots.Find(slotUid, slot) && slot)
		{
			eventMissionId = slot.eventMissionId;
			orbatSlotId = slot.orbatSlotId;
			return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! A new world: nothing loaded, nothing in flight, nothing reported.
	static void Reset()
	{
		s_iGeneration++;
		s_Assignments = null;
		s_Slots = null;
		s_Loaded = false;
		s_SettleReason = "pending";
		s_bSlotTableLoaded = false;
		s_bFetchInFlight = false;
		s_iUnansweredFetches = 0;
		s_mReportedRefusals = null;

		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(FetchAgain);
	}

	//------------------------------------------------------------------------------------------------
	static void BeginLoad()
	{
		if (s_Loaded || s_bFetchInFlight)
			return;

		if (TBD_DeployedMission.GetEventId().IsEmpty())
		{
			SettleSeating("no-event");
			TBD_Log.Event(CH_ROSTER, "the running mission is deployed for no event - round-robin slot assignment, and no seat needs platform authorization.");
			return;
		}

		Fetch();
	}

	//------------------------------------------------------------------------------------------------
	protected static void Fetch()
	{
		string eventId = TBD_DeployedMission.GetEventId();
		TBD_Log.Kv(CH_ROSTER, "fetch", string.Format("event=%1 mission=%2 backend=%3", eventId, TBD_DeployedMission.GetMissionId(), TBD_GameRuntimeHttp.DescribeBackend()));

		TBD_RosterFetchCall call = new TBD_RosterFetchCall();
		call.m_iGeneration = s_iGeneration;

		// Marked before sending, so an answer can never find the fetch unmarked.
		s_bFetchInFlight = true;
		string failure;
		if (TBD_GameRuntimeHttp.Get(call, string.Format(ROSTER_PATH, eventId), failure))
			return;

		s_bFetchInFlight = false;
		if (TBD_GameRuntimeHttp.IsConfigured())
			Unanswered(failure);
		else
			Refused("unconfigured", string.Format("%1; backend=%2", failure, TBD_GameRuntimeHttp.DescribeBackend()));
	}

	//------------------------------------------------------------------------------------------------
	//! Called by TBD_RosterFetchCall with the platform's answer.
	static void OnFetchAnswered(notnull TBD_RosterFetchCall call, notnull TBD_GameRuntimeAnswer answer)
	{
		if (call.m_iGeneration != s_iGeneration)
			return;

		s_bFetchInFlight = false;

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.SUCCESS)
		{
			Store(answer.m_sBody);
			return;
		}

		if (answer.m_eOutcome == TBD_EGameRuntimeOutcome.TRANSIENT)
		{
			Unanswered(answer.m_sDetail);
			return;
		}

		Refused(typename.EnumToString(HttpCode, answer.m_eCode), answer.m_sDetail);
	}

	//------------------------------------------------------------------------------------------------
	//! Parse and check the wire version, then load the slot table, and seating when it has not
	//! settled yet.
	protected static void Store(string body)
	{
		TBD_RosterResponseStruct roster = new TBD_RosterResponseStruct();
		JsonLoadContext context = new JsonLoadContext();
		if (body.IsEmpty() || !context.LoadFromString(body) || !context.ReadValue("", roster))
		{
			Refused("unreadable", "the answer is not readable JSON: " + TBD_GameRuntimeAnswer.LoggableBody(body));
			return;
		}

		if (roster.version != WIRE_VERSION)
		{
			Refused("version-" + roster.version, string.Format("roster wire version %1; this server reads version %2 only", roster.version, WIRE_VERSION));
			return;
		}

		// The fetch URL already names the event, so a different one is a backend or proxy mix-up
		// worth saying out loud.
		string expectedEventId = TBD_DeployedMission.GetEventId();
		if (!roster.eventId.IsEmpty() && roster.eventId != expectedEventId)
			TBD_Log.Warn(CH_ROSTER, string.Format("roster eventId '%1' differs from the deployment's event '%2'", roster.eventId, expectedEventId));

		string runningMissionId = TBD_DeployedMission.GetMissionId();
		if (roster.missionId != runningMissionId)
		{
			string listed = roster.missionId;
			if (listed.IsEmpty())
				listed = "no mission of this event";

			Refused("mission:" + roster.missionId, string.Format("the platform runs %1 on this server for event %2, while this world runs mission '%3'",
				listed, roster.eventId, runningMissionId));
			return;
		}

		bool withSeating = !s_Loaded;
		if (withSeating)
		{
			SettleSeating("loaded");
			if (roster.assignments)
			{
				foreach (TBD_RosterAssignmentStruct assignment : roster.assignments)
				{
					if (assignment && !assignment.armaId.IsEmpty() && !assignment.slotUid.IsEmpty())
						s_Assignments.Set(assignment.armaId, assignment);
				}
			}
		}

		s_Slots = new map<string, ref TBD_RosterSlotStruct>();
		int duplicates = 0;
		if (roster.slots)
		{
			foreach (TBD_RosterSlotStruct slot : roster.slots)
			{
				if (!slot || slot.slotUid.IsEmpty())
					continue;

				if (s_Slots.Contains(slot.slotUid))
				{
					duplicates++;
					continue;
				}

				s_Slots.Set(slot.slotUid, slot);
			}
		}

		s_bSlotTableLoaded = true;
		s_iUnansweredFetches = 0;
		s_mReportedRefusals = null;

		if (duplicates > 0)
			TBD_Log.Warn(CH_ROSTER, string.Format("%1 slot listing(s) repeat a slot uid of an earlier event mission - deployments into those slots name the earliest event mission", duplicates));

		if (!withSeating)
		{
			TBD_Log.Kv(CH_ROSTER, "slot-table-loaded", string.Format("event=%1 slots=%2 - deployments into event seats are authorized from now on; seating stays as settled (%3)",
				roster.eventId, s_Slots.Count(), s_SettleReason));
			return;
		}

		TBD_Log.Kv(CH_ROSTER, "loaded", string.Format("event=%1 version=%2 assignments=%3 slots=%4",
			roster.eventId, roster.version, s_Assignments.Count(), s_Slots.Count()));
	}

	//------------------------------------------------------------------------------------------------
	//! The fetch got no answer (network, server error, watchdog): back off and fetch again.
	protected static void Unanswered(string detail)
	{
		if (!s_Loaded)
			SettleSeating("failed");

		s_iUnansweredFetches++;
		int delay = TBD_GameRuntimeHttp.BackoffMs(s_iUnansweredFetches, RETRY_BASE_MS, RETRY_CAP_MS);
		TBD_Log.Warn(CH_ROSTER, string.Format("roster fetch failed (%1) - attempt %2, fetching again in %3 ms; deployments into event seats are refused until the roster loads.",
			detail, s_iUnansweredFetches, delay));
		ScheduleFetch(delay);
	}

	//------------------------------------------------------------------------------------------------
	//! The platform refused the fetch, or its answer cannot be used: an ERROR the first time this
	//! refusal happens, then a new fetch every 60 s.
	protected static void Refused(string refusal, string detail)
	{
		if (!s_Loaded)
		{
			if (refusal == "unconfigured")
				SettleSeating("unconfigured");
			else
				SettleSeating("failed");
		}

		if (!s_mReportedRefusals)
			s_mReportedRefusals = new map<string, bool>();

		if (!s_mReportedRefusals.Contains(refusal))
		{
			s_mReportedRefusals.Set(refusal, true);
			TBD_Log.Error(CH_ROSTER, string.Format("roster of event %1 not loaded (%2) - deployments into event seats are REFUSED until it loads. Fetching again every %3 s; a machine credential corrected in the profile, or the event bound to this server on the platform, is picked up without a restart.",
				TBD_DeployedMission.GetEventId(), detail, RETRY_CAP_MS / 1000));
		}

		ScheduleFetch(RETRY_CAP_MS);
	}

	//------------------------------------------------------------------------------------------------
	protected static void ScheduleFetch(int delayMs)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
			return;

		queue.Remove(FetchAgain);
		queue.CallLater(FetchAgain, delayMs, false);
	}

	//------------------------------------------------------------------------------------------------
	protected static void FetchAgain()
	{
		if (s_bSlotTableLoaded || s_bFetchInFlight)
			return;

		TBD_BackendConfig.Reload();
		Fetch();
	}

	//------------------------------------------------------------------------------------------------
	//! Settle seating with a fresh lookup (filled by Store when the roster arrives in time).
	protected static void SettleSeating(string reason)
	{
		s_Assignments = new map<string, ref TBD_RosterAssignmentStruct>();
		s_Loaded = true;
		s_SettleReason = reason;
	}
}
