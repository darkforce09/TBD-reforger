//! Pre-game rebuild (2026-09-12) — the segmented control and the tab inside it.
//!
//! Four mockup surfaces are this widget: the top bar's Scenario Browser / Lobby / Briefing switch
//! (horizontal), the briefing's primary navigation and topic tree (vertical), and the lobby
//! header's ORBAT / Briefing toggle that the top bar supersedes.
//!
//! Two classes, one file, because a nav item never exists outside a strip:
//!   * `TBD_NavItemComponent`  — one tab. Attached in `TBD_NavItem.layout`. Widget contract:
//!     `Background`, `Border`, `Icon`, `Label`, `Badge`.
//!   * `TBD_TabStripComponent` — the strip. Attached in `TBD_TabStrip.layout`. Widget contract:
//!     `StripBorder`, `StripBG`, `ItemsRow` (horizontal), `ItemsColumn` (vertical). The
//!     `m_bVertical` attribute picks which container is used; the other stays hidden.
//!
//! One click = the switch (direct manipulation). The strip echoes the active tab visually and
//! fires `GetOnSelected()(strip, index)`; the owning screen decides what a tab means.

//! Description of one tab. Plain data so a screen can build a strip from a table.
class TBD_NavItemData
{
	string m_sLabel;
	string m_sIcon;   //!< TBD_UIIcons key; empty = no glyph
	string m_sBadge;  //!< trailing count text; empty = hidden
	bool m_bEnabled = true;

	void TBD_NavItemData(string label, string icon = "", string badge = "", bool enabled = true)
	{
		m_sLabel = label;
		m_sIcon = icon;
		m_sBadge = badge;
		m_bEnabled = enabled;
	}
}

class TBD_NavItemComponent : TBD_UIInteractive
{
	protected Widget m_wBackground;
	protected Widget m_wBorder;
	protected ImageWidget m_wIcon;
	protected TextWidget m_wLabel;
	protected TextWidget m_wBadge;

	protected TBD_TabStripComponent m_Owner; //!< weak — the strip owns its items
	protected int m_iIndex = -1;
	protected bool m_bActive;

	//------------------------------------------------------------------------------------------------
	override protected void OnBind(Widget w)
	{
		m_wBackground = w.FindAnyWidget("Background");
		m_wBorder = w.FindAnyWidget("Border");
		m_wIcon = ImageWidget.Cast(w.FindAnyWidget("Icon"));
		m_wLabel = TextWidget.Cast(w.FindAnyWidget("Label"));
		m_wBadge = TextWidget.Cast(w.FindAnyWidget("Badge"));

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
	}

	//------------------------------------------------------------------------------------------------
	void Bind(TBD_TabStripComponent owner, int index, TBD_NavItemData data)
	{
		m_Owner = owner;
		m_iIndex = index;

		TBD_UITheme.Write(m_wLabel, data.m_sLabel);
		TBD_UITheme.Write(m_wBadge, data.m_sBadge);
		TBD_UITheme.Show(m_wBadge, !data.m_sBadge.IsEmpty());

		if (m_wIcon)
		{
			if (data.m_sIcon.IsEmpty())
				m_wIcon.SetVisible(false);
			else
				TBD_UIIcons.Load(m_wIcon, data.m_sIcon);
		}

		SetInteractive(data.m_bEnabled);
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	void SetActive(bool active)
	{
		if (m_bActive == active)
			return;

		m_bActive = active;
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		int fill;
		int border;
		int ink;

		if (m_bActive)
		{
			fill   = TBD_UITheme.NAV_ACTIVE_FILL;
			border = TBD_UITheme.NAV_ACTIVE_BORDER;
			ink    = TBD_UITheme.BRIGHT_INK;
		}
		else if (IsHighlighted() && m_bInteractive)
		{
			fill   = TBD_UITheme.NAV_HOVER_FILL;
			border = TBD_UITheme.TRANSPARENT;
			ink    = TBD_UITheme.ON_SURFACE;
		}
		else
		{
			fill   = TBD_UITheme.TRANSPARENT;
			border = TBD_UITheme.TRANSPARENT;
			ink    = TBD_UITheme.MUTED_INK;
		}

		if (!m_bInteractive)
			ink = TBD_UITheme.ROW_DISABLED_TEXT;

		int ground = TBD_UITheme.Ground();
		if (m_Owner)
			ground = m_Owner.GetItemGround();

		TBD_UITheme.PaintOver(m_wBackground, fill, ground);
		TBD_UITheme.PaintOver(m_wBorder, border, ground);
		TBD_UITheme.Paint(m_wLabel, ink);
		TBD_UITheme.Paint(m_wIcon, ink);
		TBD_UITheme.Paint(m_wBadge, TBD_UITheme.PRIMARY);
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnItemActivated(m_iIndex);
	}

	//------------------------------------------------------------------------------------------------
	int GetIndex()
	{
		return m_iIndex;
	}
}

class TBD_TabStripComponent : ScriptedWidgetComponent
{
	[Attribute("0", UIWidgets.CheckBox, "Stack items vertically (briefing navigation) instead of in a row")]
	protected bool m_bVertical;

	[Attribute("1", UIWidgets.CheckBox, "Draw the strip's own pill background (off for a bare vertical list)")]
	protected bool m_bChrome;

	protected Widget m_wRoot;
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected Widget m_wItemsRow;
	protected Widget m_wItemsColumn;

