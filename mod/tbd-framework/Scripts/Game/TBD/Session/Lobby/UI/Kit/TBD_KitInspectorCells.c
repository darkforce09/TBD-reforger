/**
 * @file TBD_KitInspectorCells.c
 * @brief Mounts the stat cells and weapon cards inside the kit inspector's section cards.
 *
 * Role: lays kit lines out as TBD_StatCell grids two, three or four wide, and builds one
 * TBD_KitWeaponCard with its attachment and ammunition rows.  Position: called by
 * TBD_KitInspectorPanel with a section card's body and ground.
 * State: none.  Invariants: a missing column or failed layout skips that cell or card and never
 * throws; a line with `None` or an empty value reads dimmed; a positive count shows `x<count>`.
 */

//! Static builders of the kit inspector's cells and weapon cards.
class TBD_KitInspectorCells
{
	//! Lay `entries` out `columns` wide (2, 3 or 4), one columns row per `columns` entries; stops when a row layout fails.
	static void FillCells(Widget body, array<ref TBD_KitEntry> entries, int columns, int cardGround, bool countOnly)
	{
		if (!body)
			return;

		ResourceName rowLayout = TBD_UILayouts.COLUMNS_4;
		if (columns == 3)
			rowLayout = TBD_UILayouts.COLUMNS_3;
		else if (columns == 2)
			rowLayout = TBD_UILayouts.COLUMNS_2;

		Widget row;
		int inRow = columns;
		foreach (TBD_KitEntry entry : entries)
		{
			if (inRow >= columns)
			{
				row = TBD_UILayouts.CreateStretched(rowLayout, body);
				inRow = 0;
				if (!row)
					return;
			}

			Widget column = row.FindAnyWidget(ColumnName(inRow));
			inRow++;
			MountCell(column, entry, cardGround, countOnly);
		}
	}

	//! @return `ColumnA` .. `ColumnC` for 0 .. 2, else `ColumnD`
	static string ColumnName(int index)
	{
		switch (index)
		{
			case 0: return "ColumnA";
			case 1: return "ColumnB";
			case 2: return "ColumnC";
		}

		return "ColumnD";
	}

