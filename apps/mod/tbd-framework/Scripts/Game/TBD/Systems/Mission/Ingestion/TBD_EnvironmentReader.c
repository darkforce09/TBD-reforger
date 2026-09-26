/**
 * @file TBD_EnvironmentReader.c
 * @brief Applies the mission's authored fog, wind, wind direction and view distance at load.
 *
 * Role: binds `$defs/environment` and applies its fog and wind through the
 * `BaseWeatherManagerEntity` overrides and `viewDistance` through `ChimeraGame.SetViewDistance`.
 * Position: bound by `JsonLoadContext` onto `TBD_MissionDocumentStruct.environment`; `Apply`
 * runs from `TBD_MissionLoader` after a valid parse, on the server load path; the engine
 * replicates weather to clients.
 * State: none.  Invariants: every number starts at `ABSENT`, because 0 is a legal fog, wind and
 * direction, and an absent key leaves the world default; `dateTime` and `weatherPreset` are bound
 * but not applied; a value outside its range logs a WARNING and is skipped.
 */

//! Bound by `JsonLoadContext` onto `TBD_MissionDocumentStruct.environment`. Field names MUST
//! equal the JSON keys.
//! @contract mission.schema.json#/$defs/environment
class TBD_MissionEnvironmentStruct
{
	//! A presence flag, not a magic default -- see the header.
	static const float ABSENT = -1000000; //!< "key absent from JSON"

	string dateTime;       //!< ISO-8601. Bound, not applied here.
	string weatherPreset;  //!< Bound, not applied here.
	float windDirDeg = ABSENT;    //!< Degrees 0..360. ATTR direction for `wind`.
	float fog = ABSENT;           //!< Density 0..1.
	float wind = ABSENT;          //!< Strength m/s.
	float viewDistance = ABSENT;  //!< Metres; schema exclusiveMinimum 0.
}

//! Applies authored fog / wind / viewDistance through the world's weather manager and ChimeraGame.
class TBD_EnvironmentReader
{
	//! Apply the authored axes of the loaded mission's `environment`. Called from
	//! `TBD_MissionLoader` after a valid parse. Does nothing when none of the four axes is
	//! authored or no mission is loaded.
	//! @authority server
	static void Apply()
	{
		TBD_MissionDocumentStruct mission = TBD_MissionLoader.GetMission();
		if (!mission)
			return;

		TBD_MissionEnvironmentStruct env = mission.environment;
		if (!env)
			return;

		bool anyFogWind = false;
		if (env.fog != TBD_MissionEnvironmentStruct.ABSENT)
			anyFogWind = true;
		if (env.wind != TBD_MissionEnvironmentStruct.ABSENT)
			anyFogWind = true;
		if (env.windDirDeg != TBD_MissionEnvironmentStruct.ABSENT)
			anyFogWind = true;

		if (anyFogWind)
			ApplyFogAndWind(env);

		if (env.viewDistance != TBD_MissionEnvironmentStruct.ABSENT)
			ApplyViewDistance(env.viewDistance);
	}

	//! Apply the authored fog, wind speed and wind direction; a world without a weather manager
	//! logs an ERROR and applies none.
	protected static void ApplyFogAndWind(TBD_MissionEnvironmentStruct env)
	{
		BaseWorld baseWorld = GetGame().GetWorld();
		ChimeraWorld world = ChimeraWorld.CastFrom(baseWorld);
		if (!world)
		{
			Print("[TBD][Environment] no ChimeraWorld -- fog/wind not applied", LogLevel.ERROR);
			return;
		}

		TimeAndWeatherManagerEntity tw = world.GetTimeAndWeatherManager();
		BaseWeatherManagerEntity weather = BaseWeatherManagerEntity.Cast(tw);
		if (!weather)
		{
			Print("[TBD][Environment] no BaseWeatherManagerEntity -- fog/wind not applied", LogLevel.ERROR);
			return;
		}

		if (env.fog != TBD_MissionEnvironmentStruct.ABSENT)
			ApplyFog(weather, env.fog);

		if (env.wind != TBD_MissionEnvironmentStruct.ABSENT)
			ApplyWindSpeed(weather, env.wind);

		if (env.windDirDeg != TBD_MissionEnvironmentStruct.ABSENT)
			ApplyWindDir(weather, env.windDirDeg);
	}

	//! Override fog density; outside 0..1 or a refused override logs a WARNING.
	protected static void ApplyFog(BaseWeatherManagerEntity weather, float fog)
	{
		if (fog < 0 || fog > 1)
		{
			Print(string.Format("[TBD][Environment] fog=%1 outside 0..1 -- not applied", fog), LogLevel.WARNING);
			return;
		}

		if (!weather.SetFogAmountOverride(true, fog))
		{
			Print(string.Format("[TBD][Environment] SetFogAmountOverride failed fog=%1", fog), LogLevel.WARNING);
			return;
		}

		Print(string.Format("[TBD][Environment] fog=%1 applied", fog), LogLevel.NORMAL);
	}

	//! Override wind speed in m/s; negative or a refused override logs a WARNING.
	protected static void ApplyWindSpeed(BaseWeatherManagerEntity weather, float wind)
	{
		if (wind < 0)
		{
			Print(string.Format("[TBD][Environment] wind=%1 m/s is negative -- not applied", wind), LogLevel.WARNING);
			return;
		}

		if (!weather.SetWindSpeedOverride(true, wind))
		{
			Print(string.Format("[TBD][Environment] SetWindSpeedOverride failed wind=%1", wind), LogLevel.WARNING);
			return;
		}

		Print(string.Format("[TBD][Environment] wind=%1 m/s applied", wind), LogLevel.NORMAL);
	}

	//! Override wind direction in degrees; outside 0..360 or a refused override logs a WARNING.
	protected static void ApplyWindDir(BaseWeatherManagerEntity weather, float windDirDeg)
	{
		if (windDirDeg < 0 || windDirDeg > 360)
		{
			Print(string.Format("[TBD][Environment] windDirDeg=%1 outside 0..360 -- not applied", windDirDeg), LogLevel.WARNING);
			return;
		}

		if (!weather.SetWindDirectionOverride(true, windDirDeg))
		{
			Print(string.Format("[TBD][Environment] SetWindDirectionOverride failed windDirDeg=%1", windDirDeg), LogLevel.WARNING);
			return;
		}

		Print(string.Format("[TBD][Environment] windDirDeg=%1 applied", windDirDeg), LogLevel.NORMAL);
	}

	//! Set the game's view distance in metres; 0 or below logs a WARNING.
	protected static void ApplyViewDistance(float viewDistance)
	{
		if (viewDistance <= 0)
		{
			Print(string.Format("[TBD][Environment] viewDistance=%1 is not > 0 -- not applied", viewDistance), LogLevel.WARNING);
			return;
		}

		GetGame().SetViewDistance(viewDistance);
		Print(string.Format("[TBD][Environment] viewDistance=%1 m applied", viewDistance), LogLevel.NORMAL);
	}
}
