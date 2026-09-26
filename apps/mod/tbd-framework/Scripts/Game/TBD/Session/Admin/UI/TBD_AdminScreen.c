/**
 * @file TBD_AdminScreen.c
 * @brief The admin menu: mission, stage, players and audit trail, with one primary recovery action.
 *
 * Role: renders the latest TBD_AdminPayload through TBD_AdminScreenSections, keeps the admin's
 * selection, disclosure and arming across refreshes, and asks for the power the selected player
 * needs.  Position: TBD_AdminClient opens it through TBD_MenuStack (preset `TBD_UIAdmin`); it reads
 * TBD_AdminClient's cache and invokers and sends requests through TBD_AdminClient.Request and Act.
 * State: the snapshot shown, the selected player id, the disclosure and arming flags and the held
 * footer verdict, on the client; a repeating poll while open.  Invariants: it renders only what the
 * server sent and holds no path from a widget to a power; the selection is a player id, never a row
 * index; a stage change under an armed force disarms it; no modal ever opens; every colour comes
 * from TBD_UITheme.
 *
 *   | ADMIN                                           [ Back ]     |
 *   | LIVE - 12 connected - 2 lives spent                          |
 *   | MISSION                              Bridgehead at Levie     |  <- section
 *   |   Validation         FAILED -- 3 error(s), 1 warning  +      |  <- pick to disclose
 *   | STAGE                                           LIVE         |
 *   |   Force stage -> END              irreversible - pick twice  |  <- arm, then confirm
 *   | PLAYERS                           12 - 2 lives spent         |
 *   |   Pvt. Vasquez        us_army - ALPHA/RFL - LIFE SPENT       |  <- pick, then RESPAWN
 *   | ADMIN ACTIONS                             4 this session     |
 *   |   Show the audit trail                             +         |
 *   | ONE LIFE -- Respawn hands Vasquez ...  [ RESPAWN VASQUEZ ]   |  <- one primary action
 */

//! The admin screen over the shared shell. Respawn under ONE LIFE spends the event's single
//! sanctioned exception, and the footer says so whenever a dead player is selected.
class TBD_AdminScreen : TBD_ShellScreen
{
	protected static const int REFRESH_MS = 3000; //!< ms between snapshot requests while open

	protected ref TBD_AdminPayload m_Payload; //!< the snapshot shown; null until one arrives
	protected int m_iSelectedPlayer = -1; //!< selected player id, -1 for none; an id, so a refresh that reorders rows cannot retarget it
	protected bool m_bValidationExpanded; //!< validator findings disclosed; survives a refresh
	protected bool m_bAuditExpanded; //!< audit trail disclosed; survives a refresh
	protected bool m_bStageArmed; //!< the first pick armed the force-stage; the second fires it
	protected string m_sLastStage; //!< stage of the last snapshot; a change disarms the force-stage
	protected string m_sPendingResult; //!< footer line held until the admin picks something else, so a refresh cannot wipe the server's verdict

	//! Wire the list, the primary action and TBD_AdminClient's invokers, draw the cached snapshot,
	//! request a fresh one, and start the poll.
	override protected void OnScreenOpen()
	{
		super.OnScreenOpen();

		TBD_ListBox list = GetList();
		if (list)
			list.GetOnActivate().Insert(OnRowPicked);

		GetOnPrimaryAction().Insert(OnPrimaryPressed);

		TBD_AdminClient.GetOnPayloadChanged().Insert(OnPayloadChanged);
		TBD_AdminClient.GetOnActionResult().Insert(OnActionResult);

		AdoptPayload(TBD_AdminClient.GetPayload());
		TBD_AdminClient.Request();

		// A poll, not a subscription: who is alive, who has a body and the stage live in
		// server-side maps with no replicated change notification.
		GetGame().GetCallqueue().CallLater(Poll, REFRESH_MS, true);
	}

	//! Stop the poll and unwire everything OnScreenOpen wired.
	override protected void OnScreenClose()
	{
		// The call queue outlives the screen; a live repeat pointed at a destroyed menu leaks.
		GetGame().GetCallqueue().Remove(Poll);

		TBD_ListBox list = GetList();
		if (list)
			list.GetOnActivate().Remove(OnRowPicked);

		GetOnPrimaryAction().Remove(OnPrimaryPressed);

		TBD_AdminClient.GetOnPayloadChanged().Remove(OnPayloadChanged);
		TBD_AdminClient.GetOnActionResult().Remove(OnActionResult);

		super.OnScreenClose();
	}

	//! @return the shell title
	override protected string GetScreenTitle()
	{
		return "ADMIN";
	}

