/**
 * @file TBD_LobbySquadCardComponent.c
 * @brief One squad card of the lobby roster: callsign, vehicle and fill chips over its seat rows.
 *
 * Role: paints one TBD_LobbySquadInfo header on TBD_LobbySquadCard.layout (widgets `Border`,
 * `Background`, `HeaderButton`, `HeaderBG`, `CallsignChipDock`, `VehicleChipDock`, `CountChipDock`,
 * `ActionDock`, `Chevron`, `HeaderRule`, `SlotsContent`) and folds its rows on a header click.
 * Position: created and bound by TBD_LobbyRosterPanel.Rebuild, which mounts the seat rows into
 * GetSlotsContent and a Locate button into GetActionDock; a fold calls
 * TBD_LobbyRosterPanel.OnCardToggled.
 * State: the bound callsign, fold state, ground colour and the header chips; client UI only.
 * Invariants: the callsign chip wears the side tint; the count chip reads `<filled>/<seats>`; a
 * folded card hides its rows and header rule and shows `>` instead of `v`.
 */

//! Squad card handler on TBD_LobbySquadCard.layout.
class TBD_LobbySquadCardComponent : ScriptedWidgetComponent
{
	protected Widget m_wRoot; //!< the card root; null after detach
	protected Widget m_wBorder; //!< `Border`: rounded outline
	protected Widget m_wBackground; //!< `Background`: rounded body fill
	protected Widget m_wHeaderButton; //!< `HeaderButton`: the fold toggle
	protected Widget m_wHeaderBG; //!< `HeaderBG`: header band fill
	protected Widget m_wCallsignDock; //!< `CallsignChipDock`
	protected Widget m_wVehicleDock; //!< `VehicleChipDock`
	protected Widget m_wCountDock; //!< `CountChipDock`
	protected Widget m_wActionDock; //!< `ActionDock`: the briefing's Locate button
	protected TextWidget m_wChevron; //!< `Chevron`: `v` unfolded, `>` folded
	protected Widget m_wHeaderRule; //!< `HeaderRule`: divider under the header
	protected Widget m_wSlotsContent; //!< `SlotsContent`: the seat rows

	protected TBD_ChipComponent m_CallsignChip; //!< callsign chip in the side tint
	protected TBD_ChipComponent m_VehicleChip; //!< vehicle chip; hidden without a vehicle
	protected TBD_ChipComponent m_CountChip; //!< `<filled>/<seats>` chip

	protected bool m_bExpanded = true; //!< rows shown; default true
	protected int m_iGround; //!< opaque ARGB under the card
	protected string m_sCallsign; //!< the bound squad's callsign
	protected TBD_LobbyRosterPanel m_Owner; //!< weak; told when the player folds or unfolds the card

