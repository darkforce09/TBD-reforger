//! Briefing pass (2026-09-14) — the collapsible card every long list is made of.
//!
//! Generalises the lobby squad card's header: a full-width button (icon · title · badge chip ·
//! action dock · chevron) over a rule and a `Body` vertical layout the owner fills. Clicking the
//! header folds the body. Widget contract of `TBD_Section.layout`: `Border`, `Background`,
//! `HeaderButton`, `HeaderOverlay` (clips the header band's bottom arcs), `HeaderBG`, `HeaderIcon`,
//! `Title`, `BadgeDock`, `ActionDock`, `Chevron`, `HeaderRule`, `Body`.
//!
//! Used by: rules groups, asset types, "Vehicle Info", asset instances. Tint: NEUTRAL border is the
//! strip border; BLUFOR / OPFOR borders come from `FactionRowBorder` so an enemy page reads red.
class TBD_SectionComponent : ScriptedWidgetComponent
{
	protected Widget m_wRoot;
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected Widget m_wHeaderButton;
	protected Widget m_wHeaderBG;
	protected ImageWidget m_wHeaderIcon;
	protected TextWidget m_wTitle;
	protected Widget m_wBadgeDock;
	protected Widget m_wActionDock;
	protected TextWidget m_wChevron;
	protected Widget m_wHeaderRule;
	protected Widget m_wBody;

	protected TBD_ChipComponent m_Badge;
	protected bool m_bExpanded = true;
	protected int m_iGround;
	protected TBD_EUITint m_eTint = TBD_EUITint.NEUTRAL;

	//! (TBD_SectionComponent section, bool expanded)
	protected ref ScriptInvoker m_OnToggled;

	//------------------------------------------------------------------------------------------------
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wHeaderButton = w.FindAnyWidget("HeaderButton");
		m_wHeaderBG = w.FindAnyWidget("HeaderBG");
		m_wHeaderIcon = ImageWidget.Cast(w.FindAnyWidget("HeaderIcon"));
		m_wTitle = TextWidget.Cast(w.FindAnyWidget("Title"));
		m_wBadgeDock = w.FindAnyWidget("BadgeDock");
		m_wActionDock = w.FindAnyWidget("ActionDock");
		m_wChevron = TextWidget.Cast(w.FindAnyWidget("Chevron"));
		m_wHeaderRule = w.FindAnyWidget("HeaderRule");
		m_wBody = w.FindAnyWidget("Body");

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
		TBD_UILayouts.MountRounded(m_wHeaderBG, TBD_UITheme.RADIUS_ROW - 1); // overlay clips the bottom arcs

		if (m_wHeaderIcon)
			m_wHeaderIcon.SetVisible(false);

		m_iGround = TBD_UITheme.PanelGround();
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

	//------------------------------------------------------------------------------------------------
	//! Create a section under `parent` (a vertical layout), titled, over an opaque ground.
	static TBD_SectionComponent Mount(Widget parent, string title, int ground)
	{
		Widget w = TBD_UILayouts.CreateStretched(TBD_UILayouts.SECTION, parent);
		if (!w)
			return null;

		TBD_SectionComponent section = TBD_SectionComponent.Cast(w.FindHandler(TBD_SectionComponent));
		if (!section)
			return null;

		section.SetGround(ground);
		section.SetTitle(title);
		return section;
	}

	// ── API ─────────────────────────────────────────────────────────────────────────────────

	void SetTitle(string title)
	{
		TBD_UITheme.Write(m_wTitle, title);
	}

	//! `argb` 0 = the muted default.
	void SetIcon(string iconKey, int argb = 0)
	{
		if (!m_wHeaderIcon)
			return;

		if (iconKey.IsEmpty())
		{
			m_wHeaderIcon.SetVisible(false);
			return;
		}

		if (argb == 0)
			argb = TBD_UITheme.MUTED_INK;

		if (TBD_UIIcons.Load(m_wHeaderIcon, iconKey))
			TBD_UITheme.Paint(m_wHeaderIcon, argb);
	}

	//! Trailing chip after the title (count, callsign); empty text hides it.
	void SetBadge(string text, TBD_EUITint tint)
	{
		if (text.IsEmpty())
		{
			if (m_Badge)
				m_Badge.SetChipVisible(false);
			return;
		}

		if (!m_Badge)
			m_Badge = TBD_ChipComponent.Mount(m_wBadgeDock, text, tint, HeaderGround());
		else
			m_Badge.Set(text, tint);

		if (m_Badge)
		{
			m_Badge.SetUppercase(false);
			m_Badge.SetChipVisible(true);
		}
	}

	void SetTint(TBD_EUITint tint)
	{
		m_eTint = tint;
		Repaint();
	}

	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		if (m_Badge)
			m_Badge.SetGround(HeaderGround());
		Repaint();
	}

	void SetExpanded(bool expanded)
	{
		m_bExpanded = expanded;
		TBD_UITheme.Show(m_wBody, expanded);
		TBD_UITheme.Show(m_wHeaderRule, expanded);
		if (expanded)
			TBD_UITheme.Write(m_wChevron, "v");
		else
			TBD_UITheme.Write(m_wChevron, ">");
	}

	bool IsExpanded()
	{
		return m_bExpanded;
	}

	//! The vertical layout the owner fills.
	Widget GetBody()
	{
		return m_wBody;
	}

	//! Right side of the header, before the chevron (a Locate button, a chip).
	Widget GetActionDock()
	{
		return m_wActionDock;
	}

	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//! Opaque colour under the body's children.
	int GetBodyGround()
	{
		return TBD_UITheme.Over(TBD_UITheme.SQUAD_BODY_FILL, CardGround());
	}

	//! Opaque colour under the header's chips / buttons.
	int HeaderGround()
	{
		return TBD_UITheme.Over(TBD_UITheme.SQUAD_HEADER_FILL, CardGround());
	}

	ScriptInvoker GetOnToggled()
	{
		if (!m_OnToggled)
			m_OnToggled = new ScriptInvoker();

		return m_OnToggled;
	}

	// ── Internals ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Clicks bubble up from the header button; a button in the ActionDock consumes its own first.
	override bool OnClick(Widget w, int x, int y, int button)
	{
		if (w != m_wHeaderButton || button != 0)
			return false;

		SetExpanded(!m_bExpanded);
		if (m_OnToggled)
			m_OnToggled.Invoke(this, m_bExpanded);

		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected int CardGround()
	{
		return TBD_UITheme.Over(TBD_UITheme.SQUAD_CARD_FILL, m_iGround);
	}

	//------------------------------------------------------------------------------------------------
	protected void Repaint()
	{
		if (!m_wRoot)
			return;

		int border = TBD_UITheme.STRIP_BORDER;
		if (m_eTint == TBD_EUITint.BLUFOR || m_eTint == TBD_EUITint.OPFOR)
			border = TBD_UITheme.FactionRowBorder(m_eTint);

		int cardGround = CardGround();
		TBD_UITheme.PaintOver(m_wBorder, border, m_iGround);
		TBD_UITheme.PaintOver(m_wBackground, TBD_UITheme.SQUAD_BODY_FILL, cardGround);
		TBD_UITheme.PaintOver(m_wHeaderBG, TBD_UITheme.SQUAD_HEADER_FILL, cardGround);
		TBD_UITheme.PaintOver(m_wHeaderRule, TBD_UITheme.STRIP_BORDER, cardGround);
		TBD_UITheme.Paint(m_wTitle, TBD_UITheme.BRIGHT_INK);
		TBD_UITheme.Paint(m_wChevron, TBD_UITheme.MUTED_INK);
	}
}