	//! @return stage, headcount and lives spent, or the waiting or refusal line
	override protected string GetScreenSubtitle()
	{
		if (!m_Payload)
			return "Asking the server...";

		if (!m_Payload.m_bAuthorised)
			return "Not authorised";

		return string.Format("%1 - %2 connected - %3 lives spent",
			m_Payload.m_sStage, m_Payload.m_iConnected, m_Payload.m_iSpent);
	}

	//! Poll tick: request a fresh snapshot.
	protected void Poll()
	{
		TBD_AdminClient.Request();
	}

	//! TBD_AdminClient invoker: adopt the new snapshot.
	protected void OnPayloadChanged(TBD_AdminPayload payload)
	{
		AdoptPayload(payload);
	}

	//! Take a new snapshot without losing what the admin was doing: disclosure stays open, the
	//! selection survives while that player is connected, the held verdict stays on the footer, and
	//! a stage change disarms the force-stage.
	//! @param payload the snapshot, or null
	protected void AdoptPayload(TBD_AdminPayload payload)
	{
		m_Payload = payload;

		if (m_Payload && m_iSelectedPlayer > 0 && !m_Payload.FindPlayer(m_iSelectedPlayer))
		{
			// They left. Drop the selection rather than leave a loud button aimed at nobody.
			m_iSelectedPlayer = -1;
		}

		// The round moved while the admin was reading; anything armed was armed against another
		// stage.
		string stage;
		if (m_Payload)
			stage = m_Payload.m_sStage;

		if (stage != m_sLastStage)
		{
			m_sLastStage = stage;
			DisarmStage();
		}

		SetSubtitle(GetScreenSubtitle());
		Rebuild();
		RefreshFooter();
	}

	//! TBD_AdminClient invoker: show the server's verdict verbatim and hold it on the footer, since a
	//! fresh snapshot and the poll follow within seconds.
	protected void OnActionResult(string message, bool ok)
	{
		m_sPendingResult = message;
		SetStatus(message);
	}

	//! Write the whole list from the snapshot and the disclosure and arming flags. TBD_ListBox pools
	//! its rows, so a rebuild is property writes on existing widgets.
	protected void Rebuild()
	{
		TBD_ListBox list = GetList();
		if (!list)
			return;

		list.BeginUpdate();

		if (!m_Payload)
		{
			list.AddSection("Asking the server...");
			list.EndUpdate();
			return;
		}

		if (!m_Payload.m_bAuthorised)
		{
			// An empty state says why. Never a void -- and never anything the server did not send.
			list.AddSection(Reason());
			list.EndUpdate();
			return;
		}

		TBD_AdminScreenSections.EmitMission(list, m_Payload, m_bValidationExpanded);
		TBD_AdminScreenSections.EmitStage(list, m_Payload, m_bStageArmed);
		TBD_AdminScreenSections.EmitPlayers(list, m_Payload);
		TBD_AdminScreenSections.EmitAudit(list, m_Payload, m_bAuditExpanded);

		list.EndUpdate();

		// Re-assert the visual selection: EndUpdate re-applies it from the tag, and the tag survives
		// a rebuild because it is derived from the player id, not the row order.
		list.SetSelectedTag(TBD_AdminScreenSections.PlayerTag(m_iSelectedPlayer));
	}

	//! Set the primary action and the footer line for the current selection. Respawn and Deploy are
	//! separate: AdminRespawn refuses the living and DeployPlayerEx refuses the dead.
	protected void RefreshFooter()
	{
		SetPrimaryAction(PrimaryLabel(), PrimaryEnabled());
		SetStatus(ComposeStatus());
	}

	//! The footer line: the held verdict first, else the waiting or refusal line, else what the
	//! primary action would do for the selected player.
	//! @return the line
	protected string ComposeStatus()
	{
		if (!m_sPendingResult.IsEmpty())
			return m_sPendingResult;

		if (!m_Payload)
			return "Waiting for the server...";

		if (!m_Payload.m_bAuthorised)
			return Reason();

		TBD_AdminPlayerRow row = m_Payload.FindPlayer(m_iSelectedPlayer);
		if (!row)
			return "Pick a player to act on them.";

		if (row.m_bDead)
		{
			// ONE LIFE: death is terminal by design and this is the single sanctioned exception.
			return string.Format("ONE LIFE -- this hands %1 their life back and rebuilds them on their own slot. It is the event's escape hatch: use it for glitch deaths, not for losing a fight.",
				row.m_sName);
		}

		if (!row.m_bInWorld)
		{
			return string.Format("%1 still has their life but no body -- this puts them in the world. It neither spends nor restores a life.",
				row.m_sName);
		}

		return string.Format("%1 is alive and in the world -- nothing to recover.", row.m_sName);
	}

