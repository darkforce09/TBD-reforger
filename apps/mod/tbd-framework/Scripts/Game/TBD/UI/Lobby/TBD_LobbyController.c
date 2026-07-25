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
	//!
	//! T-181.29 — note what this is and is NOT. It is "the authority accepted MY deploy click", and
	//! only a `V DEPLOY ok` verdict sets it. It is not "I am in the world" — see `m_bInWorld`, which
	//! is the fact that answers that, and which is what the screen actually needed.
	protected static bool m_bDeployed;

	//! T-181.29 — the authority's answer to "does this player already have a body", refreshed from
	//! every roster that arrives (2 s at worst, immediately on screen open).
	//!
	//! ── The bug this closes ─────────────────────────────────────────────────────────────────
	//! The screen only ever stood down on `m_bDeployed`, and `m_bDeployed` is set by exactly one
	//! event: the reply to a deploy the player CLICKED. `TBD_SpawnManager`'s LOBBY auto-deploy wave
	//! (`m_bAutoDeploy`, still 1 — it deploys everyone ~250 ms into LOBBY) never sets it, and neither
	//! does the JIP `DeployJoiner` path or `AdminRespawn`. All three are server-side and silent to
	//! this client, so a player any of them deployed got a character AND kept the picker on top of
	//! it, permanently: `TBD_LobbyStage.Tick`'s T-181.28 guard suppresses the RE-raise but has
	//! nothing to say about a screen that is already up.
	//!
	//! ── Separate from m_bDeployed on purpose ────────────────────────────────────────────────
	//! Folding this into `m_bDeployed` would have been one line fewer and wrong in two directions.
	//! `m_bDeployed` is LATCHED and must stay latched — `AcceptDeployVerdict` sets it on a verdict
	//! that will not be repeated, and a later roster must not un-set it. This one must NOT latch:
	//! it is an observation, so a wrong reading self-corrects on the next refresh and the picker
	//! returns via `TBD_LobbyStage.Tick`'s unconditional re-raise. Keeping them apart is what lets
	//! each have the lifetime it needs.
	protected static bool m_bInWorld;

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
	//! T-181.29 — is this player already in the world, by the authority's own reckoning? True for a
	//! body that arrived by ANY door, including the ones this client never asked for.
	static bool IsInWorld()
	{
		return m_bInWorld;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.29 — the one question the screen asks before deciding to stand down: is there a
	//! character under this menu? Either half is sufficient and they cover different windows —
	//! `IsDeployed` fires on the player's own verdict with no round trip to wait for, `IsInWorld`
	//! catches every server-side deploy the client was never told about.
	static bool ShouldStandDown()
	{
		return m_bDeployed || m_bInWorld;
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

		// T-181.29 — refreshed from EVERY reply, including a plain refresh and an unavailable
		// roster, because the doors that make it true (the LOBBY auto-deploy wave, JIP, admin
		// respawn) send no verdict of their own. Assigned rather than OR-ed into: this is an
		// observation with the lifetime of the roster that carried it, so a body that goes away
		// takes the fact with it and the picker is allowed back. `m_bDeployed` above is the one
		// that latches.
		m_bInWorld = incoming.m_bInWorld;

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
	//!
	//! T-181.49 — the queue is null-checked now. This used to be reached only behind
	//! `TBD_LobbyComponent.OnDelete`'s workspace test; that test is gone (it never excluded a
	//! server anyway), so this runs on world teardown on every machine, which is exactly the
	//! moment a subsystem is most likely to already be down. Same shape `TBD_RadioComponent`
	//! already uses in its own `OnDelete`.
	static void Reset()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
			queue.Remove(ClearRejection);

		m_Roster = null;
		m_sStatus = string.Empty;
		m_sRejectedKey = string.Empty;
		m_bDeployed = false;
		m_bInWorld = false;
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

	//! T-181.49 — the arming retry. `Start()` used to be one-shot, so losing a race it had no
	//! reason to expect to win was PERMANENT. See `Start` for the measurement.
	static const int ARM_RETRY_MS = 250;
	static const int ARM_MAX_ATTEMPTS = 60; //!< 60 x 250 ms = 15 s, then give up LOUDLY.

	//! Greppable prefix. One vocabulary for the whole raise path, so
	//! `grep '\[TBD\]\[Lobby\]' console.log` returns the entire lifecycle of the picker.
	static const string LOG_TAG = "[TBD][Lobby] ";

	//! Outcome of the last `Raise()`. `Raise` runs on a 500 ms poll, so an unlatched log line
	//! there would emit twice a second forever and bury the signal it exists to carry. Logging
	//! only on a CHANGE of outcome gives exactly one line per transition, which is what an
	//! operator actually needs: not "it refused", but "it started refusing, for this reason".
	static const int RAISE_UNSEEN        = 0; //!< nothing decided yet on this world.
	static const int RAISE_OPENED        = 1;
	static const int RAISE_NO_CONTROLLER = 2;
	static const int RAISE_PRESET_DEAD   = 3;
	static const int RAISE_ALREADY_OPEN  = 4;
	static const int RAISE_OPEN_FAILED   = 5;

	protected static bool s_bRunning;
	protected static TBD_EGameStage s_LastStage;

	//! `TBD_MenuStack.Open` returned null once — the preset is not registered (the known
	//! `resourceDatabase.rdb` blocker). Latched so the re-raise below logs ONE error for the round
	//! instead of one every 500 ms forever. A log flood would bury the very line an operator greps
	//! for to know whether the Workbench pass worked.
	protected static bool s_bPresetUnavailable;

	protected static int s_iArmAttempts;
	protected static int s_iLastRaiseOutcome;

	//------------------------------------------------------------------------------------------------
	//! One line, one shape, one grep. `PrintFormat` and NEVER `Print(localVariable)` — MEASURED in
	//! this codebase: `Print` emits the DECLARATION of a local, not its value, which is why the
	//! roll-call assertion in `world-boot.sh` has to strip a trailing quote.
	protected static void Log(string message)
	{
		PrintFormat("%1", LOG_TAG + message, level: LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	//! WARNING, not ERROR, and deliberately: `world-boot.sh` triages any TBD-owned `SCRIPT (E)`
	//! line as a gate failure, so shouting at ERROR about a state the gate can legitimately reach
	//! would turn a diagnostic into a false red. MEASURED this slice: WARNING lines DO reach
	//! `console.log` (`[TBD][Radio] backbone: MISSING` is one), so nothing is lost by the level.
	protected static void LogWarn(string message)
	{
		PrintFormat("%1", LOG_TAG + message, level: LogLevel.WARNING);
	}

	//------------------------------------------------------------------------------------------------
	//! @authority client — only a machine with a local player may open a menu.
	//!
	//! ── T-181.49: this used to be a one-shot with three silent exits ─────────────────────────
	//! `TBD_LobbyComponent.OnPostInit` arms this with `CallLater(..., false)` at +2000 ms. That
	//! made `IsFrameworkWorld()` a COIN FLIP, and losing it was permanent AND invisible:
	//!
	//!   MEASURED 2026-07-25, `world-boot.sh --mission=bridgehead-at-levie`, engine 1.7.0.54:
	//!     21:02:13.963  [TBD][Lobby] wire self-check PASS   <- TBD_LobbyComponent.OnPostInit
	//!     21:02:15.96   (Start fires: +2000 ms from the line above)
	//!     21:02:16.201  [TBD] roll-call: ... Lobby=ok       <- TBD_FrameworkManager's CallLater(…, 0)
	//!
	//! `Start` fires ~240 ms BEFORE the framework manager's own deferred roll-call. The call queue
	//! does not tick during world load, so both callbacks are flushed together when it starts and
	//! their relative order is not something this class gets to choose. Ask `IsFrameworkWorld()`
	//! once, at that instant, and the answer is whatever the flush order happened to be.
	//!
	//! So it now RETRIES. The bound exists so a genuinely broken world says so instead of spinning
	//! forever, and the give-up is logged — the point of this whole slice is that no exit on this
	//! path is silent.
	//!
	//! ── Statics are reset HERE, not only in Shutdown() ───────────────────────────────────────
	//! `TBD_GameMode` is constructed TWICE per Workbench session (once for the World Editor, once
	//! for Play) while every static below survives between them. If the editor instance's
	//! `OnDelete` is skipped, `Shutdown()` never runs and the Play instance inherits `s_bRunning`
	//! true and a stale `TBD_MenuStack` entry — and the old `if (s_bRunning) return;` turned that
	//! into a picker that never opens again for the life of the process. A new world arming its
	//! watcher is unambiguous proof the previous world is gone, so that is the moment to clear.
	//! `Shutdown()` keeps doing it too; belt and braces, not one or the other.
	static void Start()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
		{
			LogWarn("Start refused — no call queue on this machine. The slot picker cannot arm.");
			return;
		}

		// Idempotent re-arm. Removing first is strictly safer than the `if (s_bRunning) return;`
		// this replaces: it cannot leave two timers running, and it cannot latch the watcher OFF
		// for the rest of the process when a previous world failed to tear itself down.
		queue.Remove(Tick);
		queue.Remove(TryArm);

		s_bRunning = false;
		s_LastStage = TBD_EGameStage.LOADING;
		s_bPresetUnavailable = false;
		s_iArmAttempts = 0;
		s_iLastRaiseOutcome = RAISE_UNSEEN;

		// A menu the engine destroyed during world teardown without firing `OnMenuClose` leaves a
		// stale weak entry in `TBD_MenuStack`'s static array, and `Raise`'s `IsOpen` check then
		// returns true forever — silently, without even latching `s_bPresetUnavailable`. Nothing
		// else in the addon calls `Reset()`; this is its one caller and this is the safe moment
		// for it, before anything on THIS world has opened a screen.
		TBD_MenuStack.Reset();

		// ── The real authority test ──────────────────────────────────────────────────────────
		// NOT `GetGame().GetWorkspace()`, which this line used to ask: that is MEASURED NON-NULL
		// on the headless dedicated server `world-boot.sh` runs, so it never excluded anything.
		// `RplSession.Mode() == RplMode.Dedicated` is what both oracles use for this question and
		// what the rest of this addon already uses (TBD_FrameworkManager, TBD_AdminService, …).
		if (RplSession.Mode() == RplMode.Dedicated)
		{
			Log("Start refused — dedicated server (RplSession.Mode()==Dedicated). The picker is a client screen; nothing to raise here.");
			return;
		}

		Log(string.Format("Start — arming watcher: retrying IsFrameworkWorld() every %1 ms, up to %2 attempts.",
			ARM_RETRY_MS, ARM_MAX_ATTEMPTS));

		queue.CallLater(TryArm, ARM_RETRY_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Retry until this world admits it is a framework world, then promote to the real `Tick` and
	//! cancel this. Bounded so a world that will never qualify says so once and stops.
	protected static void TryArm()
	{
		s_iArmAttempts++;

		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (!queue)
			return;

		if (!TBD_FrameworkManager.IsFrameworkWorld())
		{
			if (s_iArmAttempts < ARM_MAX_ATTEMPTS)
				return;

			queue.Remove(TryArm);
			LogWarn(string.Format("Start GAVE UP — IsFrameworkWorld() still false after %1 attempts over %2 ms. The slot picker will NOT open on this world. This is a wiring failure, not a timing one: check that TBD_FrameworkManager is on the same game mode prefab as TBD_LobbyComponent.",
				s_iArmAttempts, s_iArmAttempts * ARM_RETRY_MS));
			return;
		}

		queue.Remove(TryArm);

		s_bRunning = true;
		s_LastStage = TBD_EGameStage.LOADING;

		queue.CallLater(Tick, POLL_MS, true);

		Log(string.Format("Tick ARMED after %1 attempt(s) (%2 ms) — polling the replicated stage every %3 ms.",
			s_iArmAttempts, s_iArmAttempts * ARM_RETRY_MS, POLL_MS));
	}

	//------------------------------------------------------------------------------------------------
	//! Statics outlive a world inside one process, so this MUST run on world teardown or the next
	//! round starts with a tick pointed at a framework manager that no longer exists.
	//!
	//! Deliberately does NOT call `TBD_MenuStack.Reset()`: teardown is exactly when the briefing
	//! and spectator screens may still be legitimately stacked, and wiping their entries here
	//! would strand THEIR bookkeeping to fix ours. The arm path is the safe place for that, and it
	//! is where it now lives.
	static void Shutdown()
	{
		ScriptCallQueue queue = GetGame().GetCallqueue();
		if (queue)
		{
			queue.Remove(Tick);
			queue.Remove(TryArm);
		}

		s_bRunning = false;
		s_LastStage = TBD_EGameStage.LOADING;
		s_iArmAttempts = 0;
		s_iLastRaiseOutcome = RAISE_UNSEEN;

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
		//
		// ── T-181.29: this guard was only ever HALF the answer ───────────────────────────────
		// Standing the re-raise down does nothing for a screen that is ALREADY OPEN, and that is the
		// case the auto-deploy wave produces: the wave fires ~250 ms into LOBBY, this watcher raises
		// the picker on the same transition, and the result was a picker sitting over a live
		// character with only Esc to get rid of it. The close now lives where it belongs — on the
		// roster, in `TBD_LobbyScreen.OnRosterChanged` via `TBD_LobbyClient.ShouldStandDown()`.
		//
		// `ShouldStandDown()` replaces the bare `IsDeployed()` here so the re-raise and the close
		// test the same predicate. It is a strict superset of what this line asked before, so it can
		// only suppress a raise that used to happen, never permit one that did not — and every input
		// to it either latches for the round or self-corrects on the next 2 s refresh.
		if (stage != TBD_EGameStage.LOBBY)
			return;

		if (TBD_LobbyClient.ShouldStandDown() || SCR_PlayerController.GetLocalControlledEntity())
			return;

		Raise();
	}

	//------------------------------------------------------------------------------------------------
	//! Put the picker up, at most once per round if the preset cannot resolve.
	//!
	//! ── T-181.42: this is where "do I have a screen" is actually decided ────────────────────
	//! **`GetGame().GetWorkspace()` is NON-NULL on a headless dedicated server** (engine 1.7.0.54).
	//! It is not a dedicated-server test, and this class used to treat it as one. MEASURED in this
	//! repo: `world-boot.sh --mission=bridgehead-at-levie` with `TBD_WORLDBOOT_SETTLE=12` failed
	//! **3/3** with `SCRIPT (E): [TBD][ui] preset 60 did not open`, ~1000 ms (two poll ticks) after
	//! `LOADING -> LOBBY`, on a boot with ZERO players. For that line to be reachable at all, BOTH
	//! workspace guards then on the path (`TBD_LobbyComponent.OnPostInit` and `Start`) must have
	//! passed on a headless machine — the failing log is its own proof. The default 4 s settle
	//! usually ended before the watcher fired, which is why this read as an intermittent gate flake.
	//! (Both of those guards are gone as of T-181.49; `Start` now refuses on `RplMode.Dedicated`
	//! and says so, so a headless boot no longer reaches this function at all.)
	//!
	//! The reliable test is a null LOCAL PLAYER CONTROLLER — the idiom `TBD_MissionBrowser.c:285`
	//! already uses. It goes HERE rather than in `Start()` deliberately: `Tick` polls every 500 ms,
	//! so gating the raise is self-healing — a client whose controller is not up yet simply raises
	//! on a later tick. Gating `Start()` would be a one-shot test with a race, and losing that race
	//! would mean the picker silently NEVER appears, which is far worse than raising it late. Same
	//! reasoning the `Tick` comment already gives for keeping the first open unconditional.
	//!
	//! ── T-181.49: all four exits are now observable ──────────────────────────────────────────
	//! Every one of these used to `return` in silence. Nine of the eleven guards on the whole
	//! raise path did, which made "the picker did not open" a fact with no evidence attached —
	//! the real defect this slice fixes. `LogOutcome` latches on the OUTCOME so the 500 ms poll
	//! emits one line per transition, never a flood.
	protected static void Raise()
	{
		if (!GetGame().GetPlayerController())
		{
			LogOutcome(RAISE_NO_CONTROLLER, "raise deferred — no local player controller yet. The 500 ms poll will retry; this is self-healing, not a failure.");
			return;
		}

		if (s_bPresetUnavailable)
		{
			LogOutcome(RAISE_PRESET_DEAD, "raise refused — preset latched unavailable for this round (TBD_MenuStack.Open returned null once). Cleared on world teardown or the next arm.");
			return;
		}

		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
		{
			LogOutcome(RAISE_ALREADY_OPEN, "raise skipped — TBD_UILobby is already on the stack.");
			return;
		}

		if (!TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UILobby))
		{
			s_bPresetUnavailable = true;
			LogOutcome(RAISE_OPEN_FAILED, "raise FAILED — TBD_MenuStack.Open returned null. Latched off for this round; see the [TBD][ui] error above for the preset id.");
			return;
		}

		LogOutcome(RAISE_OPENED, "picker OPEN — TBD_UILobby raised.");
	}

	//------------------------------------------------------------------------------------------------
	//! One line per CHANGE of raise outcome. `Raise` is called from a 500 ms poll, so this latch is
	//! what keeps four honest diagnostics from becoming a log flood that hides them.
	protected static void LogOutcome(int outcome, string message)
	{
		if (s_iLastRaiseOutcome == outcome)
			return;

		s_iLastRaiseOutcome = outcome;
		Log(message);
	}

	//------------------------------------------------------------------------------------------------
	//! The single entry point for "the round changed phase" as far as the lobby is concerned.
	//! Kept public and complete so wiring it to the real replication hook is one line.
	//!
	//! ── T-181.29: the other OnStageChanged, and why m_bAutoDeploy is still 1 ────────────────
	//! `TBD_SpawnManager.OnStageChanged` reacts to this same transition by scheduling
	//! `DeployAllConnectedPlayers` 250 ms out. Two handlers, one transition: this one raises the
	//! picker, that one puts everybody in the world. They race, and until this slice the race had no
	//! loser-recovery — whichever order they landed in, the picker stayed up.
	//!
	//! The obvious other fix is to flip `m_bAutoDeploy` to 0 and let the picker be the only way in.
	//! It is a real fix, it is almost certainly the right END state, and it is NOT taken here:
	//!
	//!   * The wave's stated reason for defaulting ON — "on a framework world this wave is currently
	//!     the ONLY working way into the world", because no screen could open — expired when
	//!     T-181.25 unblocked the menu presets. The premise is genuinely gone.
	//!   * But flipping it makes this picker LOAD-BEARING on its first ever live run. `TBD_GameMode.et`
	//!     does not override the attribute, so the `[Attribute("1")]` default in `TBD_SpawnManager`
	//!     is what ships — a one-character change takes effect immediately, and if the picker does
	//!     not open on a real client, nobody can deploy at all. `TBD_MenuStack.Open` returning null
	//!     is a failure this class already carries a latch for (`s_bPresetUnavailable`), which is
	//!     the measure of how plausible it still is.
	//!   * The fix in this slice is additive: it removes a screen that should not be there, and
	//!     changes nothing about how a player gets into the world. It is correct whether the wave
	//!     stays on or goes off, and it is correct whether or not the symptom it was filed for
	//!     turns out to be real.
	//!
	//! So: flip `TBD_SpawnManager.m_bAutoDeploy` to 0 in its own slice, gated on ONE observation —
	//! an operator confirming a live client sees the picker and can deploy from it. Until then the
	//! wave is the safety net and this slice is what stops the safety net from leaving a menu on
	//! the screen.
	static void OnStageChanged(TBD_EGameStage stage)
	{
		// T-181.49 — the transition itself, named. Without this line "the picker never appeared"
		// and "the watcher never saw LOBBY" are indistinguishable from the log, and they need
		// completely different fixes.
		Log(string.Format("stage -> %1", typename.EnumToString(TBD_EGameStage, stage)));

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
