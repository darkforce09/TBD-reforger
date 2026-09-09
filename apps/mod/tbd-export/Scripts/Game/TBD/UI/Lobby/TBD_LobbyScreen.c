//! T-181.9.1 - the lobby. Where a player spends the only life they get.
//!
//! Supersedes T-068.13. Arma 3 hands you this screen for free; Reforger hands you nothing
//! (`TBD_MOD_DESIGN.md` S1), and under ONE LIFE (S2) choosing a seat is the single most
//! consequential click of the whole event.
//!
//! ```
//!   +------------------------------------------------------------+
//!   |  BRIDGEHEAD AT LEVIE                            [ Back ]   |
//!   |  everon . 23 of 36 seats open . LOBBY                      |
//!   +------------------------------------------------------------+
//!   |  PICK YOUR SEAT                          23 of 36 open     |  <- section
//!   |  US ARMY                        18 seats . 11 open    -    |  <- side, open
//!   |     ALPHA                        9 seats . 4 open     -    |  <- group, open
//!   |        Squad Leader                          Cpl. Hicks    |  <- HELD by another
//!   |        Team Leader                           YOUR SEAT     |  <- yours; click to give up
//!   |        Rifleman                                    OPEN    |  <- one click takes it
//!   |        Grenadier                            Pvt. Vasquez   |
//!   |        Automatic Rifleman                       - down     |  <- DEAD, not selectable
//!   |     BRAVO                        9 seats . 7 open     +    |  <- collapsed
//!   |  USSR                           18 seats . 12 open    +    |  <- collapsed
//!   |                                                            |
//!   |  ORDERS                                                    |
//!   |     View briefing                    read your orders      |
//!   +------------------------------------------------------------+
//!   |  You hold ALPHA . TL. Click it again to give it up.        |
//!   |                                          [ DEPLOY ]        |  <- ONE primary action
//!   +------------------------------------------------------------+
//! ```
//!
//! -- Design law this screen obeys (TBD_MOD_DESIGN.md S2, S6) --------------------------------
//! * **Progressive disclosure - side, then group, then slot.** Never a flat wall of 128 rows.
//!   ONE side is open at a time and ONE group within it, which is what bounds the list: two side
//!   rows, plus a squad list, plus one squad's seats. A 128-slot mission draws about twenty rows
//!   no matter how you navigate it. Letting several groups open at once would let a determined
//!   player rebuild the wall the rule exists to prevent.
//! * **Direct manipulation.** Clicking a seat TAKES it. There is no select-then-confirm step, and
//!   no confirmation dialog - clicking your own seat again gives it back, so the claim is cheap
//!   and reversible. The irreversible act has its own button.
//! * **ONE obvious primary action.** `DEPLOY`, and nothing else is loud. It is shown disabled
//!   rather than hidden while you have no seat, so the goal of the screen is visible from the
//!   moment it opens.
//! * **Optimistic feedback, authoritative truth.** A claim lands on screen before a packet
//!   leaves the machine, and is reconciled by wholesale replacement when the server answers. A
//!   refusal marks the seat, names who beat you to it, and clears itself after five seconds.
//! * **Nothing blocking.** Every answer lands in the footer status line; there is no modal
//!   anywhere in this screen.
//! * **Aegis tokens only.** Every colour comes from `TBD_UITheme` via `TBD_EUIState`; this file
//!   contains no literal colour.
//!
//! -- Where the data comes from, and why the screen is dumb ----------------------------------
//! This screen renders a `TBD_LobbyRoster` and nothing else. It never reads `TBD_MissionLoader`
//! or `TBD_SpawnManager` - it *cannot*, because a client has neither (see the header of
//! `TBD_LobbyData.c`). It also never decides whether a seat may be taken: it draws what the
//! authority reported and sends what the user clicked.
class TBD_LobbyScreen : TBD_ShellScreen
{
	//! How often the open lobby re-asks the server. Slotting is contended and other people's
	//! claims must appear without the player doing anything, so this cannot be on-demand only.
	//!
	//! COST, stated honestly: one full roster per client per interval. A 128-slot mission
	//! serialises to roughly 6 KB, so a hundred players in the lobby is on the order of 300 KB/s
	//! of reliable traffic - real, but bounded, and only during LOBBY. The right fix is a server
	//! push on claim/release; that hook is requested in the slice report rather than taken here,
	//! because it lives in `TBD_SpawnManager.c` which another slice owns this wave.
	static const int REFRESH_MS = 2000;

