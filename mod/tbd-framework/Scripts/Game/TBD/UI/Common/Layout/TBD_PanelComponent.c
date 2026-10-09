/**
 * @file TBD_PanelComponent.c
 * @brief The glass card every pre-game panel sits in: header, badge dock, body and footer.
 *
 * Role: handler of `TBD_Panel.layout` and `TBD_PanelFill.layout`; writes the header, paints the
 * glass or faction-tinted chrome and hands its composited fill to its children as their ground.
 * Position: mounted by the terrain selector, scenario browser, mission and kit inspectors, the
 * briefing pages and the lobby roster and faction panels.
 * State: the widget references, the tint, the ground and the composited fill, on the client.
 * Invariants: widget contract (every write null-safe) `PanelBorder`, `PanelBG`, `HeaderRow`,
 * `HeaderBG`, `HeaderIcon`, `HeaderTitle`, `HeaderBadgeDock`, `HeaderRule`, `BodyDock`,
 * `FooterDock`; tint colours come from `TBD_UITintColours`, never from here.
 */

//! Panel handler. Top to bottom: `HeaderRow` (44 px: icon, TITLE, badge dock on the right),
//! `HeaderRule`, `BodyDock` (the owning screen mounts here; it fills the rest) and `FooterDock`
//! (hidden until used).
//!
//! `TBD_Panel.layout` is content-sized (a VerticalLayout) and stacks inside a scrolling list;
//! `TBD_PanelFill.layout` is frame-anchored, fills its dock and gives the body the remaining
//! height. A frame-anchored body mounted into the content-sized variant collapses to zero height,
//! so columns use PANEL_FILL.
class TBD_PanelComponent : ScriptedWidgetComponent
{
	[Attribute("", UIWidgets.EditBox, desc: "Header title, if the owning screen does not set one")]
	protected string m_sTitle; //!< header title; default empty

	[Attribute("", UIWidgets.EditBox, desc: "Header icon key (TBD_UIIcons)")]
	protected string m_sIcon; //!< header icon key; empty hides the icon

	[Attribute("1", UIWidgets.CheckBox, desc: "Show the header row")]
	protected bool m_bHeader; //!< true shows the header row and rule; default true

	protected Widget m_wRoot; //!< the layout root this handler sits on
	protected Widget m_wBorder; //!< `PanelBorder` frame dock
	protected Widget m_wBackground; //!< `PanelBG` frame dock
	protected Widget m_wHeaderRow; //!< `HeaderRow`
	protected Widget m_wHeaderBG; //!< `HeaderBG` frame dock
	protected ImageWidget m_wHeaderIcon; //!< `HeaderIcon`
	protected TextWidget m_wHeaderTitle; //!< `HeaderTitle`
	protected Widget m_wHeaderBadgeDock; //!< `HeaderBadgeDock`
	protected Widget m_wHeaderRule; //!< `HeaderRule`
	protected Widget m_wBodyDock; //!< `BodyDock`
	protected Widget m_wFooterDock; //!< `FooterDock`, hidden until GetFooterDock

	protected TBD_EUITint m_eTint = TBD_EUITint.NEUTRAL; //!< the chrome tint; default NEUTRAL

	//! Opaque colour under this panel (0 = the screen backdrop). Cards inside another panel are
	//! told `PanelGround()` by whoever mounts them. See the colour law in TBD_UITheme.
	protected int m_iGround; //!< 0 = the screen backdrop
	protected int m_iFill; //!< Our own composited fill -- what children sit on. Read through GetGround().

	//! Find the widgets, round the frame docks, apply the attribute title, icon and header, paint.
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;

		m_wBorder = w.FindAnyWidget("PanelBorder");
		m_wBackground = w.FindAnyWidget("PanelBG");
		m_wHeaderRow = w.FindAnyWidget("HeaderRow");
		m_wHeaderBG = w.FindAnyWidget("HeaderBG");
		m_wHeaderIcon = ImageWidget.Cast(w.FindAnyWidget("HeaderIcon"));
		m_wHeaderTitle = TextWidget.Cast(w.FindAnyWidget("HeaderTitle"));
		m_wHeaderBadgeDock = w.FindAnyWidget("HeaderBadgeDock");
		m_wHeaderRule = w.FindAnyWidget("HeaderRule");
		m_wBodyDock = w.FindAnyWidget("BodyDock");
		m_wFooterDock = w.FindAnyWidget("FooterDock");

