/**
 * @file TBD_NavItemComponent.c
 * @brief One tab of a TBD tab strip, and the plain data a strip is built from.
 *
 * Role: handler of `TBD_NavItem.layout`; binds a `TBD_NavItemData`, paints active, hovered, idle
 * and disabled, and reports a click to its strip.
 * Position: created and pooled by `TBD_TabStripComponent`; `TBD_NavItemData` tables come from the
 * session top bar and the briefing topic navigation.
 * State: the widget references, the owning strip, the index and the active flag, on the client.
 * Invariants: widget contract `Background`, `Border`, `Icon`, `Label`, `Badge`, `SeparatorSize`,
 * `Separator`; a nav item never exists outside a strip; one click is the switch.
 */

//! Description of one tab. Plain data so a screen can build a strip from a table.
class TBD_NavItemData
{
	string m_sLabel; //!< the tab text
	string m_sIcon;   //!< TBD_UIIcons key; empty = no glyph
	string m_sBadge;  //!< trailing count text; empty = hidden
	bool m_bEnabled = true; //!< false paints the tab disabled and ignores clicks; default true
	bool m_bSeparatorBefore; //!< draw a rule above this item (briefing nav groups)

	//! Describe one tab.
	//! @param label the tab text
	//! @param icon a `TBD_UIIcons` key; empty for no glyph
	//! @param badge trailing count text; empty hides it
	//! @param enabled false disables the tab
	void TBD_NavItemData(string label, string icon = "", string badge = "", bool enabled = true)
	{
		m_sLabel = label;
		m_sIcon = icon;
		m_sBadge = badge;
		m_bEnabled = enabled;
	}
}

//! One pooled tab of a `TBD_TabStripComponent`.
class TBD_NavItemComponent : TBD_UIInteractive
{
	protected Widget m_wBackground; //!< `Background` frame dock, rounded at bind
	protected Widget m_wBorder; //!< `Border` frame dock, rounded at bind
	protected ImageWidget m_wIcon; //!< `Icon` glyph
	protected TextWidget m_wLabel; //!< `Label` text
	protected TextWidget m_wBadge; //!< `Badge` count text
	protected Widget m_wSeparatorSize; //!< `SeparatorSize`, shown when the item opens a group
	protected Widget m_wSeparator; //!< `Separator` rule

	protected TBD_TabStripComponent m_Owner; //!< weak -- the strip owns its items
	protected int m_iIndex = -1; //!< the tab's index in its strip; -1 before Bind
	protected bool m_bActive; //!< true while this is the strip's active tab

	//! Find the tab widgets, hide the separator and round the frame docks.
	override protected void OnBind(Widget w)
	{
		m_wBackground = w.FindAnyWidget("Background");
		m_wBorder = w.FindAnyWidget("Border");
		m_wIcon = ImageWidget.Cast(w.FindAnyWidget("Icon"));
		m_wLabel = TextWidget.Cast(w.FindAnyWidget("Label"));
		m_wBadge = TextWidget.Cast(w.FindAnyWidget("Badge"));
		m_wSeparatorSize = w.FindAnyWidget("SeparatorSize");
		m_wSeparator = w.FindAnyWidget("Separator");
		TBD_UITheme.Show(m_wSeparatorSize, false);

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
	}

	//! Bind the tab to its strip and index and show `data`: label, badge, separator, icon, enabled.
	void Bind(TBD_TabStripComponent owner, int index, TBD_NavItemData data)
	{
		m_Owner = owner;
		m_iIndex = index;

		TBD_UITheme.Write(m_wLabel, data.m_sLabel);
		TBD_UITheme.Write(m_wBadge, data.m_sBadge);
		TBD_UITheme.Show(m_wBadge, !data.m_sBadge.IsEmpty());
		TBD_UITheme.Show(m_wSeparatorSize, data.m_bSeparatorBefore);

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

	//! Set the active echo and repaint; does nothing when unchanged.
	void SetActive(bool active)
	{
		if (m_bActive == active)
			return;

		m_bActive = active;
		Repaint();
	}

	//! Paint active, hovered, idle or disabled over the strip's item ground.
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
		TBD_UITheme.PaintOver(m_wSeparator, TBD_UITheme.STRIP_BORDER, ground);
	}

	//! A click or gamepad activation reports this tab's index to the strip.
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnItemActivated(m_iIndex);
	}

	//! @return the tab's index in its strip; -1 before Bind
	int GetIndex()
	{
		return m_iIndex;
	}
}
