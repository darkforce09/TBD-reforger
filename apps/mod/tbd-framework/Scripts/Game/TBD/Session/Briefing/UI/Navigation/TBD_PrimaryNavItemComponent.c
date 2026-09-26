/**
 * @file TBD_PrimaryNavItemComponent.c
 * @brief One full-width item of the Briefing primary navigation.
 *
 * Role: binds and paints one item of TBD_PrimaryNavItem.layout (`Border`, `Background`, `Accent`,
 * `IconBoxBorder`, `IconBoxBG`, `Icon`, `Label`, `BadgeDock`) and reports activation.
 * Position: TBD_BriefingPrimaryNav creates and owns the items; activation calls back into it.
 * State: owner, index, active and hover state of one item on the client.  Invariants: active
 * paints the blue fill with the right accent bar and a lit icon box; hover paints the hover fill;
 * a non-interactive item draws disabled ink.
 */

//! One primary navigation item; the class name is referenced by the item layout.
class TBD_PrimaryNavItemComponent : TBD_UIInteractive
{
	protected Widget m_wBorder; //!< `Border`
	protected Widget m_wBackground; //!< `Background`
	protected Widget m_wAccentSize; //!< `AccentSize`, shown only while active
	protected Widget m_wAccent; //!< `Accent`, the right bar
	protected Widget m_wIconBoxBorder; //!< `IconBoxBorder`
	protected Widget m_wIconBoxBG; //!< `IconBoxBG`
	protected ImageWidget m_wIcon; //!< `Icon`
	protected TextWidget m_wLabel; //!< `Label`
	protected Widget m_wBadgeDock; //!< `BadgeDock`, holds count chips

	protected TBD_BriefingPrimaryNav m_Owner; //!< weak -- the nav owns its items
	protected int m_iIndex = -1; //!< the TBD_EBriefingMode index; -1 until bound
	protected bool m_bActive; //!< the active item; default false
	protected bool m_bHasIcon; //!< the icon loaded
	protected int m_iGround; //!< ARGB ground colour under the item

	//! Find the named widgets, round the frames and hide the accent.
	//! @param w the item root
	override protected void OnBind(Widget w)
	{
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wAccentSize = w.FindAnyWidget("AccentSize");
		m_wAccent = w.FindAnyWidget("Accent");
		m_wIconBoxBorder = w.FindAnyWidget("IconBoxBorder");
		m_wIconBoxBG = w.FindAnyWidget("IconBoxBG");
		m_wIcon = ImageWidget.Cast(w.FindAnyWidget("Icon"));
		m_wLabel = TextWidget.Cast(w.FindAnyWidget("Label"));
		m_wBadgeDock = w.FindAnyWidget("BadgeDock");

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UILayouts.MountRounded(m_wIconBoxBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wIconBoxBG, TBD_UITheme.RADIUS_ROW - 1);
		TBD_UITheme.Show(m_wAccentSize, false);
	}

	//! Attach the item to its nav and set its text and icon.
	//! @param owner the nav that owns the item
	//! @param index the TBD_EBriefingMode index
	//! @param label the item text
	//! @param icon the TBD_UIIcons key
	//! @param ground the ARGB ground colour under the item
	void Bind(TBD_BriefingPrimaryNav owner, int index, string label, string icon, int ground)
	{
		m_Owner = owner;
		m_iIndex = index;
		m_iGround = ground;
		TBD_UITheme.Write(m_wLabel, label);
		m_bHasIcon = TBD_UIIcons.Load(m_wIcon, icon);
		Repaint();
	}

	//! Add a mono count chip after the label, as on the Players item.
	//! @param text the chip text
	//! @param tint the chip tint
	void AddBadge(string text, TBD_EUITint tint)
	{
		TBD_ChipComponent chip = TBD_ChipComponent.Mount(m_wBadgeDock, text, tint, m_iGround);
		if (!chip)
			return;

		chip.SetUppercase(false);
		AlignableSlot.SetPadding(chip.GetRootWidget(), 4, 0, 0, 0);
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
		int boxFill;
		int boxBorder;
		int iconInk;

		if (m_bActive)
		{
			fill = TBD_UITheme.NAV_ITEM_ACTIVE_FILL;
			border = TBD_UITheme.NAV_ITEM_ACTIVE_BORDER;
			ink = TBD_UITheme.ON_ACTION;
			boxFill = TBD_UITheme.ICON_BOX_ACTIVE_FILL;
			boxBorder = TBD_UITheme.ICON_BOX_ACTIVE_BORDER;
			iconInk = TBD_UITheme.ON_ACTION;
		}
		else if (IsHighlighted() && m_bInteractive)
		{
			fill = TBD_UITheme.NAV_ITEM_HOVER_FILL;
			border = TBD_UITheme.TRANSPARENT;
			ink = TBD_UITheme.BRIGHT_INK;
			boxFill = TBD_UITheme.ICON_BOX_HOVER_FILL;
			boxBorder = TBD_UITheme.ICON_BOX_HOVER_BORDER;
			iconInk = TBD_UITheme.ACTION;
		}
		else
		{
			fill = TBD_UITheme.TRANSPARENT;
			border = TBD_UITheme.TRANSPARENT;
			ink = TBD_UITheme.ON_SURFACE;
			boxFill = TBD_UITheme.ICON_BOX_FILL;
			boxBorder = TBD_UITheme.ICON_BOX_BORDER;
			iconInk = TBD_UITheme.MUTED_INK;
		}

		if (!m_bInteractive)
			ink = TBD_UITheme.ROW_DISABLED_TEXT;

		TBD_UITheme.PaintOver(m_wBorder, border, m_iGround);
		TBD_UITheme.PaintOver(m_wBackground, fill, m_iGround);
		int itemGround = TBD_UITheme.Over(fill, m_iGround);
		TBD_UITheme.PaintOver(m_wIconBoxBorder, boxBorder, itemGround);
		TBD_UITheme.PaintOver(m_wIconBoxBG, boxFill, itemGround);
		TBD_UITheme.Paint(m_wLabel, ink);
		if (m_bHasIcon)
			TBD_UITheme.Paint(m_wIcon, iconInk);

		TBD_UITheme.Show(m_wAccentSize, m_bActive);
		if (m_bActive)
			TBD_UITheme.PaintOver(m_wAccent, TBD_UITheme.NAV_ACCENT, itemGround);
	}

	//! Report the activation to the owning nav.
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnItemActivated(m_iIndex);
	}

	//! @return the TBD_EBriefingMode index, or -1 before binding
	int GetIndex()
	{
		return m_iIndex;
	}
}
