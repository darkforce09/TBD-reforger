/**
 * @file TBD_WeatherRuntime.c
 * @brief Forces each authored `weatherTimeline` keyframe at its minute of the live round.
 *
 * Role: reads `weatherTimeline.keyframes[]` on a second `JsonLoadContext` pass and, while the
 * stage is `LIVE`, applies each keyframe once when its `atMinutes` has passed since the round went
 * live: `TimeAndWeatherManagerEntity.ForceWeatherTo` with the preset, looping so it holds until the
 * next keyframe, plus the optional `fog` and `windDirDeg` overrides.  Position: `Tick` and `Clear`
 * are driven by `TBD_RuntimeHeartbeat` every `TICK_MS` on the server; reads
 * `TBD_MissionJsonPass.LoadRoot`, `TBD_MissionLoader.GetMissionId` and the framework stage.
 * State: the static prepared keyframes keyed to the mission id and the latched live-start time,
 * server only; clients follow the engine's weather replication.  Invariants: each keyframe fires at
 * most once per build; a keyframe without a preset is skipped with a WARNING; presence of the
 * timeline is `keyframes.Count()`, because `JsonLoadContext` allocates an absent nested object;
 * every transition logs a `[TBD][Weather]` line.
 */

//! One `weatherTimeline.keyframes[]` entry. Field names are the JSON keys.
//! @contract mission.schema.json#/$defs/weatherKeyframe
class TBD_WeatherKeyframeStruct
{
	int atMinutes; //!< `atMinutes`: minutes after the round went live
	string weatherPreset; //!< `weatherPreset`: TBD snake_case preset
	float windDirDeg; //!< `windDirDeg`: degrees 0..360; `ABSENT` when omitted
	float fog; //!< `fog`: density 0..1; `ABSENT` when omitted

	//! Start both optional numbers at `TBD_WeatherRuntime.ABSENT`.
	void TBD_WeatherKeyframeStruct()
	{
		windDirDeg = TBD_WeatherRuntime.ABSENT;
		fog = TBD_WeatherRuntime.ABSENT;
	}
}

//! The mission's `weatherTimeline` object.
//! @contract mission.schema.json#/$defs/weatherTimeline
class TBD_WeatherTimelineStruct
{
	ref array<ref TBD_WeatherKeyframeStruct> keyframes; //!< `keyframes[]`
}

//! The document root for the weather pass: declares `weatherTimeline` and nothing else.
//! @contract mission.schema.json#/ partial
class TBD_WeatherDocStruct
{
	ref TBD_WeatherTimelineStruct weatherTimeline; //!< `weatherTimeline`, always allocated
}

//! One prepared keyframe. Server-owned; clients see the weather manager's replicated state.
class TBD_WeatherKeyframe
{
	int m_iAtMinutes; //!< minutes after the round went live
	string m_sWeatherPreset; //!< the authored preset
	string m_sWeatherId; //!< the engine weather state name the preset maps to
	float m_fWindDirDeg; //!< degrees; meaningful when `m_bHasWindDir`
	float m_fFog; //!< density; meaningful when `m_bHasFog`
	bool m_bHasWindDir; //!< the keyframe authors `windDirDeg`
	bool m_bHasFog; //!< the keyframe authors `fog`
	bool m_bApplied; //!< the keyframe has fired
}

//! Reads `weatherTimeline`, and at each authored offset forces the world's weather to that preset.
class TBD_WeatherRuntime
{
	static const string CH = "Weather"; //!< log channel
	protected static const string ANNOUNCE_IDLE_KEY = "Weather.idle"; //!< `TBD_AnnounceOnce` key of the idle line

	//! Same sentinel `TBD_MissionEnvironmentStruct` uses: 0 is a legal fog / windDirDeg.
	static const float ABSENT = -1e6; //!< "key absent from JSON"

	static const int TICK_MS = 1000; //!< milliseconds between heartbeat ticks

	protected static ref array<ref TBD_WeatherKeyframe> s_aKeyframes; //!< prepared keyframes; null until built
	protected static bool s_bBuilt; //!< `s_aKeyframes` is built for `s_sBuiltForMission`
	protected static string s_sBuiltForMission; //!< mission id the keyframes were built for
	protected static bool s_bLiveClockLatched; //!< `s_fLiveStartMs` holds this live round's start
	protected static float s_fLiveStartMs; //!< world time in milliseconds when the round went live

	//! Drop the built keyframes and the live clock, and rearm the idle line.
	//! @authority server
	static void Clear()
	{
		s_aKeyframes = null;
		s_bBuilt = false;
		s_sBuiltForMission = string.Empty;
		TBD_AnnounceOnce.Rearm(ANNOUNCE_IDLE_KEY);
		s_bLiveClockLatched = false;
		s_fLiveStartMs = 0;
	}

