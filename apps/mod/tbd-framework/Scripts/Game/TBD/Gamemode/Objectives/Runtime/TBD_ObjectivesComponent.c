/**
 * @file TBD_ObjectivesComponent.c
 * @brief Game-mode component that runs the mission's objectives once a second on the server.
 *
 * Role: the objective runner: builds the registry once the mission is valid, and while the stage
 * is LIVE samples who stands on which objective, advances every objective, delivers completion
 * lines to chat and the board to each player's HUD, and logs a met end trigger.
 * Position: attached by `Prefabs/Systems/TBD_GameMode.et`; drives `TBD_ObjectiveRegistry`,
 * `TBD_ObjectiveProgression` and `TBD_ObjectiveHudPublisher`; read by `TBD_ObjectiveHud` for a
 * client's pull.
 * State: the LIVE edge, the end-trigger latch and the two owned helpers, on the server; one
 * repeating `CallLater` tick per world, cancelled in `OnDelete`.  Invariants: nothing advances
 * outside LIVE (safe start is no land grab); progress survives a stage change and is cleared only
 * with the world; one tick re-reads the live player list, so no deferred call carries a
 * recycled player id; every word players see is composed on the server.
 */

//! Editor class of `TBD_ObjectivesComponent`.
[ComponentEditorProps(category: "TBD/Framework", description: "TBD objectives -- capture progress, hold timers and destroy targets from mission JSON; drives the objective win conditions.")]
class TBD_ObjectivesComponentClass : SCR_BaseGameModeComponentClass {}

//! Server-authoritative objective runner on the game mode.
class TBD_ObjectivesComponent : SCR_BaseGameModeComponent
{
	static const int TICK_MS = 1000; //!< Evaluation cadence. 1 Hz -- see the class header.
	static const float TICK_SECONDS = 1.0; //!< `TICK_MS` in seconds, the unit the rules are authored in
	static const string ANNOUNCE_ARMED_KEY = "Obj.armed"; //!< `TBD_AnnounceOnce` key of the once-per-world armed line
	static const string ANNOUNCE_NO_OBJECTIVES_KEY = "Obj.noObjectives"; //!< `TBD_AnnounceOnce` key of the once-per-world no-objectives line

	protected static TBD_ObjectivesComponent s_Instance; //!< the live instance; null outside a world
	protected bool m_bLive; //!< the stage was LIVE on the previous tick, to detect the LIVE edge
	protected bool m_bEndTriggerAnnounced; //!< the met-end-trigger banner has been logged this world
	protected ref TBD_ObjectiveProgression m_Progression; //!< advances the objectives each tick; built in `OnPostInit`
	protected ref TBD_ObjectiveHudPublisher m_HudPublisher; //!< per-player HUD delivery; built in `OnPostInit` on every machine

	//! The component of the current world.
	//! @return the instance, or null outside a world
	static TBD_ObjectivesComponent GetInstance()
	{
		return s_Instance;
	}

	//! Build the helpers, rearm the once-per-world announcements, and arm the 1 Hz tick off a
	//! client: clients hold no mission document, so there is nothing to run there.
	//! @authority server
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		s_Instance = this;
		m_Progression = new TBD_ObjectiveProgression();

		// Built before the client return, so teardown on either machine never meets a null.
		m_HudPublisher = new TBD_ObjectiveHudPublisher();
		TBD_AnnounceOnce.Rearm(ANNOUNCE_ARMED_KEY);
		TBD_AnnounceOnce.Rearm(ANNOUNCE_NO_OBJECTIVES_KEY);

		if (TBD_Authority.IsClient())
			return;

