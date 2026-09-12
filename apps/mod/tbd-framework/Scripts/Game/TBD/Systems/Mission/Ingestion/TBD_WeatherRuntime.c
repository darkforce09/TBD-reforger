//! T-936.4 - apply authored `weatherTimeline.keyframes[]` through the world's weather manager.
//!
//! Server applies each keyframe at `atMinutes` from LIVE start via
//! `TimeAndWeatherManagerEntity.ForceWeatherTo` (looping so the preset holds until the next
//! keyframe). Optional `fog` / `windDirDeg` use the same override path T-682's
//! `TBD_EnvironmentReader` already uses. ForceWeatherTo is server-only; clients follow engine
//! weather replication. Every transition is logged (`[TBD][Weather]`) so the human checklist can
//! see the authored offsets fire.
//!
//! JsonLoadContext ALLOCATES nested refs when the key is absent. Presence is `keyframes.Count()`,
//! not `if (doc.weatherTimeline)`.

//------------------------------------------------------------------------------------------------
class TBD_WeatherKeyframeStruct
{
	int atMinutes;
	string weatherPreset;
	float windDirDeg;
	float fog;

	void TBD_WeatherKeyframeStruct()
	{
		windDirDeg = TBD_WeatherRuntime.ABSENT;
		fog = TBD_WeatherRuntime.ABSENT;
	}
}

//------------------------------------------------------------------------------------------------
class TBD_WeatherTimelineStruct
{
	ref array<ref TBD_WeatherKeyframeStruct> keyframes;
}

//------------------------------------------------------------------------------------------------
//! The document root for the weather pass: declares `weatherTimeline` and nothing else.
class TBD_WeatherDocStruct
{
	ref TBD_WeatherTimelineStruct weatherTimeline;
}

//------------------------------------------------------------------------------------------------
//! One prepared keyframe. Server-owned; clients see the weather manager's replicated state.
class TBD_WeatherKeyframe
{
	int m_iAtMinutes;
	string m_sWeatherPreset;
	string m_sWeatherId;
	float m_fWindDirDeg;
	float m_fFog;
	bool m_bHasWindDir;
	bool m_bHasFog;
	bool m_bApplied;
}

//------------------------------------------------------------------------------------------------
//! Reads `weatherTimeline`, and at each authored offset forces the world's weather to that preset.
class TBD_WeatherRuntime
{
	static const string CH = "Weather";

	//! Same sentinel `TBD_MissionEnvironmentStruct` uses: JsonLoadContext defaults missing floats
	//! to 0, and 0 is a legal fog / windDirDeg.
	static const float ABSENT = -1e6;

	static const int TICK_MS = 1000;

	protected static ref array<ref TBD_WeatherKeyframe> s_aKeyframes;
	protected static bool s_bBuilt;
	protected static string s_sBuiltForMission;
	protected static bool s_bAnnounced;
	protected static bool s_bLiveClockLatched;
	protected static float s_fLiveStartMs;

	//------------------------------------------------------------------------------------------------
	static void Clear()
	{
		s_aKeyframes = null;
		s_bBuilt = false;
		s_sBuiltForMission = string.Empty;
		s_bAnnounced = false;
		s_bLiveClockLatched = false;
		s_fLiveStartMs = 0;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//------------------------------------------------------------------------------------------------
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		string missionId = CurrentMissionId();
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

	//------------------------------------------------------------------------------------------------
	static void Tick()
	{
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		string liveId = CurrentMissionId();
		if (s_bBuilt && !s_sBuiltForMission.IsEmpty() && liveId != s_sBuiltForMission)
			Clear();

		if (!Build())
			return;

		if (!s_aKeyframes || s_aKeyframes.Count() == 0)
		{
			AnnounceEmptyOnce();
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected static TimeAndWeatherManagerEntity GetTimeAndWeather()
	{
		BaseWorld baseWorld = GetGame().GetWorld();
		ChimeraWorld world = ChimeraWorld.CastFrom(baseWorld);
		if (!world)
			return null;
		return world.GetTimeAndWeatherManager();
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected static string CurrentMissionId()
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !doc.meta)
			return string.Empty;
		return doc.meta.id;
	}

	//------------------------------------------------------------------------------------------------
	protected static array<ref TBD_WeatherKeyframeStruct> ReadWire()
	{
		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return null;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
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

	//------------------------------------------------------------------------------------------------
	protected static void AnnounceEmptyOnce()
	{
		if (s_bAnnounced)
			return;
		s_bAnnounced = true;
		TBD_Log.Kv(CH, "idle", "this mission authors no weatherTimeline");
	}

	//------------------------------------------------------------------------------------------------
	protected static string FogLog(notnull TBD_WeatherKeyframe kf)
	{
		if (!kf.m_bHasFog)
			return "omitted";
		return kf.m_fFog.ToString();
	}

	//------------------------------------------------------------------------------------------------
	protected static string WindLog(notnull TBD_WeatherKeyframe kf)
	{
		if (!kf.m_bHasWindDir)
			return "omitted";
		return kf.m_fWindDirDeg.ToString();
	}
}

//------------------------------------------------------------------------------------------------
modded class SCR_BaseGameMode
{
	protected bool m_bTBD_WeatherTickArmed;

	//------------------------------------------------------------------------------------------------
	//! @authority server - ForceWeatherTo is server-only; clients follow replication.
	protected override void OnGameStart()
	{
		super.OnGameStart();

		TBD_WeatherRuntime.Clear();

		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		if (m_bTBD_WeatherTickArmed)
			return;

		m_bTBD_WeatherTickArmed = true;
		GetGame().GetCallqueue().CallLater(TBD_WeatherTick, TBD_WeatherRuntime.TICK_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	//! One-shot and self-re-arming rather than a repeating CallLater, for the same reason
	//! TBD_WinConditionEvaluator's twin records: ScriptCallQueue.Remove cancels BY FUNCTION.
	void TBD_WeatherTick()
	{
		if (GetGame().GetGameMode() != this)
			return;

		TBD_WeatherRuntime.Tick();
		GetGame().GetCallqueue().CallLater(TBD_WeatherTick, TBD_WeatherRuntime.TICK_MS, false);
	}
}
