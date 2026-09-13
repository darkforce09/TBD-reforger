//! Pre-game rebuild (2026-09-13) — the KIT INSPECTOR column of the Lobby (slot_kit_inspector
//! mockup).
//!
//! ```
//!   ┌ 👤 KIT INSPECTOR ─────────────────────────────────────────────┐
//!   │    8: RIFLEMAN (AT) [AK-74] [RPG-7] [Alpha 2-1]               │
//!   ├───────────────────────────────────────────────────────────────┤
//!   │ ┌ PREVIEW (empty frame until the visual-preview pass) ┐       │
//!   │ ┌ GEAR ─────┐ 4 cells a row · BACKPACK spans the last row     │
//!   │ ┌ WEAPONS ──┐ 3 slot cards: name · MOUNTED ATTACHMENTS · AMMO │
//!   │ ┌ GRENADES ─┐ ┌ GADGETS ─┐ ┌ TOOLS ─┐ ┌ MEDICAL ─┐ ┌ MISC ─┐   │
//!   └───────────────────────────────────────────────────────────────┘
//! ```
//!
//! `Show(slot, squad)` rebuilds the card stack from `TBD_LobbyCatalog.GetKit(slot.m_sKitKey)`;
//! nothing is pooled (a slot pick is user-paced). Widget contract of
//! `TBD_KitInspector.layout`: `PanelBorder`, `PanelBG`, `Header`, `HeaderBG`, `HeaderIcon`,
//! `Title`, `SlotTitle`, `SlotChipsDock`, `HeaderRule`, `BodyFrame`, `Scroll`, `CardsContent`,
//! `ScrollBarDock`, `EmptyState`. Section cards are `TBD_Panel`s; grids are `TBD_Columns3/4` of
//! `TBD_KitCell`; weapons are `TBD_KitWeaponCard`s with `TBD_KeyValueRow`s inside.
class TBD_KitInspectorPanel
{
	protected Widget m_wRoot;
	protected ImageWidget m_wHeaderIcon;
	protected TextWidget m_wTitle;
	protected TextWidget m_wSlotTitle;
	protected Widget m_wSlotChipsDock;
	protected Widget m_wCardsContent;
	protected Widget m_wEmptyState;
	protected ref TBD_UIScrollBar m_ScrollBar;
	protected ScrollLayoutWidget m_wScroll;

	protected TBD_LobbyCatalog m_Catalog;
	protected int m_iGround;     //!< the inspector's own glass
	protected int m_iCardGround; //!< a section card's fill

	static const int CARD_GAP = 12;