	//! @return `RESPAWN <name>`, `DEPLOY <name>`, or empty when there is nothing to recover
	protected string PrimaryLabel()
	{
		TBD_AdminPlayerRow row = SelectedActionable();
		if (!row)
			return string.Empty;

		if (row.m_bDead)
			return string.Format("RESPAWN %1", row.m_sName);

		return string.Format("DEPLOY %1", row.m_sName);
	}

	//! @return whether the selected player needs a recovery
	protected bool PrimaryEnabled()
	{
		return SelectedActionable() != null;
	}

	//! The selected player when there is something to recover for them: a spent life or no body.
	//! @return the row, or null, which hides the primary action
	protected TBD_AdminPlayerRow SelectedActionable()
	{
		if (!m_Payload || !m_Payload.m_bAuthorised)
			return null;

		TBD_AdminPlayerRow row = m_Payload.FindPlayer(m_iSelectedPlayer);
		if (!row)
			return null;

		if (row.m_bDead || !row.m_bInWorld)
			return row;

		return null;
	}

	//! Primary action: ask for RESPAWN for a spent life, else DEPLOY, and hold a waiting line.
	protected void OnPrimaryPressed(TBD_ShellScreen screen)
	{
		TBD_AdminPlayerRow row = SelectedActionable();
		if (!row)
			return;

		// The client picks which action to ask for; TBD_AdminService re-derives the caller,
		// re-checks the admin list and refuses a mismatched action whatever the client asked.
		if (row.m_bDead)
		{
			TBD_AdminClient.Act(TBD_EAdminAction.RESPAWN, row.m_iPlayerId);
			Announce(string.Format("Respawning %1 -- waiting for the server...", row.m_sName));
			return;
		}

		TBD_AdminClient.Act(TBD_EAdminAction.DEPLOY, row.m_iPlayerId);
		Announce(string.Format("Deploying %1 -- waiting for the server...", row.m_sName));
	}

	//! Put a line in the footer and hold it until the admin picks something else; the server's
	//! verdict overwrites it.
	//! @param text the line
	protected void Announce(string text)
	{
		m_sPendingResult = text;
		SetStatus(text);
	}

	//! A row pick: the stage row arms or fires; any other pick disarms, clears the held verdict,
	//! and toggles a disclosure or selects a player.
	protected void OnRowPicked(TBD_ListBox list, int tag)
	{
		if (tag == TBD_AdminScreenSections.TAG_STAGE)
		{
			OnStagePicked();
			return;
		}

		// Any other pick disarms a primed stage change, so it can never be the accidental second
		// half of an unrelated pair of clicks, and clears the last verdict so the footer goes back
		// to describing what the admin is now looking at.
		DisarmStage();
		m_sPendingResult = string.Empty;

		if (tag == TBD_AdminScreenSections.TAG_VALIDATION)
		{
			m_bValidationExpanded = !m_bValidationExpanded;
			Rebuild();
			RefreshFooter();
			return;
		}

		if (tag == TBD_AdminScreenSections.TAG_AUDIT)
		{
			m_bAuditExpanded = !m_bAuditExpanded;
			Rebuild();
			RefreshFooter();
			return;
		}

		if (tag >= TBD_AdminScreenSections.TAG_PLAYER_BASE)
		{
			m_iSelectedPlayer = tag - TBD_AdminScreenSections.TAG_PLAYER_BASE;
			Rebuild();
			RefreshFooter();
		}
	}

	//! The force-stage row: the first pick arms, the second asks for STAGE_ADVANCE.
	protected void OnStagePicked()
	{
		if (!m_Payload || !m_Payload.m_bAuthorised || !m_Payload.m_bStageReady || m_Payload.m_sNextStage.IsEmpty())
			return;

		if (!m_bStageArmed)
		{
			m_bStageArmed = true;
			Rebuild();
			Announce(string.Format("Pick again to force the round from %1 to %2. This moves everybody and cannot be undone.",
				m_Payload.m_sStage, m_Payload.m_sNextStage));
			return;
		}

		m_bStageArmed = false;
		TBD_AdminClient.Act(TBD_EAdminAction.STAGE_ADVANCE, 0);
		Rebuild();
		Announce("Forcing the stage -- waiting for the server...");
	}

	//! Clear the force-stage arming.
	protected void DisarmStage()
	{
		m_bStageArmed = false;
	}

	//! @return the server's refusal reason, or "The server did not answer."
	protected string Reason()
	{
		if (m_Payload && !m_Payload.m_sDeniedReason.IsEmpty())
			return m_Payload.m_sDeniedReason;

		return "The server did not answer.";
	}
}
