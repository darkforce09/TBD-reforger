//! Pre-game rebuild (2026-09-12) — the popover: a trigger pill plus a menu that floats over the
//! screen. Three mockup surfaces are this widget: the browser's *Modes* multi-select, the
//! inspector's *version* single-select, the markers panel's *plan* picker.
//!
//! ── Shape ────────────────────────────────────────────────────────────────────────────────
//!   * The trigger is `TBD_Dropdown.layout` (a ButtonWidget; this handler sits on it). Widget
//!     contract: `TriggerBorder`, `TriggerBG`, `TriggerLabel`, `TriggerBadgeDock`, `Chevron`.
//!   * The menu is `TBD_DropdownMenu.layout`, created on demand into the owning screen's
//!     **OverlayDock** (`SetOverlayHost`) and positioned under the trigger with FrameSlot. Widget
//!     contract: `Scrim` (full-bleed ButtonWidget — click outside closes), `Menu`, `MenuBorder`,
//!     `MenuBG`, `MenuTitle`, `SelectAll`, `DeselectAll`, `MenuRule`, `MenuList` (a TBD_ListBox).
//!   * Items are pooled TBD_ListRows: title = label, detail = badge or count. Reuse, not a fourth
//!     row widget.
//!
//! ── Behaviour ────────────────────────────────────────────────────────────────────────────
//!   * Single-select: click a row -> selection, menu closes, `OnChanged(dropdown, tag)`.
//!   * Multi-select: click a row -> toggles its check, menu stays, `OnChanged(dropdown, tag)`;
//!     the trigger badge shows the checked count; Select All / Deselect All appear in the header.
//!   * Direct manipulation throughout — no OK button, no confirm.

//! One entry of a dropdown. `m_iTag` is the caller's id; `m_sBadge` is the trailing mono text
//! (`LATEST`, `STABLE`, a count).
class TBD_DropdownItem
{
	string m_sLabel;
	string m_sBadge;
	int m_iTag;
	bool m_bChecked;

	void TBD_DropdownItem(string label, int tag, string badge = "", bool checked = false)
	{
		m_sLabel = label;
		m_iTag = tag;
		m_sBadge = badge;
		m_bChecked = checked;
	}
}

//! Forwards the menu's own widget events (the scrim click) to the dropdown. A handler may only
//! be attached to one widget, so the menu root gets this stub instead of the dropdown itself.
class TBD_DropdownMenuBridge : ScriptedWidgetComponent
{
	protected TBD_DropdownComponent m_Owner;

	void TBD_DropdownMenuBridge(TBD_DropdownComponent owner)
	{
		m_Owner = owner;
	}

	override bool OnClick(Widget w, int x, int y, int button)
	{
		if (m_Owner && w && w.GetName() == "Scrim")
		{
			m_Owner.Close();
			return true;
		}

		return super.OnClick(w, x, y, button);
	}
}

class TBD_DropdownComponent : TBD_UIInteractive
{
	[Attribute("", UIWidgets.EditBox, "Trigger label (single-select shows the chosen item instead)")]
	protected string m_sLabel;

	[Attribute("", UIWidgets.EditBox, "Menu header title")]
	protected string m_sMenuTitle;

	[Attribute("0", UIWidgets.CheckBox, "Multi-select checklist (Modes) instead of a single choice (version)")]
	protected bool m_bMultiSelect;

	[Attribute("224", UIWidgets.EditBox, "Menu width in reference pixels")]
	protected int m_iMenuWidth;

	[Attribute("1", UIWidgets.CheckBox, "Align the menu's right edge to the trigger's right edge")]
	protected bool m_bAlignRight;

	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected TextWidget m_wLabel;
	protected Widget m_wBadgeDock;
	protected TextWidget m_wChevron;
	protected TBD_ChipComponent m_BadgeChip;

	protected Widget m_wOverlayHost;
	protected Widget m_wMenuRoot;
	protected Widget m_wMenu;
	protected TBD_ListBox m_MenuList;
	protected ref TBD_UIScrollBar m_MenuScrollBar;
	protected TBD_UIButton m_SelectAll;
	protected TBD_UIButton m_DeselectAll;
	protected ref TBD_DropdownMenuBridge m_Bridge;

	protected ref array<ref TBD_DropdownItem> m_aItems;
	//! Opaque colour under the trigger; 0 = glass panel.
	protected int m_iGround;
	protected int m_iSelectedTag = -1;

	//! (TBD_DropdownComponent dropdown, int tag) — the row the user clicked. Read state with
	//! GetSelectedTag() / IsChecked().
	protected ref ScriptInvoker m_OnChanged;

