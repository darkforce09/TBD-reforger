//! The missions an in-game administrator may deploy to this server,
//! `GET /api/v1/game-runtime/missions` (mod_runtime credential): every live mission whose approved
//! artifact this server can run, in the platform's order (by title). The admin surfaces number them
//! from 1 in that order - `#tbd missions` and the mission browser keys - and a number selects the
//! entry TBD_MissionDeploymentRelay asks the platform to deploy. The list is refreshed when the
//! stage machine enters LOBBY and by `#tbd refresh`; it belongs to the server, not to a world.
//! @authority server

//! One mission an in-game administrator may deploy. Field names are the JSON keys.
class TBD_DeployableMissionStruct
{
	string mission_id;
	string title;
	string terrain_key;
	string artifact_id;
	string artifact_sha256;
}

//! `GET /api/v1/game-runtime/missions` answer. The field name is the JSON key.
class TBD_DeployableMissionListStruct
{
	ref array<ref TBD_DeployableMissionStruct> missions;
}

//! The list fetch on its way to the platform.
class TBD_DeployableMissionListCall : TBD_GameRuntimeCall
{
	//------------------------------------------------------------------------------------------------
	override void OnAnswered(notnull TBD_GameRuntimeAnswer answer)
	{
		TBD_DeployableMissionList.OnAnswered(answer);
	}
}

class TBD_DeployableMissionList
{
	//! Greppable channel: `grep '\[TBD\]\[Missions\]' console.log`.
	static const string CH_MISSIONS = "Missions";

	protected static ref array<ref TBD_DeployableMissionStruct> s_aEntries;
	protected static bool s_bLoaded;
	protected static bool s_bInFlight;
	//! Why the last refresh did not load the list, or empty.
	protected static string s_sLastFailure;

	//------------------------------------------------------------------------------------------------
	//! Fetch the list again. False when a fetch is in flight already or none can be sent.
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

	//------------------------------------------------------------------------------------------------
	//! Called by TBD_DeployableMissionListCall with the platform's answer.
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

	//------------------------------------------------------------------------------------------------
	static int Count()
	{
		if (!s_aEntries)
			return 0;

		return s_aEntries.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! The entry numbered `number` (from 1) in the list shown to admins, or null.
	static TBD_DeployableMissionStruct GetEntryByNumber(int number)
	{
		int index = number - 1;
		if (!s_aEntries || index < 0 || index >= s_aEntries.Count())
			return null;

		return s_aEntries[index];
	}

	//------------------------------------------------------------------------------------------------
	//! `#tbd missions`: a header and one numbered line per mission, or one line saying why none.
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

	//------------------------------------------------------------------------------------------------
	//! `n) Title [terrain]`, marked when this world runs the mission.
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

	//------------------------------------------------------------------------------------------------
	//! Why there is no list to show.
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
