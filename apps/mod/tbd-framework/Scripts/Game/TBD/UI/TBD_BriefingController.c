//! T-181.9.2 — how the briefing gets to the player who is allowed to read it, and how the screen
//! is raised and dropped by the stage machine.
//!
//! Three things live here, in the order they matter:
//!   1. `TBD_BriefingReadyRegistry` — SERVER: who has marked ready, and the per-side tally.
//!   2. `SCR_PlayerController` (modded) — the wire: two request/reply RPC pairs, plus the
//!      client-side stage handler that opens and closes the screen (pushed by
//!      `TBD_FrameworkManager` since T-181.23; it used to poll), plus the T-181.28 JIP catch-up
//!      that reads the stage once when this machine's local controller first appears.
//!   3. `TBD_BriefingClient` — CLIENT: the last payload received, and the invokers the screen
//!      listens to so it never has to poll.
//!
//! ── How a client learns the round is in BRIEFING (all three paths, one handler) ─────────────
//! Every one of these ends at `TBD_OnStageChanged`, which is the only thing that opens or closes
//! the screen. None of them is redundant:
//!   * stage changes while we are here, dedicated  -> `OnStageReplicated` (the proxy callback);
//!   * stage changes while we are here, listen host -> `SetStage` (authority never gets onRplName);
//!   * stage changed BEFORE we arrived              -> `UpdateLocalPlayerController` (T-181.28).
//! The first two are pushes and are dropped when this client has no controller yet; the third is
//! the only one that can run after the fact, and it is what makes a late joiner or a reconnect
//! see the briefing at all.
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

