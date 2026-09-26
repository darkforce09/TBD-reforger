/**
 * @file TBD_LobbySlotRowComponent.c
 * @brief One seat row of a lobby squad card: role, weapon and tag chips, holder and status.
 *
 * Role: paints one TBD_LobbySlotInfo on TBD_LobbySlotRow.layout (widgets `Background`, `RoleText`,
 * `ChipsDock`, `HolderText`, `StatusChipDock`, `RowRule`) and reports a click to its panel.
 * Position: created and bound by TBD_LobbyRosterPanel.Rebuild inside a TBD_LobbySquadCardComponent;
 * activation calls TBD_LobbyRosterPanel.OnSlotActivated.
 * State: the bound seat's key and flags, its selection, and the chips it mounted; client UI only.
 * Invariants: a DEAD seat is not interactive; the holder line shows only for a held seat; the status
 * chip reads `DEAD` or `Unslotted` and hides otherwise.
 */

//! Seat row handler on TBD_LobbySlotRow.layout.
class TBD_LobbySlotRowComponent : TBD_UIInteractive
{
	protected Widget m_wBackground; //!< `Background`: the row fill
	protected TextWidget m_wRole; //!< `RoleText`: the upper-cased headline
	protected Widget m_wChipsDock; //!< `ChipsDock`: weapon and role-tag chips
	protected TextWidget m_wHolder; //!< `HolderText`: the holder's name
	protected Widget m_wStatusDock; //!< `StatusChipDock`: the DEAD or Unslotted chip
	protected Widget m_wRule; //!< `RowRule`: the divider under the row, hidden on the last row
	protected TBD_ChipComponent m_StatusChip; //!< the status chip; null until a status is first shown
	protected ref array<TBD_ChipComponent> m_aChips; //!< the weapon and tag chips of the current bind

	protected TBD_LobbyRosterPanel m_Owner; //!< weak; the panel told about activation
	protected string m_sKey; //!< the bound seat's slot key
	protected bool m_bSelected; //!< the panel's selected seat; default false
	protected bool m_bOwn; //!< the reader holds this seat
	protected bool m_bOpen = true; //!< the seat is OPEN; default true
	protected bool m_bDead; //!< the seat is DEAD (a spent life)
	protected int m_iGround; //!< opaque ARGB under the row; 0 = the panel ground

	//! Resolve the row's widgets and reset the chip list.
	override protected void OnBind(Widget w)
	{
		m_wBackground = w.FindAnyWidget("Background");
		m_wRole = TextWidget.Cast(w.FindAnyWidget("RoleText"));
		m_wChipsDock = w.FindAnyWidget("ChipsDock");
		m_wHolder = TextWidget.Cast(w.FindAnyWidget("HolderText"));
		m_wStatusDock = w.FindAnyWidget("StatusChipDock");
		m_wRule = w.FindAnyWidget("RowRule");
		m_aChips = {};
	}

	//! Paint `slot`: headline, weapon chips (neutral), tag chips (`MED` success, others primary), holder and status.
	//! @param owner the panel told on activation
	//! @param slot the seat to show
	//! @param ground opaque ARGB under the row; 0 = the panel ground
	//! @param last true on the card's last row, which hides the divider
	void Bind(TBD_LobbyRosterPanel owner, TBD_LobbySlotInfo slot, int ground, bool last)
	{
		m_Owner = owner;
		m_sKey = slot.m_sKey;
		m_bOwn = slot.m_bOwn;
		m_bOpen = slot.IsOpen();
		m_bDead = slot.IsDead();
		m_iGround = ground;

		string headline = slot.Headline();
		headline.ToUpper();
		TBD_UITheme.Write(m_wRole, headline);

		// Weapon chips (neutral mono) then role tags (MED emerald, ENG cyan-ish -> PRIMARY).
		TBD_UILayouts.Clear(m_wChipsDock);
		m_aChips.Clear();
		foreach (string weapon : slot.m_aWeapons)
		{
			AddChip(weapon, TBD_EUITint.NEUTRAL);
		}

		foreach (string tag : slot.m_aTags)
		{
			TBD_EUITint tint = TBD_EUITint.PRIMARY;
			if (tag == "MED")
				tint = TBD_EUITint.SUCCESS;

			AddChip(tag, tint);
		}

		TBD_UITheme.Write(m_wHolder, slot.m_sHolder);
		TBD_UITheme.Show(m_wRule, !last);

		string status;
		TBD_EUITint statusTint = TBD_EUITint.NEUTRAL;
		if (m_bDead)
		{
			status = "DEAD";
			statusTint = TBD_EUITint.DANGER;
		}
		else if (m_bOpen)
		{
			status = "Unslotted";
		}

		if (status.IsEmpty())
		{
			if (m_StatusChip)
				m_StatusChip.SetChipVisible(false);
		}
		else
		{
			if (!m_StatusChip)
			{
				m_StatusChip = TBD_ChipComponent.Mount(m_wStatusDock, status, statusTint);
				if (m_StatusChip)
					m_StatusChip.SetUppercase(false);
			}
			else
			{
				m_StatusChip.Set(status, statusTint);
			}

			if (m_StatusChip)
				m_StatusChip.SetChipVisible(true);
		}

		SetInteractive(!m_bDead);
		Repaint();
	}

