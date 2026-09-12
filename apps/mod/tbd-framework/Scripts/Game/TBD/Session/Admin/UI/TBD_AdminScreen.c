//! T-181.11.2 — the admin menu. Under ONE LIFE this screen is the event's safety valve: it is the
//! only way a player who died to a glitch gets back in.
//!
//! ```
//!   ┌──────────────────────────────────────────────────────────────┐
//!   │  ADMIN                                          [ Back ]     │
//!   │  LIVE · 12 connected · 2 lives spent                         │
//!   ├──────────────────────────────────────────────────────────────┤
//!   │  MISSION                              Bridgehead at Levie    │  <- section
//!   │    Validation             FAILED — 3 error(s), 1 warning  +  │  <- pick to disclose
//!   │  STAGE                                            LIVE       │  <- section
//!   │    Force stage -> END               irreversible · pick twice │  <- arm, then confirm
//!   │  PLAYERS                            12 · 2 lives spent       │  <- section
//!   │    Cpl. Hicks             ADMIN · us_army · ALPHA/SL · in    │
//!   │    Pvt. Vasquez         us_army · ALPHA/RFL · LIFE SPENT     │  <- pick, then RESPAWN
//!   │  ADMIN ACTIONS                              4 this session   │  <- section
//!   │    Show the audit trail                              +       │
//!   ├──────────────────────────────────────────────────────────────┤
//!   │  ONE LIFE — Respawn hands Vasquez …    [ RESPAWN VASQUEZ ]   │  <- ONE primary action
//!   └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! ── Design law this screen obeys (TBD_MOD_DESIGN.md §2, §6) ────────────────────────────────
//! * **ONE obvious primary action.** Whatever recovers the currently selected player, and nothing
//!   else. The shell physically cannot grow a second loud button.
//! * **Progressive disclosure.** Validator findings and the audit trail are one pick away, not a
//!   wall of text an admin has to scroll past to reach the player they came here for.
//! * **Immediate feedback, nothing blocking.** The server's verdict lands in the footer status
//!   line. No modal, ever — an admin fixing a broken round must never be stuck behind a dialog.
//! * **Aegis tokens only.** Every colour comes from `TBD_UITheme`; this file contains no literal.
//!
//! ── Honesty about what the buttons do ───────────────────────────────────────────────────────
//! "Respawn" under ONE LIFE is not a normal respawn — TBD events are one life, death is terminal
//! by design, and this spends the event's single sanctioned exception. The screen says so in the
//! status line every time a dead player is selected, in those words, because an admin under
//! pressure should not have to remember the design doc.
//!
//! ── Why this screen is dumb ─────────────────────────────────────────────────────────────────
//! It renders a `TBD_AdminPayload` and nothing else. It cannot read `TBD_SpawnManager`,
//! `TBD_MissionLoader` or `TBD_MissionValidator` — on a dedicated server none of them hold
//! anything in the client's process (see the header of `TBD_AdminData.c`). Every list here was
//! built on the authority and sent to this one client. There is therefore no local state a
//! patched client could render instead, and no local path from a widget to a power.
class TBD_AdminScreen : TBD_ShellScreen
{
	//! How often the open screen re-asks the server. An admin panel that lies about who is alive is
	//! worse than no panel; 3 s is well inside human reaction time and costs one small string.
	protected static const int REFRESH_MS = 3000;

	//! Row tags. Negative = inert content; the rest decode without a lookup table.
	protected static const int TAG_INERT = -1;
	protected static const int TAG_VALIDATION = 1;
	protected static const int TAG_STAGE = 2;
	protected static const int TAG_AUDIT = 3;
	protected static const int TAG_PLAYER_BASE = 1000;

	protected ref TBD_AdminPayload m_Payload;

	//! Selection is held as a PLAYER ID, never a row index: the roster is rebuilt every 3 s and a
	//! player who disconnects shifts every index below them. An id cannot be aimed at the wrong
	//! person by a refresh.
	protected int m_iSelectedPlayer = -1;

	//! Disclosure state. Survives a refresh — a rebuild must not collapse what the admin opened.
	protected bool m_bValidationExpanded;
	protected bool m_bAuditExpanded;

	//! Force-stage is armed by the first pick and executed by the second. Not a modal (design law:
	//! nothing blocking) and not a bare one-click either, because this one moves the round for
	//! everybody and cannot be undone.
	protected bool m_bStageArmed;

	//! Stage the round was in when the admin armed. A refresh that moves the stage underneath them
	//! disarms, so a confirming second pick can never land on a transition they did not read.
	protected string m_sLastStage;