//! The wire, and the client-side stage handler.
modded class SCR_PlayerController
{
	//! Client: last stage we acted on, so open/close fire on TRANSITIONS only.
	protected TBD_EGameStage m_TBD_LastStage = TBD_EGameStage.LOADING;

	//! T-181.28 — the JIP catch-up has already run on this controller.
	//!
	//! Instance state and deliberately NOT static: a reconnecting player is handed a FRESH
	//! controller, and that player is exactly the one the push-only delivery misses. A static latch
	//! would remember the previous connection and skip them.
	protected bool m_TBD_StageCaughtUp;

	//------------------------------------------------------------------------------------------------
	//! T-181.28 — JIP CATCH-UP. Read the stage ONCE, when this machine's local player controller
	//! first appears.
	//!
	//! ── The hole this closes ────────────────────────────────────────────────────────────────
	//! Delivery has been push-only since T-181.23. `TBD_FrameworkManager.NotifyLocalStageUI()` runs
	//! on a stage CHANGE, and it RETURNS SILENTLY when `GetGame().GetPlayerController()` is null.
	//! A client that joins — or reconnects — while the round already sits in BRIEFING therefore
	//! receives nothing at all: there is no change left to push, and `m_TBD_LastStage` starts at
	//! LOADING on the fresh controller so nothing infers one either.
	//!
	//! BRIEFING is admin-driven, so rounds genuinely sit in it, and T-181.32's stage gate can
	//! legitimately hold a round there longer than before. This is the normal case, not an edge.
	//!
	//! ── Why THIS hook, and why it is not the poll T-181.23 deleted ──────────────────────────
	//! `UpdateLocalPlayerController()` is VANILLA's own one-shot latch for "this controller belongs
	//! to the local player". `SCR_PlayerController.OnUpdate` calls it every frame while the static
	//! `s_pLocalPlayerController` is null; the method tests `this == GetGame().GetPlayerController()`,
	//! and on the frame that first holds it sets the static and registers vanilla's own local input
	//! listeners. So the waiting loop is the ENGINE'S, it is already running for every player
	//! controller in every build and topology, and this slice contributes no timer whatsoever.
	//! T-181.23 removed a 500 ms poll that ran for the whole round; nothing here brings it back.
	//! This fires at most once per controller per world.
	//!
	//! It is also as reliable as vanilla's own input binding, because it IS vanilla's own input
	//! binding: if this latch ever failed to fire, the local player would lose Walk, Focus,
	//! Inventory and Tactical Ping.
	//!
	//! ── Chosen over OnOwnershipChanged deliberately ─────────────────────────────────────────
	//! Vanilla states in its own comment that "listen server or SP client will not call
	//! OnOwnershipChanged as there is no transfer of ownership". Hooking that would have fixed
	//! dedicated clients and silently skipped a listen host — this program's recorded both-paths
	//! landmine in its exact original shape. `OnControlledEntityChanged` is worse still: it is
	//! already the addon's ONE vanilla override (`TBD_MissionBrowser.c`), so a second would be a
	//! duplicate method name across modded blocks, and a player refused a body by `flow.jip` never
	//! fires it at all.
	//!
	//! ── Cost where it does nothing ──────────────────────────────────────────────────────────
	//! On a DEDICATED SERVER `s_pLocalPlayerController` never latches, so vanilla keeps calling this
	//! every frame for every controller. The first guard below is therefore a pointer compare that
	//! is always false there — the same compare `super` itself makes two lines later.
	override protected void UpdateLocalPlayerController()
	{
		super.UpdateLocalPlayerController();
		TBD_CatchUpStage();
	}

	//------------------------------------------------------------------------------------------------
	//! @authority client — one read of the server-owned stage, routed through the SAME handler the
	//! push path uses.
	//!
	//! ── Why this delivers T-181.27's ORDERS without mentioning them ─────────────────────────
	//! It calls `TBD_OnStageChanged`, which is the ONE door to the screen, and the screen's own
	//! `OnScreenOpen` re-request is what fetches the payload. A late joiner therefore opens the
	//! briefing by the identical route a punctual one does, and the three `array<string>` orders
	//! parameters ride `TBD_RpcDo_Briefing` exactly as they already do. This path cannot forget
	//! them because it never touches the payload: there is one delivery path, not two that have to
	//! be kept in step.
	//!
	//! ── The flow.jip interaction (T-181.38), decided rather than assumed ────────────────────
	//! A player joining under `flow.jip: "disabled"` is refused a BODY at the deploy door
	//! (`TBD_SpawnManager.OnPlayerAuditSuccess` -> `DENIED-jip-disabled`; `last-stand-at-montfort`
	//! authors it). They still get the briefing, and the catch-up is deliberately NOT conditioned
	//! on that verdict:
	//!
	//!   * `ReclaimDepartedSeat` runs BEFORE the JIP refusal, so a player who dropped out of THIS
	//!     round and came back still holds their seat. They are on the roster, their squad is
	//!     planning around them, and one life guarantees the admin-respawn path stays open. Gating
	//!     the briefing on the deploy verdict would blind precisely the player who most needs it.
	//!   * A walk-up latecomer with no seat is shown nothing either way. `BuildForPlayer` fails
	//!     closed on `GetAssignedSlot` and returns "No slot assigned yet", which the screen renders
	//!     as an empty state that says why — strictly better than a silent void. Side discipline
	//!     therefore does not depend on this gate, so adding one would buy no security.
	//!   * The client holds no mission document and cannot read `flow.jip` at all, so a gate here
	//!     would mean shipping the policy to the client for no benefit.
	//!
	//! The briefing is a READ, not a door. `flow.jip` governs the door.
	protected void TBD_CatchUpStage()
	{
		if (m_TBD_StageCaughtUp)
			return;

		// ── THE load-bearing guard, and it must stay FIRST ──────────────────────────────────
		// Not this machine's player. On a dedicated server `GetPlayerController()` is null, so this
		// is false for every controller and nothing below ever runs. It is the same test vanilla
		// itself makes inside `super` to decide `m_bIsLocalPlayerController`, and the same one
		// `TBD_MissionBrowser` and `TBD_RadioController` already rely on — if it could ever be true
		// on a server, vanilla would be binding local input there.
		if (GetGame().GetPlayerController() != this)
			return;

		// Cheap belt, NOT a dedicated-server test. MEASURED 2026-07-25 on this slice's own gate:
		// `GetGame().GetWorkspace()` is NON-NULL on the headless dedicated server that
		// `world-boot.sh` runs — `TBD_LobbyStage.Start()` passes this very check there and then
		// logs `preset 60 did not open` two poll ticks after LOADING -> LOBBY, with zero players
		// connected. So "no workspace = dedicated server" is FALSE on engine 1.7.0.54, and the
		// guard above is what actually protects this path. Kept only because a null workspace is
		// still a reason not to drive a menu.
		if (!GetGame().GetWorkspace())
			return;

		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (!framework)
			return;

		m_TBD_StageCaughtUp = true;

		TBD_EGameStage stage = framework.GetStage();

		// One line per join, not per frame — the latch above is what makes that true. This is the
		// only operator-visible evidence the catch-up ran, so it names the stage it read.
		TBD_Log.Event(TBD_BriefingService.CH_BRIEFING,
			string.Format("jip catch-up — local controller up, stage=%1",
				typename.EnumToString(TBD_EGameStage, stage)));

		// Idempotent by construction: TBD_OnStageChanged acts on TRANSITIONS only, so reading a
		// stage this controller has already acted on costs nothing and cannot wipe a payload.
		TBD_OnStageChanged(stage);

		if (stage != TBD_EGameStage.BRIEFING)
			return;

		// The screen asks for its own payload on open, so this only runs when it could NOT open —
		// today that is always, because `TBD_UIBriefing` is not in `resourceDatabase.rdb` yet. It
		// warms the client cache so the orders are already there the moment the screen can appear,
		// and it is the only half of this fix that can produce evidence before that Workbench pass.
		if (!TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UIBriefing))
			TBD_BriefingClient.Request();
	}

	//------------------------------------------------------------------------------------------------
	//! The single entry point for "the round changed phase" on this client.
	//!
	//! ── T-181.23: the 500 ms poll is gone ───────────────────────────────────────────────────
	//! T-181.9.2 had to poll `TBD_FrameworkManager.GetStage()` every 500 ms because
	//! `OnStageReplicated()` was an empty stub and `TBD_FrameworkManager.c` belonged to another
	//! slice that wave. That hook is now wired, so the manager PUSHES the stage here instead:
	//!   • proxy path     — `OnStageReplicated()`, the `[RplProp(onRplName:)]` callback;
	//!   • authority path — `SetStage()`, because onRplName never fires on authority, which is
	//!                      what keeps a listen host working now that nothing polls.
	//! Both funnel through `TBD_FrameworkManager.NotifyLocalStageUI()`, which is also where the
	//! "dedicated server has no workspace" guard now lives. **That guard does not do what its name
	//! says** — measured on this slice's gate, `GetGame().GetWorkspace()` is NON-NULL on the
	//! headless dedicated server `world-boot.sh` runs (see `TBD_CatchUpStage`). What actually keeps
	//! a server out of both paths is the null local player controller, which is checked separately
	//! two lines below it there and first in the catch-up here.
	//!
	//! ── T-181.23's blind spot, closed by T-181.28 ────────────────────────────────────────────
	//! BOTH of those are PUSHES, and `NotifyLocalStageUI` drops one silently when this client has
	//! no player controller yet. A joiner or reconnecter arriving into a round that is ALREADY in
	//! BRIEFING is pushed nothing, because nothing changes. `TBD_CatchUpStage` above is the third
	//! caller, and the only one that runs after the fact.
	//!
	//! The transition test below is retained from the poll and is load-bearing, not vestigial: a
	//! redundant replication callback carrying an unchanged value would otherwise call
	//! `TBD_BriefingClient.Reset()` and wipe a payload the player had already received. It is also
	//! what makes the catch-up free to read a stage this controller has already acted on.
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

	// ── Briefing payload: client asks, server answers, requester alone receives ──────────────

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER. On a listen host the caller is already the authority, so build in
	//! place instead of RPCing ourselves.
	//!
	//! The payload built here is handed over WHOLE — it never goes through `Serialise` / `Parse` /
	//! `AdoptOrders`, so the orders arrays `BuildForPlayer` filled are already on it. That is the
	//! same short-circuit the rest of this payload takes, so the two topologies cannot diverge on
	//! orders without diverging on everything else too.
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

		Rpc(TBD_RpcDo_Briefing, wire,
			payload.m_aSituation, payload.m_aMission, payload.m_aExecution);
	}

	//! @authority owner — executes on the requesting client only (RplRcver.Owner).
	//!
	//! ── T-181.27: why the orders are three ARRAYS and not three more wire records ────────
	//! Everything in `wire` is a short structured field that survives being flattened. The written
	//! orders are free prose: newlines are part of the author's meaning, and any delimiter we chose
	//! could legitimately occur in the text. Carrying them as `array<string>` parameters means
	//! there is no delimiter to collide with and no dependence on `string.Split`'s empty-token
	//! behaviour — a RUNTIME property nothing on this lane can settle, and the fragility T-181.26
	//! exists to put a sentinel under. Element i is paragraph i; an empty array is "this side
	//! authored none", which is also what an absent key and a blank string produce.
	//!
	//! This is T-181.19's precedent (`TBD_RpcDo_Markers`), for the same reason.
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_Briefing(string wire, array<string> situation, array<string> mission, array<string> execution)
	{
		TBD_BriefingPayload payload = TBD_BriefingService.Parse(wire);
		TBD_BriefingService.AdoptOrders(payload, situation, mission, execution);
		TBD_BriefingClient.Accept(payload);
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