	//! Mount one chip into the chips dock with 6 px right padding; skipped when the chip layout fails.
	protected void AddChip(string text, TBD_EUITint tint)
	{
		TBD_ChipComponent chip = TBD_ChipComponent.Mount(m_wChipsDock, text, tint);
		if (!chip)
			return;

		AlignableSlot.SetPadding(chip.GetRootWidget(), 0, 0, 6, 0);
		m_aChips.Insert(chip);
	}

	//! Mark the row as the panel's selection and repaint when it changes.
	void SetSelected(bool selected)
	{
		if (m_bSelected == selected)
			return;

		m_bSelected = selected;
		Repaint();
	}

	//! Paint fill, divider, inks and chip grounds for own, selected, hovered, open or dead; no-op before bind.
	override void Repaint()
	{
		if (!m_wRoot)
			return;

		int fill = TBD_UITheme.TRANSPARENT;
		int roleInk = TBD_UITheme.MUTED_INK;
		int holderInk = TBD_UITheme.HOLDER_INK;

		if (m_bOwn)
		{
			fill = TBD_UITheme.CARD_SELECTED_FILL;
			roleInk = TBD_UITheme.PRIMARY_FIXED;
			holderInk = TBD_UITheme.HOLDER_INK;
		}
		else if (m_bSelected)
		{
			fill = TBD_UITheme.CARD_HOVER_FILL;
			roleInk = TBD_UITheme.ON_SURFACE;
		}
		else if (IsHighlighted() && m_bInteractive)
		{
			fill = TBD_UITheme.SLOT_HOVER_FILL;
			roleInk = TBD_UITheme.ON_SURFACE;
		}
		else if (m_bOpen)
		{
			fill = TBD_UITheme.SLOT_OPEN_FILL;
			roleInk = TBD_UITheme.DIM_INK;
		}

		if (m_bDead)
			roleInk = TBD_UITheme.DIM_INK;

		int ground = m_iGround;
		if (ground == 0)
			ground = TBD_UITheme.PanelGround();

		TBD_UITheme.PaintOver(m_wBackground, fill, ground);
		TBD_UITheme.PaintOver(m_wRule, TBD_UITheme.SLOT_RULE, ground);
		TBD_UITheme.Paint(m_wRole, roleInk);
		TBD_UITheme.Paint(m_wHolder, holderInk);
		TBD_UITheme.Show(m_wHolder, !m_bOpen && !m_bDead);

		int rowGround = TBD_UITheme.Over(fill, ground);
		foreach (TBD_ChipComponent chip : m_aChips)
		{
			chip.SetGround(rowGround);
		}

		if (m_StatusChip)
			m_StatusChip.SetGround(rowGround);
	}

	//! Report a click to the owning panel.
	override protected void OnActivated()
	{
		if (m_Owner)
			m_Owner.OnSlotActivated(m_sKey);
	}

	//! @return the bound seat's slot key
	string GetKey()
	{
		return m_sKey;
	}
}
