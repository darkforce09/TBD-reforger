/**
 * @file TBD_SpectatorScreen.c
 * @brief The spectator roster: who is still alive, by faction and group, with one click to watch any of them.
 *
 * Role: lists the followable players from TBD_SpectatorTargets once a second, marks the followed
 * one, and routes clicks: a player row follows in third person, a second click on the followed row
 * toggles first person, the FREE CAMERA primary action returns to free flight.
 * Position: a TBD_ShellScreen subclass on the TBD_Spectator preset over TBD_ScreenShell.layout,
 * opened and closed by TBD_SpectatorController; reads TBD_SpectatorTargets and drives
 * TBD_SpectatorTargeting.
 * State: the target rows and the not-in-view count of the last refresh, per screen instance, client.
 * Invariants: nothing blocks, so the camera keeps flying while the list is open (transparent
 * backdrop, glass panel); FREE CAMERA is the one primary action; the status line names the mode
 * and what a second click does; players outside replication range are counted, never offered.
 */

//! The spectator roster screen over the live camera.
class TBD_SpectatorScreen : TBD_ShellScreen
{
	static const int REFRESH_MS = 1000; //!< refresh period in ms; the list is pooled, so a refresh is property writes

	protected ref array<ref TBD_SpectatorTarget> m_aTargets; //!< rows of the last refresh
	protected int m_iNotInView; //!< connected players not on this machine at the last refresh

	//! Repaint the backdrop transparent and the panel glass, wire the row and primary actions,
	//! refresh, and arm the repeating refresh.
	override protected void OnScreenOpen()
	{
		super.OnScreenOpen();

		m_aTargets = {};

		// The shell paints a full-bleed scrim by default, which is right for the lobby and wrong
		// here: this list sits over a live camera the player is still flying. Glass, not a wall.
		TBD_UITheme.Paint(Find("Backdrop"), TBD_UITheme.TRANSPARENT);
		TBD_UITheme.PaintAlpha(Find("Panel"), TBD_UITheme.SURFACE_GLASS);

		TBD_ListBox list = GetList();
		if (list)
			list.GetOnActivate().Insert(OnTargetPicked);

		GetOnPrimaryAction().Insert(OnFreeCameraPicked);
		SetPrimaryAction("FREE CAMERA", true);

		Refresh();
		GetGame().GetCallqueue().CallLater(Refresh, REFRESH_MS, true);
	}

	//! Cancel the refresh and unwire the row and primary actions.
	override protected void OnScreenClose()
	{
		GetGame().GetCallqueue().Remove(Refresh);

		TBD_ListBox list = GetList();
		if (list)
			list.GetOnActivate().Remove(OnTargetPicked);

		GetOnPrimaryAction().Remove(OnFreeCameraPicked);

		super.OnScreenClose();
	}

	//! @return the screen title
	override protected string GetScreenTitle()
	{
		return "SPECTATOR";
	}

	//! @return the subtitle naming the faction restriction
	override protected string GetScreenSubtitle()
	{
		if (TBD_SpectatorTargets.IsFactionRestricted())
			return "Your life is spent. You may watch your own side.";

		return "Your life is spent. All sides visible.";
	}


	//! Rebuild the list. Progressive disclosure: faction is a section, group is a section, and only
	//! the people are rows -- so a full 128-slot mission reads as a handful of headings and the
	//! dozen names that are actually near you, not a wall.
	void Refresh()
	{
		TBD_ListBox list = GetList();
		if (!list)
			return;

		TBD_SpectatorTargets.Collect(m_aTargets, m_iNotInView);

		int followed = TBD_SpectatorTargeting.GetFollowedPlayerId();

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
				if (TBD_SpectatorTargeting.IsFirstPerson())
					detail = "FIRST PERSON";
				else
					detail = "FOLLOWING";
			}

			TBD_EUIState state = TBD_EUIState.NORMAL;
			if (isFollowed)
				state = TBD_EUIState.ACTIVE;

			list.AddItem(target.m_sName, detail, target.m_iPlayerId, state, true);
		}

		// A player outside this client's replication range was never sent to this machine and
		// cannot be rendered, so they are counted, not offered: "the list is empty" and "everyone
		// is far away" are different facts.
		if (m_iNotInView > 0)
			list.AddSection(string.Format("%1 more not in view -- fly closer", m_iNotInView));

		list.EndUpdate();

		list.SetSelectedTag(followed);

		SetStatus(BuildStatus());
	}

	//! What the camera is doing plus, when there is nothing to watch, why.
	protected string BuildStatus()
	{
		if (m_aTargets.IsEmpty())
		{
			if (TBD_SpectatorTargets.IsFactionRestricted() && TBD_SpectatorTargets.GetViewerFactionKey().IsEmpty())
				return "Your faction could not be resolved -- no targets shown.";

			if (m_iNotInView > 0)
				return "Nobody alive nearby. Fly toward the AO to pick players up.";

			return "Nobody left alive to watch.";
		}

		return TBD_SpectatorTargeting.GetStatusLine();
	}


	//! One click follows. A second click on the same player toggles first person -- the status line
	//! advertises it, so there is nothing to memorise.
	protected void OnTargetPicked(TBD_ListBox list, int tag)
	{
		if (tag <= 0)
			return;

		if (TBD_SpectatorTargeting.GetFollowedPlayerId() == tag)
		{
			TBD_SpectatorTargeting.ToggleFirstPerson();
		}
		else if (!TBD_SpectatorTargeting.FollowPlayer(tag, false))
		{
			// The player died between the last refresh and this click. Say so instead of leaving
			// the camera pointed at a corpse and the row looking selected.
			SetStatus("That player is no longer alive -- back to free camera.");
			Refresh();
			return;
		}

		Refresh();
	}

	//! FREE CAMERA: back to free flight, then refresh.
	//! @param screen this screen
	protected void OnFreeCameraPicked(TBD_ShellScreen screen)
	{
		TBD_SpectatorTargeting.SetFree();
		Refresh();
	}
}