	//! Whether the keyframes are built for the current mission.
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//! Build the prepared keyframes once per mission; skipped keyframes log a WARNING.
	//! @return false when no mission is loaded; true once built, empty or not
	//! @authority server
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		string missionId = TBD_MissionLoader.GetMissionId();
		if (missionId.IsEmpty())
			return false;

		array<ref TBD_WeatherKeyframeStruct> raw = ReadWire();
		s_aKeyframes = new array<ref TBD_WeatherKeyframe>();
		s_bBuilt = true;
		s_sBuiltForMission = missionId;

		if (!raw)
			return true;

		foreach (int index, TBD_WeatherKeyframeStruct rawKf : raw)
		{
			if (!rawKf)
			{
				TBD_Log.Warn(CH, string.Format("weatherTimeline.keyframes[%1] is null - skipped", index));
				continue;
			}

			TBD_WeatherKeyframe kf = Prepare(rawKf, index);
			if (kf)
				s_aKeyframes.Insert(kf);
		}

		TBD_Log.Kv(CH, "built", string.Format("keyframes=%1", s_aKeyframes.Count()));
		return true;
	}

	//! One heartbeat: rebuild on a mission change, and while `LIVE` fire every keyframe whose
	//! minute has passed. Does nothing without a framework manager or a loaded mission.
	//! @authority server
	static void Tick()
	{
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		string liveId = TBD_MissionLoader.GetMissionId();
		if (s_bBuilt && !s_sBuiltForMission.IsEmpty() && liveId != s_sBuiltForMission)
			Clear();

		if (!Build())
			return;

		if (!s_aKeyframes || s_aKeyframes.Count() == 0)
		{
			TBD_AnnounceOnce.Kv(CH, ANNOUNCE_IDLE_KEY, "idle", "this mission authors no weatherTimeline");
			return;
		}

		if (fm.GetStage() != TBD_EGameStage.LIVE)
		{
			s_bLiveClockLatched = false;
			return;
		}

		int elapsedS = MissionElapsedS();
		int elapsedMin = elapsedS / 60;

		foreach (int index, TBD_WeatherKeyframe kf : s_aKeyframes)
		{
			if (!kf)
				continue;
			if (kf.m_bApplied)
				continue;
			if (elapsedMin < kf.m_iAtMinutes)
				continue;

			Apply(kf, index, elapsedMin);
		}
	}

	//! Map one wire keyframe to a prepared keyframe.
	//! @return the keyframe, or null with a WARNING when it has no preset
	protected static TBD_WeatherKeyframe Prepare(notnull TBD_WeatherKeyframeStruct raw, int index)
	{
		if (raw.weatherPreset.IsEmpty())
		{
			TBD_Log.Warn(CH, string.Format("weatherTimeline.keyframes[%1] has no weatherPreset - skipped", index));
			return null;
		}

		TBD_WeatherKeyframe kf = new TBD_WeatherKeyframe();
		kf.m_iAtMinutes = raw.atMinutes;
		kf.m_sWeatherPreset = raw.weatherPreset;
		kf.m_sWeatherId = CanonicalWeatherId(raw.weatherPreset);
		kf.m_fWindDirDeg = raw.windDirDeg;
		kf.m_fFog = raw.fog;
		kf.m_bHasWindDir = raw.windDirDeg != ABSENT;
		kf.m_bHasFog = raw.fog != ABSENT;
		kf.m_bApplied = false;
		return kf;
	}

	//! Fire one keyframe: force its weather, then its optional fog and wind direction, and log the
	//! transition. Marks it applied first, so a failure never retries; no weather manager logs an
	//! ERROR, an out-of-range or refused override a WARNING.
	protected static void Apply(notnull TBD_WeatherKeyframe kf, int index, int elapsedMin)
	{
		kf.m_bApplied = true;

		TimeAndWeatherManagerEntity tw = TBD_WeatherRuntime.GetTimeAndWeather();
		if (!tw)
		{
			TBD_Log.Error(CH, string.Format(
				"keyframes[%1] atMinutes=%2 preset=%3 - no TimeAndWeatherManagerEntity",
				index, kf.m_iAtMinutes, kf.m_sWeatherPreset));
			return;
		}

		string weatherId = ResolveWeatherId(tw, kf);
		// Looping true: hold this preset until the next authored keyframe. Instant transition.
		tw.ForceWeatherTo(true, weatherId, 0, 0.001);

		if (kf.m_bHasFog)
		{
			if (kf.m_fFog < 0 || kf.m_fFog > 1)
				TBD_Log.Warn(CH, string.Format("keyframes[%1] fog=%2 outside 0..1 - not applied", index, kf.m_fFog));
			else if (!tw.SetFogAmountOverride(true, kf.m_fFog))
				TBD_Log.Warn(CH, string.Format("keyframes[%1] SetFogAmountOverride failed fog=%2", index, kf.m_fFog));
		}

		if (kf.m_bHasWindDir)
		{
			if (kf.m_fWindDirDeg < 0 || kf.m_fWindDirDeg > 360)
				TBD_Log.Warn(CH, string.Format(
					"keyframes[%1] windDirDeg=%2 outside 0..360 - not applied", index, kf.m_fWindDirDeg));
			else if (!tw.SetWindDirectionOverride(true, kf.m_fWindDirDeg))
				TBD_Log.Warn(CH, string.Format(
					"keyframes[%1] SetWindDirectionOverride failed windDirDeg=%2", index, kf.m_fWindDirDeg));
		}

		TBD_Log.Kv(CH, "transition", string.Format(
			"index=%1 atMinutes=%2 elapsedMin=%3 preset=%4 weatherId=%5 fog=%6 windDirDeg=%7",
			index,
			kf.m_iAtMinutes,
			elapsedMin,
			kf.m_sWeatherPreset,
			weatherId,
			FogLog(kf),
			WindLog(kf)));
	}

	//! TBD snake_case -> Everon `WeatherState.GetStateName()` ids ForceWeatherTo accepts.
	protected static string CanonicalWeatherId(string preset)
	{
		if (preset == "clear")
			return "Clear";
		if (preset == "overcast")
			return "Overcast";
		if (preset == "heavy_rain")
			return "Rainy";
		if (preset == "dense_fog")
			return "Foggy";
		return preset;
	}

	//! Prefer a live world's own state list so a world that names rain "HeavyRain" still matches.
	protected static string ResolveWeatherId(TimeAndWeatherManagerEntity tw, notnull TBD_WeatherKeyframe kf)
	{
		array<ref WeatherState> states = {};
		tw.GetWeatherStatesList(states);
		if (!states || states.Count() == 0)
			return kf.m_sWeatherId;

		string want = kf.m_sWeatherId;
		string preset = kf.m_sWeatherPreset;

		foreach (WeatherState st : states)
		{
			if (!st)
				continue;
			string name = st.GetStateName();
			if (name == want || name == preset)
				return name;
		}

		string wantLower = want;
		wantLower.ToLower();
		string presetLower = preset;
		presetLower.ToLower();

		foreach (WeatherState st : states)
		{
			if (!st)
				continue;
			string name = st.GetStateName();
			string lower = name;
			lower.ToLower();
			if (lower == wantLower || lower == presetLower)
				return name;
			if (preset == "heavy_rain" && (lower.Contains("rain")))
				return name;
			if (preset == "dense_fog" && (lower.Contains("fog")))
				return name;
			if (preset == "overcast" && (lower.Contains("overcast") || lower.Contains("cloud")))
				return name;
			if (preset == "clear" && lower.Contains("clear"))
				return name;
		}

		return want;
	}

	//! The world's time and weather manager, or null.
	protected static TimeAndWeatherManagerEntity GetTimeAndWeather()
	{
		BaseWorld baseWorld = GetGame().GetWorld();
		ChimeraWorld world = ChimeraWorld.CastFrom(baseWorld);
		if (!world)
			return null;
		return world.GetTimeAndWeatherManager();
	}

	//! Whole seconds since the round went live, latching the start on the first live call.
	//! @return the seconds, or 0 outside `LIVE`
	protected static int MissionElapsedS()
	{
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return 0;
		if (fm.GetStage() != TBD_EGameStage.LIVE)
			return 0;
		if (!GetGame() || !GetGame().GetWorld())
			return 0;

		float now = GetGame().GetWorld().GetWorldTime();
		if (!s_bLiveClockLatched)
		{
			s_bLiveClockLatched = true;
			s_fLiveStartMs = now;
		}

		int elapsed = (now - s_fLiveStartMs) / 1000;
		if (elapsed < 0)
			elapsed = 0;
		return elapsed;
	}

	//! Read `weatherTimeline.keyframes[]` on the weather pass.
	//! @return the keyframes, or null when there is no document, it does not read, or it authors none
	protected static array<ref TBD_WeatherKeyframeStruct> ReadWire()
	{
		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (!ctx)
			return null;

		TBD_WeatherDocStruct doc = new TBD_WeatherDocStruct();
		if (!ctx.ReadValue("", doc))
			return null;

		// NOT `if (doc.weatherTimeline)`. JsonLoadContext ALLOCATES a nested ref even when the
		// key is absent. Presence is Count() on keyframes.
		if (!doc.weatherTimeline)
			return null;
		if (!doc.weatherTimeline.keyframes)
			return null;
		if (doc.weatherTimeline.keyframes.Count() == 0)
			return null;

		return doc.weatherTimeline.keyframes;
	}

	//! The keyframe's fog for the transition line, or `omitted`.
	protected static string FogLog(notnull TBD_WeatherKeyframe kf)
	{
		if (!kf.m_bHasFog)
			return "omitted";
		return kf.m_fFog.ToString();
	}

	//! The keyframe's wind direction for the transition line, or `omitted`.
	protected static string WindLog(notnull TBD_WeatherKeyframe kf)
	{
		if (!kf.m_bHasWindDir)
			return "omitted";
		return kf.m_fWindDirDeg.ToString();
	}
}