	//! The last thing the SERVER said about an action, held so the 3 s refresh cannot wipe it off
	//! the footer a heartbeat after it arrives. Cleared the moment the admin picks something else.
	protected string m_sPendingResult;

	//------------------------------------------------------------------------------------------------
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

		// A poll, not a subscription: the things this screen shows (who is alive, who has a body,
		// what the stage is) live in server-side maps with no replicated change notification to
		// hang off, and the two classes that own them belong to other slices this wave.
		GetGame().GetCallqueue().CallLater(Poll, REFRESH_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenClose()
	{
		// Statics outlive the screen; a live repeat pointed at a destroyed menu is exactly the kind
		// of leak this codebase has already measured once.
		GetGame().GetCallqueue().Remove(Poll);

		TBD_ListBox list = GetList();
		if (list)
			list.GetOnActivate().Remove(OnRowPicked);

		GetOnPrimaryAction().Remove(OnPrimaryPressed);

		TBD_AdminClient.GetOnPayloadChanged().Remove(OnPayloadChanged);
		TBD_AdminClient.GetOnActionResult().Remove(OnActionResult);

		super.OnScreenClose();
	}

	//------------------------------------------------------------------------------------------------
	override protected string GetScreenTitle()
	{
		return "ADMIN";
	}

	//------------------------------------------------------------------------------------------------
	//! Stage, headcount and how many lives the round has already spent — the three numbers an admin
	//! wants before they have read anything else.
	override protected string GetScreenSubtitle()
	{
		if (!m_Payload)
			return "Asking the server…";

		if (!m_Payload.m_bAuthorised)
			return "Not authorised";

		return string.Format("%1 · %2 connected · %3 lives spent",
			m_Payload.m_sStage, m_Payload.m_iConnected, m_Payload.m_iSpent);
	}

	// ── Data ────────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected void Poll()
	{
		TBD_AdminClient.Request();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnPayloadChanged(TBD_AdminPayload payload)
	{
		AdoptPayload(payload);
	}

	//------------------------------------------------------------------------------------------------
	//! Take a new snapshot without throwing away what the admin was doing: disclosure stays open,
	//! the selected player survives if they are still connected, and the server's last verdict
	//! stays on the footer.
	protected void AdoptPayload(TBD_AdminPayload payload)
	{
		m_Payload = payload;

		if (m_Payload && m_iSelectedPlayer > 0 && !m_Payload.FindPlayer(m_iSelectedPlayer))
		{
			// They left. Drop the selection rather than leave a loud button aimed at nobody.
			m_iSelectedPlayer = -1;
		}

		// The round moved while the admin was reading. Anything they armed was armed against a
		// stage that no longer exists, so it must not survive into this one.
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

	//------------------------------------------------------------------------------------------------
	//! The authority's own words, verbatim, and they STAY on screen.
	//!
	//! A fresh snapshot follows on the same round trip and the poll fires every 3 s after that —
	//! so without holding this, the one line telling the admin whether the respawn worked would be
	//! overwritten by contextual guidance within a heartbeat of arriving.
	protected void OnActionResult(string message, bool ok)
	{
		m_sPendingResult = message;
		SetStatus(message);
	}

	// ── Rendering ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Write the whole list from the snapshot plus the disclosure flags. Cheap by construction —
	//! `TBD_ListBox` pools its rows, so a rebuild is property writes on widgets that already exist.
	protected void Rebuild()
	{
		TBD_ListBox list = GetList();
		if (!list)
			return;

		list.BeginUpdate();

		if (!m_Payload)
		{
			list.AddSection("Asking the server…");
			list.EndUpdate();
			return;
		}

		if (!m_Payload.m_bAuthorised)
		{
			// An empty state says why. Never a void — and never anything the server did not send.
			list.AddSection(Reason());
			list.EndUpdate();
			return;
		}

		EmitMission(list);
		EmitStage(list);
		EmitPlayers(list);
		EmitAudit(list);

		list.EndUpdate();

		// Re-assert the visual selection: EndUpdate re-applies it from the tag, and the tag survives
		// a rebuild because it is derived from the player id, not the row order.
		list.SetSelectedTag(PlayerTag(m_iSelectedPlayer));
	}

	//------------------------------------------------------------------------------------------------
	//! Mission identity, and the validator verdict.
	//!
	//! T-181.14: a mission the validator REJECTED never leaves LOADING and nothing in game says so.
	//! This is where that becomes visible, which is the whole reason it is on the admin screen and
	//! not buried in a log an operator would have to SSH for.
	protected void EmitMission(TBD_ListBox list)
	{
		string name = m_Payload.m_sMissionName;
		if (name.IsEmpty())
			name = "none loaded";
		else if (!m_Payload.m_sTerrain.IsEmpty())
			name = string.Format("%1 · %2", name, m_Payload.m_sTerrain);

		list.AddSection("MISSION", name);

		if (!m_Payload.m_bValidationRun)
		{
			list.AddItem("    Validation", "not run yet", TAG_INERT, TBD_EUIState.NORMAL, false);
			return;
		}

		string verdict = string.Format("PASSED — %1 warning(s)", m_Payload.m_iValidationWarnings);
		TBD_EUIState state = TBD_EUIState.NORMAL;

		if (!m_Payload.m_bValidationPassed)
		{
			verdict = string.Format("FAILED — %1 error(s), %2 warning(s)",
				m_Payload.m_iValidationErrors, m_Payload.m_iValidationWarnings);
			state = TBD_EUIState.DANGER;
		}

		bool hasFindings = !m_Payload.m_aValidationLines.IsEmpty();
		if (hasFindings)
			verdict = string.Format("%1  %2", verdict, DisclosureMark(m_bValidationExpanded));

		list.AddItem("    Validation", verdict, TAG_VALIDATION, state, hasFindings);

		if (!hasFindings || !m_bValidationExpanded)
			return;

		foreach (string finding : m_Payload.m_aValidationLines)
		{
			list.AddItem("        " + finding, string.Empty, TAG_INERT, state, false);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Stage, and the force-advance lever.
	//!
	//! Exposed on purpose: the stage machine is exactly what strands an event (a rejected mission
	//! sits in LOADING forever), and an admin is the only recovery. Armed-then-confirmed because it
	//! moves the round for everybody and there is no undo.
	protected void EmitStage(TBD_ListBox list)
	{
		list.AddSection("STAGE", m_Payload.m_sStage);

		if (!m_Payload.m_bStageReady)
		{
			list.AddItem("    Force stage", "the stage machine is not up yet", TAG_INERT, TBD_EUIState.NORMAL, false);
			return;
		}

		if (m_Payload.m_sNextStage.IsEmpty())
		{
			list.AddItem("    Force stage", "already at the last stage", TAG_INERT, TBD_EUIState.NORMAL, false);
			return;
		}

		// ASCII arrow on purpose. `·`, `—` and `…` are already rendered by the shipped briefing
		// screen so they are the established set; `→` appears nowhere in a widget in this codebase,
		// and nothing in this lane can render a framebuffer to find out whether the UI font has it.
		// A missing glyph would draw as a tofu box on the one control that moves the whole round.
		string label = string.Format("    Force stage -> %1", m_Payload.m_sNextStage);

		if (m_bStageArmed)
		{
			list.AddItem(label, "ARMED — pick again to move the whole round", TAG_STAGE, TBD_EUIState.DANGER, true);
			return;
		}

		list.AddItem(label, "irreversible · pick twice", TAG_STAGE, TBD_EUIState.NORMAL, true);
	}

	//------------------------------------------------------------------------------------------------
	//! Who is here, and what state they are in. This is the list the headline action operates on.
	protected void EmitPlayers(TBD_ListBox list)
	{
		list.AddSection("PLAYERS", string.Format("%1 connected · %2 lives spent",
			m_Payload.m_iConnected, m_Payload.m_iSpent));

		if (m_Payload.m_aPlayers.IsEmpty())
		{
			list.AddItem("    Nobody is connected.", string.Empty, TAG_INERT, TBD_EUIState.NORMAL, false);
			return;
		}

		foreach (TBD_AdminPlayerRow row : m_Payload.m_aPlayers)
		{
			list.AddItem("    " + row.m_sName, DescribePlayer(row), PlayerTag(row.m_iPlayerId),
				PlayerState(row), true);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! `ADMIN · us_army · ALPHA/RFL · LIFE SPENT` — seat first, then the thing an admin acts on.
	protected string DescribePlayer(TBD_AdminPlayerRow row)
	{
		string detail;

		if (row.m_bIsAdmin)
			detail = "ADMIN";

		if (row.m_bHasSlot)
		{
			string seat = string.Format("%1 · %2/%3", row.m_sFaction, row.m_sGroup, row.m_sRole);
			if (detail.IsEmpty())
				detail = seat;
			else
				detail = detail + " · " + seat;
		}
		else
		{
			if (detail.IsEmpty())
				detail = "no slot";
			else
				detail = detail + " · no slot";
		}

		string status = "in world";
		if (row.m_bDead)
			status = "LIFE SPENT";
		else if (!row.m_bInWorld)
			status = "NO BODY";

		return detail + " · " + status;
	}

	//------------------------------------------------------------------------------------------------
	//! Colour carries the triage: a spent life is the thing this screen exists for, and a player
	//! with no body is the other thing that needs an admin.
	protected TBD_EUIState PlayerState(TBD_AdminPlayerRow row)
	{
		if (row.m_bDead)
			return TBD_EUIState.DANGER;

		if (!row.m_bInWorld)
			return TBD_EUIState.TAKEN;

		return TBD_EUIState.NORMAL;
	}

	//------------------------------------------------------------------------------------------------
	//! The audit trail — who did what to whom. These are powers, not conveniences, so their use is
	//! on the same screen that grants them rather than in a log nobody opens.
	protected void EmitAudit(TBD_ListBox list)
	{
		list.AddSection("ADMIN ACTIONS", string.Format("%1 this session", m_Payload.m_iAuditTotal));

		if (m_Payload.m_aAudit.IsEmpty())
		{
			list.AddItem("    No admin actions yet.", string.Empty, TAG_INERT, TBD_EUIState.NORMAL, false);
			return;
		}

		list.AddItem("    Show the audit trail", DisclosureMark(m_bAuditExpanded), TAG_AUDIT,
			TBD_EUIState.NORMAL, true);

		if (!m_bAuditExpanded)
			return;

		foreach (TBD_AdminAuditRow audit : m_Payload.m_aAudit)
		{
			TBD_EUIState state = TBD_EUIState.NORMAL;
			if (audit.m_bDenied)
				state = TBD_EUIState.DANGER;

			list.AddItem("        " + audit.m_sTime, audit.m_sText, TAG_INERT, state, false);
		}
	}

	// ── The one primary action ──────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! The recovery the selected player needs — and the honest sentence about what it costs.
	//!
	//! Respawn and Deploy are deliberately NOT one button. `AdminRespawn` refuses a player who is
	//! not dead and `DeployPlayerEx` refuses one who is, so a merged button would silently do
	//! nothing half the time.
	protected void RefreshFooter()
	{
		SetPrimaryAction(PrimaryLabel(), PrimaryEnabled());
		SetStatus(ComposeStatus());
	}

	//------------------------------------------------------------------------------------------------
	//! The footer line. The server's last verdict wins over guidance — an admin who just pressed
	//! RESPAWN needs to know what happened more than they need to be told what the button does.
	protected string ComposeStatus()
	{
		if (!m_sPendingResult.IsEmpty())
			return m_sPendingResult;

		if (!m_Payload)
			return "Waiting for the server…";

		if (!m_Payload.m_bAuthorised)
			return Reason();

		TBD_AdminPlayerRow row = m_Payload.FindPlayer(m_iSelectedPlayer);
		if (!row)
			return "Pick a player to act on them.";

		if (row.m_bDead)
		{
			// Say what it actually costs, in the words the design doc uses. TBD events are ONE
			// LIFE and death is terminal by design; this is the single sanctioned exception.
			return string.Format("ONE LIFE — this hands %1 their life back and rebuilds them on their own slot. It is the event's escape hatch: use it for glitch deaths, not for losing a fight.",
				row.m_sName);
		}

		if (!row.m_bInWorld)
		{
			return string.Format("%1 still has their life but no body — this puts them in the world. It neither spends nor restores a life.",
				row.m_sName);
		}

		return string.Format("%1 is alive and in the world — nothing to recover.", row.m_sName);
	}

	//------------------------------------------------------------------------------------------------
	protected string PrimaryLabel()
	{
		TBD_AdminPlayerRow row = SelectedActionable();
		if (!row)
			return string.Empty;

		if (row.m_bDead)
			return string.Format("RESPAWN %1", row.m_sName);

		return string.Format("DEPLOY %1", row.m_sName);
	}

	//------------------------------------------------------------------------------------------------
	protected bool PrimaryEnabled()
	{
		return SelectedActionable() != null;
	}

	//------------------------------------------------------------------------------------------------
	//! The selected player IF there is something an admin can actually do for them. Null otherwise,
	//! which is what hides the loud button — a screen with nothing to commit shows no primary at all.
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

	//------------------------------------------------------------------------------------------------
	protected void OnPrimaryPressed(TBD_ShellScreen screen)
	{
		TBD_AdminPlayerRow row = SelectedActionable();
		if (!row)
			return;

		// The client picks which action to ASK for; the server decides whether it happens. This
		// branch is a convenience, never a permission — `TBD_AdminService` re-derives the caller,
		// re-checks the admin list, and refuses a respawn for a live player or a deploy for a dead
		// one regardless of which one the client asked for.
		if (row.m_bDead)
		{
			TBD_AdminClient.Act(TBD_EAdminAction.RESPAWN, row.m_iPlayerId);
			Announce(string.Format("Respawning %1 — waiting for the server…", row.m_sName));
			return;
		}

		TBD_AdminClient.Act(TBD_EAdminAction.DEPLOY, row.m_iPlayerId);
		Announce(string.Format("Deploying %1 — waiting for the server…", row.m_sName));
	}

	//------------------------------------------------------------------------------------------------
	//! Put a line in the footer and hold it there until the admin picks something else. Used for
	//! optimistic "asking the server…" feedback, which the server's real answer then overwrites.
	protected void Announce(string text)
	{
		m_sPendingResult = text;
		SetStatus(text);
	}

	// ── Interaction ─────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected void OnRowPicked(TBD_ListBox list, int tag)
	{
		if (tag == TAG_STAGE)
		{
			OnStagePicked();
			return;
		}

		// Any other pick disarms a primed stage change, so it can never be the accidental second
		// half of an unrelated pair of clicks, and clears the last verdict so the footer goes back
		// to describing what the admin is now looking at.
		DisarmStage();
		m_sPendingResult = string.Empty;

		if (tag == TAG_VALIDATION)
		{
			m_bValidationExpanded = !m_bValidationExpanded;
			Rebuild();
			RefreshFooter();
			return;
		}

		if (tag == TAG_AUDIT)
		{
			m_bAuditExpanded = !m_bAuditExpanded;
			Rebuild();
			RefreshFooter();
			return;
		}

		if (tag >= TAG_PLAYER_BASE)
		{
			m_iSelectedPlayer = tag - TAG_PLAYER_BASE;
			Rebuild();
			RefreshFooter();
		}
	}

	//------------------------------------------------------------------------------------------------
	//! First pick arms, second pick fires.
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
		Announce("Forcing the stage — waiting for the server…");
	}

	//------------------------------------------------------------------------------------------------
	protected void DisarmStage()
	{
		m_bStageArmed = false;
	}

	// ── Helpers ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Player rows carry `TAG_PLAYER_BASE + playerId`, so a tag decodes straight back to a player
	//! without a side table that a rebuild could desynchronise.
	protected int PlayerTag(int playerId)
	{
		if (playerId <= 0)
			return -1;

		return TAG_PLAYER_BASE + playerId;
	}

	//------------------------------------------------------------------------------------------------
	protected string Reason()
	{
		if (m_Payload && !m_Payload.m_sDeniedReason.IsEmpty())
			return m_Payload.m_sDeniedReason;

		return "The server did not answer.";
	}

	//------------------------------------------------------------------------------------------------
	//! The only affordance a row has for "there is more behind me". Deliberately plain ASCII:
	//! nothing in this lane can render a framebuffer, so the UI font's glyph coverage is
	//! unverifiable here — a geometric triangle the font lacks would draw as a tofu box.
	protected string DisclosureMark(bool expanded)
	{
		if (expanded)
			return "-";

		return "+";
	}
}

//! The admin preset. Bound to a layout and this class in `Configs/System/chimeraMenus.conf`.
//!
//! ── KNOWN BLOCKER, expected, not this slice's ──────────────────────────────────────────────
//! Adding this enum value and the `.conf` block is necessary but NOT sufficient. Until the addon's
//! `resourceDatabase.rdb` lists `Configs/System/chimeraMenus.conf`, the engine cannot see the
//! preset and logs, at every startup:
//!
//!     GUI       (E): Menu preset 'TBD_UIAdmin' not found!
//!
//! Only a Workbench pass regenerates that index; the headless compile lane cannot. Everything in
//! this slice compiles and is structurally complete; the screen cannot OPEN until that one pass.
//! Same wall the shell hit at T-181.7 and the briefing at T-181.9.2 — see the measured note in
//! `TBD_UILayouts`.
//!
//! **The `#tbd` chat commands are unaffected and remain the operable admin surface meanwhile**,
//! and they now write to the same audit trail this screen reads.
modded enum ChimeraMenuPreset
{
	TBD_UIAdmin
}
