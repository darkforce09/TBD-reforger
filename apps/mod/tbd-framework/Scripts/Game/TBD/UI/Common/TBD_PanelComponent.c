//! Pre-game rebuild (2026-09-12) — the glass card every Stitch panel sits in.
//!
//! `TBD_Panel.layout` is one shape reused by the terrain selector, the scenario browser, the four
//! inspector cards, the briefing panels and the lobby sidebar:
//!
//! ```
//!   ┌──────────────────────────────────────────────┐
//!   │ [icon] TITLE                     [badge dock] │  HeaderRow (44)
//!   ├──────────────────────────────────────────────┤  HeaderRule
//!   │                                              │
//!   │  BodyDock — the owning screen mounts here    │  fills the rest
//!   │                                              │
//!   └──────────────────────────────────────────────┘  FooterDock (hidden until used)
//! ```
//!
//! Widget contract (all optional, every write is null-safe): `PanelBorder`, `PanelBG`,
//! `HeaderRow`, `HeaderBG`, `HeaderIcon`, `HeaderTitle`, `HeaderBadgeDock`, `HeaderRule`,
//! `BodyDock`, `FooterDock`.
//!
//! Faction-tinted variants (BLUFOR / OPFOR columns) are the same layout with `SetTint()`; the
//! colours live in TBD_UITheme.PanelFill / PanelBorder, never here.
//!
//! Two layouts share this handler and widget contract:
//!   * `TBD_Panel.layout`     — content-sized (a VerticalLayout); stacks inside a scrolling list.
//!   * `TBD_PanelFill.layout` — frame-anchored; fills the dock it is mounted into and gives the
//!     body the remaining height. MEASURED 2026-09-12: a frame-anchored body mounted into the
//!     content-sized variant collapses to zero height (empty TERRAINS column) — columns must use
//!     PANEL_FILL.
class TBD_PanelComponent : ScriptedWidgetComponent
{
	[Attribute("", UIWidgets.EditBox, "Header title, if the owning screen does not set one")]
	protected string m_sTitle;

	[Attribute("", UIWidgets.EditBox, "Header icon key (TBD_UIIcons)")]
	protected string m_sIcon;

	[Attribute("1", UIWidgets.CheckBox, "Show the header row")]
	protected bool m_bHeader;

	protected Widget m_wRoot;
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected Widget m_wHeaderRow;
	protected Widget m_wHeaderBG;
	protected ImageWidget m_wHeaderIcon;
	protected TextWidget m_wHeaderTitle;
	protected Widget m_wHeaderBadgeDock;
	protected Widget m_wHeaderRule;
	protected Widget m_wBodyDock;
	protected Widget m_wFooterDock;

	protected TBD_EUITint m_eTint = TBD_EUITint.NEUTRAL;

	//! Opaque colour under this panel (0 = the screen backdrop). Cards inside another panel are
	//! told `PanelGround()` by whoever mounts them. See the colour law in TBD_UITheme.
	protected int m_iGround;
	//! Our own composited fill — what children sit on. Read through GetGround().
	protected int m_iFill;

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

	//------------------------------------------------------------------------------------------------
	//! Header text. Titles are uppercase in every mockup; the caller passes the words, we shout.
	void SetTitle(string title)
	{
		m_sTitle = title;
		string shout = title;
		shout.ToUpper();
		TBD_UITheme.Write(m_wHeaderTitle, shout);
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	void ShowHeader(bool shown)
	{
		m_bHeader = shown;
		TBD_UITheme.Show(m_wHeaderRow, shown);
		TBD_UITheme.Show(m_wHeaderRule, shown);
	}

	//------------------------------------------------------------------------------------------------
	void SetTint(TBD_EUITint tint)
	{
		m_eTint = tint;
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	//! The opaque colour this panel sits on. Columns on the backdrop keep the default; a card
	//! mounted inside another panel is given that panel's GetGround().
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	//! Our composited fill — the ground for everything mounted into the body or header.
	int GetGround()
	{
		return m_iFill;
	}

	//------------------------------------------------------------------------------------------------
	//! Where a chip or count pill goes, right-aligned in the header. Null-safe for callers.
	Widget GetBadgeDock()
	{
		return m_wHeaderBadgeDock;
	}

	//------------------------------------------------------------------------------------------------
	//! Where the panel's content is mounted. Falls back to the root so a stripped layout still
	//! receives children somewhere visible.
	Widget GetBodyDock()
	{
		if (m_wBodyDock)
			return m_wBodyDock;

		return m_wRoot;
	}

	//------------------------------------------------------------------------------------------------
	//! Optional bottom strip; showing it is the only way to make it take space.
	Widget GetFooterDock()
	{
		TBD_UITheme.Show(m_wFooterDock, true);
		return m_wFooterDock;
	}

	//------------------------------------------------------------------------------------------------
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//------------------------------------------------------------------------------------------------
	protected void Repaint()
	{
		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.Ground();

		m_iFill = TBD_UITheme.Over(TBD_UITheme.PanelFill(m_eTint), ground);

		TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.PanelBorder(m_eTint), ground);
		TBD_UITheme.PaintOver(m_wBackground, m_iFill, ground);
		TBD_UITheme.PaintOver(m_wHeaderBG, TBD_UITheme.PANEL_HEADER_FILL, m_iFill);
		TBD_UITheme.PaintOver(m_wHeaderRule, TBD_UITheme.GLASS_BORDER, m_iFill);
		TBD_UITheme.Paint(m_wHeaderTitle, TBD_UITheme.BRIGHT_INK);
	}
}
