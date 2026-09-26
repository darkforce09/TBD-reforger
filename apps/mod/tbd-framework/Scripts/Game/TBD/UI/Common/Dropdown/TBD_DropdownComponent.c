/**
 * @file TBD_DropdownComponent.c
 * @brief The popover: a trigger pill plus a single- or multi-select menu floating over the screen.
 *
 * Role: handler of `TBD_Dropdown.layout` (a ButtonWidget); keeps the items, the selection or the
 * checks, paints the trigger and its badge, and opens a `TBD_DropdownMenu` on activation.
 * Position: mounted by the briefing markers panel (plan picker), the mission inspector (version)
 * and the scenario browser (modes); reports each row click through `GetOnChanged()`.
 * State: the items, the selected tag, the ground colour and the open menu, on the client.
 * Invariants: widget contract `TriggerBorder`, `TriggerBG`, `TriggerLabel`, `TriggerBadgeDock`,
 * `Chevron`; single-select closes on a row click, multi-select toggles the check and stays open
 * with Select All and Deselect All in its header; the trigger badge shows the checked count in
 * multi-select; there is no OK or confirm step.
 */

//! Dropdown trigger handler: items, selection, trigger paint and the open menu.
class TBD_DropdownComponent : TBD_UIInteractive
{
	[Attribute("", UIWidgets.EditBox, desc: "Trigger label (single-select shows the chosen item instead)")]
	protected string m_sLabel; //!< trigger text; single-select shows the chosen item instead; default empty

	[Attribute("", UIWidgets.EditBox, desc: "Menu header title")]
	protected string m_sMenuTitle; //!< menu header text, written upper case; default empty

	[Attribute("0", UIWidgets.CheckBox, desc: "Multi-select checklist (Modes) instead of a single choice (version)")]
	protected bool m_bMultiSelect; //!< true = checklist, false = single choice; default false

	[Attribute("224", UIWidgets.EditBox, desc: "Menu width in reference pixels")]
	protected int m_iMenuWidth; //!< menu width, reference pixels; default 224

	[Attribute("1", UIWidgets.CheckBox, desc: "Align the menu's right edge to the trigger's right edge")]
	protected bool m_bAlignRight; //!< true aligns the menu's right edge to the trigger's; default true

	protected Widget m_wBorder; //!< `TriggerBorder` frame dock, rounded at bind
	protected Widget m_wBackground; //!< `TriggerBG` frame dock, rounded at bind
	protected TextWidget m_wLabel; //!< `TriggerLabel` text
	protected Widget m_wBadgeDock; //!< `TriggerBadgeDock`, hidden while there is no badge
	protected TextWidget m_wChevron; //!< `Chevron` text: v closed, ^ open
	protected TBD_ChipComponent m_BadgeChip; //!< the badge chip, mounted on first use

	protected Widget m_wOverlayHost; //!< the screen's OverlayDock the menu opens into; null uses the trigger's parent
	protected ref TBD_DropdownMenu m_Menu; //!< the open menu; null while closed

	protected ref array<ref TBD_DropdownItem> m_aItems; //!< the items in display order; empty until SetItems
	protected int m_iGround; //!< opaque ARGB under the trigger; 0 = the glass panel ground
	protected int m_iSelectedTag = -1; //!< the single-select choice; -1 = none

	//! (TBD_DropdownComponent dropdown, int tag) -- the row the user clicked. Read state with
	//! GetSelectedTag() / IsChecked().
	protected ref ScriptInvoker m_OnChanged; //!< created on first GetOnChanged

	//! Find the trigger widgets, round the frame docks, write the label and the chevron.
	override protected void OnBind(Widget w)
	{
		m_wBorder = w.FindAnyWidget("TriggerBorder");
		m_wBackground = w.FindAnyWidget("TriggerBG");
		m_wLabel = TextWidget.Cast(w.FindAnyWidget("TriggerLabel"));
		m_wBadgeDock = w.FindAnyWidget("TriggerBadgeDock");
		m_wChevron = TextWidget.Cast(w.FindAnyWidget("Chevron"));

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);

