//! T-181.9.1 — the wire, the client-side cache, and the stage watcher that raises the lobby.
//!
//! Three things live here, in the order they matter:
//!   1. `SCR_PlayerController` (modded) — three asks, ONE reply. The transport.
//!   2. `TBD_LobbyClient` — CLIENT: the roster the screen draws, the optimistic edit, and the
//!      reconciliation against the authority's answer.
//!   3. `TBD_LobbyStage` — CLIENT: watches the replicated game stage and raises/drops the screen.
//!
//! ── Why the transport hangs off SCR_PlayerController ────────────────────────────────────────
//! The player controller is replicated and owned by exactly one client, so `RplRcver.Owner`
//! delivers a reply to the requester and to nobody else. Two precedents already in the tree do
//! this: `TBD_MissionBrowser.c` (admin mission list) and `TBD_BriefingController.c` (briefing
//! payload). This is the third `modded class SCR_PlayerController` block in the addon — see the
//! slice report; the compile gate proves it compiles, nothing here can prove how three blocks
//! behave at runtime.
//!
//! ── ONE reply, not three ───────────────────────────────────────────────────────────────────
//! Claim, release, deploy and a plain refresh all answer with the SAME message: a whole fresh
//! roster, optionally carrying a `V` verdict record saying what just happened. That is what makes
//! optimistic reconciliation a **wholesale replace** instead of a merge — the client never has to
//! reason about whether a partial update arrived before or after the refresh that overlapped it.
//! Out-of-order replies converge on the truth because every one of them IS the truth.
//!
//! ── Host vs dedicated ──────────────────────────────────────────────────────────────────────
//! On a listen host the requester IS the authority, so the request short-circuits and builds the
//! payload in place rather than round-tripping an RPC to itself. It still goes through
//! `Serialise` -> `Accept` -> `Parse`, so both topologies run one code path and a serialisation
//! bug cannot hide on the host.
modded class SCR_PlayerController
{
	// ── Roster ──────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: "what does the board look like right now".
	void TBD_RequestLobbyRoster()
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_LobbyRoster);
			return;
		}

		TBD_LobbyClient.Accept(TBD_LobbyService.Serialise(TBD_LobbyService.BuildForPlayer(GetPlayerId())));
	}

	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_LobbyRoster()
	{
		Rpc(TBD_RpcDo_LobbyRoster, TBD_LobbyService.Serialise(TBD_LobbyService.BuildForPlayer(GetPlayerId())));
	}

	//! @authority owner — the ONE reply. Executes on the requesting client only (RplRcver.Owner).
	//! @rpc Reliable Owner
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void TBD_RpcDo_LobbyRoster(string wire)
	{
		TBD_LobbyClient.Accept(wire);
	}

	// ── Claim ───────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: "I want this seat."
	//!
	//! The slot key is client-supplied and that is safe: `ClaimSlot` resolves it against the
	//! mission's own slot list and refuses anything it does not recognise, anything held by
	//! somebody else, and anything asked for by a dead player. A modified client can name any
	//! string it likes and gets a refusal.
	void TBD_RequestClaimSlot(string slotKey)
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_ClaimSlot, slotKey);
			return;
		}

		TBD_LobbyClient.Accept(TBD_BuildClaimReply(GetPlayerId(), slotKey));
	}

	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_ClaimSlot(string slotKey)
	{
		Rpc(TBD_RpcDo_LobbyRoster, TBD_BuildClaimReply(GetPlayerId(), slotKey));
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — do it, then answer with the board as it stands AFTER doing it.
	//! Building the roster second is what makes a rejection self-explaining: the same message that
	//! says "no" already shows who does hold the seat.
	protected string TBD_BuildClaimReply(int playerId, string slotKey)
	{
		bool accepted;
		string reason = TBD_LobbyService.ApplyClaim(playerId, slotKey, accepted);

		TBD_LobbyRoster roster = TBD_LobbyService.BuildForPlayer(playerId);
		roster.m_sAction = TBD_LobbyService.ACTION_CLAIM;
		roster.m_bActionOk = accepted;
		roster.m_sActionReason = reason;
		roster.m_sActionKey = slotKey;

		return TBD_LobbyService.Serialise(roster);
	}

	// ── Release ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: "I have changed my mind." No argument — you can only give up the
	//! seat you hold, and the server already knows which one that is.
	void TBD_RequestReleaseSlot()
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_ReleaseSlot);
			return;
		}

		TBD_LobbyClient.Accept(TBD_BuildReleaseReply(GetPlayerId()));
	}

	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_ReleaseSlot()
	{
		Rpc(TBD_RpcDo_LobbyRoster, TBD_BuildReleaseReply(GetPlayerId()));
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected string TBD_BuildReleaseReply(int playerId)
	{
		bool accepted;
		string reason = TBD_LobbyService.ApplyRelease(playerId, accepted);

		TBD_LobbyRoster roster = TBD_LobbyService.BuildForPlayer(playerId);
		roster.m_sAction = TBD_LobbyService.ACTION_RELEASE;
		roster.m_bActionOk = accepted;
		roster.m_sActionReason = reason;

		return TBD_LobbyService.Serialise(roster);
	}

	// ── Deploy ──────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! CLIENT (owner) -> SERVER: the one consequential click. Takes no argument for the same
	//! reason release does not — the seat is server state, and a client cannot name one.
	void TBD_RequestDeploy()
	{
		if (RplSession.Mode() == RplMode.Client)
		{
			Rpc(TBD_RpcAsk_Deploy);
			return;
		}

		TBD_LobbyClient.Accept(TBD_BuildDeployReply(GetPlayerId()));
	}

	//! @authority server
	//! @rpc Reliable Server
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void TBD_RpcAsk_Deploy()
	{
		Rpc(TBD_RpcDo_LobbyRoster, TBD_BuildDeployReply(GetPlayerId()));
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server
	protected string TBD_BuildDeployReply(int playerId)
	{
		bool accepted;
		string resultName;
		string reason = TBD_LobbyService.ApplyDeploy(playerId, accepted, resultName);

		TBD_LobbyRoster roster = TBD_LobbyService.BuildForPlayer(playerId);
		roster.m_sAction = TBD_LobbyService.ACTION_DEPLOY;
		roster.m_bActionOk = accepted;
		roster.m_sActionReason = reason;
		roster.m_sActionKey = resultName;

		return TBD_LobbyService.Serialise(roster);
	}
}

//! CLIENT — the roster this player is looking at, the optimistic edit, and the reconciliation.
//!
//! Static because the screen is created and destroyed by the menu manager: parking the roster on
//! the screen would lose it on every close and re-request it on every open with nothing to draw in
//! the meantime.
//!
//! ── Optimistic feedback, authoritative truth ────────────────────────────────────────────────
//! A claim is reflected **immediately** — the row goes to your name before a single packet leaves
//! the machine — because under ONE LIFE the moment between clicking and being sure is the worst
//! moment of the whole lobby, and a spinner does not shorten it.
//!
//! It is reconciled by **replacement**, not by merge: `Accept()` throws the local roster away and
//! rebuilds from the server's string. So an optimistic claim survives exactly as long as the
//! server agrees with it, and a rejection reverts the row in the same message that explains why.
//! There is no state machine to get stuck in, and a dropped or reordered reply cannot leave a
//! phantom claim on screen — the next refresh (2 s, at worst) overwrites it regardless.
class TBD_LobbyClient
{
	//! How long a refused seat stays marked. Long enough to read the reason under it, short enough
	//! that it does not become part of the furniture.
	static const int REJECT_HIGHLIGHT_MS = 5000;

	protected static ref TBD_LobbyRoster m_Roster;

	//! Non-blocking feedback line. Never a modal — design law.
	protected static string m_sStatus;

	//! The seat the authority most recently refused us, or empty.
	protected static string m_sRejectedKey;

	//! Set once the server has accepted a deploy, so the screen can stand down.
	protected static bool m_bDeployed;

	//! A deploy request is in flight. Lives here rather than on the screen so the footer derives
	//! the button's enabled state from ONE place — a screen that disabled its own button would be
	//! re-enabled by the very next roster refresh.
	protected static bool m_bDeployPending;

	//! ── The in-flight intent, and why it has to exist ────────────────────────────────────────
	//! The screen re-asks for the roster every 2 s, so a refresh REQUESTED BEFORE the click can
	//! land AFTER it. Without this, the sequence is: click (row goes to your name) -> stale
	//! refresh arrives and replaces everything (row goes back to OPEN) -> claim verdict arrives
	//! (row goes to your name again). The player sees their seat flicker away and back, which
	//! reads exactly like losing it.
	//!
	//! So an optimistic edit is remembered until the server rules ON THAT EXACT INTENT, and any
	//! roster arriving in the meantime is overlaid with it. Keyed on the slot, so double-clicking
	//! two different seats resolves in order instead of the first verdict cancelling the second
	//! click's optimism.
	protected static string m_sPendingClaimKey;
	protected static bool m_bPendingRelease;

	//! (TBD_LobbyRoster roster)
	protected static ref ScriptInvoker m_OnRosterChanged;

	//------------------------------------------------------------------------------------------------
	static TBD_LobbyRoster GetRoster()
	{
		return m_Roster;
	}

	//------------------------------------------------------------------------------------------------
	static string GetStatus()
	{
		return m_sStatus;
	}

	//------------------------------------------------------------------------------------------------
	static string GetRejectedKey()
	{
		return m_sRejectedKey;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsDeployed()
	{
		return m_bDeployed;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsDeployPending()
	{
		return m_bDeployPending;
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_LobbyRoster) — lazily created. Fires on every change, whether it came from the server
	//! or from an optimistic local edit, so the screen has exactly one thing to listen to.
	static ScriptInvoker GetOnRosterChanged()
	{
		if (!m_OnRosterChanged)
			m_OnRosterChanged = new ScriptInvoker();

		return m_OnRosterChanged;
	}

	// ── Requests ────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Ask the server for the board. No-op without a local controller.
	static void Request()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		pc.TBD_RequestLobbyRoster();
	}

	//------------------------------------------------------------------------------------------------
	//! Take a seat. Reflected locally first, then asked for.
	static void Claim(string slotKey)
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		m_sPendingClaimKey = slotKey;
		m_bPendingRelease = false;

		ApplyOptimisticClaim(slotKey);
		pc.TBD_RequestClaimSlot(slotKey);
	}

	//------------------------------------------------------------------------------------------------
	//! Give the seat back.
	static void Release()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		m_sPendingClaimKey = string.Empty;
		m_bPendingRelease = true;

		ApplyOptimisticRelease();
		pc.TBD_RequestReleaseSlot();
	}

	//------------------------------------------------------------------------------------------------
	//! The one consequential click. Deliberately NOT optimistic: a deploy that turns out to have
	//! been refused must not have already torn the lobby down, or a refused player is left staring
	//! at a world they were never put into with no way back to the picker.
	static void Deploy()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		// Latch BEFORE the request and announce it, so the button is already dead when the click
		// finishes. A double-click on the one irreversible action must not become two deploys.
		m_bDeployPending = true;
		SetStatus("Deploying…");
		Changed();

		pc.TBD_RequestDeploy();
	}

	// ── Optimistic edits ────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Move our own flag onto `slotKey` locally. Refuses to touch a seat that is not open, so the
	//! optimistic path can never show something the authority would obviously refuse — and so
	//! re-applying it over a fresher roster that already shows the seat lost is a no-op.
	protected static bool MutateClaim(string slotKey)
	{
		if (!m_Roster)
			return false;

		TBD_LobbySlot target = m_Roster.FindSlot(slotKey);
		if (!target || !target.IsOpen())
			return false;

		TBD_LobbySlot previous = m_Roster.FindSlot(m_Roster.m_sOwnKey);
		if (previous)
		{
			previous.m_bIsOwn = false;
			previous.m_sState = TBD_LobbyService.STATE_OPEN;
			previous.m_sHolder = string.Empty;
		}

		target.m_bIsOwn = true;
		target.m_sState = TBD_LobbyService.STATE_HELD;
		target.m_sHolder = LocalPlayerName();

		m_Roster.Recount();
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool MutateRelease()
	{
		if (!m_Roster)
			return false;

		TBD_LobbySlot own = m_Roster.FindSlot(m_Roster.m_sOwnKey);
		if (!own)
			return false;

		own.m_bIsOwn = false;
		own.m_sState = TBD_LobbyService.STATE_OPEN;
		own.m_sHolder = string.Empty;

		m_Roster.Recount();
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static void ApplyOptimisticClaim(string slotKey)
	{
		if (!MutateClaim(slotKey))
			return;

		m_sRejectedKey = string.Empty;
		SetStatus(string.Format("Taking %1…", m_Roster.m_sOwnLabel));
		Changed();
	}

	//------------------------------------------------------------------------------------------------
	protected static void ApplyOptimisticRelease()
	{
		if (!MutateRelease())
			return;

		SetStatus("Giving the seat up…");
		Changed();
	}

	//------------------------------------------------------------------------------------------------
	//! Re-state, on top of a roster that just arrived, whatever the server has not yet answered.
	//! Silent by design: the caller owns the status line and the change notification, and this
	//! runs on every incoming message.
	protected static void ReapplyPendingIntent()
	{
		if (!m_sPendingClaimKey.IsEmpty())
		{
			MutateClaim(m_sPendingClaimKey);
			return;
		}

		if (m_bPendingRelease)
			MutateRelease();
	}

	// ── Reconciliation ──────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! The authority has spoken. Replace everything, then say what it said.
	static void Accept(string wire)
	{
		TBD_LobbyRoster incoming = TBD_LobbyService.Parse(wire);
		if (!incoming)
			return;

		m_Roster = incoming;

		// Has the authority ruled on the intent we are still holding? Only the MATCHING verdict
		// retires it. A verdict for a seat the player has already moved on from is stale: it must
		// neither retire the current intent nor put its reason in the status line, or clicking
		// two seats quickly would leave "someone got there first" under a seat you now hold.
		bool current = true;

		if (incoming.m_sAction == TBD_LobbyService.ACTION_CLAIM)
		{
			if (incoming.m_sActionKey == m_sPendingClaimKey)
				m_sPendingClaimKey = string.Empty;
			else
				current = false;
		}
		else if (incoming.m_sAction == TBD_LobbyService.ACTION_RELEASE)
		{
			m_bPendingRelease = false;
		}

		// Anything the server has NOT yet ruled on is re-applied on top of its answer, so a stale
		// refresh cannot flicker a claim away and back. See m_sPendingClaimKey.
		ReapplyPendingIntent();

		if (incoming.m_sAction.IsEmpty() || !current)
		{
			// A plain refresh. It must NOT clear a rejection message the player has not had time
			// to read — the 2 s poll would otherwise wipe every explanation half a second after it
			// appeared. The rejection expires on its own timer instead.
			Changed();
			return;
		}

		if (incoming.m_sAction == TBD_LobbyService.ACTION_DEPLOY)
		{
			AcceptDeployVerdict(incoming);
			return;
		}

		if (incoming.m_bActionOk)
		{
			// Deliberately silent. The screen derives a better line from the roster it just
			// received ("You hold ALPHA · SL") than any fixed acknowledgement could be, and a
			// sticky "Seat taken." would still be sitting there ten minutes later.
			ClearRejection();
			SetStatus(string.Empty);
			Changed();
			return;
		}

		// REFUSED. Mark the seat, say why, and let the replaced roster show who actually has it.
		m_sRejectedKey = incoming.m_sActionKey;
		SetStatus(incoming.m_sActionReason);

		GetGame().GetCallqueue().Remove(ClearRejection);
		GetGame().GetCallqueue().CallLater(ClearRejection, REJECT_HIGHLIGHT_MS, false);

		Changed();
	}

	//------------------------------------------------------------------------------------------------
	//! A deploy verdict is the only one that ends the screen, so it is handled apart from the
	//! claim/release path rather than sharing its "ok = quietly proceed" shape.
	protected static void AcceptDeployVerdict(TBD_LobbyRoster incoming)
	{
		// Released either way. A RETRY ("the server is not ready to deploy you yet") has to leave
		// the button live again, or the player is stranded on a dead control with a seat they
		// cannot use.
		m_bDeployPending = false;
		SetStatus(incoming.m_sActionReason);

		if (incoming.m_bActionOk)
			m_bDeployed = true;

		Changed();
	}

	//------------------------------------------------------------------------------------------------
	//! The refusal has had its five seconds. Drop the mark AND the sentence — leaving the sentence
	//! behind would keep telling the player they were beaten to a seat long after they took
	//! another one.
	protected static void ClearRejection()
	{
		if (m_sRejectedKey.IsEmpty())
			return;

		m_sRejectedKey = string.Empty;
		m_sStatus = string.Empty;
		Changed();
	}

	//------------------------------------------------------------------------------------------------
	static void SetStatus(string status)
	{
		m_sStatus = status;
	}

	//------------------------------------------------------------------------------------------------
	protected static void Changed()
	{
		if (m_OnRosterChanged)
			m_OnRosterChanged.Invoke(m_Roster);
	}

	//------------------------------------------------------------------------------------------------
	//! What to write in our own row while the claim is in flight. The server will overwrite it
	//! with the same name a moment later; this is only ever on screen for one round trip.
	protected static string LocalPlayerName()
	{
		PlayerController pc = GetGame().GetPlayerController();
		PlayerManager players = GetGame().GetPlayerManager();
		if (!pc || !players)
			return "You";

		string name = players.GetPlayerName(pc.GetPlayerId());
		if (name.IsEmpty())
			return "You";

		return name;
	}

	//------------------------------------------------------------------------------------------------
	//! New lobby phase: forget the last round's answers, and drop any pending rejection timer so
	//! it cannot fire into a screen that no longer exists.
	static void Reset()
	{
		GetGame().GetCallqueue().Remove(ClearRejection);

		m_Roster = null;
		m_sStatus = string.Empty;
		m_sRejectedKey = string.Empty;
		m_bDeployed = false;
		m_bDeployPending = false;
		m_sPendingClaimKey = string.Empty;
		m_bPendingRelease = false;
	}
}

//! CLIENT — watches the replicated game stage and raises/drops the lobby.
//!
//! ── Why a poll, and why it is hosted on a game-mode component ──────────────────────────────
//! `TBD_FrameworkManager.m_Stage` is an `[RplProp(onRplName: "OnStageReplicated")]`, so the VALUE
//! is replicated and `GetStage()` is correct on a client. But `OnStageReplicated()` is an empty
//! stub, and `TBD_FrameworkManager.c` belongs to another slice this wave (T-181.23) — so this
//! slice must not write into it. A 500 ms poll of the replicated value is the self-contained way
//! to be correct today; it costs one enum compare per tick.
//!
//! It is started and stopped by `TBD_LobbyComponent` (a component on the game mode prefab) rather
//! than from a `SCR_PlayerController` override, for the reason `TBD_SpectatorComponent` gives in
//! its own header: statics outlive a world inside one process, so a watcher needs a lifetime tied
//! to the world, and the game-mode component graph is exactly that lifetime. It also means this
//! slice adds no third override of a vanilla method.
//!
//! `OnStageChanged` is public and side-effect-complete precisely so the eventual real hook is a
//! one-line call — see the slice report.
class TBD_LobbyStage
{
	static const int POLL_MS = 500;

	protected static bool s_bRunning;
	protected static TBD_EGameStage s_LastStage;

	//! `TBD_MenuStack.Open` returned null once — the preset is not registered (the known
	//! `resourceDatabase.rdb` blocker). Latched so the re-raise below logs ONE error for the round
	//! instead of one every 500 ms forever. A log flood would bury the very line an operator greps
	//! for to know whether the Workbench pass worked.
	protected static bool s_bPresetUnavailable;

	//------------------------------------------------------------------------------------------------
	//! @authority client — a dedicated server has no workspace and must never open a menu.
	static void Start()
	{
		if (s_bRunning)
			return;

		if (!GetGame().GetWorkspace())
			return;

		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		s_bRunning = true;
		s_LastStage = TBD_EGameStage.LOADING;

		GetGame().GetCallqueue().CallLater(Tick, POLL_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process, so this MUST run on world teardown or the next
	//! round starts with a tick pointed at a framework manager that no longer exists.
	static void Shutdown()
	{
		GetGame().GetCallqueue().Remove(Tick);
		s_bRunning = false;
		s_LastStage = TBD_EGameStage.LOADING;

		// Cleared with the world: the next round gets a fresh chance to open the preset, which
		// matters precisely because the Workbench pass that registers it may land between rounds.
		s_bPresetUnavailable = false;

		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UILobby);

		TBD_LobbyClient.Reset();
	}

	//------------------------------------------------------------------------------------------------
	protected static void Tick()
	{
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		TBD_EGameStage stage = fm.GetStage();

		if (stage != s_LastStage)
		{
			s_LastStage = stage;
			OnStageChanged(stage);
			return;
		}

		// ── LOBBY is a phase you are IN, not a screen you visit ─────────────────────────────
		// Esc closes any menu and no script can stop it. For the briefing that is fine — you can
		// re-open your orders from the lobby. For the LOBBY it is a dead end: a player who
		// dismisses the picker has no seat, no way to get one, and one life to lose by missing
		// the round. So the picker comes back.
		//
		// Deliberately a soft modal and nothing stronger: it costs one `IsOpen` check per tick,
		// it stops the moment the round leaves LOBBY or the player reaches the world, and it
		// cannot flood the log because a preset that will not open is latched off after the first
		// failure.
		//
		// "In the world" is asked of the LOCAL controlled entity, not just of our own deploy flag,
		// and that is what makes this safe next to `TBD_SpawnManager.m_bAutoDeploy` — the PIE wave
		// this picker exists to replace, which still defaults ON (see the slice report). A player
		// the wave already deployed has a body, so the re-raise stands down and Esc dismisses the
		// picker for good, instead of trapping someone who is already playing behind it.
		//
		// Note this guards only the RE-raise. The first open (on the transition into LOBBY) is
		// unconditional on purpose: if this test were ever wrong, gating the initial open would
		// mean the picker silently never appears, which is a far worse failure than one that can
		// be dismissed.
		if (stage != TBD_EGameStage.LOBBY)
			return;

		if (TBD_LobbyClient.IsDeployed() || SCR_PlayerController.GetLocalControlledEntity())
			return;

		Raise();
	}

	//------------------------------------------------------------------------------------------------
	//! Put the picker up, at most once per round if the preset cannot resolve.
	protected static void Raise()
	{
		if (s_bPresetUnavailable)
			return;

		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
			return;

		if (!TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UILobby))
			s_bPresetUnavailable = true;
	}

	//------------------------------------------------------------------------------------------------
	//! The single entry point for "the round changed phase" as far as the lobby is concerned.
	//! Kept public and complete so wiring it to the real replication hook is one line.
	static void OnStageChanged(TBD_EGameStage stage)
	{
		if (stage == TBD_EGameStage.LOBBY)
		{
			TBD_LobbyClient.Reset();
			Raise();
			return;
		}

		// Any other phase: slotting is over. Closing through the stack hands input and focus back
		// correctly (TBD_MenuStack invariants 3 and 4).
		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UILobby);
	}
}
