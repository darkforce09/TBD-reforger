//! T-181.9.2 — how the briefing gets to the player who is allowed to read it, and how the screen
//! is raised and dropped by the stage machine.
//!
//! Three things live here, in the order they matter:
//!   1. `TBD_BriefingReadyRegistry` — SERVER: who has marked ready, and the per-side tally.
//!   2. `SCR_PlayerController` (modded) — the wire: two request/reply RPC pairs, plus the
//!      client-side stage watcher that opens and closes the screen.
//!   3. `TBD_BriefingClient` — CLIENT: the last payload received, and the invokers the screen
//!      listens to so it never has to poll.
//!
//! ── Why the transport hangs off SCR_PlayerController ────────────────────────────────────────
//! The player controller is replicated and owned by exactly one client, so `RplRcver.Owner`
//! delivers a reply to the requester and to nobody else. This is the precedent already in the
//! tree — `TBD_MissionBrowser.c` moves the admin mission list the same way, and says so in its
//! header. A second `modded class SCR_PlayerController` block alongside that one compiles clean
//! (probed: two modded blocks in one addon, methods visible across both).
//!
//! ── Host vs dedicated ──────────────────────────────────────────────────────────────────────
//! On a listen host the requester IS the authority, so the request short-circuits and builds the
//! payload in place rather than round-tripping an RPC to itself. Same code path, both topologies.

//! One readiness record. The mission id is stored so a mission switch invalidates readiness
//! automatically, with no reset hook to forget to call.
class TBD_BriefingReadyEntry
{
	string m_sFaction;
	string m_sMissionId;

	//------------------------------------------------------------------------------------------------
	void TBD_BriefingReadyEntry(string faction, string missionId)
	{
		m_sFaction = faction;
		m_sMissionId = missionId;
	}
}

//! SERVER — readiness bookkeeping for the briefing stage.
//!
//! Deliberately static and local to this slice: readiness is a briefing-screen concept, and
//! parking it here keeps the slice from reaching into `TBD_SpawnManager` or
//! `TBD_FrameworkManager`, both of which other slices hold this wave.
class TBD_BriefingReadyRegistry
{
	//! playerId -> what they were ready FOR. Both fields are part of the validity test, which is
	//! why this is a record and not just a faction string.
	protected static ref map<int, ref TBD_BriefingReadyEntry> m_mReady;

	//------------------------------------------------------------------------------------------------
	protected static void Ensure()
	{
		if (!m_mReady)
			m_mReady = new map<int, ref TBD_BriefingReadyEntry>();
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	static void SetReady(int playerId, string factionKey)
	{
		Ensure();
		m_mReady.Set(playerId, new TBD_BriefingReadyEntry(factionKey, CurrentMissionId()));
	}

	//------------------------------------------------------------------------------------------------
	//! Ready players on one side of the CURRENT mission, counting only those still connected.
	//!
	//! Two staleness rules, both applied at count time rather than through a hook — every
	//! callback that would carry the news (`OnPlayerDisconnected`, the stage machine) lives in
	//! `SCR_BaseGameMode` / `TBD_FrameworkManager` / `TBD_SpawnManager`, which other slices own
	//! this wave. Validating on read keeps the whole fix inside this slice and cannot go stale:
	//!
	//!  1. **Disconnected.** Readiness is keyed on playerId. Someone who marks ready and quits
	//!     would otherwise stay ready forever, reporting "5 of 4 ready" to the players still in
	//!     the briefing.
	//!  2. **Previous mission.** An admin switching missions starts a new briefing with new
	//!     orders. Readiness recorded against the old mission id must not carry over, or half the
	//!     server shows ready for a briefing nobody has read.
	//!
	//! Rows failing either test are pruned as they are found, so the map cannot grow without
	//! bound across a long session of reconnects and mission switches.
	static int CountReadyForFaction(string factionKey)
	{
		Ensure();

		array<int> connected = {};
		GetGame().GetPlayerManager().GetPlayers(connected);

		string missionId = CurrentMissionId();

		array<int> stale = {};
		int n = 0;

		foreach (int playerId, TBD_BriefingReadyEntry entry : m_mReady)
		{
			if (!entry || connected.Find(playerId) < 0 || entry.m_sMissionId != missionId)
			{
				stale.Insert(playerId);
				continue;
			}

			if (entry.m_sFaction == factionKey)
				n++;
		}

		// Enfusion maps remove BY KEY, arrays BY INDEX (TBD_MOD_DESIGN.md §5). Collecting first
		// is what avoids mutating the map during its own foreach.
		foreach (int gone : stale)
		{
			m_mReady.Remove(gone);
		}

		return n;
	}

	//------------------------------------------------------------------------------------------------
	//! Empty when no mission is loaded — which makes every stored entry stale, correctly.
	protected static string CurrentMissionId()
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !doc.meta)
			return string.Empty;

