//! T-682 -- apply payload `environment.fog` / `wind` / `windDirDeg` / `viewDistance` at mission
//! boot. The Enfusion half of `mission.schema.json#/$defs/environment` for those four axes.
//!
//! == What was missing ========================================================================
//! T-706 put the keys on the wire. `ModEnvironment` did not even serialise `windDirDeg`, and
//! nothing in `apps/mod` read `fog` / `wind` / `viewDistance`. An author (or a hand-staged
//! 1.3 document) could carry the values to the dedicated server and the round would still run
//! at the world's default weather and camera far-plane. This file is the reader.
//!
//! == Reader before control ===================================================================
//! The editor's `author_env` gate still refuses to author these keys. That is deliberate: a
//! control whose value stops at the editor boundary is worse than no control. This reader is
//! the missing destination. Editor fog/wind/view controls stay out of this slice.
//!
//! == Server-side only ========================================================================
//! `TBD_FrameworkManager.OnPostInit` returns early for `RplMode.Client` before `BeginLoad()`,
//! so this runs on the authority. `BaseWeatherManagerEntity.SetFogAmountOverride` (and the
//! wind overrides) are documented as authority-only; replication carries the weather. View
//! distance is `ChimeraGame.SetViewDistance` on the same server-only path.
//!
//! == Presence: the nested-ref landmine =======================================================
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref <class>` field even when the JSON key
//! is ABSENT. `if (doc.environment)` is ALWAYS TRUE. Fog `0` is clear, wind `0` is calm,
//! windDirDeg `0` is north -- a zero-test would erase authored statements. Every numeric field
//! therefore initialises to `ABSENT` (-1e6). dateTime / weatherPreset bind so the primary parse
//! sees them, but T-682 does NOT apply them: every compiled document already carries those two
//! keys, and applying them here would change boot for missions that never authored fog/wind/
//! viewDistance.
//!
//! == What this file CANNOT prove =============================================================
//! The gate is `cargo xtask mod compile`. It proves the symbols exist. It cannot run a round.
//! Whether authored fog is visible in-game is a human checklist item.
//! @contract mission.schema.json#/$defs/environment

//------------------------------------------------------------------------------------------------
//! Bound by `JsonLoadContext` onto `TBD_MissionDocumentStruct.environment`. Field names MUST
//! equal the JSON keys.
class TBD_MissionEnvironmentStruct
{
	//! "key absent from JSON". A presence flag, not a magic default -- see the header.
	static const float ABSENT = -1000000;

	string dateTime;       //!< ISO-8601. Bound, not applied by T-682.
	string weatherPreset;  //!< Bound, not applied by T-682.
	float windDirDeg = ABSENT;    //!< Degrees 0..360. ATTR direction for `wind`.
	float fog = ABSENT;           //!< Density 0..1.
	float wind = ABSENT;          //!< Strength m/s.
	float viewDistance = ABSENT;  //!< Metres; schema exclusiveMinimum 0.
}

//------------------------------------------------------------------------------------------------
//! Applies authored fog / wind / viewDistance through the world's weather manager and ChimeraGame.
class TBD_EnvironmentReader
{
	//------------------------------------------------------------------------------------------------
	//! Called from `TBD_MissionLoader.ParseMissionJson` after a valid parse, on the server-only
	//! load path. No-ops when none of the four axes were authored, so missions without them boot
	//! unchanged.
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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
