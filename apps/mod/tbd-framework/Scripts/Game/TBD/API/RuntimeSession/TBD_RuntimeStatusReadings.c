/**
 * @file TBD_RuntimeStatusReadings.c
 * @brief The live readings a runtime-session heartbeat reports.
 *
 * Role: reads connected players, player capacity, server frame rate, process uptime, in-game time
 * of day and weather from the engine when a heartbeat is built.  Position: called by
 * `TBD_RuntimeSession` for online heartbeats and by `TBD_RuntimeSessionClosing` for the offline one.
 * State: none; pure reads.  Invariants: a reading the engine cannot provide is omitted, never sent
 * as zero, because the platform keeps its stored value for an absent key while a zero is recorded
 * as a real measurement.
 */

//! Heartbeat reading fields, built as JSON members without braces.
//! @authority server
class TBD_RuntimeStatusReadings
{
	//! The reading keys of a heartbeat from a running server.
	//! @return `"is_online":true,...` without braces
	static string BuildOnlineFields()
	{
		string fields = "\"is_online\":true";

		PlayerManager players = GetGame().GetPlayerManager();
		if (players)
			fields += string.Format(",\"player_count\":%1", players.GetPlayerCount());

		int maxPlayers = MaxPlayers();
		if (maxPlayers > 0)
			fields += string.Format(",\"max_players\":%1", maxPlayers);

		// Whole frames per second (the engine's average over its last frames): an integer keeps
		// float formatting off the wire, and the platform's low-FPS threshold is a whole number.
		// Zero means no frame has been measured yet.
		int fps = Math.Round(System.GetFPS());
		if (fps > 0)
			fields += string.Format(",\"server_fps\":%1", fps);

		fields += string.Format(",\"uptime_seconds\":%1", System.GetTickCount() / 1000);

		string time = InGameTime();
		if (!time.IsEmpty())
			fields += string.Format(",\"ingame_time\":\"%1\"", TBD_BackendText.JsonEscape(time));

		string weather = InGameWeather();
		if (!weather.IsEmpty())
			fields += string.Format(",\"ingame_weather\":\"%1\"", TBD_BackendText.JsonEscape(weather));

		return fields;
	}

	//! The reading keys of the last heartbeat of a runtime that is shutting down: offline, and
	//! serving nobody. The platform flips a server offline on its own only when a session expires,
	//! so a clean shutdown says so itself.
	//! @return `"is_online":false,"player_count":0`
	static string BuildOfflineFields()
	{
		return "\"is_online\":false,\"player_count\":0";
	}

	//! The server's player limit, else the mission header's player count.
	//! @return the capacity, or 0 (omitted)
	protected static int MaxPlayers()
	{
		ServerInfo server = GetGame().GetServerInfo();
		if (server)
		{
			int limit = server.GetPlayerLimit();
			if (limit > 0)
				return limit;
		}

		SCR_MissionHeader header = SCR_MissionHeader.Cast(GetGame().GetMissionHeader());
		if (header && header.m_iPlayerCount > 0)
			return header.m_iPlayerCount;

		return 0;
	}

	//! In-game time of day.
	//! @return `HH:MM`, or empty when the world has no time manager
	protected static string InGameTime()
	{
		TimeAndWeatherManagerEntity manager = TimeAndWeather();
		if (!manager)
			return string.Empty;

		int hours;
		int minutes;
		int seconds;
		manager.GetHoursMinutesSeconds(hours, minutes, seconds);
		return string.Format("%1:%2", TBD_BackendText.Pad2(hours), TBD_BackendText.Pad2(minutes));
	}

	//! The current weather state's name in lower case (`clear`, `overcast`, ...).
	//! @return the name, or empty without a manager or state
	protected static string InGameWeather()
	{
		TimeAndWeatherManagerEntity manager = TimeAndWeather();
		if (!manager)
			return string.Empty;

		WeatherState state = manager.GetCurrentWeatherState();
		if (!state)
			return string.Empty;

		// A copy: `ToLower` mutates in place and returns a count.
		string name = string.Format("%1", state.GetStateName());
		name.ToLower();
		return name;
	}

	//! The world's time and weather manager.
	//! @return the manager, or null outside a Chimera world
	protected static TimeAndWeatherManagerEntity TimeAndWeather()
	{
		ChimeraWorld world = ChimeraWorld.CastFrom(GetGame().GetWorld());
		if (!world)
			return null;

		return world.GetTimeAndWeatherManager();
	}
}