	//! Pool, index-stable. Weak elements: each item handler is owned by its widget.
	protected ref array<TBD_NavItemComponent> m_aItems;
	protected int m_iLive;
	protected int m_iActive = -1;
	//! Opaque colour under the strip; 0 = the backdrop.
	protected int m_iGround;

	//! (TBD_TabStripComponent strip, int index)
	protected ref ScriptInvoker m_OnSelected;

	//------------------------------------------------------------------------------------------------
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_aItems = {};

		m_wBorder = w.FindAnyWidget("StripBorder");
		m_wBackground = w.FindAnyWidget("StripBG");
		m_wItemsRow = w.FindAnyWidget("ItemsRow");
		m_wItemsColumn = w.FindAnyWidget("ItemsColumn");

		TBD_UITheme.Show(m_wItemsRow, !m_bVertical);
		TBD_UITheme.Show(m_wItemsColumn, m_bVertical);

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
		RepaintChrome();
	}

	//------------------------------------------------------------------------------------------------
	//! Opaque colour under the strip (the top bar sits on the backdrop; a briefing nav column
	//! sits on a panel). Items recompute their ground from it.
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		RepaintChrome();
		foreach (TBD_NavItemComponent item : m_aItems)
		{
			if (item)
				item.Repaint();
		}
	}

	//------------------------------------------------------------------------------------------------
	//! What the items sit on: the strip's own fill when it draws chrome, else the strip's ground.
	int GetItemGround()
	{
		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.Ground();

		if (m_bChrome)
			return TBD_UITheme.Over(TBD_UITheme.STRIP_FILL, ground);

		return ground;
	}

	//------------------------------------------------------------------------------------------------
	protected void RepaintChrome()
	{
		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.Ground();

		if (m_bChrome)
		{
			TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.STRIP_BORDER, ground);
			TBD_UITheme.PaintOver(m_wBackground, TBD_UITheme.STRIP_FILL, ground);
		}
		else
		{
			TBD_UITheme.Paint(m_wBorder, TBD_UITheme.TRANSPARENT);
			TBD_UITheme.Paint(m_wBackground, TBD_UITheme.TRANSPARENT);
		}
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		if (m_aItems)
			m_aItems.Clear();

		m_wRoot = null;
		m_iLive = 0;
		super.HandlerDeattached(w);
	}

	//------------------------------------------------------------------------------------------------
	//! Rebuild the strip from a table. Items are pooled: widgets are created only for indices the
	//! strip has never reached, surplus ones are hidden.
	void SetItems(notnull array<ref TBD_NavItemData> items)
	{
		Widget container = GetContainer();
		if (!container)
			return;

		int count = items.Count();
		for (int i = 0; i < count; i++)
		{
			TBD_NavItemComponent item = AcquireItem(i, container);
			if (!item)
				continue;

			item.Bind(this, i, items[i]);
			TBD_UITheme.Show(item.GetRootWidget(), true);
			item.SetActive(i == m_iActive);
		}

		m_iLive = count;
		for (int j = count; j < m_aItems.Count(); j++)
		{
			TBD_UITheme.Show(m_aItems[j].GetRootWidget(), false);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Visual echo only — does not fire OnSelected. Use it to reflect the screen that is open.
	void SetActive(int index)
	{
		m_iActive = index;
		for (int i = 0; i < m_aItems.Count(); i++)
		{
			m_aItems[i].SetActive(i == index);
		}
	}

	//------------------------------------------------------------------------------------------------
	int GetActive()
	{
		return m_iActive;
	}

	//------------------------------------------------------------------------------------------------
	int GetItemCount()
	{
		return m_iLive;
	}

	//------------------------------------------------------------------------------------------------
	//! (TBD_TabStripComponent strip, int index)
	ScriptInvoker GetOnSelected()
	{
		if (!m_OnSelected)
			m_OnSelected = new ScriptInvoker();

		return m_OnSelected;
	}

	//------------------------------------------------------------------------------------------------
	//! Focus the active tab (or the first) so a gamepad user lands on the strip.
	bool FocusActive()
	{
		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace || m_iLive == 0)
			return false;

		int index = m_iActive;
		if (index < 0 || index >= m_iLive)
			index = 0;

		Widget target = m_aItems[index].GetRootWidget();
		if (!target)
			return false;

		workspace.SetFocusedWidget(target);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	// ── Called by TBD_NavItemComponent ──────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	void OnItemActivated(int index)
	{
		if (index < 0 || index >= m_iLive)
			return;

		SetActive(index);

		if (m_OnSelected)
			m_OnSelected.Invoke(this, index);
	}

	// ── Internals ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected Widget GetContainer()
	{
		if (m_bVertical && m_wItemsColumn)
			return m_wItemsColumn;

		if (m_wItemsRow)
			return m_wItemsRow;

		return m_wRoot;
	}

	//------------------------------------------------------------------------------------------------
	protected TBD_NavItemComponent AcquireItem(int index, Widget container)
	{
		if (index < m_aItems.Count())
			return m_aItems[index];

		TBD_NavItemComponent item = TBD_NavItemComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.NAV_ITEM, container, TBD_NavItemComponent));
		if (!item)
			return null;

		// Vertical strips want every tab the full column width; a row keeps tabs text-sized.
		if (m_bVertical)
			AlignableSlot.SetHorizontalAlign(item.GetRootWidget(), LayoutHorizontalAlign.Stretch);

		m_aItems.Insert(item);
		return item;
	}
}