		GetGame().GetCallqueue().CallLater(Tick, TICK_MS, true);
	}

	//! Cancel the tick, clear the registry and hide every HUD. Statics outlive a world when a
	//! mission loads in-process, so without the clear the next mission would inherit these
	//! objectives and could win at kickoff.
	override void OnDelete(IEntity owner)
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(Tick);

		// Clears the rules reader too. Deliberately does NOT clear `TBD_ZoneRegistry` -- that belongs
		// to `TBD_PlayAreaComponent`, and two components racing to tear down one static buys nothing.
		// `TBD_Objective.m_Zone` is a strong reference so the teardown order cannot matter.
		TBD_ObjectiveRegistry.Clear();

		if (m_HudPublisher)
		{
			m_HudPublisher.HideAll();
			m_HudPublisher.ForgetAll();
		}

		s_Instance = null;

		super.OnDelete(owner);
	}

	//! Drop the leaver's HUD record at once, so a recycled player id never inherits their frame.
	//! @authority server
	override void OnPlayerDisconnected(int playerId, KickCauseCode cause, int timeout)
	{
		super.OnPlayerDisconnected(playerId, cause, timeout);

		if (m_HudPublisher)
			m_HudPublisher.Forget(playerId);
	}

	//! One 1 Hz pass: build the registry until the mission is ready, hide the HUDs when the stage
	//! leaves LIVE, and while LIVE sample presence, advance every usable objective, deliver, and
	//! check the end triggers.
	//! @authority server
	protected void Tick()
	{
		// The registry cannot be built until the mission is loaded AND valid, which happens some
		// seconds after this component exists. Retry silently until it does; `Build()` refuses
		// rather than caching an empty registry.
		if (!TBD_ObjectiveRegistry.IsBuilt())
		{
			if (!TBD_ObjectiveRegistry.Build())
				return;

			AnnounceBuilt();
		}

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm || fm.GetStage() != TBD_EGameStage.LIVE)
		{
			if (m_bLive)
				m_HudPublisher.HideAll();
			m_bLive = false;
			return;
		}

		if (!m_bLive)
		{
			m_bLive = true;
			OnEnterLive();
		}

		if (UsableCount() == 0)
			return;

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> connected = new array<int>();
		players.GetPlayers(connected);

		array<ref TBD_Objective> objectives = TBD_ObjectiveRegistry.GetAll();
		if (!objectives)
			return;

		m_Progression.BeginTick();

		SamplePresence(players, connected, objectives);

		foreach (TBD_Objective objective : objectives)
		{
			if (!objective || !objective.m_bUsable)
				continue;

			m_Progression.Advance(objective);
		}

		Deliver(players, connected, objectives);
		CheckEndTriggers();
	}

	//! How many objectives can run, re-read every tick because a destroy objective can go inert
	//! when it is armed.
	protected int UsableCount()
	{
		int total = TBD_ObjectiveRegistry.GetCaptureCount();
		total += TBD_ObjectiveRegistry.GetDestroyCount();
		total += TBD_ObjectiveRegistry.GetHoldCount();
		return total;
	}

	//! Log once per world, when the registry is built, either the armed counts or that the
	//! mission has no usable objective.
	protected void AnnounceBuilt()
	{
		if (UsableCount() == 0)
		{
			TBD_AnnounceOnce.Event(TBD_ObjectiveRegistry.CH, ANNOUNCE_NO_OBJECTIVES_KEY,
				"no usable objective zone in this mission -- objective win conditions cannot fire, and nothing here will run");
			return;
		}

		TBD_AnnounceOnce.Kv(TBD_ObjectiveRegistry.CH, ANNOUNCE_ARMED_KEY, "armed", string.Format("capture=%1 destroy=%2 hold=%3 cadence=%4ms",
			TBD_ObjectiveRegistry.GetCaptureCount(),
			TBD_ObjectiveRegistry.GetDestroyCount(),
			TBD_ObjectiveRegistry.GetHoldCount(),
			TICK_MS));
	}

	//! The round just went LIVE: arm every destroy objective's target search (a search at load
	//! could run before `TBD_MissionWorldApplier.SpawnMissionEntities` and other subsystems place the
	//! targets), and skip the hold ladder rungs at or above each hold's length so a short hold
	//! does not log them all at once.
	protected void OnEnterLive()
	{
		array<ref TBD_Objective> objectives = TBD_ObjectiveRegistry.GetAll();
		if (!objectives)
			return;

		foreach (TBD_Objective objective : objectives)
		{
			if (!objective || !objective.m_bUsable)
				continue;

			if (objective.m_eKind == TBD_EObjectiveKind.DESTROY && !objective.m_bArmed)
			{
				TBD_ObjectiveDestroyTargets.ArmDestroyTargets(objective);
				continue;
			}

			if (objective.m_eKind != TBD_EObjectiveKind.HOLD_UNTIL)
				continue;

			// Skip every announcement mark that is at or above the total hold length.
			while (TBD_ObjectiveProgression.NextHoldMark(objective.m_iHoldMarkIndex) > 0
				&& TBD_ObjectiveProgression.NextHoldMark(objective.m_iHoldMarkIndex) >= objective.m_fHoldSeconds)
			{
				objective.m_iHoldMarkIndex = objective.m_iHoldMarkIndex + 1;
			}
		}
	}

	//! One walk of the player list: count each connected player's living body, with a life left
	//! and a side, into every usable objective whose zone and height band contain its origin.
	protected void SamplePresence(notnull PlayerManager players, notnull array<int> connected, notnull array<ref TBD_Objective> objectives)
	{
		foreach (TBD_Objective objective : objectives)
		{
			if (objective)
				objective.BeginSample();
		}

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();

		foreach (int playerId : connected)
		{
			IEntity body = players.GetPlayerControlledEntity(playerId);
			if (!body)
				continue;

			if (spawn && spawn.IsPlayerDead(playerId))
				continue;

			if (TBD_CharacterUtil.IsDead(body))
				continue;

			string factionKey = TBD_PlayerFaction.Of(spawn, playerId);
			if (factionKey.IsEmpty())
				continue;

			vector origin = body.GetOrigin();
			float px = origin[0];
			float pz = origin[2];

			foreach (TBD_Objective objective : objectives)
			{
				if (!objective || !objective.m_bUsable)
					continue;

				// The zone's footprint test, then its authored height band.
				if (!objective.m_Zone.Contains(px, pz))
					continue;

				if (!TBD_ZoneVolume.ContainsAgl(objective.m_sId, origin))
					continue;

				objective.AddPresence(factionKey);
				objective.m_aPresentPlayers.Insert(playerId);
			}
		}
	}

	//! Send this tick's completion lines to every connected player's chat, then the HUD.
	protected void Deliver(notnull PlayerManager players, notnull array<int> connected, notnull array<ref TBD_Objective> objectives)
	{
		foreach (string broadcast : m_Progression.GetCompletionLines())
		{
			foreach (int playerId : connected)
			{
				TBD_PlayerChat.Tell(playerId, broadcast);
			}
		}

		m_HudPublisher.Replicate(players, connected, objectives);
	}

	//! Log a met objective end trigger, with a banner, once per world. Ending the round belongs to
	//! `TBD_FactionElimination`, whose 2 s tick calls `TBD_ObjectiveRegistry.EvaluateEndTriggers`
	//! on its own cadence, so the banner can precede that end.
	protected void CheckEndTriggers()
	{
		if (m_bEndTriggerAnnounced)
			return;

		string winner;
		string trigger = TBD_ObjectiveRegistry.EvaluateEndTriggers(winner);
		if (trigger.IsEmpty())
			return;

		m_bEndTriggerAnnounced = true;

		TBD_Log.Kv(TBD_ObjectiveRegistry.CH, "endTriggerMet", string.Format("trigger=%1 winner='%2'", trigger, winner));
		TBD_Log.Banner(TBD_ObjectiveRegistry.CH,
			"OBJECTIVE END CONDITION MET but nothing acted on it -- TBD_FactionElimination must call TBD_ObjectiveRegistry.EvaluateEndTriggers(). The round will NOT end on its own.",
			false);
	}

	//! The objective board as this player may see it; the side comes from their assigned slot, so
	//! nothing a client sends can choose it.
	//! @return the board lines
	array<string> BuildBoardForPlayer(int playerId)
	{
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		string factionKey = TBD_PlayerFaction.Of(spawn, playerId);
		return TBD_ObjectiveText.BoardForFaction(TBD_ObjectiveRegistry.GetAll(), factionKey);
	}

	//! Push the current board to one player, ungated; the answer to a client's HUD pull.
	//! @authority server
	void PushHudTo(int playerId)
	{
		if (m_HudPublisher)
			m_HudPublisher.PushTo(playerId);
	}
}