		// rounded-xl. The header fill is rounded too so it cannot poke square corners past the
		// border; its bottom corners hide behind the rule at 3 % alpha.
		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UILayouts.MountRounded(m_wHeaderBG, TBD_UITheme.RADIUS_PANEL - 1);

		SetTitle(m_sTitle);
		SetIcon(m_sIcon);
		ShowHeader(m_bHeader);
		TBD_UITheme.Show(m_wFooterDock, false);
		Repaint();
	}

	//! Drop the root when the handler leaves its widget.
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

	//! Header text. Titles are uppercase in every mockup; the caller passes the words, we shout.
	void SetTitle(string title)
	{
		m_sTitle = title;
		string shout = title;
		shout.ToUpper();
		TBD_UITheme.Write(m_wHeaderTitle, shout);
	}

	//! Show the header icon `iconKey` in PRIMARY_CONTAINER; empty hides it.
	void SetIcon(string iconKey)
	{
		m_sIcon = iconKey;
		if (!m_wHeaderIcon)
			return;

		if (iconKey.IsEmpty())
		{
			m_wHeaderIcon.SetVisible(false);
			return;
		}

		TBD_UIIcons.Load(m_wHeaderIcon, iconKey);
		TBD_UITheme.Paint(m_wHeaderIcon, TBD_UITheme.PRIMARY_CONTAINER);
	}

	//! Header icon ink (default PRIMARY_CONTAINER): faction pages paint it BLUFOR / OPFOR.
	void SetIconTint(int argb)
	{
		TBD_UITheme.Paint(m_wHeaderIcon, argb);
	}

	//! Show or hide the header row and its rule.
	void ShowHeader(bool shown)
	{
		m_bHeader = shown;
		TBD_UITheme.Show(m_wHeaderRow, shown);
		TBD_UITheme.Show(m_wHeaderRule, shown);
	}

	//! Set the chrome tint (BLUFOR and OPFOR columns) and repaint.
	void SetTint(TBD_EUITint tint)
	{
		m_eTint = tint;
		Repaint();
	}

	//! The opaque colour this panel sits on. Columns on the backdrop keep the default; a card
	//! mounted inside another panel is given that panel's GetGround().
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		Repaint();
	}

	//! Our composited fill -- the ground for everything mounted into the body or header.
	int GetGround()
	{
		return m_iFill;
	}

	//! Where a chip or count pill goes, right-aligned in the header. Null-safe for callers.
	Widget GetBadgeDock()
	{
		return m_wHeaderBadgeDock;
	}

	//! Where the panel's content is mounted. Falls back to the root so a stripped layout still
	//! receives children somewhere visible.
	Widget GetBodyDock()
	{
		if (m_wBodyDock)
			return m_wBodyDock;

		return m_wRoot;
	}

	//! Optional bottom strip; showing it is the only way to make it take space.
	Widget GetFooterDock()
	{
		TBD_UITheme.Show(m_wFooterDock, true);
		return m_wFooterDock;
	}

	//! @return the layout root this handler sits on
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//! Composite the tinted fill over the ground and paint border, fill, header fill, rule and title.
	protected void Repaint()
	{
		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.Ground();

		m_iFill = TBD_UITheme.Over(TBD_UITintColours.PanelFill(m_eTint), ground);

		TBD_UITheme.PaintOver(m_wBorder, TBD_UITintColours.PanelBorder(m_eTint), ground);
		TBD_UITheme.PaintOver(m_wBackground, m_iFill, ground);
		TBD_UITheme.PaintOver(m_wHeaderBG, TBD_UITheme.PANEL_HEADER_FILL, m_iFill);
		TBD_UITheme.PaintOver(m_wHeaderRule, TBD_UITheme.GLASS_BORDER, m_iFill);
		TBD_UITheme.Paint(m_wHeaderTitle, TBD_UITheme.BRIGHT_INK);
	}
}
