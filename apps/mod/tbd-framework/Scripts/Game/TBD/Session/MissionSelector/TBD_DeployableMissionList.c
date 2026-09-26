/**
 * @file TBD_DeployableMissionList.c
 * @brief The missions an in-game admin may deploy to this server, fetched from the platform.
 *
 * Role: fetches `GET /api/v1/game-runtime/missions` (mod_runtime credential) and numbers the
 * entries for the admin surfaces.  Position: the stage machine entering LOBBY and `#tbd refresh`
 * call Refresh; `#tbd missions`, TBD_MissionBrowserService and TBD_MissionDeploymentRelay read the
 * entries by 1-based number.
 * State: the static entry list, the loaded and in-flight flags and the last failure, on the
 * server; it belongs to the server process, not to a world.  Invariants: one fetch at a time;
 * entries keep the platform's order (by title) and are numbered from 1 in it; a failed fetch keeps
 * the previous list.
 */

//! One mission an in-game administrator may deploy. Field names are the JSON keys.
//! @contract mission-deployment.schema.json#/definitions/DeployableMission
class TBD_DeployableMissionStruct
{
	string mission_id; //!< JSON `mission_id`
	string title; //!< JSON `title`
	string terrain_key; //!< JSON `terrain_key`
	string artifact_id; //!< JSON `artifact_id`: the approved artifact this server would run
	string artifact_sha256; //!< JSON `artifact_sha256`
}

//! `GET /api/v1/game-runtime/missions` answer. The field name is the JSON key.
//! @contract mission-deployment.schema.json#/definitions/DeployableMissionList
class TBD_DeployableMissionListStruct
{
	ref array<ref TBD_DeployableMissionStruct> missions; //!< JSON `missions`, in the platform's order
}

//! The list fetch on its way to the platform.
class TBD_DeployableMissionListCall : TBD_GameRuntimeCall
{
	//! Hand the platform's answer to TBD_DeployableMissionList.OnAnswered.
	//! @param answer the platform's answer
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_DeployableMissionList.OnAnswered(answer);
	}
}

//! The deployable mission list of this server, numbered from 1.
//! @authority server
class TBD_DeployableMissionList
{
	static const string CH_MISSIONS = "Missions"; //!< TBD_Log channel: `grep '\[TBD\]\[Missions\]' console.log`

	protected static ref array<ref TBD_DeployableMissionStruct> s_aEntries; //!< the list in platform order; null before the first load
	protected static bool s_bLoaded; //!< a fetch has loaded a list
	protected static bool s_bInFlight; //!< a fetch is on its way
	protected static string s_sLastFailure; //!< why the last refresh did not load the list, or empty

	//! Fetch the list again.
	//! @return false when a fetch is in flight already or none can be sent
	//! @route GET /api/v1/game-runtime/missions
	static bool Refresh()
	{
		if (s_bInFlight)
			return false;

		// Marked before sending, so an answer can never find the fetch unmarked.
		s_bInFlight = true;
		string failure;
		if (TBD_GameRuntimeHttp.Get(new TBD_DeployableMissionListCall(), TBD_GameRuntimeHttp.ROUTE_PREFIX + "/missions", failure))
		{
			TBD_Log.Kv(CH_MISSIONS, "list-fetch", "backend=" + TBD_GameRuntimeHttp.DescribeBackend());
			return true;
		}

		s_bInFlight = false;
		s_sLastFailure = failure;
		TBD_Log.Warn(CH_MISSIONS, "deployable mission list not fetched - " + failure);
		return false;
	}

	//! Take the platform's answer: replace the list on success, else keep it and remember why.
	//! @param answer the platform's answer
	static void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		s_bInFlight = false;

		TBD_DeployableMissionListStruct list = new TBD_DeployableMissionListStruct();
		JsonLoadContext context = new JsonLoadContext();
		if (answer.m_eOutcome != TBD_EGameRuntimeOutcome.SUCCESS || !context.LoadFromString(answer.m_sBody) || !context.ReadValue("", list))
		{
			s_sLastFailure = answer.m_sDetail;
			TBD_Log.Warn(CH_MISSIONS, "deployable mission list not loaded - " + answer.m_sDetail);
			return;
		}

		s_aEntries = list.missions;
		if (!s_aEntries)
			s_aEntries = new array<ref TBD_DeployableMissionStruct>();

		s_bLoaded = true;
		s_sLastFailure = string.Empty;
		TBD_Log.Kv(CH_MISSIONS, "list-loaded", string.Format("missions=%1", s_aEntries.Count()));
	}

	//! @return how many missions the list holds
	static int Count()
	{
		if (!s_aEntries)
			return 0;

		return s_aEntries.Count();
	}

	//! The entry numbered `number` in the list shown to admins.
	//! @param number the 1-based list number
	//! @return the entry, or null when out of range
	static TBD_DeployableMissionStruct GetEntryByNumber(int number)
	{
		int index = number - 1;
		if (!s_aEntries || index < 0 || index >= s_aEntries.Count())
			return null;

		return s_aEntries[index];
	}

	//! The `#tbd missions` reply.
	//! @return a header and one numbered line per mission, or one line saying why there are none
	static array<string> BuildListLines()
	{
		array<string> lines = {};
		if (!s_bLoaded || Count() == 0)
		{
			lines.Insert(DescribeEmpty());
			return lines;
		}

		string terrain = TBD_DeployedMission.GetTerrainKey();
		if (terrain.IsEmpty())
			terrain = "none";

		lines.Insert(string.Format("TBD missions (%1) - current terrain: %2", Count(), terrain));
		for (int i = 0; i < Count(); i++)
			lines.Insert("  " + DescribeEntry(i + 1));

		return lines;
	}

	//! One list line, marked when this world runs the mission.
	//! @param number the 1-based list number
	//! @return `n) Title [terrain]` with `(running)` or `(running another version)`; empty when out of range
	static string DescribeEntry(int number)
	{
		TBD_DeployableMissionStruct entry = GetEntryByNumber(number);
		if (!entry)
			return string.Empty;

		string line = string.Format("%1) %2 [%3]", number, entry.title, entry.terrain_key);
		if (entry.mission_id == TBD_DeployedMission.GetMissionId())
		{
			if (entry.artifact_id == TBD_DeployedMission.GetArtifactId())
				line += " (running)";
			else
				line += " (running another version)";
		}

		return line;
	}

	//! @return why there is no list to show
	static string DescribeEmpty()
	{
		if (s_bLoaded)
			return "TBD: the platform lists no mission this server can run.";

		if (s_bInFlight)
			return "TBD: the mission list is loading - try again in a moment.";

		if (!s_sLastFailure.IsEmpty())
			return "TBD: no mission list (" + s_sLastFailure + ") - '#tbd refresh' tries again.";

		return "TBD: no missions loaded yet - try '#tbd refresh' in a moment.";
	}
}
