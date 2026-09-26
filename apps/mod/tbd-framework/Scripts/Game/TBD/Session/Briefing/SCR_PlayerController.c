/**
 * @file SCR_PlayerController.c
 * @brief Briefing delivery on the player controller: two request and reply RPC pairs and the stage handler.
 *
 * Role: carries each player's briefing and ready tally between the server and that player's
 * client, and opens or closes the Briefing screen when the round changes stage.
 * Position: TBD_BriefingClient calls TBD_RequestBriefing and TBD_ReportReady; TBD_FrameworkManager pushes
 * TBD_OnStageChanged; vanilla's UpdateLocalPlayerController triggers the joining-client catch-up;
 * payloads come from TBD_BriefingService and TBD_BriefingWire, readiness from
 * TBD_BriefingReadyRegistry.
 * State: per controller, the last stage acted on and the catch-up latch, on the owning client.
 * Invariants: TBD_OnStageChanged is the only opener and closer of the screen and acts on stage
 * transitions only; the side is derived on the server from the caller's assigned slot and no RPC
 * carries a faction; RplRcver.Owner answers the requester alone; on a listen host the requester is
 * the authority, so requests run in place and the payload is handed over whole; the method names
 * differ from every other modded block of SCR_PlayerController, which share one namespace.
 */

