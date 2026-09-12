//! Lobby feature module - CLIENT roster cache, optimistic edit + reconciliation. Screens bind here. Split out of TBD_LobbyController.c (UI reorg 2026-09-12); logic unchanged.
//!
//! CLIENT - the roster this player is looking at, the optimistic edit, and the reconciliation.
//!
//! Static because the screen is created and destroyed by the menu manager: parking the roster on
//! the screen would lose it on every close and re-request it on every open with nothing to draw in
//! the meantime.
//!
//! -- Optimistic feedback, authoritative truth ------------------------------------------------
//! A claim is reflected **immediately** - the row goes to your name before a single packet leaves
//! the machine - because under ONE LIFE the moment between clicking and being sure is the worst
//! moment of the whole lobby, and a spinner does not shorten it.
//!
//! It is reconciled by **replacement**, not by merge: `Accept()` throws the local roster away and
//! rebuilds from the server's string. So an optimistic claim survives exactly as long as the
//! server agrees with it, and a rejection reverts the row in the same message that explains why.
//! There is no state machine to get stuck in, and a dropped or reordered reply cannot leave a
//! phantom claim on screen - the next refresh (2 s, at worst) overwrites it regardless.
class TBD_LobbyClient
{
	//! How long a refused seat stays marked. Long enough to read the reason under it, short enough
	//! that it does not become part of the furniture.
	static const int REJECT_HIGHLIGHT_MS = 5000;

	protected static ref TBD_LobbyRoster m_Roster;

	//! Non-blocking feedback line. Never a modal - design law.
	protected static string m_sStatus;

	//! The seat the authority most recently refused us, or empty.
	protected static string m_sRejectedKey;

	//! Set once the server has accepted a deploy, so the screen can stand down.
	//!
	//! T-181.29 - note what this is and is NOT. It is "the authority accepted MY deploy click", and
	//! only a `V DEPLOY ok` verdict sets it. It is not "I am in the world" - see `m_bInWorld`, which
	//! is the fact that answers that, and which is what the screen actually needed.
	protected static bool m_bDeployed;

	//! T-181.29 - the authority's answer to "does this player already have a body", refreshed from
	//! every roster that arrives (2 s at worst, immediately on screen open).
	//!
	//! -- The bug this closes -----------------------------------------------------------------
	//! The screen only ever stood down on `m_bDeployed`, and `m_bDeployed` is set by exactly one
	//! event: the reply to a deploy the player CLICKED. `TBD_SpawnManager`'s LOBBY auto-deploy wave
	//! (`m_bAutoDeploy`, still 1 - it deploys everyone ~250 ms into LOBBY) never sets it, and neither
	//! does the JIP `DeployJoiner` path or `AdminRespawn`. All three are server-side and silent to
	//! this client, so a player any of them deployed got a character AND kept the picker on top of
	//! it, permanently: `TBD_LobbyStage.Tick`'s T-181.28 guard suppresses the RE-raise but has
	//! nothing to say about a screen that is already up.
	//!
	//! -- Separate from m_bDeployed on purpose ------------------------------------------------
	//! Folding this into `m_bDeployed` would have been one line fewer and wrong in two directions.
	//! `m_bDeployed` is LATCHED and must stay latched - `AcceptDeployVerdict` sets it on a verdict
	//! that will not be repeated, and a later roster must not un-set it. This one must NOT latch:
	//! it is an observation, so a wrong reading self-corrects on the next refresh and the picker
	//! returns via `TBD_LobbyStage.Tick`'s unconditional re-raise. Keeping them apart is what lets
	//! each have the lifetime it needs.
	protected static bool m_bInWorld;

	//! A deploy request is in flight. Lives here rather than on the screen so the footer derives
	//! the button's enabled state from ONE place - a screen that disabled its own button would be
	//! re-enabled by the very next roster refresh.
	protected static bool m_bDeployPending;

	//! -- The in-flight intent, and why it has to exist ----------------------------------------
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
	//! T-181.29 - is this player already in the world, by the authority's own reckoning? True for a
	//! body that arrived by ANY door, including the ones this client never asked for.
	static bool IsInWorld()
	{
		return m_bInWorld;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.29 - the one question the screen asks before deciding to stand down: is there a
	//! character under this menu? Either half is sufficient and they cover different windows -
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
	//! (TBD_LobbyRoster) - lazily created. Fires on every change, whether it came from the server
	//! or from an optimistic local edit, so the screen has exactly one thing to listen to.
	static ScriptInvoker GetOnRosterChanged()
	{
		if (!m_OnRosterChanged)
			m_OnRosterChanged = new ScriptInvoker();

		return m_OnRosterChanged;
	}

	// -- Requests ----------------------------------------------------------------------------

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
		SetStatus("Deploying...");
		Changed();

		pc.TBD_RequestDeploy();
	}

	// -- Optimistic edits --------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Move our own flag onto `slotKey` locally. Refuses to touch a seat that is not open, so the
	//! optimistic path can never show something the authority would obviously refuse - and so
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
		SetStatus(string.Format("Taking %1...", m_Roster.m_sOwnLabel));
		Changed();
	}

	//------------------------------------------------------------------------------------------------
	protected static void ApplyOptimisticRelease()
	{
		if (!MutateRelease())
			return;

		SetStatus("Giving the seat up...");
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

	// -- Reconciliation ----------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! The authority has spoken. Replace everything, then say what it said.
	static void Accept(string wire)
	{
		TBD_LobbyRoster incoming = TBD_LobbyService.Parse(wire);
		if (!incoming)
			return;

		m_Roster = incoming;

		// T-181.29 - refreshed from EVERY reply, including a plain refresh and an unavailable
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
			// to read - the 2 s poll would otherwise wipe every explanation half a second after it
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
			// received ("You hold ALPHA . SL") than any fixed acknowledgement could be, and a
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
	//! The refusal has had its five seconds. Drop the mark AND the sentence - leaving the sentence
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
	//! T-181.49 - the queue is null-checked now. This used to be reached only behind
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
