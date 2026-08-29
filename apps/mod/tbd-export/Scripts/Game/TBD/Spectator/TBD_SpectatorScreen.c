//! T-181.12 - the unit list. Who is still alive, by faction and by group, and one click to watch
//! any of them.
//!
//! Built on the T-181.7 framework rather than a bespoke widget, and reusing
//! `TBD_ScreenShell.layout` unchanged - the shell was designed to be subclassed exactly like this
//! ("register the subclass in chimeraMenus.conf against your own preset"). Reusing it means this
//! slice ships **no new `.layout`**, so the only non-script resource it adds is the preset line
//! itself. `TBD_ListBox` pools its rows, so the refresh timer below costs property writes, not
//! widget churn, for the whole rest of the event.
//!
//! -- The interaction, and why it is shaped this way ------------------------------------------
//! One obvious primary action: **FREE CAMERA**. It is the way out of anything, it is always
//! available, and it is the only loud button on the screen.
//!
//! Everything else is direct manipulation on the rows:
//!   * click a player          -> follow them (third person). No select-then-confirm step.
//!   * click them AGAIN        -> first person, through their eyes. Click again to come back.
//! The status line says which of the three you are in, and what clicking again will do, so the
//! second click is discoverable without a legend. That is also why first person is not a second
//! button: design law allows exactly one.
//!
//! The backdrop is repainted transparent and the panel to `SURFACE_GLASS`, so the world you are
//! flying stays visible behind the list. Nothing blocking - the camera keeps moving while this is
//! open, because a spectator who has to close a menu to look at something has been given a modal
//! dialog with extra steps.
class TBD_SpectatorScreen : TBD_ShellScreen
{
	//! Slow enough to be free, fast enough that a kill disappears from the list while you are
	//! still looking at it. The list is pooled, so this is a property-write pass, not a rebuild.
	static const int REFRESH_MS = 1000;

	protected ref array<ref TBD_SpectatorTarget> m_aTargets;
	protected int m_iNotInView;

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenOpen()
	{
		super.OnScreenOpen();

		m_aTargets = {};

		// The shell paints a full-bleed scrim by default, which is right for the lobby and wrong
		// here: this list sits over a live camera the player is still flying. Glass, not a wall.
		TBD_UITheme.Paint(Find("Backdrop"), TBD_UITheme.TRANSPARENT);
		TBD_UITheme.Paint(Find("Panel"), TBD_UITheme.SURFACE_GLASS);

		TBD_ListBox list = GetList();
		if (list)
			list.GetOnActivate().Insert(OnTargetPicked);

		GetOnPrimaryAction().Insert(OnFreeCameraPicked);
		SetPrimaryAction("FREE CAMERA", true);

		Refresh();
		GetGame().GetCallqueue().CallLater(Refresh, REFRESH_MS, true);
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenClose()
	{
		GetGame().GetCallqueue().Remove(Refresh);

		TBD_ListBox list = GetList();
		if (list)
			list.GetOnActivate().Remove(OnTargetPicked);

		GetOnPrimaryAction().Remove(OnFreeCameraPicked);

		super.OnScreenClose();
	}

	//------------------------------------------------------------------------------------------------
	override protected string GetScreenTitle()
	{
		return "SPECTATOR";
	}

	//------------------------------------------------------------------------------------------------
	override protected string GetScreenSubtitle()
	{
		if (TBD_SpectatorTargets.IsFactionRestricted())
			return "Your life is spent. You may watch your own side.";

		return "Your life is spent. All sides visible.";
	}

	// -- Content -----------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! Rebuild the list. Progressive disclosure: faction is a section, group is a section, and only
	//! the people are rows - so a full 128-slot mission reads as a handful of headings and the
	//! dozen names that are actually near you, not a wall.
	void Refresh()
	{
		TBD_ListBox list = GetList();
		if (!list)
			return;

		TBD_SpectatorTargets.Collect(m_aTargets, m_iNotInView);

		int followed = TBD_SpectatorController.GetFollowedPlayerId();

		list.BeginUpdate();

		string currentFaction;
		string currentGroup;

		foreach (TBD_SpectatorTarget target : m_aTargets)
		{
			if (target.m_sFactionName != currentFaction)
			{
				currentFaction = target.m_sFactionName;
				currentGroup = string.Empty;
				list.AddSection(currentFaction);
			}

			if (target.m_sGroupName != currentGroup)
			{
				currentGroup = target.m_sGroupName;
				list.AddSection(string.Format("   %1", currentGroup));
			}

			bool isFollowed = target.m_iPlayerId == followed;

			string detail;
			if (isFollowed)
			{
				if (TBD_SpectatorController.IsFirstPerson())
					detail = "FIRST PERSON";
				else
					detail = "FOLLOWING";
			}

			TBD_EUIState state = TBD_EUIState.NORMAL;
			if (isFollowed)
				state = TBD_EUIState.ACTIVE;

			list.AddItem(target.m_sName, detail, target.m_iPlayerId, state, true);
		}

		// Honest about what the list is NOT showing. See the streaming landmine on
		// TBD_SpectatorController: a player outside this client's replication range has never been
		// sent to this machine and cannot be rendered by any camera, so offering them as a target
		// would be a promise we cannot keep. Counting them is the truth; hiding them silently is
		// not, because "the list is empty" and "everyone is far away" are very different facts.
		if (m_iNotInView > 0)
			list.AddSection(string.Format("%1 more not in view - fly closer", m_iNotInView));

		list.EndUpdate();

		list.SetSelectedTag(followed);

		SetStatus(BuildStatus());
	}

	//------------------------------------------------------------------------------------------------
	//! What the camera is doing plus, when there is nothing to watch, why.
	protected string BuildStatus()
	{
		if (m_aTargets.IsEmpty())
		{
			if (TBD_SpectatorTargets.IsFactionRestricted() && TBD_SpectatorTargets.GetViewerFactionKey().IsEmpty())
				return "Your faction could not be resolved - no targets shown.";

			if (m_iNotInView > 0)
				return "Nobody alive nearby. Fly toward the AO to pick players up.";

			return "Nobody left alive to watch.";
		}

		return TBD_SpectatorController.GetStatusLine();
	}

	// -- Interaction -------------------------------------------------------------------------

	//------------------------------------------------------------------------------------------------
	//! One click follows. A second click on the same player toggles first person - the status line
	//! advertises it, so there is nothing to memorise.
	protected void OnTargetPicked(TBD_ListBox list, int tag)
	{
		if (tag <= 0)
			return;

		if (TBD_SpectatorController.GetFollowedPlayerId() == tag)
		{
			TBD_SpectatorController.ToggleFirstPerson();
		}
		else if (!TBD_SpectatorController.FollowPlayer(tag, false))
		{
			// The player died between the last refresh and this click. Say so instead of leaving
			// the camera pointed at a corpse and the row looking selected.
			SetStatus("That player is no longer alive - back to free camera.");
			Refresh();
			return;
		}

		Refresh();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnFreeCameraPicked(TBD_ShellScreen screen)
	{
		TBD_SpectatorController.SetFree();
		Refresh();
	}
}

//! The spectator roster. Bound to the shared shell layout in
//! `Configs/System/chimeraMenus.conf`; see the registration note in TBD_ShellScreen for why that
//! file needs one Workbench pass before the engine can see it.
modded enum ChimeraMenuPreset
{
	TBD_Spectator
}