//! Briefing RPCs and the client stage handler of the player controller.
modded class SCR_PlayerController
{
	protected TBD_EGameStage m_TBD_LastStage = TBD_EGameStage.LOADING; //!< last stage this client acted on; default LOADING
	protected bool m_TBD_StageCaughtUp; //!< the joining-client stage read has run on this controller; per instance, so a reconnect's fresh controller reads again

	//! Vanilla calls this every frame until the local player's controller latches, and it also
	//! binds the local input there. Adds the one-time stage catch-up, so a player who joins or
	//! reconnects while the round already sits in BRIEFING still sees the screen. On a dedicated
	//! server the local controller never latches and the catch-up's first guard returns.
	override protected void UpdateLocalPlayerController()
	{
		super.UpdateLocalPlayerController();
		TBD_CatchUpStage();
	}

	//! Read the server-owned stage once, when this machine's local controller first appears, and
	//! route it through TBD_OnStageChanged, the same handler the push path uses. The screen's own
	//! open requests the payload. The read is not gated on `flow.jip`: a player refused a body still
	//! reads their side's briefing, and a player with no slot receives the "No slot assigned yet"
	//! state from TBD_BriefingService.
	//! @authority client
	protected void TBD_CatchUpStage()
	{
		if (m_TBD_StageCaughtUp)
			return;

		// Not this machine's player. On a dedicated server `GetPlayerController()` is null, so this
		// is false for every controller and nothing below ever runs. It is the same test vanilla
		// itself makes inside `super` to decide `m_bIsLocalPlayerController`, and the same one
		// `TBD_MissionBrowser` and `TBD_RadioController` already rely on -- if it could ever be true
		// on a server, vanilla would be binding local input there.
		if (GetGame().GetPlayerController() != this)
			return;

		// Not a dedicated-server test: the headless dedicated server has a workspace. The
		// `GetPlayerController() != this` guard above protects this path; a null workspace is
		// still a reason not to drive a menu.
		if (!GetGame().GetWorkspace())
			return;

		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (!framework)
			return;

		m_TBD_StageCaughtUp = true;

		TBD_EGameStage stage = framework.GetStage();

		// One line per join, not per frame -- the latch above is what makes that true. This is the
		// only operator-visible evidence the catch-up ran, so it names the stage it read.
		TBD_Log.Event(TBD_BriefingService.CH_BRIEFING,
			string.Format("jip catch-up -- local controller up, stage=%1",
				typename.EnumToString(TBD_EGameStage, stage)));

		// Idempotent by construction: TBD_OnStageChanged acts on TRANSITIONS only, so reading a
		// stage this controller has already acted on costs nothing and cannot wipe a payload.
		TBD_OnStageChanged(stage);

		if (stage != TBD_EGameStage.BRIEFING)
			return;

		// The screen requests its own payload on open, so this runs only when the screen did not
		// open; it warms the client cache so the orders are there when the screen appears.
		if (!TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UIBriefing))
			TBD_BriefingClient.Request();
	}

	//! The client's one entry point for a stage change. BRIEFING resets TBD_BriefingClient and opens
	//! the screen; any other stage closes it through TBD_MenuStack. Callers:
	//! TBD_FrameworkManager.NotifyLocalStageUI on a proxy (the replication callback) and on the
	//! authority (SetStage), and TBD_CatchUpStage for a late joiner. A repeated stage does nothing,
	//! so a redundant callback cannot wipe a received payload.
	//! @param stage the stage the round is in
	//! @authority client
	void TBD_OnStageChanged(TBD_EGameStage stage)
	{
		if (stage == m_TBD_LastStage)
			return;

		m_TBD_LastStage = stage;

		if (stage == TBD_EGameStage.BRIEFING)
		{
			TBD_BriefingClient.Reset();
			TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UIBriefing);
			return;
		}

		// Any other phase: the briefing is over. Closing through the stack hands input and focus
		// back correctly (TBD_MenuStack invariants 3 and 4).
		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UIBriefing))
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UIBriefing);
	}

	//! Ask for this player's briefing. On a client it sends TBD_RpcAsk_Briefing; on the authority
	//! (listen host) it builds the payload in place and hands it to TBD_BriefingClient whole,
	//! orders included, without the wire.
	//! @authority owner
	void TBD_RequestBriefing()
	{
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_Briefing);
			return;
		}

		TBD_BriefingPayload payload = TBD_BriefingService.BuildForPlayer(GetPlayerId());
		TBD_BriefingClient.Accept(payload);
	}

	//! Build the caller's briefing and send it back to the owner. The request carries no
	//! parameters, so a client cannot name a side; the side comes from the caller's assigned slot.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_Briefing()
	{
		int playerId = GetPlayerId();

		TBD_BriefingPayload payload = TBD_BriefingService.BuildForPlayer(playerId);
		string wire = TBD_BriefingWire.Serialise(payload);

		Rpc(TBD_RpcDo_Briefing, wire,
			payload.m_aSituation, payload.m_aMission, payload.m_aExecution);
	}

	//! Receive this player's briefing: parse the wire, attach the orders paragraphs carried as
	//! arrays (element i is paragraph i; empty means none authored) and store it.
	//! @param wire the TBD_BriefingWire string
	//! @param situation the situation paragraphs
	//! @param mission the mission paragraphs
	//! @param execution the execution paragraphs
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_Briefing(string wire, array<string> situation, array<string> mission, array<string> execution)
	{
		TBD_BriefingPayload payload = TBD_BriefingWire.Parse(wire);
		TBD_BriefingWire.AdoptOrders(payload, situation, mission, execution);
		TBD_BriefingClient.Accept(payload);
	}

	//! Report "I have read my orders". On a client it sends TBD_RpcAsk_Ready; on the authority it
	//! records readiness in place and delivers the tally.
	//! @authority owner
	void TBD_ReportReady()
	{
		if (TBD_Authority.IsClient())
		{
			Rpc(TBD_RpcAsk_Ready);
			return;
		}

		bool accepted;
		string tally = TBD_MarkReady(GetPlayerId(), accepted);
		TBD_BriefingClient.AcceptTally(tally, accepted);
	}

	//! Record the caller's readiness and send their own side's tally back to the owner.
	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_Ready()
	{
		bool accepted;
		string tally = TBD_MarkReady(GetPlayerId(), accepted);
		Rpc(TBD_RpcDo_ReadyTally, tally, accepted);
	}

	//! Receive the tally and whether the server accepted the readiness.
	//! @param tally the tally text, or the refusal reason
	//! @param accepted false when the server refused (no slot)
	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_ReadyTally(string tally, bool accepted)
	{
		TBD_BriefingClient.AcceptTally(tally, accepted);
	}

	//! Record readiness for `playerId` and return their own side's tally; never another side's.
	//! `accepted` is what latches the client's button, so a refusal releases it.
	//! @param playerId the reporting player
	//! @param accepted set true when the player holds a slot and readiness was recorded
	//! @return the tally text, or the refusal reason when the player holds no slot
	//! @authority server
	protected string TBD_MarkReady(int playerId, out bool accepted)
	{
		accepted = false;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		TBD_MissionSlotStruct slot;
		if (spawn)
			slot = spawn.GetAssignedSlot(playerId);

		if (!slot)
		{
			// Fail closed: no seat, no side, nothing to be ready for.
			return "No slot assigned -- claim one in the lobby first.";
		}

		accepted = true;

		TBD_BriefingReadyRegistry.SetReady(playerId, slot.faction);

		string factionName = TBD_MissionFactionNames.DisplayName(TBD_MissionLoader.GetMission(), slot.faction);

		string tally = TBD_BriefingReadyRegistry.BuildTally(slot.faction, factionName);

		TBD_Log.Event(TBD_BriefingService.CH_BRIEFING,
			string.Format("ready player=%1 faction=%2 name='%3' -- %4",
				playerId, slot.faction, GetGame().GetPlayerManager().GetPlayerName(playerId), tally));

		return tally;
	}
}
