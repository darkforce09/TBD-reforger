/**
 * @file TBD_StageEnvironment.c
 * @brief Applies the mission's authored wind direction and night-vision rule to the world.
 *
 * Role: re-applies `environment.windDirDeg` through the weather manager at load, logs the latched
 * `settings`, and strips night-vision gadgets from each spawned body when `settings.nightVision`
 * is false.  Position: owned by TBD_FrameworkManager, which subscribes OnPlayerSpawnedApplyNvg to
 * the game mode's spawn invoker and latches the replicated settings; TBD_LoadingGate calls
 * ApplyAuthoredWeather.  State: one pending strip per spawn on the call queue; server only.
 * Invariants: `windDirDeg` presence is its ABSENT sentinel, never a null test; the strip runs
 * 1500 ms after the spawn so the loadout dress can finish, and re-reads the rule when it fires.
 */

//! Authored weather and night-vision rule of one framework world.
class TBD_StageEnvironment : Managed
{
	protected TBD_FrameworkManager m_Manager; //!< owning manager; holds the replicated night-vision latch

	//! Bind the helper to its manager.
	//! @param manager the owning framework manager
	void TBD_StageEnvironment(TBD_FrameworkManager manager)
	{
		m_Manager = manager;
	}

	//! Cancel every pending night-vision strip.
	void CancelCallbacks()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(StripNightVisionForPlayer);
	}

	//! Apply `environment.windDirDeg` as the weather manager's wind direction override. An absent,
	//! out-of-range or unapplied value is logged and leaves the world's wind as it is.
	//! @authority server
	static void ApplyAuthoredWeather()
	{
		TBD_MissionDocumentStruct mission = TBD_MissionLoader.GetMission();
		if (!mission)
			return;

		TBD_MissionEnvironmentStruct env = mission.environment;
		if (!env)
			return;

		// The JSON reader allocates nested blocks even when absent; presence is the sentinel.
		if (env.windDirDeg == TBD_MissionEnvironmentStruct.ABSENT)
			return;

		if (env.windDirDeg < 0 || env.windDirDeg > 360)
		{
			Print(string.Format("[TBD][Weather] windDirDeg=%1 outside 0..360 -- not applied", env.windDirDeg), LogLevel.WARNING);
			return;
		}

		BaseWorld baseWorld = GetGame().GetWorld();
		ChimeraWorld world = ChimeraWorld.CastFrom(baseWorld);
		if (!world)
		{
			Print("[TBD][Weather] no ChimeraWorld -- windDirDeg not applied", LogLevel.ERROR);
			return;
		}

		TimeAndWeatherManagerEntity tw = world.GetTimeAndWeatherManager();
		BaseWeatherManagerEntity weather = BaseWeatherManagerEntity.Cast(tw);
		if (!weather)
		{
			Print("[TBD][Weather] no BaseWeatherManagerEntity -- windDirDeg not applied", LogLevel.ERROR);
			return;
		}

		if (!weather.SetWindDirectionOverride(true, env.windDirDeg))
		{
			Print(string.Format("[TBD][Weather] SetWindDirectionOverride failed windDirDeg=%1", env.windDirDeg), LogLevel.WARNING);
			return;
		}

		Print(string.Format("[TBD][Weather] windDirDeg=%1 applied", env.windDirDeg), LogLevel.NORMAL);
	}

	//! Log the latched `settings` values.
	//! @param spectatorPolicy the authored `settings.spectatorPolicy`, empty when absent
	//! @param nightVision the authored `settings.nightVision`
	//! @authority server
	static void ReportSettings(string spectatorPolicy, bool nightVision)
	{
		if (spectatorPolicy.IsEmpty())
			Print("[TBD][Settings] spectatorPolicy=<absent> (SpectatorController keeps current enter rules)", LogLevel.NORMAL);
		else
			Print(string.Format("[TBD][Settings] spectatorPolicy=%1 latched for clients", spectatorPolicy), LogLevel.NORMAL);

		if (nightVision)
			Print("[TBD][Settings] nightVision=true -- NVG gadgets are allowed", LogLevel.NORMAL);
		else
			Print("[TBD][Settings] nightVision=false -- NVG gadgets stripped on spawn", LogLevel.NORMAL);
	}

	//! Spawn hook: schedule a night-vision strip 1500 ms out when the mission forbids night vision,
	//! so the asynchronous loadout dress finishes first.
	//! @param playerId the spawned player
	//! @param controlledEntity the spawned body; null schedules nothing
	//! @authority server
	void OnPlayerSpawnedApplyNvg(int playerId, IEntity controlledEntity)
	{
		if (TBD_Authority.IsClient() || !controlledEntity)
			return;
		if (m_Manager.IsNightVisionAllowed())
			return;

		GetGame().GetCallqueue().CallLater(StripNightVisionForPlayer, 1500, false, playerId);
	}

	//! Delete every night-vision gadget the player's body carries, unless the rule allows them.
	//! @param playerId the player whose controlled body is stripped; a missing body is skipped
	//! @authority server
	protected void StripNightVisionForPlayer(int playerId)
	{
		if (m_Manager.IsNightVisionAllowed())
			return;

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		IEntity body = players.GetPlayerControlledEntity(playerId);
		if (!body)
			return;

		SCR_GadgetManagerComponent gadgetMgr = SCR_GadgetManagerComponent.GetGadgetManager(body);
		if (!gadgetMgr)
			return;

		array<SCR_GadgetComponent> nvgs = gadgetMgr.GetGadgetsByType(EGadgetType.NIGHT_VISION);
		if (!nvgs)
			return;

		int removed = 0;
		foreach (SCR_GadgetComponent g : nvgs)
		{
			if (!g)
				continue;

			IEntity item = g.GetOwner();
			if (!item)
				continue;

			SCR_EntityHelper.DeleteEntityAndChildren(item);
			removed++;
		}

		if (removed > 0)
			Print(string.Format("[TBD][Settings] nightVision=false -- removed %1 NVG gadget(s)", removed), LogLevel.NORMAL);
	}
}