		return doc.meta.id;
	}

	//------------------------------------------------------------------------------------------------
	//! "Ready — 3 of 8 on US Army". Built on the server because only the server can count the
	//! side; shipped as finished text so the client needs no roster of its own.
	//!
	//! Note the tally covers ONE faction — the reader's. A player is never told how ready the
	//! other side is.
	static string BuildTally(string factionKey, string factionName)
	{
		int ready = CountReadyForFaction(factionKey);

		int total = ready;
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (spawn)
			total = spawn.CountClaimedForFaction(factionKey);

		if (total < ready)
			total = ready;

		return string.Format("Ready — %1 of %2 on %3", ready, total, factionName);
	}
}

//! The wire, and the client-side stage watcher.
modded class SCR_PlayerController
{
	//! Client: guards the one-time watcher registration.
	protected bool m_TBD_BriefingWatchStarted;

	//! Client: last stage we acted on, so open/close fire on TRANSITIONS only.
	protected TBD_EGameStage m_TBD_LastStage = TBD_EGameStage.LOADING;

	//------------------------------------------------------------------------------------------------
	override void OnControlledEntityChanged(IEntity from, IEntity to)
	{
		super.OnControlledEntityChanged(from, to);
		TBD_TryStartBriefingWatch();
	}

	//------------------------------------------------------------------------------------------------
	//! Start the client-side stage watcher exactly once, on the local player's controller.
	//!
	//! ── Why a poll and not a callback ──────────────────────────────────────────────────────
	//! `TBD_FrameworkManager.m_Stage` is an `[RplProp(onRplName: "OnStageReplicated")]`, so the
	//! VALUE is replicated and `GetStage()` is correct on a client. But `OnStageReplicated()` is
	//! an empty stub whose body comment reads "Client-side UI reacts to stage changes here" — and
	//! `TBD_FrameworkManager.c` belongs to another slice this wave, so this slice must not write
	//! into it.
	//!
	//! A 500 ms poll of the replicated value is therefore the self-contained way to be correct
	//! today. It costs one enum compare per tick and stops the moment this stops being the local
	//! controller. When the one-line hook lands (see the slice report), delete the poll and call
	//! `TBD_OnStageChanged` from `OnStageReplicated` instead — nothing else changes.
	protected void TBD_TryStartBriefingWatch()
	{
		if (m_TBD_BriefingWatchStarted)
			return;

		if (GetGame().GetPlayerController() != this)
			return; // local client's controller only

		// A dedicated server has no workspace and must never try to open a menu.
		if (!GetGame().GetWorkspace())
			return;

		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		m_TBD_BriefingWatchStarted = true;
		m_TBD_LastStage = TBD_EGameStage.LOADING;

		GetGame().GetCallqueue().CallLater(TBD_TickBriefingStage, 500, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Raise the briefing when the round enters BRIEFING; drop it when the round leaves.
	protected void TBD_TickBriefingStage()
	{
		// Self-cancel rather than leak a tick into the next world/controller.
		if (GetGame().GetPlayerController() != this)
		{
			GetGame().GetCallqueue().Remove(TBD_TickBriefingStage);
			m_TBD_BriefingWatchStarted = false;
			return;
		}

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		TBD_EGameStage stage = fm.GetStage();
		if (stage == m_TBD_LastStage)
			return;

		m_TBD_LastStage = stage;
		TBD_OnStageChanged(stage);
	}

	//------------------------------------------------------------------------------------------------
	//! The single entry point for "the round changed phase" on this client. Kept public and
	//! side-effect-complete so the eventual `OnStageReplicated` hook is a one-line call.
	void TBD_OnStageChanged(TBD_EGameStage stage)
	{
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

	// ── Briefing payload: client asks, server answers, requester alone receives ──────────────

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER. On a listen host the caller is already the authority, so build in
	//! place instead of RPCing ourselves.
	void TBD_RequestBriefing()
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_Briefing);
			return;
		}

		TBD_BriefingPayload payload = TBD_BriefingService.BuildForPlayer(GetPlayerId());
		TBD_BriefingClient.Accept(payload);
	}

	//! @authority server — resolves the caller's side from server-owned state and answers with
	//! that side's briefing ONLY.
	//!
	//! Note the empty parameter list. A client cannot name a faction because there is nowhere to
	//! put one: the side is derived from `TBD_SpawnManager.GetAssignedSlot(GetPlayerId())`, which
	//! no client can influence. That is the second of the three side-discipline properties
	//! documented in `TBD_BriefingData.c`.
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_Briefing()
	{
		int playerId = GetPlayerId();

		TBD_BriefingPayload payload = TBD_BriefingService.BuildForPlayer(playerId);
		string wire = TBD_BriefingService.Serialise(payload);

		Rpc(TBD_RpcDo_Briefing, wire);
	}

	//! @authority owner — executes on the requesting client only (RplRcver.Owner).
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_Briefing(string wire)
	{
		TBD_BriefingClient.Accept(TBD_BriefingService.Parse(wire));
	}

	// ── Readiness ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: "I have read my orders."
	void TBD_ReportReady()
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_Ready);
			return;
		}

		bool accepted;
		string tally = TBD_MarkReady(GetPlayerId(), accepted);
		TBD_BriefingClient.AcceptTally(tally, accepted);
	}

	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_Ready()
	{
		bool accepted;
		string tally = TBD_MarkReady(GetPlayerId(), accepted);
		Rpc(TBD_RpcDo_ReadyTally, tally, accepted);
	}

	//! @authority owner
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_ReadyTally(string tally, bool accepted)
	{
		TBD_BriefingClient.AcceptTally(tally, accepted);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — record readiness and return the caller's OWN side's tally.
	//! Never reports the other side's readiness.
	//!
	//! `accepted` is what actually latches the client's button. Without it a refusal (no slot)
	//! would still leave the client showing a spent, disabled READY button with no way to retry —
	//! the authority, not the optimistic click, decides whether readiness stuck.
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
			return "No slot assigned — claim one in the lobby first.";
		}

		accepted = true;

		TBD_BriefingReadyRegistry.SetReady(playerId, slot.faction);

		string factionName = slot.faction;
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (doc && doc.factions)
		{
			foreach (TBD_MissionFactionStruct faction : doc.factions)
			{
				if (faction && faction.key == slot.faction && !faction.displayName.IsEmpty())
				{
					factionName = faction.displayName;
					break;
				}
			}
		}

		string tally = TBD_BriefingReadyRegistry.BuildTally(slot.faction, factionName);

		TBD_Log.Event(TBD_BriefingService.CH_BRIEFING,
			string.Format("ready player=%1 faction=%2 name='%3' — %4",
				playerId, slot.faction, GetGame().GetPlayerManager().GetPlayerName(playerId), tally));

		return tally;
	}
}