	//! Row tags. Negative is inert; the bases are spaced far enough apart that a 128-slot mission
	//! cannot collide, so a tag decodes to a kind and an index with no lookup table.
	protected static const int TAG_INERT = -1;
	protected static const int TAG_BRIEFING = 1;
	protected static const int TAG_SELECTOR = 2;
	protected static const int TAG_SIDE_BASE = 1000;
	protected static const int TAG_GROUP_BASE = 2000;
	protected static const int TAG_SLOT_BASE = 3000;

	protected ref TBD_LobbyRoster m_Roster;

	//! Disclosure state. Empty means closed. ONE of each - see the class header.
	protected string m_sOpenSide;
	protected string m_sOpenGroup;

	//! Seeded once from the first roster that arrives, then the user owns it.
	protected bool m_bDisclosureSeeded;

	//! Tag -> identity, rebuilt on every render. Groups and slots need no side/group qualifier
	//! because only one of each is ever open, so a visible row is unambiguous.
	protected ref array<string> m_aSideKeys;
	protected ref array<string> m_aGroupKeys;
	protected ref array<string> m_aSlotKeys;

	//! Visual echo of the seat you hold, restored after every rebuild.
	protected int m_iOwnRowTag;

	//! T-139 - kit preview hosted beside the slot list. Null when the layout did not instantiate.
	protected TBD_LoadoutPreview m_Preview;

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenOpen()
	{
		super.OnScreenOpen();

		m_aSideKeys = {};
		m_aGroupKeys = {};
		m_aSlotKeys = {};
		m_iOwnRowTag = -1;

		// -- No Back. There is nowhere to go back TO. ----------------------------------------
		// The shell offers Back because most screens sit on top of something; the lobby does not.
		// During LOBBY the stack is empty underneath, so "Back" means "dismiss the only screen of
		// this phase and stare at a lineup of bodies you have no way to claim". Under ONE LIFE
		// that is not an escape hatch, it is a trap - so the affordance is removed rather than
		// left to be discovered.
		//
		// The engine's own Esc still closes any menu and this cannot prevent that; `TBD_LobbyStage`
		// puts the picker straight back for as long as the round is in LOBBY and the player has not
		// deployed. Hiding the button is what stops the obvious, discoverable route into that hole.
		TBD_UITheme.Show(Find("BackAction"), false);

		TBD_ListBox list = GetList();
		if (list)
		{
			list.GetOnActivate().Insert(OnRowPicked);
			list.GetOnHighlight().Insert(OnRowHighlighted);
		}

		MountLoadoutPreview();

		GetOnPrimaryAction().Insert(OnDeployPressed);

		// Re-render whenever the roster changes, whether the change came from the server or from
		// our own optimistic edit. One signal, one handler.
		TBD_LobbyClient.GetOnRosterChanged().Insert(OnRosterChanged);

		AdoptRoster(TBD_LobbyClient.GetRoster());

		// Ask immediately, then keep asking: other people are claiming seats while this is open.
		TBD_LobbyClient.Request();
		GetGame().GetCallqueue().CallLater(RequestRefresh, REFRESH_MS, true);

		InputManager im = GetGame().GetInputManager();
		if (im)
		{
			im.ActivateContext("TBD_BrowserContext");
			im.AddActionListener("TBD_MissionSelector", EActionTrigger.DOWN, OnMissionSelectorKey);
			im.AddActionListener("TBD_MissionCycle", EActionTrigger.DOWN, OnMissionSelectorKey);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void OnMissionSelectorKey(float value, EActionTrigger trigger)
	{
		TBD_MissionSelectorComponent.Toggle();
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenUpdate(float tDelta)
	{
		super.OnScreenUpdate(tDelta);

		InputManager im = GetGame().GetInputManager();
		if (im)
			im.ActivateContext("TBD_BrowserContext");
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenClose()
	{
		InputManager im = GetGame().GetInputManager();
		if (im)
		{
			im.RemoveActionListener("TBD_MissionSelector", EActionTrigger.DOWN, OnMissionSelectorKey);
			im.RemoveActionListener("TBD_MissionCycle", EActionTrigger.DOWN, OnMissionSelectorKey);
		}

		GetGame().GetCallqueue().Remove(RequestRefresh);
		GetGame().GetCallqueue().Remove(DeferredClose);

		TBD_ListBox list = GetList();
		if (list)
		{
			list.GetOnActivate().Remove(OnRowPicked);
			list.GetOnHighlight().Remove(OnRowHighlighted);
		}

		m_Preview = null;

		GetOnPrimaryAction().Remove(OnDeployPressed);
		TBD_LobbyClient.GetOnRosterChanged().Remove(OnRosterChanged);

		super.OnScreenClose();
	}

	//------------------------------------------------------------------------------------------------
	override protected string GetScreenTitle()
	{
		if (m_Roster && !m_Roster.m_sMissionName.IsEmpty())
			return m_Roster.m_sMissionName;

		return "LOBBY";
	}

	//------------------------------------------------------------------------------------------------
	//! Where you are, how much room is left, and what phase this is - in one line.
	override protected string GetScreenSubtitle()
	{
		if (!m_Roster)
			return "Asking the server for the roster...";

		if (!m_Roster.IsAvailable())
			return m_Roster.m_sUnavailableReason;

		string stage = m_Roster.m_sStage;
		if (stage.IsEmpty())
			stage = "LOBBY";

		string seats = string.Format("%1 of %2 seats open", m_Roster.TotalOpen(), m_Roster.TotalSeats());

		if (m_Roster.m_sTerrain.IsEmpty())
			return string.Format("%1 . %2", seats, stage);

		return string.Format("%1 . %2 . %3", m_Roster.m_sTerrain, seats, stage);
	}

	// -- Roster ------------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	protected void OnRosterChanged(TBD_LobbyRoster roster)
	{
		AdoptRoster(roster);

		// T-941.2 (2026-09-07) - a body does NOT close this screen. Closing was the old
		// T-181.29 answer to the deploy race: ShouldStandDown deferred-closed the picker,
		// which hid the control instead of locking it, so a second DEPLOY could still be
		// in flight and a seated player could not change slot. See DeferredClose.
	}

	//------------------------------------------------------------------------------------------------
	//! T-941.2 (2026-09-07) DECISION: the guard is the DEPLOY button, not a close.
	//!
	//! The T-181.29 race note at this site used to say the stage watcher may have closed
	//! this screen already (LOBBY -> BRIEFING landing in the same breath as a deploy) and
	//! OnRosterChanged deferred-closed whenever ShouldStandDown(). That closed the race by
	//! destroying the UI. The lock is now RefreshFooter: DEPLOY is shown disabled with a
	//! reason while IsDeployPending() or the player is already in the world, and nothing
	//! spawns during LOBBY (TBD_SpawnManager refuses). After BRIEFING the stage watcher
	//! still drops the picker; PauseMenuUI "Change slot" reopens it so a seated player can
	//! pick a different seat (despawn + redeploy).
	//!
	//! DeferredClose stays as a one-frame stack-safe closer; OnRosterChanged no longer calls it.
	protected void DeferredClose()
	{
		if (IsScreenOpen())
			CloseScreen();
	}

	//------------------------------------------------------------------------------------------------
		protected void AdoptRoster(TBD_LobbyRoster roster)
	{
		m_Roster = roster;

		SeedDisclosure();
		PruneDisclosure();

		SetTitle(GetScreenTitle());
		SetSubtitle(GetScreenSubtitle());

		Rebuild();
		RefreshFooter();
	}

	//------------------------------------------------------------------------------------------------
	//! Open the path to something useful, exactly once, then get out of the user's way.
	//!
	//! If they already hold a seat (a reconnect, or they came back to change their mind) the
	//! screen opens on it. Otherwise a mission with a single side opens that side, because making
	//! someone click "US ARMY" when it is the only choice is a step that carries no information.
	//! With two or more sides, nothing is opened: picking a side is the first real decision and
	//! pre-empting it would bias it.
	protected void SeedDisclosure()
	{
		if (m_bDisclosureSeeded || !m_Roster || !m_Roster.IsAvailable() || m_Roster.m_aSides.IsEmpty())
			return;

		m_bDisclosureSeeded = true;

		foreach (TBD_LobbySide side : m_Roster.m_aSides)
		{
			if (!side.m_bHasOwn)
				continue;

			m_sOpenSide = side.m_sKey;

			foreach (TBD_LobbyGroup group : side.m_aGroups)
			{
				if (group.m_bHasOwn)
					m_sOpenGroup = group.m_sCallsign;
			}

			return;
		}

		if (m_Roster.m_aSides.Count() == 1)
			m_sOpenSide = m_Roster.m_aSides[0].m_sKey;
	}

	//------------------------------------------------------------------------------------------------
	//! Drop disclosure that no longer names anything - an admin mission switch replaces every side
	//! and group, and a dangling name would render as a side that is open but has no rows.
	protected void PruneDisclosure()
	{
		if (!m_Roster || !m_Roster.IsAvailable())
			return;

		TBD_LobbySide open = FindSide(m_sOpenSide);
		if (!open)
		{
			m_sOpenSide = string.Empty;
			m_sOpenGroup = string.Empty;
			return;
		}

		if (m_sOpenGroup.IsEmpty())
			return;

		foreach (TBD_LobbyGroup group : open.m_aGroups)
		{
			if (group.m_sCallsign == m_sOpenGroup)
				return;
		}

		m_sOpenGroup = string.Empty;
	}

	// -- Rendering ---------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Write the whole list from the roster plus the disclosure state. Cheap by construction -
	//! `TBD_ListBox` pools its rows, so a rebuild is property writes on widgets that already exist,
	//! which is what makes a 2 s refresh free.
	protected void Rebuild()
	{
		TBD_ListBox list = GetList();
		if (!list)
			return;

		m_aSideKeys.Clear();
		m_aGroupKeys.Clear();
		m_aSlotKeys.Clear();
		m_iOwnRowTag = -1;

		list.BeginUpdate();

		if (!m_Roster)
		{
			list.AddSection("Asking the server for the roster...");
			list.EndUpdate();
			return;
		}

		if (!m_Roster.IsAvailable())
		{
			// An empty state says why. Never a void.
			list.AddSection(m_Roster.m_sUnavailableReason);
			list.EndUpdate();
			return;
		}

		list.AddSection("PICK YOUR SEAT", string.Format("%1 of %2 open", m_Roster.TotalOpen(), m_Roster.TotalSeats()));

		foreach (TBD_LobbySide side : m_Roster.m_aSides)
		{
			EmitSide(list, side);
		}

		EmitOrders(list);

		list.EndUpdate();

		// After EndUpdate: the list restores its selection inside EndUpdate, and the tag numbering
		// only became meaningful as the rows above were emitted.
		list.SetSelectedTag(m_iOwnRowTag);
		RefreshPreviewFromTag(m_iOwnRowTag);
	}

	//------------------------------------------------------------------------------------------------
	protected void EmitSide(TBD_ListBox list, TBD_LobbySide side)
	{
		bool expanded = side.m_sKey == m_sOpenSide;

		int tag = TAG_SIDE_BASE + m_aSideKeys.Count();
		m_aSideKeys.Insert(side.m_sKey);

		TBD_EUIState state = TBD_EUIState.NORMAL;
		if (side.m_bHasOwn)
			state = TBD_EUIState.ACTIVE;

		list.AddItem(side.m_sName, DetailWithMark(side.m_iSeats, side.m_iOpen, expanded), tag, state, true);

		if (!expanded)
			return;

		foreach (TBD_LobbyGroup group : side.m_aGroups)
		{
			EmitGroup(list, group);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void EmitGroup(TBD_ListBox list, TBD_LobbyGroup group)
	{
		bool expanded = group.m_sCallsign == m_sOpenGroup;

		int tag = TAG_GROUP_BASE + m_aGroupKeys.Count();
		m_aGroupKeys.Insert(group.m_sCallsign);

		TBD_EUIState state = TBD_EUIState.NORMAL;
		if (group.m_bHasOwn)
			state = TBD_EUIState.ACTIVE;

		list.AddItem("   " + group.m_sCallsign, DetailWithMark(group.Seats(), group.m_iOpen, expanded), tag, state, true);

		if (!expanded)
			return;

		foreach (TBD_LobbySlot slot : group.m_aSlots)
		{
			EmitSlot(list, slot);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! One seat. Four visual states, and the enabled flag is what actually stops a dead seat being
	//! clicked - `TBD_ListBox.OnRowActivated` refuses a row that is not selectable, so an
	//! unavailable seat cannot fire a claim even from a keyboard.
	protected void EmitSlot(TBD_ListBox list, TBD_LobbySlot slot)
	{
		int tag = TAG_SLOT_BASE + m_aSlotKeys.Count();
		m_aSlotKeys.Insert(slot.m_sKey);

		string detail;
		TBD_EUIState state;
		bool enabled;

		if (slot.m_bIsOwn && slot.IsDead())
		{
			// Your seat, and your life is spent. It stays yours - `ReleaseSlot` refuses to give up
			// a dead player's seat, on purpose - so it is shown, marked, and not clickable. Making
			// it look available would only produce a refusal.
			detail = "YOUR SEAT - down";
			state = TBD_EUIState.LOCKED;
			enabled = false;
			m_iOwnRowTag = tag;
		}
		else if (slot.m_bIsOwn)
		{
			detail = "YOUR SEAT";
			state = TBD_EUIState.ACTIVE;
			enabled = true;
			m_iOwnRowTag = tag;
		}
		else if (slot.IsDead())
		{
			// Not recyclable and not selectable: the holder spent their life, and under ONE LIFE
			// the seat stays theirs (TBD_SpawnManager.ReleaseSlot refuses to give it up).
			detail = "down";
			if (!slot.m_sHolder.IsEmpty())
				detail = string.Format("%1 - down", slot.m_sHolder);

			state = TBD_EUIState.LOCKED;
			enabled = false;
		}
		else if (slot.IsOpen())
		{
			detail = "OPEN";
			state = TBD_EUIState.NORMAL;
			enabled = true;
		}
		else
		{
			detail = slot.m_sHolder;
			if (detail.IsEmpty())
				detail = "taken";

			state = TBD_EUIState.TAKEN;
			enabled = false;
		}

		// A refusal is louder than the seat's own state for as long as it lasts, because it is the
		// answer to something the player just did.
		if (slot.m_sKey == TBD_LobbyClient.GetRejectedKey())
			state = TBD_EUIState.DANGER;

		list.AddItem("      " + slot.m_sRole, detail, tag, state, enabled);
	}

	//------------------------------------------------------------------------------------------------
	//! The route to the briefing - the seam `TBD_BriefingScreen` explicitly left open for this
	//! slice ("the lobby should offer a 'View briefing' row ... one line, no changes needed here").
	//!
	//! It opens ON TOP of the lobby rather than replacing it, so backing out of the orders returns
	//! to the picker with the player's disclosure intact. Gated on holding a seat because the
	//! briefing fails closed without one: no seat means no side, and the server would answer with
	//! "claim a slot in the lobby first" - better to say that here than to open a screen whose only
	//! content is a complaint.
	protected void EmitOrders(TBD_ListBox list)
	{
		list.AddSection("ORDERS");

		bool hasSlot = m_Roster.HasOwnSlot();

		string detail = "take a seat first";
		TBD_EUIState state = TBD_EUIState.LOCKED;

		if (hasSlot)
		{
			detail = "read your orders";
			state = TBD_EUIState.NORMAL;
		}

		list.AddItem("   View briefing", detail, TAG_BRIEFING, state, hasSlot);
		list.AddItem("   Mission Selector (Mockup)", "open selector", TAG_SELECTOR, TBD_EUIState.NORMAL, true);
	}

	//------------------------------------------------------------------------------------------------
	//! The footer: one line of context, one loud button.
	protected void RefreshFooter()
	{
		RefreshBackAffordance();

		if (!m_Roster || !m_Roster.IsAvailable())
		{
			// Nothing to deploy into yet - but the button still shows, disabled, so the shape of
			// the screen does not change under the player when the roster lands.
			SetPrimaryAction("DEPLOY", false);
			SetStatus(DeriveStatus());
			return;
		}

		// T-941.2 - lock DEPLOY in place rather than closing the screen. Disabled with a
		// reason (DeriveStatus) while a deploy is pending, while the player already has a
		// body, and for the whole of LOBBY (bodies wait for BRIEFING).
		bool pending = TBD_LobbyClient.IsDeployPending();
		bool inWorld = TBD_LobbyClient.ShouldStandDown();
		bool ready = m_Roster.HasOwnSlot()
			&& !m_Roster.m_bLifeSpent
			&& !pending
			&& !inWorld
			&& !IsLobbyPhase();

		SetPrimaryAction("DEPLOY", ready);
		SetStatus(DeriveStatus());
	}

	//------------------------------------------------------------------------------------------------
		//! What the footer says. The client's own line wins while it has one (an in-flight claim, a
	//! refusal, a deploy verdict); otherwise the line is derived from the roster, so it is always
	//! about the player's ACTUAL situation rather than a stale acknowledgement.
	protected string DeriveStatus()
	{
		string live = TBD_LobbyClient.GetStatus();
		if (!live.IsEmpty())
			return live;

		if (TBD_LobbyClient.IsDeployPending())
			return "Deploy in progress.";

		if (!m_Roster)
			return "Waiting for the server...";

		if (!m_Roster.IsAvailable())
			return m_Roster.m_sUnavailableReason;

		if (m_Roster.m_bLifeSpent)
			return "Your life is spent. Only an admin can put you back in.";

		if (TBD_LobbyClient.ShouldStandDown())
			return "You are in the world. Pick a different seat to change slot.";

		if (m_Roster.HasOwnSlot())
		{
			string hold = string.Format("You hold %1. Click it again to give it up.", m_Roster.m_sOwnLabel);
			if (IsLobbyPhase())
				return hold + " You deploy when briefing starts.";
			return hold;
		}

		if (m_Roster.TotalOpen() == 0)
			return "Every seat is taken. Wait for someone to give one up.";

		return "Pick a seat. One life - there is no second chance.";
	}


	// -- Interaction -------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! One click does one thing, immediately. Direct manipulation - there is no confirm step
	//! anywhere on this screen.

	//------------------------------------------------------------------------------------------------
	//! T-139 - park the kit preview in the right column of the shell and shrink the slot list.
	protected void MountLoadoutPreview()
	{
		if (m_Preview)
			return;

		Widget listW = Find("List");
		if (!listW)
			return;

		Widget parent = listW.GetParent();
		if (!parent)
			return;

		FrameSlot.SetOffsets(listW, 48, 162, 304, 136);
		FrameSlot.SetPos(listW, 48, 162);
		FrameSlot.SetSize(listW, -352, -298);

		Widget root = TBD_LoadoutPreview.CreateUnder(parent);
		if (!root)
			return;

		m_Preview = TBD_LoadoutPreview.Cast(root.FindHandler(TBD_LoadoutPreview));
	}

	//------------------------------------------------------------------------------------------------
	//! Hover/focus on a seat row shows that seat's kit without taking it.
	protected void OnRowHighlighted(TBD_ListBox list, int tag)
	{
		if (tag >= TAG_SLOT_BASE)
			RefreshPreviewFromTag(tag);
	}

	//------------------------------------------------------------------------------------------------
	protected void RefreshPreviewFromTag(int tag)
	{
		if (tag < TAG_SLOT_BASE)
			return;
		if (!m_aSlotKeys)
			return;

		int index = tag - TAG_SLOT_BASE;
		if (index < 0)
			return;
		if (index >= m_aSlotKeys.Count())
			return;

		RefreshPreviewForKey(m_aSlotKeys[index], index);
	}

	//------------------------------------------------------------------------------------------------
	protected void RefreshPreviewForKey(string key, int slotIndex)
	{
		if (!m_Preview)
			return;

		TBD_MissionSlotStruct slot = TBD_MissionLoader.GetSlotById(key);
		m_Preview.Refresh(slot, slotIndex);
	}

	protected void OnRowPicked(TBD_ListBox list, int tag)
	{
		if (tag == TAG_BRIEFING)
		{
			TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UIBriefing);
			return;
		}

		if (tag == TAG_SELECTOR)
		{
			TBD_MissionSelectorComponent.Toggle();
			return;
		}

		if (tag >= TAG_SLOT_BASE)
		{
			OnSlotPicked(tag - TAG_SLOT_BASE);
			return;
		}

		if (tag >= TAG_GROUP_BASE)
		{
			OnGroupPicked(tag - TAG_GROUP_BASE);
			return;
		}

		if (tag >= TAG_SIDE_BASE)
			OnSidePicked(tag - TAG_SIDE_BASE);
	}

	//------------------------------------------------------------------------------------------------
	//! Opening a side closes whichever one was open, and forgets the group with it - the group
	//! name belonged to the side you just left.
	protected void OnSidePicked(int index)
	{
		if (index < 0 || index >= m_aSideKeys.Count())
			return;

		string key = m_aSideKeys[index];

		m_sOpenGroup = string.Empty;

		if (m_sOpenSide == key)
			m_sOpenSide = string.Empty;
		else
			m_sOpenSide = key;

		Rebuild();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnGroupPicked(int index)
	{
		if (index < 0 || index >= m_aGroupKeys.Count())
			return;

		string callsign = m_aGroupKeys[index];

		if (m_sOpenGroup == callsign)
			m_sOpenGroup = string.Empty;
		else
			m_sOpenGroup = callsign;

		Rebuild();
	}

	//------------------------------------------------------------------------------------------------
	//! The consequential half of the screen. Clicking your own seat gives it back; clicking an
	//! open one takes it. Both are sent optimistically and reconciled against the server's answer
	//! - see `TBD_LobbyClient`.
	//!
	//! Nothing here decides whether the claim is allowed. That is `TBD_SpawnManager.ClaimSlot`'s
	//! job and it is first-come; this slice's job is to make the contention FEEL fair, not to
	//! re-implement the rule.
	protected void OnSlotPicked(int index)
	{
		if (index < 0 || index >= m_aSlotKeys.Count() || !m_Roster)
			return;

		string key = m_aSlotKeys[index];
		RefreshPreviewForKey(key, index);

		TBD_LobbySlot slot = m_Roster.FindSlot(key);
		if (!slot)
			return;

		if (slot.m_bIsOwn)
		{
			TBD_LobbyClient.Release();
			return;
		}

		// A row in either of these states is emitted disabled, so this is a belt-and-braces guard
		// against a rebuild racing a click rather than the real gate.
		if (!slot.IsOpen())
			return;

		TBD_LobbyClient.Claim(key);
	}

	//------------------------------------------------------------------------------------------------
	//! The one primary action. Not optimistic: a refused deploy must leave the player looking at
	//! the picker, not at a torn-down screen and a world they were never put into.
	//!
	//! Nothing is written to the footer here. `TBD_LobbyClient.Deploy()` latches the in-flight
	//! state and fires a change, which comes straight back through `OnRosterChanged` and repaints
	//! the whole footer from one place - so the button cannot be disabled here and quietly
	//! re-enabled by the next roster refresh two seconds later.
	protected void OnDeployPressed(TBD_ShellScreen screen)
	{
		if (!m_Roster || !m_Roster.HasOwnSlot() || TBD_LobbyClient.IsDeployPending())
			return;
		if (IsLobbyPhase() || TBD_LobbyClient.ShouldStandDown())
			return;

		TBD_LobbyClient.Deploy();
	}

	//------------------------------------------------------------------------------------------------
	//! Timer target. Split from `TBD_LobbyClient.Request` so `Callqueue.Remove` has a stable
	//! instance method to cancel.
	protected void RequestRefresh()
	{
		TBD_LobbyClient.Request();
	}

	// -- Helpers -----------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! T-941.2 - LOBBY is bodiless. An empty stage string is the roster's pre-stage default.
	protected bool IsLobbyPhase()
	{
		if (!m_Roster)
			return true;

		string stage = m_Roster.m_sStage;
		return stage.IsEmpty() || stage == "LOBBY";
	}

	//------------------------------------------------------------------------------------------------
	//! During LOBBY there is nowhere to go back TO, so Back stays hidden. After BRIEFING this
	//! screen is a pause-menu overlay: Back (and Esc) dismiss it and return to the body.
	protected void RefreshBackAffordance()
	{
		TBD_UITheme.Show(Find("BackAction"), !IsLobbyPhase());
	}

	//------------------------------------------------------------------------------------------------
	//! T-941.2 - pause-menu "Change slot" lands here after the pause menu has closed.
	static void OpenFromPause()
	{
		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
			return;

		TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UILobby);
	}

	//------------------------------------------------------------------------------------------------
		protected TBD_LobbySide FindSide(string key)
	{
		if (key.IsEmpty() || !m_Roster)
			return null;

		foreach (TBD_LobbySide side : m_Roster.m_aSides)
		{
			if (side.m_sKey == key)
				return side;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! "18 seats . 11 open  +" - how full a branch is, without opening it. This is the whole
	//! reason progressive disclosure is navigable rather than a guessing game.
	protected string DetailWithMark(int seats, int open, bool expanded)
	{
		return string.Format("%1 seats . %2 open  %3", seats, open, DisclosureMark(expanded));
	}

	//------------------------------------------------------------------------------------------------
	//! The only affordance a row has for "there is more behind me". Text, not an icon, because the
	//! shared row layout carries no image slot for one.
	//!
	//! Deliberately plain ASCII, for the reason `TBD_BriefingScreen` records: nothing in this lane
	//! renders a framebuffer, so the UI font's glyph coverage is unverifiable here, and a geometric
	//! triangle the font lacks would draw as a tofu box. `+`/`-` cannot miss.
	protected string DisclosureMark(bool expanded)
	{
		if (expanded)
			return "-";

		return "+";
	}
}

//! The lobby preset. Bound to a layout and this class in `Configs/System/chimeraMenus.conf`.
//!
//! It reuses `TBD_ScreenShell.layout` unchanged - the shell was designed to be subclassed exactly
//! like this, and reusing it means this slice ships **no new `.layout`**, so the only non-script
//! resource it adds is the preset block itself.
//!
//! -- KNOWN BLOCKER, expected, not this slice's ----------------------------------------------
//! Adding this enum value and the `.conf` block is necessary but NOT sufficient. Until the addon's
//! `resourceDatabase.rdb` lists `Configs/System/chimeraMenus.conf`, the engine cannot see the
//! preset and logs, at every startup:
//!
//!     GUI       (E): Menu preset 'TBD_UILobby' not found!
//!
//! Only a Workbench pass regenerates that index; the headless compile lane cannot. Everything in
//! this slice compiles and is structurally complete; the screen cannot OPEN until that one pass.
//! That startup line is also the exact green light - it disappears the moment the resource is
//! registered, and it is cheap to check from the headless lane:
//! `grep "Menu preset" <profile>/logs/logs_*/error.log`.
modded enum ChimeraMenuPreset
{
	TBD_UILobby
}

//! T-941.2 - reopen the lobby from the pause menu so a player who already has a body can
//! change slot. Reuses the vanilla LeaveFaction row: TBD_GameMode.et sets
//! m_bAllowFactionChange 0, so that button is hidden and unwired; we show it, relabel it,
//! and send it to TBD_LobbyScreen.OpenFromPause. Respawn is left alone (ONE LIFE).
modded class PauseMenuUI
{
	protected SCR_ButtonTextComponent m_TbdChangeSlotButton;

	//------------------------------------------------------------------------------------------------
	override void OnMenuOpen()
	{
		super.OnMenuOpen();
		HookTbdChangeSlot();
	}

	//------------------------------------------------------------------------------------------------
	override void OnMenuClose()
	{
		if (m_TbdChangeSlotButton)
		{
			m_TbdChangeSlotButton.m_OnClicked.Remove(OnTbdChangeSlot);
			m_TbdChangeSlotButton = null;
		}

		super.OnMenuClose();
	}

	//------------------------------------------------------------------------------------------------
	protected void HookTbdChangeSlot()
	{
		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		TBD_EGameStage stage = fm.GetStage();
		if (stage != TBD_EGameStage.BRIEFING
			&& stage != TBD_EGameStage.SAFE_START
			&& stage != TBD_EGameStage.LIVE)
			return;

		Widget root = GetRootWidget();
		if (!root)
			return;

		SCR_ButtonTextComponent btn = SCR_ButtonTextComponent.GetButtonText("LeaveFaction", root);
		if (!btn)
			return;

		Widget row = btn.GetRootWidget();
		if (row)
			row.SetVisible(true);

		btn.SetText("Change slot");
		btn.SetEnabled(true);
		btn.m_OnClicked.Insert(OnTbdChangeSlot);
		m_TbdChangeSlotButton = btn;
	}

	//------------------------------------------------------------------------------------------------
	protected void OnTbdChangeSlot()
	{
		Close();
		GetGame().GetCallqueue().CallLater(TBD_LobbyScreen.OpenFromPause, 0, false);
	}
}
