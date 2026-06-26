[ComponentEditorProps(category: "TBD/Framework", description: "TBD platform game mode manager — mission load and stage machine.")]
class TBD_FrameworkManagerClass : SCR_BaseGameModeComponentClass {}

class TBD_FrameworkManager : SCR_BaseGameModeComponent
{
	protected static TBD_FrameworkManager s_Instance;

	[RplProp(onRplName: "OnStageReplicated")]
	protected TBD_EGameStage m_Stage = TBD_EGameStage.LOADING;

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
	TBD_EGameStage GetStage()
	{
		return m_Stage;
	}

	//------------------------------------------------------------------------------------------------
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

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
			sm.BuildMissionSlotSpawnPoints();

		TBD_RosterLoader.BeginLoad();

		SetStage(TBD_EGameStage.LOBBY);
	}

	//------------------------------------------------------------------------------------------------
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

		if (stage == TBD_EGameStage.LOBBY)
			OnEnterLobby();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnEnterLobby()
	{
		// Preload the available-mission list so admins can browse/switch immediately.
		TBD_MissionListLoader.Refresh();
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
	void OnStageReplicated()
	{
		// Client-side UI reacts to stage changes here.
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
