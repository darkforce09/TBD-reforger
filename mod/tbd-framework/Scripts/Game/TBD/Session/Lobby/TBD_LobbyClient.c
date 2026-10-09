/**
 * @file TBD_LobbyClient.c
 * @brief The client's lobby roster: the cache the screen draws, the optimistic edit, and the reconciliation.
 *
 * Role: sends the roster requests, shows a claim or release at once, and replaces the whole roster
 * with every server reply, re-applying any intent the server has not ruled on yet.
 * Position: calls the modded SCR_PlayerController's TBD_Request* methods; receives
 * TBD_RpcDo_LobbyRoster through Accept; TBD_LobbyScreen and TBD_LobbyStage read it.
 * State: the roster, status line, rejected seat, deploy latches and pending intents, as statics
 * so they survive the screen closing; reset by TBD_LobbyStage on entering LOBBY and on teardown.
 * Invariants: a reply replaces the roster, never merges; only the matching verdict retires a
 * pending intent; a deploy is never optimistic; m_bDeployed latches for the round while
 * m_bInWorld follows every roster; a refusal marks its seat for REJECT_HIGHLIGHT_MS.
 */

//! Client roster cache; static because the screen is created and destroyed by the menu manager.
class TBD_LobbyClient
{
	static const int REJECT_HIGHLIGHT_MS = 5000; //!< ms a refused seat stays marked with its reason

	protected static ref TBD_LobbyRoster m_Roster; //!< the roster on screen; null until the first reply

	protected static string m_sStatus; //!< non-blocking feedback line, never a modal

	protected static string m_sRejectedKey; //!< the seat the server last refused; empty when none

	protected static bool m_bDeployed; //!< the server accepted this player's own DEPLOY click; latched for the round

	protected static bool m_bInWorld; //!< the last roster says this player controls a body; follows every roster

	protected static bool m_bDeployPending; //!< a deploy request is in flight; the one source of the button's enabled state

	protected static string m_sPendingClaimKey; //!< the claim the server has not ruled on; empty when none
	protected static bool m_bPendingRelease; //!< a release the server has not ruled on

	protected static ref ScriptInvoker m_OnRosterChanged; //!< (TBD_LobbyRoster roster); created on first GetOnRosterChanged

	//! @return the roster on screen; null before the first reply
	static TBD_LobbyRoster GetRoster()
	{
		return m_Roster;
	}

	//! @return the feedback line; empty when there is nothing to say
	static string GetStatus()
	{
		return m_sStatus;
	}

	//! @return the seat the server last refused; empty when none
	static string GetRejectedKey()
	{
		return m_sRejectedKey;
	}

	//! @return true once the server accepted this player's own deploy
	static bool IsDeployed()
	{
		return m_bDeployed;
	}

	//! @return true when the last roster says this player controls a body, however it arrived
	static bool IsInWorld()
	{
		return m_bInWorld;
	}

	//! @return true when a character is under the menu: the player's own accepted deploy (no round trip to wait for) or a body from any server-side door
	static bool ShouldStandDown()
	{
		return m_bDeployed || m_bInWorld;
	}

	//! @return true while a deploy request is in flight
	static bool IsDeployPending()
	{
		return m_bDeployPending;
	}

	//! @return the (TBD_LobbyRoster roster) invoker raised on every change, from the server or an optimistic edit
	static ScriptInvoker GetOnRosterChanged()
	{
		if (!m_OnRosterChanged)
			m_OnRosterChanged = new ScriptInvoker();

		return m_OnRosterChanged;
	}