//! CLIENT — the last briefing this player received, and the change notifications the screen
//! binds to.
//!
//! Static because the screen is created and destroyed by the menu manager: parking the payload
//! on the screen would re-request it on every open and lose it on every close. The screen still
//! re-requests on open (the ORBAT moves while players slot up), but it always has something to
//! draw in the meantime.
class TBD_BriefingClient
{
	protected static ref TBD_BriefingPayload m_Payload;
	protected static bool m_bReady;
	protected static string m_sTally;

	//! (TBD_BriefingPayload payload)
	protected static ref ScriptInvoker m_OnPayloadChanged;

	//! (string tally)
	protected static ref ScriptInvoker m_OnReadyStateChanged;

	//------------------------------------------------------------------------------------------------
	static TBD_BriefingPayload GetPayload()
	{
		return m_Payload;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsReady()
	{
		return m_bReady;
	}

	//------------------------------------------------------------------------------------------------
	static string GetReadyTally()
	{
		return m_sTally;
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_BriefingPayload) — lazily created.
	static ScriptInvoker GetOnPayloadChanged()
	{
		if (!m_OnPayloadChanged)
			m_OnPayloadChanged = new ScriptInvoker();

		return m_OnPayloadChanged;
	}

	//------------------------------------------------------------------------------------------------
	//! (string tally) — lazily created.
	static ScriptInvoker GetOnReadyStateChanged()
	{
		if (!m_OnReadyStateChanged)
			m_OnReadyStateChanged = new ScriptInvoker();

		return m_OnReadyStateChanged;
	}

	//------------------------------------------------------------------------------------------------
	//! Ask the server for this player's briefing. No-op without a local controller.
	static void Request()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		pc.TBD_RequestBriefing();
	}

	//------------------------------------------------------------------------------------------------
	static void ReportReady()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		// Optimistic only. The authority's reply (AcceptTally) is what actually latches it, so a
		// refusal releases the button instead of stranding the player on a spent one.
		m_bReady = true;
		pc.TBD_ReportReady();
	}

	//------------------------------------------------------------------------------------------------
	//! A payload arrived (or was built locally on a host).
	static void Accept(TBD_BriefingPayload payload)
	{
		m_Payload = payload;

		if (m_OnPayloadChanged)
			m_OnPayloadChanged.Invoke(m_Payload);
	}

	//------------------------------------------------------------------------------------------------
	//! The authority's verdict on a readiness report. `accepted` false means the server refused
	//! (no slot), so the button is released and the reason shows in the status line.
	static void AcceptTally(string tally, bool accepted)
	{
		m_sTally = tally;
		m_bReady = accepted;

		if (m_OnReadyStateChanged)
			m_OnReadyStateChanged.Invoke(m_sTally);
	}

	//------------------------------------------------------------------------------------------------
	//! New briefing phase: forget the last round's answers.
	static void Reset()
	{
		m_Payload = null;
		m_bReady = false;
		m_sTally = string.Empty;
	}
}
