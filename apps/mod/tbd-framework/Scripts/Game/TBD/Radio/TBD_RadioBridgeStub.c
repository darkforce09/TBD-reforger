//! Partner VOIP bridge hooks — INTENTIONAL no-op stubs until Phase 3 (T-122 T17).
//! Every method below is a deliberate no-op: radio/spawn/kill/PTT/stage events are
//! silently dropped until the partner VOIP bridge lands. Do not treat the empty bodies
//! as bugs — they are placeholders for the Phase 3 integration.
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