	//! Ask the server for the roster; no-op without a local controller.
	//! @authority client
	static void Request()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		pc.TBD_RequestLobbyRoster();
	}

	//! Take `slotKey`: shown locally first, then asked for; no-op without a local controller.
	//! @authority client
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

	//! Give the seat back: shown locally first, then asked for; no-op without a local controller.
	//! @authority client
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

	//! Ask to deploy. Not optimistic: a refused deploy must not have torn the lobby down already. No-op without a local controller.
	//! @authority client
	static void Deploy()
	{
		SCR_PlayerController pc = SCR_PlayerController.Cast(GetGame().GetPlayerController());
		if (!pc)
			return;

		// Latched before the request so the button is dead when the click finishes: a double click
		// on the one irreversible action never becomes two deploys.
		m_bDeployPending = true;
		SetStatus("Deploying...");
		Changed();

		pc.TBD_RequestDeploy();
	}

	//! Move the player's flag onto `slotKey` locally. Refuses a seat that is not OPEN, so the optimistic path never shows what the server would refuse and re-applying it over a fresher roster that shows the seat lost is a no-op.
	//! @return true when the roster changed
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

	//! Clear the player's own seat locally.
	//! @return true when the roster changed
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

	//! Show a claim at once with a `Taking ...` status and raise the change.
	protected static void ApplyOptimisticClaim(string slotKey)
	{
		if (!MutateClaim(slotKey))
			return;

		m_sRejectedKey = string.Empty;
		SetStatus(string.Format("Taking %1...", m_Roster.m_sOwnLabel));
		Changed();
	}

	//! Show a release at once with a `Giving the seat up...` status and raise the change.
	protected static void ApplyOptimisticRelease()
	{
		if (!MutateRelease())
			return;

		SetStatus("Giving the seat up...");
		Changed();
	}

	//! Re-state, on a roster that just arrived, the intent the server has not answered; silent, because the caller owns the status line and the change notification.
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

	//! Take a server reply: replace the roster, refresh the in-world fact, retire the intent its verdict answers, re-apply the rest, and show the verdict.
	//! @param wire a TBD_LobbyRosterWire string
	static void Accept(string wire)
	{
		TBD_LobbyRoster incoming = TBD_LobbyRosterWire.Parse(wire);
		if (!incoming)
			return;

		m_Roster = incoming;

		// Taken from every reply, a plain refresh and an unavailable roster included, because the
		// server-side doors into the world send no verdict of their own; assigned, not latched.
		m_bInWorld = incoming.m_bInWorld;

		// Only the matching verdict retires the held intent; a verdict for a seat the player has moved
		// on from is stale and puts neither its reason nor its retirement on screen.
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

		// Whatever the server has not ruled on is re-applied over its answer, so a refresh requested
		// before the click cannot flicker the seat away and back.
		ReapplyPendingIntent();

		if (incoming.m_sAction.IsEmpty() || !current)
		{
			// A plain refresh keeps an unread rejection: it expires on its own timer, not on the 2 s poll.
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
			// Silent on success: the screen derives a better line from the roster ("You hold ALPHA . SL").
			ClearRejection();
			SetStatus(string.Empty);
			Changed();
			return;
		}

		// Refused: mark the seat and say why; the replaced roster shows who holds it.
		m_sRejectedKey = incoming.m_sActionKey;
		SetStatus(incoming.m_sActionReason);

		GetGame().GetCallqueue().Remove(ClearRejection);
		GetGame().GetCallqueue().CallLater(ClearRejection, REJECT_HIGHLIGHT_MS, false);

		Changed();
	}

	//! Handle a DEPLOY verdict, the only one that ends the screen: release the pending latch, show the reason, and latch m_bDeployed on success.
	protected static void AcceptDeployVerdict(TBD_LobbyRoster incoming)
	{
		// Released either way: a RETRY leaves the button live, or the player is stranded on a dead control.
		m_bDeployPending = false;
		SetStatus(incoming.m_sActionReason);

		if (incoming.m_bActionOk)
			m_bDeployed = true;

		Changed();
	}

	//! Drop the refusal mark and its sentence after REJECT_HIGHLIGHT_MS; no-op when there is none.
	protected static void ClearRejection()
	{
		if (m_sRejectedKey.IsEmpty())
			return;

		m_sRejectedKey = string.Empty;
		m_sStatus = string.Empty;
		Changed();
	}

	//! Set the feedback line; the caller raises the change.
	static void SetStatus(string status)
	{
		m_sStatus = status;
	}

	//! Raise GetOnRosterChanged with the roster.
	protected static void Changed()
	{
		if (m_OnRosterChanged)
			m_OnRosterChanged.Invoke(m_Roster);
	}

	//! @return the local player's name for the optimistic row, or `You`; the server's reply overwrites it one round trip later
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

	//! Forget the last round: roster, status, latches and intents, and cancel the rejection timer; runs on entering LOBBY and on teardown, when the call queue may already be gone.
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