	//! Resolve the card's widgets and mount the rounded border, body and header shapes.
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wHeaderButton = w.FindAnyWidget("HeaderButton");
		m_wHeaderBG = w.FindAnyWidget("HeaderBG");
		m_wCallsignDock = w.FindAnyWidget("CallsignChipDock");
		m_wVehicleDock = w.FindAnyWidget("VehicleChipDock");
		m_wCountDock = w.FindAnyWidget("CountChipDock");
		m_wActionDock = w.FindAnyWidget("ActionDock");
		m_wChevron = TextWidget.Cast(w.FindAnyWidget("Chevron"));
		m_wHeaderRule = w.FindAnyWidget("HeaderRule");
		m_wSlotsContent = w.FindAnyWidget("SlotsContent");

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
		// Header band: rounded top, square bottom (its overlay clips the extra 8 px).
		TBD_UILayouts.MountRounded(m_wHeaderBG, TBD_UITheme.RADIUS_ROW - 1);
	}

	//! Drop the root reference.
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

	//! Paint `squad`'s header chips.
	//! @param owner the panel told on a fold
	//! @param squad the squad to show
	//! @param ground opaque ARGB under the card
	//! @param tint the side tint the callsign chip wears
	void Bind(TBD_LobbyRosterPanel owner, TBD_LobbySquadInfo squad, int ground, TBD_EUITint tint)
	{
		m_Owner = owner;
		m_sCallsign = squad.m_sCallsign;
		m_iGround = ground;
		int headerGround = TBD_UITheme.Over(TBD_UITheme.SQUAD_HEADER_FILL, TBD_UITheme.Over(TBD_UITheme.SQUAD_CARD_FILL, ground));

		// The callsign wears the side's colour: blue on BLUFOR, red on OPFOR (operator, run 1).
		if (!m_CallsignChip)
			m_CallsignChip = TBD_ChipComponent.Mount(m_wCallsignDock, squad.m_sCallsign, tint, headerGround);
		else
			m_CallsignChip.Set(squad.m_sCallsign, tint);

		if (m_CallsignChip)
			m_CallsignChip.SetUppercase(false);

		if (squad.m_sVehicle.IsEmpty())
		{
			if (m_VehicleChip)
				m_VehicleChip.SetChipVisible(false);
		}
		else
		{
			if (!m_VehicleChip)
				m_VehicleChip = TBD_ChipComponent.Mount(m_wVehicleDock, squad.m_sVehicle, TBD_EUITint.NEUTRAL, headerGround);
			else
				m_VehicleChip.SetText(squad.m_sVehicle);

			if (m_VehicleChip)
				m_VehicleChip.SetChipVisible(true);
		}

		string count = string.Format("%1/%2", squad.Filled(), squad.m_aSlots.Count());
		if (!m_CountChip)
			m_CountChip = TBD_ChipComponent.Mount(m_wCountDock, count, TBD_EUITint.NEUTRAL, headerGround);
		else
			m_CountChip.SetText(count);

		Repaint();
	}

	//! Show or hide the seat rows and header rule and set the chevron.
	void SetExpanded(bool expanded)
	{
		m_bExpanded = expanded;
		TBD_UITheme.Show(m_wSlotsContent, expanded);
		TBD_UITheme.Show(m_wHeaderRule, expanded);
		if (expanded)
			TBD_UITheme.Write(m_wChevron, "v");
		else
			TBD_UITheme.Write(m_wChevron, ">");
	}

	//! @return true while the seat rows are shown
	bool IsExpanded()
	{
		return m_bExpanded;
	}

	//! Header dock before the count chip (the briefing's Locate button).
	Widget GetActionDock()
	{
		return m_wActionDock;
	}

	//! @return the bound squad's callsign
	string GetCallsign()
	{
		return m_sCallsign;
	}

	//! @return the container the seat rows mount into
	Widget GetSlotsContent()
	{
		return m_wSlotsContent;
	}

	//! Opaque colour under the slot rows.
	int GetBodyGround()
	{
		return TBD_UITheme.Over(TBD_UITheme.SQUAD_BODY_FILL, TBD_UITheme.Over(TBD_UITheme.SQUAD_CARD_FILL, m_iGround));
	}

	//! @return the card root; null after detach
	Widget GetRootWidget()
	{
		return m_wRoot;
	}

	//! Toggle the fold on a left click of the header and tell the panel; a seat row consumes its own clicks.
	//! @return true when the click was the header's
	override bool OnClick(Widget w, int x, int y, int button)
	{
		if (w != m_wHeaderButton || button != 0)
			return false;

		SetExpanded(!m_bExpanded);
		if (m_Owner)
			m_Owner.OnCardToggled(m_sCallsign, m_bExpanded);

		return true;
	}

	//! Paint the border, body, header band, header rule and chevron over the card ground.
	protected void Repaint()
	{
		int cardGround = TBD_UITheme.Over(TBD_UITheme.SQUAD_CARD_FILL, m_iGround);
		TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.STRIP_BORDER, m_iGround);
		TBD_UITheme.PaintOver(m_wBackground, TBD_UITheme.SQUAD_BODY_FILL, cardGround);
		TBD_UITheme.PaintOver(m_wHeaderBG, TBD_UITheme.SQUAD_HEADER_FILL, cardGround);
		TBD_UITheme.PaintOver(m_wHeaderRule, TBD_UITheme.STRIP_BORDER, cardGround);
		TBD_UITheme.Paint(m_wChevron, TBD_UITheme.MUTED_INK);
	}
}