		if (!m_aItems)
			m_aItems = {};

		TBD_UITheme.Show(m_wBadgeDock, false);
		TBD_UITheme.Write(m_wLabel, m_sLabel);
		UpdateChevron();
	}

	//! Close the menu before the handler leaves its widget.
	override void HandlerDeattached(Widget w)
	{
		Close();
		super.HandlerDeattached(w);
	}


	//! The full-bleed widget the menu is created under -- the owning screen's OverlayDock. Without
	//! it the menu falls back to the trigger's own parent, which usually clips it.
	void SetOverlayHost(Widget host)
	{
		m_wOverlayHost = host;
	}

	//! Opaque colour under the trigger (the owning panel's GetGround()).
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		Repaint();
		if (m_BadgeChip)
			m_BadgeChip.SetGround(TBD_UITheme.Over(TBD_UITheme.INPUT_FILL, opaqueArgb));
	}

	//! Replace the items; single-select with no choice picks the first item. Rebuilds an open menu.
	void SetItems(notnull array<ref TBD_DropdownItem> items)
	{
		m_aItems = {};
		foreach (TBD_DropdownItem item : items)
		{
			if (item)
				m_aItems.Insert(item);
		}

		if (!m_bMultiSelect && m_iSelectedTag < 0 && m_aItems.Count() > 0)
			m_iSelectedTag = m_aItems[0].m_iTag;

		RefreshTrigger();
		if (IsOpen())
			RebuildRows();
	}

	//! Set the trigger text shown when no single-select choice exists, and in multi-select.
	void SetLabel(string label)
	{
		m_sLabel = label;
		RefreshTrigger();
	}

	//! Set the menu header text, applied the next time the menu opens.
	void SetMenuTitle(string title)
	{
		m_sMenuTitle = title;
	}

	//! Switch between the checklist and the single choice and refresh the trigger.
	void SetMultiSelect(bool multi)
	{
		m_bMultiSelect = multi;
		RefreshTrigger();
	}

	//! Single-select: pick by tag without firing OnChanged (restoring state).
	void SetSelectedTag(int tag)
	{
		m_iSelectedTag = tag;
		RefreshTrigger();
		if (IsOpen())
			RebuildRows();
	}

	//! @return the single-select choice's tag, or -1 when none is chosen
	int GetSelectedTag()
	{
		return m_iSelectedTag;
	}

	//! @return the single-select choice, or null when none is chosen
	TBD_DropdownItem GetSelectedItem()
	{
		return FindItem(m_iSelectedTag);
	}

	//! @return the check state of the item with `tag`; false when no item has it
	bool IsChecked(int tag)
	{
		TBD_DropdownItem item = FindItem(tag);
		if (!item)
			return false;

		return item.m_bChecked;
	}

	//! @return how many items are checked
	int GetCheckedCount()
	{
		int count;
		foreach (TBD_DropdownItem item : m_aItems)
		{
			if (item.m_bChecked)
				count++;
		}

		return count;
	}

	//! Multi-select: set every item at once without firing OnChanged.
	void SetAllChecked(bool checked)
	{
		foreach (TBD_DropdownItem item : m_aItems)
		{
			item.m_bChecked = checked;
		}

		RefreshTrigger();
		if (IsOpen())
			RebuildRows();
	}

	//! (TBD_DropdownComponent dropdown, int tag)
	ScriptInvoker GetOnChanged()
	{
		if (!m_OnChanged)
			m_OnChanged = new ScriptInvoker();

		return m_OnChanged;
	}

	//! @return true while the menu is open
	bool IsOpen()
	{
		return m_Menu != null;
	}

	//! Open the menu under the trigger in the overlay host (the trigger's parent without one).
	//! Does nothing when already open, unbound, hostless, or when the menu layout cannot be created.
	void Open()
	{
		if (IsOpen() || !m_wRoot)
			return;

		Widget host = m_wOverlayHost;
		if (!host)
			host = m_wRoot.GetParent();

		if (!host)
			return;

		TBD_UITheme.Show(host, true);

		TBD_DropdownMenu menu = new TBD_DropdownMenu(this);
		if (!menu.Build(host, m_sMenuTitle, m_bMultiSelect))
			return;

		m_Menu = menu;
		TBD_ListBox list = m_Menu.GetList();
		if (list)
			list.GetOnActivate().Insert(OnRowActivated);

		TBD_UIButton selectAll = m_Menu.GetSelectAll();
		if (selectAll)
			selectAll.GetOnActivate().Insert(OnSelectAll);

		TBD_UIButton deselectAll = m_Menu.GetDeselectAll();
		if (deselectAll)
			deselectAll.GetOnActivate().Insert(OnDeselectAll);

		RebuildRows();
		PlaceMenu(host);
		UpdateChevron();
		m_Menu.FocusFirst();
	}

	//! Close the menu: unsubscribe, destroy it, hide an emptied overlay dock, repaint the trigger.
	//! Does nothing when the menu is closed.
	void Close()
	{
		if (!m_Menu)
			return;

		TBD_ListBox list = m_Menu.GetList();
		if (list)
			list.GetOnActivate().Remove(OnRowActivated);

		TBD_UIButton selectAll = m_Menu.GetSelectAll();
		if (selectAll)
			selectAll.GetOnActivate().Remove(OnSelectAll);

		TBD_UIButton deselectAll = m_Menu.GetDeselectAll();
		if (deselectAll)
			deselectAll.GetOnActivate().Remove(OnDeselectAll);

		Widget host = m_Menu.Destroy();
		m_Menu = null;

		// The overlay dock hides itself again once nothing floats in it, so it never eats a click.
		if (host && host == m_wOverlayHost && !host.GetChildren())
			TBD_UITheme.Show(host, false);

		UpdateChevron();
		Repaint();
	}

	//! Open a closed menu, close an open one.
	void Toggle()
	{
		if (IsOpen())
			Close();
		else
			Open();
	}

	//! A click or gamepad activation on the trigger toggles the menu.
	override protected void OnActivated()
	{
		Toggle();
	}

	//! Paint the trigger lit while highlighted or open, quiet otherwise, over the ground colour.
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		bool lit = (IsHighlighted() && m_bInteractive) || IsOpen();
		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.PanelGround();

		if (lit)
		{
			TBD_UITheme.PaintOver(m_wBackground, TBD_UITheme.SURFACE_CONTAINER_HIGH, ground);
			TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.CARD_HOVER_BORDER, ground);
			TBD_UITheme.Paint(m_wLabel, TBD_UITheme.BRIGHT_INK);
		}
		else
		{
			TBD_UITheme.PaintOver(m_wBackground, TBD_UITheme.INPUT_FILL, ground);
			TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.GLASS_BORDER, ground);
			TBD_UITheme.Paint(m_wLabel, TBD_UITheme.ON_SURFACE);
		}

		TBD_UITheme.Paint(m_wChevron, TBD_UITheme.MUTED_INK);
	}

	//! A row click: multi-select toggles the check and stays open; single-select chooses and closes.
	//! Fires OnChanged with the row's tag either way; an unknown tag is ignored.
	protected void OnRowActivated(TBD_ListBox list, int tag)
	{
		TBD_DropdownItem item = FindItem(tag);
		if (!item)
			return;

		if (m_bMultiSelect)
		{
			item.m_bChecked = !item.m_bChecked;
			RebuildRows();
			RefreshTrigger();
		}
		else
		{
			m_iSelectedTag = tag;
			RefreshTrigger();
			Close();
		}

		if (m_OnChanged)
			m_OnChanged.Invoke(this, tag);
	}

	//! Check every item and fire OnChanged with tag -1.
	protected void OnSelectAll(TBD_UIButton button)
	{
		SetAllChecked(true);
		if (m_OnChanged)
			m_OnChanged.Invoke(this, -1);
	}

	//! Uncheck every item and fire OnChanged with tag -1.
	protected void OnDeselectAll(TBD_UIButton button)
	{
		SetAllChecked(false);
		if (m_OnChanged)
			m_OnChanged.Invoke(this, -1);
	}

	//! Re-bind the open menu's pooled rows to the items and the selection.
	protected void RebuildRows()
	{
		if (m_Menu)
			m_Menu.Fill(m_aItems, m_bMultiSelect, m_iSelectedTag);
	}

	//! Anchor the open menu under the trigger inside `host`.
	protected void PlaceMenu(Widget host)
	{
		if (m_Menu && host)
			m_Menu.Place(m_wRoot, host, m_iMenuWidth, m_bAlignRight, m_aItems.Count());
	}

	//! Write the trigger label and badge: the checked count in multi-select, the chosen item and its
	//! badge in single-select, the plain label when nothing is chosen.
	protected void RefreshTrigger()
	{
		if (m_bMultiSelect)
		{
			TBD_UITheme.Write(m_wLabel, m_sLabel);
			SetBadge(GetCheckedCount().ToString(), TBD_EUITint.PRIMARY);
			return;
		}

		TBD_DropdownItem chosen = FindItem(m_iSelectedTag);
		if (!chosen)
		{
			TBD_UITheme.Write(m_wLabel, m_sLabel);
			SetBadge(string.Empty, TBD_EUITint.NEUTRAL);
			return;
		}

		TBD_UITheme.Write(m_wLabel, chosen.m_sLabel);
		if (chosen.m_sBadge.IsEmpty())
			SetBadge(string.Empty, TBD_EUITint.NEUTRAL);
		else
			SetBadge(chosen.m_sBadge, BadgeTint(chosen.m_sBadge));
	}

	//! LATEST is green, everything else the quiet blue -- the mockup's two badge colours.
	protected TBD_EUITint BadgeTint(string badge)
	{
		if (badge == "LATEST")
			return TBD_EUITint.SUCCESS;

		return TBD_EUITint.PRIMARY;
	}

	//! Show `text` in the trigger badge chip with `tint`, mounting the chip on first use; empty text
	//! hides the badge dock.
	protected void SetBadge(string text, TBD_EUITint tint)
	{
		if (!m_wBadgeDock)
			return;

		if (text.IsEmpty())
		{
			TBD_UITheme.Show(m_wBadgeDock, false);
			return;
		}

		if (!m_BadgeChip)
		{
			int ground = m_iGround;
			if (ground == 0)
				ground = TBD_UITheme.PanelGround();

			m_BadgeChip = TBD_ChipComponent.Mount(m_wBadgeDock, text, tint, TBD_UITheme.Over(TBD_UITheme.INPUT_FILL, ground));
			if (m_BadgeChip)
				m_BadgeChip.SetPill(true);
		}
		else
		{
			m_BadgeChip.Set(text, tint);
		}

		TBD_UITheme.Show(m_wBadgeDock, m_BadgeChip != null);
	}

	//! Point the chevron down while closed and up while open.
	protected void UpdateChevron()
	{
		if (!m_wChevron)
			return;

		if (IsOpen())
			m_wChevron.SetText("^");
		else
			m_wChevron.SetText("v");
	}

	//! @return the item with `tag`, or null when none has it
	protected TBD_DropdownItem FindItem(int tag)
	{
		foreach (TBD_DropdownItem item : m_aItems)
		{
			if (item.m_iTag == tag)
				return item;
		}

		return null;
	}

	//! Mount a trigger into `dock` and return its handler.
	static TBD_DropdownComponent Mount(Widget dock, Widget overlayHost, string label, bool multi)
	{
		if (!dock)
			return null;

		TBD_DropdownComponent dropdown = TBD_DropdownComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.DROPDOWN, dock, TBD_DropdownComponent));
		if (!dropdown)
			return null;

		dropdown.SetOverlayHost(overlayHost);
		dropdown.SetMultiSelect(multi);
		dropdown.SetLabel(label);
		return dropdown;
	}
}
