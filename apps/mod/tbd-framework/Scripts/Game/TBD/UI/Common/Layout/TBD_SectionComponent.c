/**
 * @file TBD_SectionComponent.c
 * @brief The collapsible card every long list is made of: header button over a body the owner fills.
 *
 * Role: handler of `TBD_Section.layout`: a full-width header button (icon, title, badge chip,
 * action dock, chevron) over a rule and a `Body` vertical layout; a header click folds the body.
 * Position: mounted by the briefing rules and assets pages (rules groups, asset types, "Vehicle
 * Info", asset instances) through the static `Mount`; reports `GetOnToggled()(section, expanded)`.
 * State: the widget references, the badge chip, the expanded flag, the ground and the tint, on
 * the client.
 * Invariants: widget contract `Border`, `Background`, `HeaderButton`, `HeaderOverlay` (clips the
 * header band's bottom arcs), `HeaderBG`, `HeaderIcon`, `Title`, `BadgeDock`, `ActionDock`,
 * `Chevron`, `HeaderRule`, `Body`; a NEUTRAL border is the strip border, BLUFOR and OPFOR borders
 * come from `TBD_UITintColours.FactionRowBorder` so an enemy page reads red.
 */

//! Collapsible section handler.
class TBD_SectionComponent : ScriptedWidgetComponent
{
	protected Widget m_wRoot; //!< the layout root this handler sits on
	protected Widget m_wBorder; //!< `Border` frame dock
	protected Widget m_wBackground; //!< `Background` frame dock
	protected Widget m_wHeaderButton; //!< `HeaderButton`, the fold toggle
	protected Widget m_wHeaderBG; //!< `HeaderBG` frame dock
	protected ImageWidget m_wHeaderIcon; //!< `HeaderIcon`, hidden until SetIcon
	protected TextWidget m_wTitle; //!< `Title`
	protected Widget m_wBadgeDock; //!< `BadgeDock`
	protected Widget m_wActionDock; //!< `ActionDock`
	protected TextWidget m_wChevron; //!< `Chevron`: v expanded, > folded
	protected Widget m_wHeaderRule; //!< `HeaderRule`, hidden while folded
	protected Widget m_wBody; //!< `Body`, the vertical layout the owner fills

	protected TBD_ChipComponent m_Badge; //!< the header badge chip, mounted on first use
	protected bool m_bExpanded = true; //!< true while the body shows; default true
	protected int m_iGround; //!< opaque ARGB under the section; the panel ground by default
	protected TBD_EUITint m_eTint = TBD_EUITint.NEUTRAL; //!< the border tint; default NEUTRAL

	protected ref ScriptInvoker m_OnToggled; //!< (TBD_SectionComponent section, bool expanded)

	//! Find the widgets, round the frame docks, hide the icon, paint over the panel ground.
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

	//! Drop the root when the handler leaves its widget.
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

	//! Create a section under `parent` (a vertical layout), titled, over an opaque ground.
	//! @return the section, or null when the layout or its handler is missing
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

	//! Write the header title.
	void SetTitle(string title)
	{
		TBD_UITheme.Write(m_wTitle, title);
	}

	//! Show the header icon `iconKey` painted `argb` (0 = the muted default); empty hides it.
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

	//! Set the border tint and repaint.
	void SetTint(TBD_EUITint tint)
	{
		m_eTint = tint;
		Repaint();
	}

	//! Set the opaque colour under the section and repaint it and its badge.
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		if (m_Badge)
			m_Badge.SetGround(HeaderGround());
		Repaint();
	}

	//! Show or fold the body and its rule and turn the chevron; fires nothing.
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

	//! @return true while the body shows
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

	//! @return the layout root this handler sits on
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

	//! (TBD_SectionComponent section, bool expanded), fired on a header click.
	ScriptInvoker GetOnToggled()
	{
		if (!m_OnToggled)
			m_OnToggled = new ScriptInvoker();

		return m_OnToggled;
	}

	//! Clicks bubble up from the header button; a button in the ActionDock consumes its own first.
	//! @return true when a left click on the header folded or unfolded the body
	override bool OnClick(Widget w, int x, int y, int button)
	{
		if (w != m_wHeaderButton || button != 0)
			return false;

		SetExpanded(!m_bExpanded);
		if (m_OnToggled)
			m_OnToggled.Invoke(this, m_bExpanded);

		return true;
	}

	//! @return the opaque card fill over the section's ground
	protected int CardGround()
	{
		return TBD_UITheme.Over(TBD_UITheme.SQUAD_CARD_FILL, m_iGround);
	}

	//! Paint border (tinted for BLUFOR and OPFOR), fill, header band, rule, title and chevron.
	protected void Repaint()
	{
		if (!m_wRoot)
			return;

		int border = TBD_UITheme.STRIP_BORDER;
		if (m_eTint == TBD_EUITint.BLUFOR || m_eTint == TBD_EUITint.OPFOR)
			border = TBD_UITintColours.FactionRowBorder(m_eTint);

		int cardGround = CardGround();
		TBD_UITheme.PaintOver(m_wBorder, border, m_iGround);
		TBD_UITheme.PaintOver(m_wBackground, TBD_UITheme.SQUAD_BODY_FILL, cardGround);
		TBD_UITheme.PaintOver(m_wHeaderBG, TBD_UITheme.SQUAD_HEADER_FILL, cardGround);
		TBD_UITheme.PaintOver(m_wHeaderRule, TBD_UITheme.STRIP_BORDER, cardGround);
		TBD_UITheme.Paint(m_wTitle, TBD_UITheme.BRIGHT_INK);
		TBD_UITheme.Paint(m_wChevron, TBD_UITheme.MUTED_INK);
	}
}