	//! Mount one stat cell: item label only when `countOnly`, else upper-cased label and value (`None` dimmed, WARNING tint in holder ink); `x<count>` when the count is positive.
	static void MountCell(Widget column, TBD_KitEntry entry, int cardGround, bool countOnly)
	{
		if (!column)
			return;

		Widget cell = TBD_UILayouts.CreateStretched(TBD_UILayouts.STAT_CELL, column);
		if (!cell)
			return;

		Widget border = cell.FindAnyWidget("Border");
		Widget background = cell.FindAnyWidget("Background");
		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_TAG);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_TAG - 1);
		TBD_UITheme.PaintOver(border, TBD_UITheme.KIT_CELL_BORDER, cardGround);
		TBD_UITheme.PaintOver(background, TBD_UITheme.KIT_CELL_FILL, cardGround);

		TextWidget label = TextWidget.Cast(cell.FindAnyWidget("Label"));
		TextWidget value = TextWidget.Cast(cell.FindAnyWidget("Value"));
		TextWidget count = TextWidget.Cast(cell.FindAnyWidget("Count"));

		if (countOnly)
		{
			// "Bandages  x4": the label line carries the item, the value line is not used.
			string item = entry.m_sLabel;
			TBD_UITheme.Write(label, item);
			TBD_UITheme.Paint(label, TBD_UITheme.ON_SURFACE);
			TBD_UITheme.Show(value, false);
		}
		else
		{
			string shown = entry.m_sLabel;
			shown.ToUpper();
			TBD_UITheme.Write(label, shown);
			TBD_UITheme.Paint(label, TBD_UITheme.MUTED_INK);

			string text = entry.m_sValue;
			if (entry.IsNone())
				text = "None";

			TBD_UITheme.Write(value, text);
			if (entry.IsNone())
				TBD_UITheme.Paint(value, TBD_UITheme.DIM_INK);
			else if (entry.m_eTint == TBD_EUITint.WARNING)
				TBD_UITheme.Paint(value, TBD_UITheme.HOLDER_INK);
			else
				TBD_UITheme.Paint(value, TBD_UITheme.ON_SURFACE);
		}

		if (entry.m_iCount > 0)
		{
			TBD_UITheme.Write(count, string.Format("x%1", entry.m_iCount));
			TBD_UITheme.Paint(count, TBD_UITheme.HOLDER_INK);
			TBD_UITheme.Show(count, true);
		}
		else
		{
			TBD_UITheme.Show(count, false);
		}
	}

	//! Mount one weapon card: slot chip, name, attachment rows and ammo rows with `x<count>` chips.
	static void MountWeapon(Widget column, TBD_KitWeapon weapon, int cardGround)
	{
		if (!column)
			return;

		Widget item = TBD_UILayouts.CreateStretched(TBD_UILayouts.LOBBY_KIT_WEAPON, column);
		if (!item)
			return;

		Widget border = item.FindAnyWidget("Border");
		Widget background = item.FindAnyWidget("Background");
		Widget nameBorder = item.FindAnyWidget("NameBorder");
		Widget nameBG = item.FindAnyWidget("NameBG");
		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_ROW - 1);
		TBD_UILayouts.MountRounded(nameBorder, TBD_UITheme.RADIUS_TAG);
		TBD_UILayouts.MountRounded(nameBG, TBD_UITheme.RADIUS_TAG - 1);

		int weaponGround = TBD_UITheme.Over(TBD_UITheme.KIT_WEAPON_FILL, cardGround);
		TBD_UITheme.PaintOver(border, TBD_UITheme.KIT_CELL_BORDER, cardGround);
		TBD_UITheme.PaintOver(background, TBD_UITheme.KIT_WEAPON_FILL, cardGround);
		TBD_UITheme.PaintOver(nameBorder, TBD_UITheme.KIT_WEAPON_BORDER, weaponGround);
		TBD_UITheme.PaintOver(nameBG, TBD_UITheme.KIT_CELL_FILL, weaponGround);
		TBD_UITheme.PaintOver(item.FindAnyWidget("AmmoRule"), TBD_UITheme.KIT_WEAPON_BORDER, weaponGround);
		TBD_UITheme.Paint(item.FindAnyWidget("Name"), TBD_UITheme.BRIGHT_INK);
		TBD_UITheme.Paint(item.FindAnyWidget("AttachmentsTitle"), TBD_UITheme.MUTED_INK);
		TBD_UITheme.Paint(item.FindAnyWidget("AmmoTitle"), TBD_UITheme.MUTED_INK);
		TBD_UITheme.Paint(item.FindAnyWidget("AmmoSummary"), TBD_UITheme.DIM_INK);

		TBD_ChipComponent slotChip = TBD_ChipComponent.Mount(item.FindAnyWidget("SlotChipDock"), weapon.m_sSlotLabel, TBD_EUITint.NEUTRAL, weaponGround);
		TBD_UITheme.Write(TextWidget.Cast(item.FindAnyWidget("Name")), weapon.m_sName);
		TBD_UITheme.Write(TextWidget.Cast(item.FindAnyWidget("AmmoSummary")), weapon.m_sAmmoSummary);

		Widget attachments = item.FindAnyWidget("AttachmentsContent");
		foreach (TBD_KitEntry attachment : weapon.m_aAttachments)
		{
			TBD_KeyValueRowComponent kv = TBD_KeyValueRowComponent.Mount(attachments);
			if (!kv)
				continue;

			kv.SetGround(weaponGround);
			if (attachment.IsNone())
				kv.Set(attachment.m_sLabel, "None", TBD_EUITint.NEUTRAL);
			else
				kv.Set(attachment.m_sLabel, attachment.m_sValue, attachment.m_eTint);
		}

		Widget ammo = item.FindAnyWidget("AmmoContent");
		foreach (TBD_KitEntry round : weapon.m_aAmmo)
		{
			TBD_KeyValueRowComponent kv = TBD_KeyValueRowComponent.Mount(ammo);
			if (!kv)
				continue;

			kv.SetGround(weaponGround);
			kv.Set(round.m_sLabel, "");
			kv.SetValueChip(string.Format("x%1", round.m_iCount), TBD_EUITint.WARNING);
		}
	}
}
