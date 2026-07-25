[ComponentEditorProps(category: "TBD/Framework", description: "TBD platform game mode manager — mission load and stage machine.")]
class TBD_FrameworkManagerClass : SCR_BaseGameModeComponentClass {}

class TBD_FrameworkManager : SCR_BaseGameModeComponent
{
	protected static TBD_FrameworkManager s_Instance;

	//! @replicated m_Stage — server-owned; clients react in OnStageReplicated (onRplName hook).
	[RplProp(onRplName: "OnStageReplicated")]
	protected TBD_EGameStage m_Stage = TBD_EGameStage.LOADING;

	//! A5 — roster settle ticks elapsed (500 ms cadence; 4 = the 2 s force-settle deadline).
	protected int m_iRosterSettleTicks;

	//------------------------------------------------------------------------------------------------
	void TBD_FrameworkManager(IEntityComponentSource src, IEntity ent, IEntity parent)
	{
		s_Instance = this;
	}

	//------------------------------------------------------------------------------------------------
	static TBD_FrameworkManager GetInstance()
	{
		return s_Instance;
	}

	//------------------------------------------------------------------------------------------------
	//! True when the CURRENTLY loaded world runs the TBD framework — the guard every
	//! vanilla-suppressing modded class asks before standing vanilla down. Resolved off
	//! the live game mode rather than s_Instance because statics outlive a world inside
	//! one Workbench process (measured landmine), which would leave a stale instance
	//! claiming ownership of a plain vanilla world.
	static bool IsFrameworkWorld()
	{
		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
		if (!gm)
			return false;

		return gm.FindComponent(TBD_FrameworkManager) != null;
	}

	//------------------------------------------------------------------------------------------------
	TBD_EGameStage GetStage()
	{
		return m_Stage;
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — mission load and the stage machine run on the server only.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// Authority only — clients never drive mission load or the stage machine.
		if (RplSession.Mode() == RplMode.Client)
			return;

		SetStage(TBD_EGameStage.LOADING);
		TBD_MissionLoader.BeginLoad();
		GetGame().GetCallqueue().CallLater(TickLoading, 1000, true);
	}

	//------------------------------------------------------------------------------------------------
	protected void TickLoading()
	{
		if (m_Stage != TBD_EGameStage.LOADING)
		{
			GetGame().GetCallqueue().Remove(TickLoading);
			return;
		}

		if (!TBD_MissionLoader.IsLoaded())
			return;

		if (!TBD_MissionLoader.IsValid())
		{
			Print("[TBD] Mission loaded but invalid — staying in LOADING.", LogLevel.ERROR);
			return;
		}

		GetGame().GetCallqueue().Remove(TickLoading);

		TBD_Registry.Load();

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm)
			sm.MaterializeSlotBodies();

