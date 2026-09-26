/**
 * @file TBD_TopicNavItemComponent.c
 * @brief One item of the Briefing topic navigation.
 *
 * Role: binds and paints one item of TBD_TopicNavItem.layout (`SeparatorSize`/`Separator`,
 * `Border`, `Background`, `Icon`, `Label`) and reports activation.  Position: TBD_BriefingTopicNav
 * creates and owns the items; activation calls back into it.
 * State: owner, index, active and hover state of one item on the client.  Invariants: the
 * separator shows when the item data asks for one; active paints solid tactical blue, hover the
 * hover fill with a faint border; a disabled item draws disabled ink.
 */

//! One topic navigation item; the class name is referenced by the item layout.
class TBD_TopicNavItemComponent : TBD_UIInteractive
{
	protected Widget m_wSeparatorSize; //!< `SeparatorSize`, shown for a separated item
	protected Widget m_wSeparator; //!< `Separator`, the rule above the item
	protected Widget m_wBorder; //!< `Border`
	protected Widget m_wBackground; //!< `Background`
	protected ImageWidget m_wIcon; //!< `Icon`
	protected TextWidget m_wLabel; //!< `Label`

	protected TBD_BriefingTopicNav m_Owner; //!< weak -- the nav owns its items
	protected int m_iIndex = -1; //!< the TBD_EBriefingPage index; -1 until bound
	protected bool m_bActive; //!< the active item; default false
	protected bool m_bHasIcon; //!< the icon loaded
	protected int m_iGround; //!< ARGB ground colour under the item

	//! Find the named widgets, round the frames and hide the separator.
	//! @param w the item root
	override protected void OnBind(Widget w)
	{
		m_wSeparatorSize = w.FindAnyWidget("SeparatorSize");
		m_wSeparator = w.FindAnyWidget("Separator");
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wIcon = ImageWidget.Cast(w.FindAnyWidget("Icon"));
		m_wLabel = TextWidget.Cast(w.FindAnyWidget("Label"));

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UITheme.Show(m_wSeparatorSize, false);
	}

	//! Attach the item to its nav and apply its data: text, icon, separator, enabled state.
	//! @param owner the nav that owns the item
	//! @param index the TBD_EBriefingPage index
	//! @param data the item data
	//! @param ground the ARGB ground colour under the item
	void Bind(TBD_BriefingTopicNav owner, int index, TBD_NavItemData data, int ground)
	{
		m_Owner = owner;
		m_iIndex = index;
		m_iGround = ground;
		TBD_UITheme.Write(m_wLabel, data.m_sLabel);
		m_bHasIcon = TBD_UIIcons.Load(m_wIcon, data.m_sIcon);
		TBD_UITheme.Show(m_wSeparatorSize, data.m_bSeparatorBefore);
		SetInteractive(data.m_bEnabled);
		Repaint();
	}

	//! Mark the item active or not, repainting on change.
	//! @param active the new state
	void SetActive(bool active)
	{
		if (m_bActive == active)
			return;

		m_bActive = active;
		Repaint();
	}

	//! Paint the item for its active, hover, idle and interactive state.
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		int fill;
		int border;
		int ink;

		if (m_bActive)
		{
			fill = TBD_UITheme.TOPIC_ITEM_ACTIVE_FILL;
			border = TBD_UITheme.TRANSPARENT;
			ink = TBD_UITheme.ON_ACTION;
		}
		else if (IsHighlighted() && m_bInteractive)
		{
			fill = TBD_UITheme.TOPIC_ITEM_HOVER_FILL;
			border = TBD_UITheme.TOPIC_ITEM_HOVER_BORDER;
			ink = TBD_UITheme.BRIGHT_INK;
		}
		else
		{
			fill = TBD_UITheme.TRANSPARENT;
			border = TBD_UITheme.TRANSPARENT;
			ink = TBD_UITheme.ON_SURFACE_VARIANT;
		}

		if (!m_bInteractive)
			ink = TBD_UITheme.ROW_DISABLED_TEXT;

		TBD_UITheme.PaintOver(m_wBorder, border, m_iGround);
		TBD_UITheme.PaintOver(m_wBackground, fill, m_iGround);
		TBD_UITheme.PaintOver(m_wSeparator, TBD_UITheme.STRIP_BORDER, m_iGround);
		TBD_UITheme.Paint(m_wLabel, ink);
		if (m_bHasIcon)
			TBD_UITheme.Paint(m_wIcon, ink);
	}

	//! Report the activation to the owning nav.
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnItemActivated(m_iIndex);
	}
}
