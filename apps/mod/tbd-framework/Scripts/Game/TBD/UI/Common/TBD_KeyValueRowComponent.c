//! Pre-game rebuild (2026-09-12) — "label left, mono value right" in an inset box.
//!
//! Parameters (`View Distance  2,500 m`), frequencies (`Aux Channels:  133.3, 276.9 MHz`), the
//! kit inspector's attachment lines, the inspector's vehicle / objective rows (`2x  BMP-2`) —
//! all one row shape. Either side may carry a small chip instead of plain text.
//!
//! Widget contract on `TBD_KeyValueRow.layout`: `RowBorder`, `RowBG`, `KeyIcon`, `KeyChipDock`,
//! `KeyText`, `ValueText`, `ValueChipDock`.
class TBD_KeyValueRowComponent : ScriptedWidgetComponent
{
	protected Widget m_wRoot;
	protected Widget m_wBorder;
	protected Widget m_wBackground;
	protected ImageWidget m_wKeyIcon;
	protected Widget m_wKeyChipDock;
	protected TextWidget m_wKeyText;
	protected TextWidget m_wValueText;
	protected Widget m_wValueChipDock;

	protected TBD_ChipComponent m_KeyChip;
	protected TBD_ChipComponent m_ValueChip;
	protected TBD_EUITint m_eTint = TBD_EUITint.NEUTRAL;
	//! Opaque colour under the row; 0 = glass panel. Faction columns pass their GetGround().
	protected int m_iGround;
	//! Our composited fill — what the key / value chips sit on.
	protected int m_iFill;

	//------------------------------------------------------------------------------------------------
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_wBorder = w.FindAnyWidget("RowBorder");
		m_wBackground = w.FindAnyWidget("RowBG");
		TBD_UILayouts.MountRounded(m_wBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(m_wBackground, TBD_UITheme.RADIUS_ROW - 1);
		m_wKeyIcon = ImageWidget.Cast(w.FindAnyWidget("KeyIcon"));
		m_wKeyChipDock = w.FindAnyWidget("KeyChipDock");
		m_wKeyText = TextWidget.Cast(w.FindAnyWidget("KeyText"));
		m_wValueText = TextWidget.Cast(w.FindAnyWidget("ValueText"));
		m_wValueChipDock = w.FindAnyWidget("ValueChipDock");

		TBD_UITheme.Show(m_wKeyIcon, false);
		TBD_UITheme.Show(m_wKeyChipDock, false);
		TBD_UITheme.Show(m_wValueChipDock, false);
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		m_wRoot = null;
		super.HandlerDeattached(w);
	}

	//------------------------------------------------------------------------------------------------
	//! Plain key / value. `valueTint` colours the value text (amber safe-start, red Disabled…).
	void Set(string key, string value, TBD_EUITint valueTint = TBD_EUITint.NEUTRAL)
	{
		TBD_UITheme.Write(m_wKeyText, key);
		TBD_UITheme.Write(m_wValueText, value);
		TBD_UITheme.Show(m_wValueText, !value.IsEmpty());
		TBD_UITheme.Show(m_wValueChipDock, false);

		if (valueTint == TBD_EUITint.NEUTRAL)
			TBD_UITheme.Paint(m_wValueText, TBD_UITheme.ON_SURFACE);
		else
			TBD_UITheme.Paint(m_wValueText, TBD_UITheme.ChipInk(valueTint));
	}

	//------------------------------------------------------------------------------------------------
	//! Leading chip before the key (`2x`, `1x`).
	void SetKeyChip(string text, TBD_EUITint tint)
	{
		if (!m_wKeyChipDock)
			return;

		if (text.IsEmpty())
		{
			TBD_UITheme.Show(m_wKeyChipDock, false);
			return;
		}

		if (!m_KeyChip)
			m_KeyChip = TBD_ChipComponent.Mount(m_wKeyChipDock, text, tint, m_iFill);
		else
			m_KeyChip.Set(text, tint);

		TBD_UITheme.Show(m_wKeyChipDock, m_KeyChip != null);
	}

	//------------------------------------------------------------------------------------------------
	//! Trailing chip instead of a value (`v2.4.0`, `Disabled`).
	void SetValueChip(string text, TBD_EUITint tint)
	{
		if (!m_wValueChipDock)
			return;

		TBD_UITheme.Show(m_wValueText, false);

		if (!m_ValueChip)
			m_ValueChip = TBD_ChipComponent.Mount(m_wValueChipDock, text, tint, m_iFill);
		else
			m_ValueChip.Set(text, tint);

		TBD_UITheme.Show(m_wValueChipDock, m_ValueChip != null);
	}

	//------------------------------------------------------------------------------------------------
	void SetIcon(string iconKey, int argb)
	{
		if (!m_wKeyIcon)
			return;

		if (TBD_UIIcons.Load(m_wKeyIcon, iconKey))
			TBD_UITheme.Paint(m_wKeyIcon, argb);
	}

	//------------------------------------------------------------------------------------------------
	//! Row chrome tint (faction columns paint their rows in the faction hue on hover only; the
	//! resting state is the neutral inset).
	void SetTint(TBD_EUITint tint)
	{
		m_eTint = tint;
		Repaint();
	}

	//------------------------------------------------------------------------------------------------
	//! Opaque colour under the row (the owning panel's / faction column's GetGround()).
	void SetGround(int opaqueArgb)
	{
		m_iGround = opaqueArgb;
		Repaint();
		if (m_KeyChip)
			m_KeyChip.SetGround(m_iFill);
		if (m_ValueChip)
			m_ValueChip.SetGround(m_iFill);
	}

	//------------------------------------------------------------------------------------------------
	void SetRowVisible(bool visible)
	{
		TBD_UITheme.Show(m_wRoot, visible);
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
			ground = TBD_UITheme.PanelGround();

		m_iFill = TBD_UITheme.Over(TBD_UITheme.INSET_FILL, ground);
		TBD_UITheme.PaintOver(m_wBackground, m_iFill, ground);
		TBD_UITheme.PaintOver(m_wBorder, TBD_UITheme.PanelBorder(m_eTint), ground);
		TBD_UITheme.Paint(m_wKeyText, TBD_UITheme.ON_SURFACE);
	}

	//------------------------------------------------------------------------------------------------
	//! Mount a row into a vertical container and return its handler.
	static TBD_KeyValueRowComponent Mount(Widget container)
	{
		if (!container)
			return null;

		TBD_KeyValueRowComponent row = TBD_KeyValueRowComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.KEY_VALUE_ROW, container, TBD_KeyValueRowComponent));
		if (row)
			AlignableSlot.SetHorizontalAlign(row.GetRootWidget(), LayoutHorizontalAlign.Stretch);

		return row;
	}
}
