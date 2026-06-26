//! Partner VOIP bridge hooks — stub implementations until Phase 3.
class TBD_RadioBridgeStub
{
	//------------------------------------------------------------------------------------------------
	static void OnPlayerSpawned(string identityId, array<string> radioNetIds)
	{
		// Partner bridge listens here in Phase 3.
	}

	//------------------------------------------------------------------------------------------------
	static void OnPlayerKilled(string identityId)
	{
	}

	//------------------------------------------------------------------------------------------------
	static void OnRadioRetune(string identityId, string netId)
	{
	}

	//------------------------------------------------------------------------------------------------
	static void OnPTT(string identityId, string netId, bool pressed)
	{
	}

	//------------------------------------------------------------------------------------------------
	static void OnStageChanged(TBD_EGameStage stage)
	{
	}
}