		// A5 (determinism): the roster fetch must SETTLE before LOBBY so slot
		// assignment is a pure function of settled state — the old same-tick
		// BeginLoad()+SetStage(LOBBY) let the 250 ms deploy wave race the REST
		// round-trip (roster vs round-robin flipped run-to-run).
		TBD_RosterLoader.BeginLoad();
		m_iRosterSettleTicks = 0;
		GetGame().GetCallqueue().CallLater(TickRosterSettle, 500, true);
	}

	//------------------------------------------------------------------------------------------------
	//! A5 — wait for the roster to settle (loaded or failed), force-settle at the 2 s
	//! deadline, then enter LOBBY exactly once.
	protected void TickRosterSettle()
	{
		m_iRosterSettleTicks++;

		if (!TBD_RosterLoader.IsLoaded() && m_iRosterSettleTicks < 4)
			return;

		GetGame().GetCallqueue().Remove(TickRosterSettle);

		if (!TBD_RosterLoader.IsLoaded())
			TBD_RosterLoader.ForceSettle();

		Print(string.Format("[TBD][Spawn] roster settled=%1 assignments=%2",
			TBD_RosterLoader.GetSettleReason(), TBD_RosterLoader.GetAssignmentCount()));

		SetStage(TBD_EGameStage.LOBBY);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — mutates the replicated m_Stage and calls Replication.BumpMe() to push it.
	void SetStage(TBD_EGameStage stage)
	{
		if (m_Stage == stage)
			return;

		m_Stage = stage;
		Replication.BumpMe();
		TBD_RadioBridgeStub.OnStageChanged(stage);

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (sm)
			sm.OnStageChanged(stage);

		Print("[TBD] Stage → " + typename.EnumToString(TBD_EGameStage, stage));

		// Authority path for the local UI. onRplName does NOT fire on authority, so a listen host
		// needs this explicit call; a dedicated server no-ops inside it. See NotifyLocalStageUI().
		NotifyLocalStageUI();

		if (stage == TBD_EGameStage.LOBBY)
			OnEnterLobby();
		else if (stage == TBD_EGameStage.LIVE)
			OnEnterLive();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnEnterLobby()
	{
		// Preload the available-mission list so admins can browse/switch immediately.
		TBD_MissionListLoader.Refresh();
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.13 — start watching for the round to end. Only armed if the mission actually
	//! declared `faction_eliminated`; a mission that declared nothing runs until an admin ends
	//! it rather than ending on its own.
	//! @authority server
	protected void OnEnterLive()
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!TBD_MissionLoader.HasEndTrigger("faction_eliminated"))
		{
			Print("[TBD][Win] no faction_eliminated trigger in mission — round runs until admin ends it");
			return;
		}

		// 2 s cadence: an elimination is not time-critical, and this walks every claimed slot.
		GetGame().GetCallqueue().CallLater(TickWinConditions, 2000, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Ends the round when a playable faction has no living claimed slots left.
	//!
	//! Guards that matter under ONE LIFE: it only fires while LIVE, it ignores factions that
	//! never had anyone claim in (0 claimed != eliminated, otherwise an unplayed side would end
	//! the round at kickoff), and it needs at least two factions with players so a solo test
	//! session does not instantly end.
	//! @authority server
	protected void TickWinConditions()
	{
		if (m_Stage != TBD_EGameStage.LIVE)
		{
			GetGame().GetCallqueue().Remove(TickWinConditions);
			return;
		}

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		array<ref TBD_MissionFactionStruct> factions = TBD_MissionLoader.GetFactions();
		if (!sm || !factions)
			return;

		int contesting;      // factions that had at least one claimed slot
		int stillAlive;      // ...of those, how many still have a living player
		string lastAlive;

		foreach (TBD_MissionFactionStruct f : factions)
		{
			if (!f || f.key.IsEmpty())
				continue;

			int claimed = sm.CountClaimedForFaction(f.key);
			if (claimed == 0)
				continue;      // never fielded — cannot be "eliminated"

			contesting++;
			if (sm.CountAliveForFaction(f.key) > 0)
			{
				stillAlive++;
				lastAlive = f.key;
			}
		}

		if (contesting < 2)
			return;            // need a real contest before anyone can win

		if (stillAlive > 1)
			return;

		GetGame().GetCallqueue().Remove(TickWinConditions);
		Print(string.Format("[TBD][Win] faction_eliminated — winner=%1 (%2 factions contested)",
			lastAlive, contesting));
		SetStage(TBD_EGameStage.END);
	}

	//------------------------------------------------------------------------------------------------
	//! Current mission's terrain key (empty if no mission loaded).
	protected string GetCurrentTerrain()
	{
		TBD_MissionDocumentStruct m = TBD_MissionLoader.GetMission();
		if (!m || !m.meta)
			return string.Empty;
		return m.meta.terrain;
	}

	//------------------------------------------------------------------------------------------------
	//! Admin: numbered mission list as display lines.
	array<string> BuildMissionListText()
	{
		array<string> lines = new array<string>();
		array<ref TBD_MissionListEntry> entries = TBD_MissionListLoader.GetEntries();
		if (!entries || entries.IsEmpty())
		{
			lines.Insert("TBD: no missions loaded yet — try '#tbd refresh' in a moment.");
			return lines;
		}

		lines.Insert(string.Format("TBD missions (%1) — current terrain: %2", entries.Count(), GetCurrentTerrain()));
		for (int i = 0; i < entries.Count(); i++)
		{
			TBD_MissionListEntry e = entries[i];
			lines.Insert(string.Format("  %1) %2 [%3] %4 slots", i + 1, e.name, e.terrain, e.slotCount));
		}
		return lines;
	}

	//------------------------------------------------------------------------------------------------
	//! Admin: refresh the mission list from the backend.
	void RefreshMissionList()
	{
		TBD_MissionListLoader.Refresh();
	}

	//------------------------------------------------------------------------------------------------
	//! Admin: select a mission by 1-based number — persist it and reload the world.
	string SelectMissionByNumber(int number)
	{
		TBD_MissionListEntry e = TBD_MissionListLoader.GetEntryByNumber(number);
		if (!e)
			return string.Format("TBD: no mission #%1.", number);

		if (e.slotCount <= 0)
			Print(string.Format("[TBD] Selected mission %1 has 0 slots — players will have no spawn.", e.id), LogLevel.WARNING);

		if (!TBD_BackendConfig.SetMissionId(e.id))
			return "TBD: failed to persist mission selection.";

		string target = e.terrain;
		string current = GetCurrentTerrain();

		if (target.IsEmpty() || target == current)
		{
			Print(string.Format("[TBD] Admin selected %1 (%2) — same terrain, restarting scenario.", e.id, target));
			GameStateTransitions.RequestScenarioRestart();
			return string.Format("TBD: loading %1…", e.name);
		}

		string scenario = TBD_ScenarioRouter.GetScenarioForTerrain(target);
		if (scenario.IsEmpty())
			return string.Format("TBD: no scenario for terrain '%1' yet (mission stays selected for next %1 load).", target);

		Print(string.Format("[TBD] Admin selected %1 (%2) — switching scenario to %3.", e.id, target, scenario));
		GameStateTransitions.RequestScenarioChangeTransition(scenario, string.Empty, TBD_ScenarioRouter.GetAddonList());
		return string.Format("TBD: switching to %1 on %2…", e.name, target);
	}

	//------------------------------------------------------------------------------------------------
	//! Admin: repoint the backend URL (and optionally token), then refresh the list.
	string SetBackend(string url, string token)
	{
		if (url.IsEmpty())
			return "Usage: #tbd backend <url> [token]";
		if (!TBD_BackendConfig.SetBackend(url, token))
			return "TBD: failed to set backend.";
		TBD_MissionListLoader.Refresh();
		return string.Format("TBD: backend set to %1 — refreshing list…", url);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority client — onRpl hook for m_Stage (RplProp onRplName); runs on clients on replication.
	void OnStageReplicated()
	{
		NotifyLocalStageUI();
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.23 — hand the current stage to THIS machine's local player UI, if it has one.
	//!
	//! Called from two places on purpose, because either one alone is wrong:
	//!   • `OnStageReplicated()` — the PROXY path. `[RplProp(onRplName:)]` fires only on the proxy
	//!     (TBD_MOD_DESIGN.md §5), so this is how a dedicated-server client hears about a stage
	//!     change at all.
	//!   • `SetStage()` — the AUTHORITY path. On a listen host the authority IS the player, and
	//!     authority never receives its own onRplName callback. Without this call the host's
	//!     briefing screen would never open — which is exactly the regression the 500 ms poll this
	//!     replaces was papering over, since a poll reads `GetStage()` on both topologies.
	//!
	//! A DEDICATED server no-ops here: it has no workspace and no local player controller, so both
	//! guards below fail and nothing happens. The server-side stage machine is untouched — this
	//! method only ever drives local UI, and never feeds back into replication.
	protected void NotifyLocalStageUI()
	{
		// No workspace = dedicated server. It must never try to drive a menu.
		if (!GetGame().GetWorkspace())
			return;

		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		// Idempotent on the receiving side: TBD_OnStageChanged acts on TRANSITIONS only, so a
		// redundant replication callback cannot re-open the briefing or wipe a received payload.
		pc.TBD_OnStageChanged(m_Stage);
	}

	//------------------------------------------------------------------------------------------------
	//! Admin chat command entry — `#stage next` / `#stage LOBBY` etc.
	void HandleAdminStageCommand(string args)
	{
		if (args.IsEmpty())
			return;

		if (args == "next")
		{
			int next = m_Stage + 1;
			if (next > TBD_EGameStage.DEBRIEF)
				return;
			SetStage(next);
			return;
		}

		// Named stage: LOBBY, LIVE, …
		for (int i = TBD_EGameStage.LOADING; i <= TBD_EGameStage.DEBRIEF; i++)
		{
			string name = typename.EnumToString(TBD_EGameStage, i);
			if (args == name)
			{
				SetStage(i);
				return;
			}
		}
	}
}
