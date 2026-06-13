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

		GetGame().GetCallqueue().Remove(TickLoading);
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
		Print("[TBD] Stage → " + typename.EnumToString(TBD_EGameStage, stage));

		if (stage == TBD_EGameStage.LOBBY)
			OnEnterLobby();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnEnterLobby()
	{
		TBD_Registry.Load();
		// ORBAT enforcement, briefing timers, and admin commands land in Phase 1 follow-ups.
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