	static const int ROW_HEIGHT = 46;   //!< TBD_ListRow: 44 body + 2 padding
	static const int HEADER_HEIGHT = 36;
	static const int MENU_PADDING = 8;
	static const int MENU_GAP = 6;      //!< space between trigger and menu

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		Close();
		super.HandlerDeattached(w);
	}

	// ── Public surface ──────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! The full-bleed widget the menu is created under — the owning screen's OverlayDock. Without
	//! it the menu falls back to the trigger's own parent, which usually clips it.
	void SetOverlayHost(Widget host)
	{
		m_wOverlayHost = host;
	}

	//------------------------------------------------------------------------------------------------
	//! Opaque colour under the trigger (the owning panel's GetGround()).
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		Repaint();
		if (m_BadgeChip)
			m_BadgeChip.SetGround(TBD_UITheme.Over(TBD_UITheme.INPUT_FILL, opaqueArgb));
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	void SetLabel(string label)
	{
		m_sLabel = label;
		RefreshTrigger();
	}

	//------------------------------------------------------------------------------------------------
	void SetMenuTitle(string title)
	{
		m_sMenuTitle = title;
	}

	//------------------------------------------------------------------------------------------------
	void SetMultiSelect(bool multi)
	{
		m_bMultiSelect = multi;
		RefreshTrigger();
	}

	//------------------------------------------------------------------------------------------------
	//! Single-select: pick by tag without firing OnChanged (restoring state).
	void SetSelectedTag(int tag)
	{
		m_iSelectedTag = tag;
		RefreshTrigger();
		if (IsOpen())
			RebuildRows();
	}

	//------------------------------------------------------------------------------------------------
	int GetSelectedTag()
	{
		return m_iSelectedTag;
	}

	//------------------------------------------------------------------------------------------------
	TBD_DropdownItem GetSelectedItem()
	{
		return FindItem(m_iSelectedTag);
	}

	//------------------------------------------------------------------------------------------------
	bool IsChecked(int tag)
	{
		TBD_DropdownItem item = FindItem(tag);
		if (!item)
			return false;

		return item.m_bChecked;
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! (TBD_DropdownComponent dropdown, int tag)
	ScriptInvoker GetOnChanged()
	{
		if (!m_OnChanged)
			m_OnChanged = new ScriptInvoker();

		return m_OnChanged;
	}

	//------------------------------------------------------------------------------------------------
	bool IsOpen()
	{
		return m_wMenuRoot != null;
	}

	//------------------------------------------------------------------------------------------------
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

		m_wMenuRoot = TBD_UILayouts.Create(TBD_UILayouts.DROPDOWN_MENU, host);
		if (!m_wMenuRoot)
			return;

		m_wMenu = m_wMenuRoot.FindAnyWidget("Menu");
		m_MenuList = TBD_ListBox.Cast(FindHandlerIn(m_wMenuRoot, "MenuList", TBD_ListBox));
		m_MenuScrollBar = TBD_UIScrollBar.Mount(m_wMenuRoot.FindAnyWidget("ScrollBarDock"), ScrollLayoutWidget.Cast(m_wMenuRoot.FindAnyWidget("Scroll")), m_wMenuRoot.FindAnyWidget("Content"), TBD_UITheme.PanelGround());
		m_SelectAll = TBD_UIButton.Cast(FindHandlerIn(m_wMenuRoot, "SelectAll", TBD_UIButton));
		m_DeselectAll = TBD_UIButton.Cast(FindHandlerIn(m_wMenuRoot, "DeselectAll", TBD_UIButton));

		m_Bridge = new TBD_DropdownMenuBridge(this);
		m_wMenuRoot.AddHandler(m_Bridge);

		if (m_MenuList)
			m_MenuList.GetOnActivate().Insert(OnRowActivated);

		if (m_SelectAll)
		{
			m_SelectAll.GetOnActivate().Insert(OnSelectAll);
			TBD_UITheme.Show(m_SelectAll.GetRootWidget(), m_bMultiSelect);
		}

		if (m_DeselectAll)
		{
			m_DeselectAll.GetOnActivate().Insert(OnDeselectAll);
			TBD_UITheme.Show(m_DeselectAll.GetRootWidget(), m_bMultiSelect);
		}

		TextWidget title = TextWidget.Cast(m_wMenuRoot.FindAnyWidget("MenuTitle"));
		string shout = m_sMenuTitle;
		shout.ToUpper();
		TBD_UITheme.Write(title, shout);
		TBD_UITheme.Paint(title, TBD_UITheme.MUTED_INK);

		// The menu floats over whatever is under the trigger; a glass panel is the honest guess.
		Widget menuBorder = m_wMenuRoot.FindAnyWidget("MenuBorder");
		Widget menuBG = m_wMenuRoot.FindAnyWidget("MenuBG");
		TBD_UILayouts.MountRounded(menuBorder, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(menuBG, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UITheme.PaintOver(menuBorder, TBD_UITheme.GLASS_BORDER_STRONG, TBD_UITheme.PanelGround());
		TBD_UITheme.PaintOver(menuBG, TBD_UITheme.PANEL_FILL, TBD_UITheme.PanelGround());
		TBD_UITheme.Paint(m_wMenuRoot.FindAnyWidget("MenuRule"), TBD_UITheme.GLASS_BORDER);

		RebuildRows();
		PlaceMenu(host);
		UpdateChevron();

		if (m_MenuList)
			m_MenuList.FocusFirst();
	}

	//------------------------------------------------------------------------------------------------
	void Close()
	{
		if (!m_wMenuRoot)
			return;

		if (m_MenuList)
			m_MenuList.GetOnActivate().Remove(OnRowActivated);

		if (m_SelectAll)
			m_SelectAll.GetOnActivate().Remove(OnSelectAll);

		if (m_DeselectAll)
			m_DeselectAll.GetOnActivate().Remove(OnDeselectAll);

		Widget host = m_wMenuRoot.GetParent();
		m_wMenuRoot.RemoveFromHierarchy();
		m_wMenuRoot = null;
		if (m_MenuScrollBar)
			m_MenuScrollBar.Destroy();

		m_MenuScrollBar = null;
		m_wMenu = null;
		m_MenuList = null;
		m_SelectAll = null;
		m_DeselectAll = null;
		m_Bridge = null;

		// The overlay dock hides itself again once nothing floats in it, so it never eats a click.
		if (host && host == m_wOverlayHost && !host.GetChildren())
			TBD_UITheme.Show(host, false);

		UpdateChevron();
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	void Toggle()
	{
		if (IsOpen())
			Close();
		else
			Open();
	}

	// ── TBD_UIInteractive ───────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	override protected void OnActivated()
	{
		Toggle();
	}

	//------------------------------------------------------------------------------------------------
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

	// ── Internals ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected void OnSelectAll(TBD_UIButton button)
	{
		SetAllChecked(true);
		if (m_OnChanged)
			m_OnChanged.Invoke(this, -1);
	}

	//------------------------------------------------------------------------------------------------
	protected void OnDeselectAll(TBD_UIButton button)
	{
		SetAllChecked(false);
		if (m_OnChanged)
			m_OnChanged.Invoke(this, -1);
	}

	//------------------------------------------------------------------------------------------------
	//! Rows are pooled by the list; this is O(items) property writes.
	protected void RebuildRows()
	{
		if (!m_MenuList)
			return;

		m_MenuList.BeginUpdate();
		foreach (TBD_DropdownItem item : m_aItems)
		{
			TBD_EUIState state = TBD_EUIState.NORMAL;
			string detail = item.m_sBadge;

			if (m_bMultiSelect)
			{
				if (item.m_bChecked)
					state = TBD_EUIState.ACTIVE;
			}
			else if (item.m_iTag == m_iSelectedTag)
			{
				state = TBD_EUIState.ACTIVE;
			}

			m_MenuList.AddItem(item.m_sLabel, detail, item.m_iTag, state, true);
		}
		m_MenuList.EndUpdate();

		// The list's own "last clicked" echo would fight the checkmarks in multi mode.
		if (m_bMultiSelect)
			m_MenuList.SetSelectedTag(-1);
		else
			m_MenuList.SetSelectedTag(m_iSelectedTag);
	}

	//------------------------------------------------------------------------------------------------
	//! Anchor the menu under the trigger inside `host`. Screen coordinates come back in real
	//! pixels; FrameSlot wants reference pixels, hence DPIUnscale.
	protected void PlaceMenu(Widget host)
	{
		if (!m_wMenu || !m_wRoot)
			return;

		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace)
			return;

		float tx, ty, tw, th, hx, hy;
		m_wRoot.GetScreenPos(tx, ty);
		m_wRoot.GetScreenSize(tw, th);
		host.GetScreenPos(hx, hy);

		float x = workspace.DPIUnscale(tx - hx);
		float y = workspace.DPIUnscale(ty - hy + th) + MENU_GAP;

		if (m_bAlignRight)
			x = workspace.DPIUnscale(tx + tw - hx) - m_iMenuWidth;

		int rows = m_aItems.Count();
		float height = HEADER_HEIGHT + 2 + rows * ROW_HEIGHT + MENU_PADDING * 2;

		FrameSlot.SetPos(m_wMenu, x, y);
		FrameSlot.SetSize(m_wMenu, m_iMenuWidth, height);
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! LATEST is green, everything else the quiet blue — the mockup's two badge colours.
	protected TBD_EUITint BadgeTint(string badge)
	{
		if (badge == "LATEST")
			return TBD_EUITint.SUCCESS;

		return TBD_EUITint.PRIMARY;
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected void UpdateChevron()
	{
		if (!m_wChevron)
			return;

		if (IsOpen())
			m_wChevron.SetText("^");
		else
			m_wChevron.SetText("v");
	}

	//------------------------------------------------------------------------------------------------
	protected TBD_DropdownItem FindItem(int tag)
	{
		foreach (TBD_DropdownItem item : m_aItems)
		{
			if (item.m_iTag == tag)
				return item;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	protected static ScriptedWidgetComponent FindHandlerIn(Widget root, string name, typename handler)
	{
		if (!root)
			return null;

		Widget w = root.FindAnyWidget(name);
		if (!w)
			return null;

		return ScriptedWidgetComponent.Cast(w.FindHandler(handler));
	}

	//------------------------------------------------------------------------------------------------
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
