/**
 * @file TBD_LobbyFactionRowComponent.c
 * @brief One pooled faction row of the Lobby FACTIONS column: name, role chip and claimed count.
 *
 * Role: paints one TBD_LobbyFactionInfo on TBD_LobbyFactionRow.layout (widgets `Border`,
 * `Background`, `Name`, `RoleChipDock`, `CountChipDock`) and reports a click by row index.
 * Position: pooled and bound by TBD_LobbyFactionPanel.Refresh; activation calls
 * TBD_LobbyFactionPanel.OnRowActivated.
 * State: the bound index, tint, ground and selection, and the two chips; client UI only.
 * Invariants: the count chip reads `<claimed> / <seats>`; the role chip hides when the faction has
 * no role label; a selected row paints lit like a hovered one.
 */

//! Faction row handler on TBD_LobbyFactionRow.layout.
class TBD_LobbyFactionRowComponent : TBD_UIInteractive
{
	protected Widget m_wBorder; //!< `Border`: rounded outline in the side tint
	protected Widget m_wBackground; //!< `Background`: rounded fill
	protected TextWidget m_wName; //!< `Name`: the faction display name
	protected Widget m_wRoleDock; //!< `RoleChipDock`: DEFENDING or ATTACKING
	protected Widget m_wCountDock; //!< `CountChipDock`: claimed / seats
	protected TBD_ChipComponent m_RoleChip; //!< role chip; hidden without a role label
	protected TBD_ChipComponent m_CountChip; //!< claimed-count chip

	protected TBD_LobbyFactionPanel m_Owner; //!< weak; the panel told about activation
	protected int m_iIndex = -1; //!< pool index; -1 until bound
	protected TBD_EUITint m_eTint = TBD_EUITint.NEUTRAL; //!< side tint; default NEUTRAL
	protected bool m_bSelected; //!< the panel's selected faction; default false
	protected int m_iGround; //!< opaque ARGB under the row; 0 = the panel ground

	//! Resolve the row's widgets and mount the rounded border and fill.
	override protected void OnBind(Widget w)
	{
		m_wBorder = w.FindAnyWidget("Border");
		m_wBackground = w.FindAnyWidget("Background");
		m_wName = TextWidget.Cast(w.FindAnyWidget("Name"));
		m_wRoleDock = w.FindAnyWidget("RoleChipDock");
		m_wCountDock = w.FindAnyWidget("CountChipDock");

		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
	}

	//! Paint `faction`.
	//! @param owner the panel told on activation
	//! @param index this row's pool index
	//! @param faction the faction to show
	//! @param claimed seats of the faction currently held
	//! @param ground opaque ARGB under the row; 0 = the panel ground
	void Bind(TBD_LobbyFactionPanel owner, int index, TBD_LobbyFactionInfo faction, int claimed, int ground)
	{
		m_Owner = owner;
		m_iIndex = index;
		m_eTint = faction.m_eTint;
		m_iGround = ground;

		TBD_UITheme.Write(m_wName, faction.m_sName);

		if (faction.m_sRoleLabel.IsEmpty())
		{
			if (m_RoleChip)
				m_RoleChip.SetChipVisible(false);
		}
		else
		{
			if (!m_RoleChip)
				m_RoleChip = TBD_ChipComponent.Mount(m_wRoleDock, faction.m_sRoleLabel, TBD_EUITint.NEUTRAL);
			else
				m_RoleChip.SetText(faction.m_sRoleLabel);

			if (m_RoleChip)
				m_RoleChip.SetChipVisible(true);
		}

		string count = string.Format("%1 / %2", claimed, faction.m_iSeats);
		if (!m_CountChip)
			m_CountChip = TBD_ChipComponent.Mount(m_wCountDock, count, TBD_EUITint.NEUTRAL);
		else
			m_CountChip.SetText(count);

		Repaint();
	}

	//! Mark the row as the panel's selection and repaint when it changes.
	void SetSelected(bool selected)
	{
		if (m_bSelected == selected)
			return;

		m_bSelected = selected;
		Repaint();
	}

	//! Paint fill, border, name ink and chip grounds from the tint, hover and selection; no-op before bind.
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		bool hovered = IsHighlighted() && m_bInteractive;
		int fill = TBD_UITintColours.FactionRowFill(m_eTint, hovered || m_bSelected); // selected = lit like hover
		int border = TBD_UITintColours.FactionRowBorder(m_eTint, m_bSelected);
		int ink = TBD_UITintColours.FactionRowInk(m_eTint);
		if (m_eTint == TBD_EUITint.NEUTRAL && (hovered || m_bSelected))
			ink = TBD_UITheme.ON_SURFACE;

		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.PanelGround();

		TBD_UITheme.PaintOver(m_wBackground, fill, ground);
		TBD_UITheme.PaintOver(m_wBorder, border, ground);
		TBD_UITheme.Paint(m_wName, ink);

		int rowGround = TBD_UITheme.Over(fill, ground);
		if (m_RoleChip)
			m_RoleChip.SetGround(rowGround);

		if (m_CountChip)
			m_CountChip.SetGround(rowGround);
	}

	//! Report a click to the owning panel by index.
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnRowActivated(m_iIndex);
	}

	//! Show or hide the pooled row.
	void SetRowVisible(bool visible)
	{
		TBD_UITheme.Show(m_wRoot, visible);
	}
}