	//------------------------------------------------------------------------------------------------
	//! `root` is a mounted `TBD_KitInspector.layout`.
	bool Build(Widget root, TBD_LobbyCatalog catalog)
	{
		m_Catalog = catalog;
		m_wRoot = root;
		if (!root)
			return false;

		m_wHeaderIcon = ImageWidget.Cast(root.FindAnyWidget("HeaderIcon"));
		m_wTitle = TextWidget.Cast(root.FindAnyWidget("Title"));
		m_wSlotTitle = TextWidget.Cast(root.FindAnyWidget("SlotTitle"));
		m_wSlotChipsDock = root.FindAnyWidget("SlotChipsDock");
		m_wCardsContent = root.FindAnyWidget("CardsContent");
		m_wEmptyState = root.FindAnyWidget("EmptyState");

		Widget panelBorder = root.FindAnyWidget("PanelBorder");
		Widget panelBG = root.FindAnyWidget("PanelBG");
		Widget headerBG = root.FindAnyWidget("HeaderBG");
		TBD_UILayouts.MountRounded(panelBorder, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(panelBG, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UILayouts.MountRounded(headerBG, TBD_UITheme.RADIUS_PANEL - 1); // Header clips its bottom arcs

		m_iGround = TBD_UITheme.PanelGround();
		TBD_UITheme.PaintOver(panelBorder, TBD_UITheme.PanelBorder(TBD_EUITint.NEUTRAL), TBD_UITheme.Ground());
		TBD_UITheme.PaintOver(panelBG, TBD_UITheme.PANEL_FILL, TBD_UITheme.Ground());
		TBD_UITheme.PaintOver(headerBG, TBD_UITheme.KIT_HEADER_FILL, m_iGround);
		TBD_UITheme.PaintOver(root.FindAnyWidget("HeaderRule"), TBD_UITheme.KIT_CARD_BORDER, m_iGround);
		TBD_UITheme.Paint(m_wTitle, TBD_UITheme.BRIGHT_INK);
		TBD_UITheme.Paint(m_wSlotTitle, TBD_UITheme.ON_SURFACE);
		TBD_UITheme.Paint(m_wEmptyState, TBD_UITheme.MUTED_INK);
		m_iCardGround = TBD_UITheme.Over(TBD_UITheme.KIT_CARD_FILL, m_iGround);

		if (m_wHeaderIcon)
		{
			TBD_UIIcons.Load(m_wHeaderIcon, "person");
			TBD_UITheme.Paint(m_wHeaderIcon, TBD_UITheme.PRIMARY_FIXED);
		}

		m_wScroll = ScrollLayoutWidget.Cast(root.FindAnyWidget("Scroll"));
		m_ScrollBar = TBD_UIScrollBar.Mount(root.FindAnyWidget("ScrollBarDock"), m_wScroll, m_wCardsContent, m_iGround);

		Show(null, null);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	void Destroy()
	{
		if (m_ScrollBar)
			m_ScrollBar.Destroy();

		m_ScrollBar = null;
		m_wRoot = null;
		m_wCardsContent = null;
		m_Catalog = null;
	}

	//------------------------------------------------------------------------------------------------
	//! Rebind to a seat. Null shows the empty state.
	void Show(TBD_LobbySlotInfo slot, TBD_LobbySquadInfo squad)
	{
		TBD_UILayouts.Clear(m_wCardsContent);
		TBD_UILayouts.Clear(m_wSlotChipsDock);

		bool has = slot != null;
		TBD_UITheme.Show(m_wEmptyState, !has);
		TBD_UITheme.Show(m_wSlotTitle, has);
		TBD_UITheme.Show(m_wSlotChipsDock, has);
		if (!has)
		{
			ResetScroll();
			return;
		}

		string headline = slot.Headline();
		headline.ToUpper();
		TBD_UITheme.Write(m_wSlotTitle, headline);

		int headerGround = TBD_UITheme.Over(TBD_UITheme.KIT_HEADER_FILL, m_iGround);
		foreach (string weapon : slot.m_aWeapons)
		{
			AddHeaderChip(weapon, TBD_EUITint.NEUTRAL, headerGround);
		}

		if (squad)
			AddHeaderChip(squad.m_sCallsign, TBD_EUITint.PRIMARY, headerGround);

		TBD_KitInfo kit;
		if (m_Catalog)
			kit = m_Catalog.GetKit(slot.m_sKitKey);

		MountPreview();
		if (!kit)
		{
			ResetScroll();
			return;
		}

		FillGear(kit);
		FillWeapons(kit);
		FillCountGrid("Grenades", kit.m_aGrenades, 3);
		FillLabelGrid("Gadgets", kit.m_aGadgets, 4);
		FillLabelGrid("Tools", kit.m_aTools, 4);
		FillCountGrid("Medical", kit.m_aMedical, 3);
		FillLabelGrid("Miscellaneous", kit.m_aMisc, 4);
		ResetScroll();
	}

	//------------------------------------------------------------------------------------------------
	//! A rebuilt (shorter) stack under a scroll offset from the previous kit shows nothing until the
	//! player scrolls (MEASURED run 1) - the viewport is parked past the new content. Back to top.
	protected void ResetScroll()
	{
		if (m_wScroll)
			m_wScroll.SetSliderPos(0, 0);

		if (m_wCardsContent)
			m_wCardsContent.Update();
	}

	// ── Cards ───────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected void AddHeaderChip(string text, TBD_EUITint tint, int ground)
	{
		TBD_ChipComponent chip = TBD_ChipComponent.Mount(m_wSlotChipsDock, text, tint, ground);
		if (!chip)
			return;

		chip.SetUppercase(false);
		AlignableSlot.SetPadding(chip.GetRootWidget(), 0, 0, 6, 0);
	}

	//------------------------------------------------------------------------------------------------
	protected void MountPreview()
	{
		Widget preview = TBD_UILayouts.CreateStretched(TBD_UILayouts.LOBBY_KIT_PREVIEW, m_wCardsContent);
		if (!preview)
			return;

		Widget cardBorder = preview.FindAnyWidget("CardBorder");
		Widget cardBG = preview.FindAnyWidget("CardBG");
		Widget border = preview.FindAnyWidget("Border");
		Widget background = preview.FindAnyWidget("Background");
		TBD_UILayouts.MountRounded(cardBorder, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(cardBG, TBD_UITheme.RADIUS_ROW - 1);
		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_TAG);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_TAG - 1);

		TBD_UITheme.PaintOver(cardBorder, TBD_UITheme.KIT_CARD_BORDER, m_iGround);
		TBD_UITheme.PaintOver(cardBG, TBD_UITheme.KIT_CARD_FILL, m_iGround);
		TBD_UITheme.PaintOver(border, TBD_UITheme.KIT_WEAPON_BORDER, m_iCardGround);
		TBD_UITheme.PaintOver(background, TBD_UITheme.KIT_WEAPON_FILL, m_iCardGround);
		TBD_UITheme.Paint(preview.FindAnyWidget("Label"), TBD_UITheme.DIM_INK);

		// The mockup's dotted grid: the topo art at 20 % until the real preview lands here.
		ImageWidget grid = ImageWidget.Cast(preview.FindAnyWidget("GridImage"));
		if (TBD_UILayouts.LoadTexture(grid, TBD_UILayouts.HERO_TOPO))
			TBD_UITheme.PaintAlpha(grid, 0x3338BDF8);
	}

	//------------------------------------------------------------------------------------------------
	protected TBD_PanelComponent MountCard(string title)
	{
		if (!m_wCardsContent)
			return null;

		TBD_PanelComponent panel = TBD_PanelComponent.Cast(TBD_UILayouts.CreateHandler(TBD_UILayouts.PANEL, m_wCardsContent, TBD_PanelComponent));
		if (!panel)
			return null;

		Widget root = panel.GetRootWidget();
		AlignableSlot.SetHorizontalAlign(root, LayoutHorizontalAlign.Stretch);
		AlignableSlot.SetPadding(root, 0, 0, 0, CARD_GAP);

		panel.SetGround(m_iGround);
		panel.SetTitle(title);
		return panel;
	}

	//------------------------------------------------------------------------------------------------
	//! Body container of a card with the mockup's inner padding.
	protected Widget CardBody(TBD_PanelComponent panel)
	{
		Widget body = panel.GetBodyDock();
		if (!body)
			return null;

		// The body dock is an overlay; a vertical layout inside it stacks the grids.
		Widget stack = TBD_UILayouts.CreateStretched(TBD_UILayouts.COLUMNS_2, body);
		// COLUMNS_2 is only borrowed for its stretched root; use ColumnA as a one-column stack and
		// collapse ColumnB. (No dedicated single-stack layout exists; this avoids inventing one.)
		if (!stack)
			return null;

		Widget columnB = stack.FindAnyWidget("ColumnB");
		if (columnB)
			columnB.SetVisible(false);

		Widget columnA = stack.FindAnyWidget("ColumnA");
		AlignableSlot.SetPadding(stack, 14, 12, 14, 6);
		if (columnA)
			AlignableSlot.SetPadding(columnA, 0, 0, 0, 0);

		return columnA;
	}

	//------------------------------------------------------------------------------------------------
	//! GEAR: four cells a row; a trailing partial row keeps its cell width by padding with blanks.
	protected void FillGear(TBD_KitInfo kit)
	{
		TBD_PanelComponent card = MountCard("Gear");
		if (!card)
			return;

		Widget body = CardBody(card);
		FillCells(body, kit.m_aGear, 4, card.GetGround(), false);
	}

	//------------------------------------------------------------------------------------------------
	protected void FillLabelGrid(string title, array<ref TBD_KitEntry> entries, int columns)
	{
		if (entries.IsEmpty())
			return;

		TBD_PanelComponent card = MountCard(title);
		if (!card)
			return;

		FillCells(CardBody(card), entries, columns, card.GetGround(), false);
	}

	//------------------------------------------------------------------------------------------------
	//! GRENADES / MEDICAL: the item name is the value, the count is the story.
	protected void FillCountGrid(string title, array<ref TBD_KitEntry> entries, int columns)
	{
		if (entries.IsEmpty())
			return;

		TBD_PanelComponent card = MountCard(title);
		if (!card)
			return;

		FillCells(CardBody(card), entries, columns, card.GetGround(), true);
	}

	//------------------------------------------------------------------------------------------------
	//! Lay `entries` out `columns` wide, one TBD_Columns3/4 row per `columns` entries.
	protected void FillCells(Widget body, array<ref TBD_KitEntry> entries, int columns, int cardGround, bool countOnly)
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

	//------------------------------------------------------------------------------------------------
	protected static string ColumnName(int index)
	{
		switch (index)
		{
			case 0: return "ColumnA";
			case 1: return "ColumnB";
			case 2: return "ColumnC";
		}

		return "ColumnD";
	}

	//------------------------------------------------------------------------------------------------
	protected void MountCell(Widget column, TBD_KitEntry entry, int cardGround, bool countOnly)
	{
		if (!column)
			return;

		Widget cell = TBD_UILayouts.CreateStretched(TBD_UILayouts.LOBBY_KIT_CELL, column);
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

	//------------------------------------------------------------------------------------------------
	//! WEAPONS: up to three slot cards side by side.
	protected void FillWeapons(TBD_KitInfo kit)
	{
		if (kit.m_aWeapons.IsEmpty())
			return;

		TBD_PanelComponent card = MountCard("Weapons");
		if (!card)
			return;

		Widget body = CardBody(card);
		if (!body)
			return;

		ResourceName rowLayout = TBD_UILayouts.COLUMNS_3;
		if (kit.m_aWeapons.Count() == 1)
			rowLayout = TBD_UILayouts.COLUMNS_2;

		Widget row = TBD_UILayouts.CreateStretched(rowLayout, body);
		if (!row)
			return;

		int cardGround = card.GetGround();
		for (int i = 0; i < kit.m_aWeapons.Count() && i < 3; i++)
		{
			MountWeapon(row.FindAnyWidget(ColumnName(i)), kit.m_aWeapons[i], cardGround);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void MountWeapon(Widget column, TBD_KitWeapon weapon, int cardGround)
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
