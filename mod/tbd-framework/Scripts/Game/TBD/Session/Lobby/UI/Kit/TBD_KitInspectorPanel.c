/**
 * @file TBD_KitInspectorPanel.c
 * @brief The KIT INSPECTOR column of the Lobby: the chosen seat's headline, 3D preview and kit cards.
 *
 * Role: rebuilds, per seat, a header (headline, weapon and callsign chips), a preview card hosting
 * TBD_KitPreviewComponent, and section cards GEAR, WEAPONS, GRENADES, GADGETS, TOOLS, MEDICAL and
 * MISC from TBD_LobbyCatalog.GetKit, filled through TBD_KitInspectorCells.  Position: built by
 * TBD_LobbyScreen and TBD_BriefingOrbatPage into a mounted TBD_KitInspector.layout (widgets `PanelBorder`, `PanelBG`, `HeaderBG`,
 * `HeaderIcon`, `Title`, `SlotTitle`, `SlotChipsDock`, `HeaderRule`, `Scroll`, `CardsContent`,
 * `ScrollBarDock`, `EmptyState`); Show is called on every seat selection.
 * State: the widgets, the catalog, the grounds and the live preview; client UI only.
 * Invariants: nothing is pooled; the preview is destroyed before the cards are cleared; every
 * rebuild scrolls back to the top; a null seat shows the empty state.
 */

//! Controller of the KIT INSPECTOR column.
class TBD_KitInspectorPanel
{
	protected Widget m_wRoot; //!< the mounted TBD_KitInspector.layout; null after Destroy
	protected ImageWidget m_wHeaderIcon; //!< `HeaderIcon`: the person glyph
	protected TextWidget m_wTitle; //!< `Title`: KIT INSPECTOR
	protected TextWidget m_wSlotTitle; //!< `SlotTitle`: the upper-cased seat headline
	protected Widget m_wSlotChipsDock; //!< `SlotChipsDock`: weapon and callsign chips
	protected Widget m_wCardsContent; //!< `CardsContent`: preview and section cards
	protected Widget m_wEmptyState; //!< `EmptyState`: shown without a seat
	protected ref TBD_UIScrollBar m_ScrollBar; //!< scroll bar over `Scroll`
	protected ref TBD_KitPreviewComponent m_Preview; //!< the doll; rebuilt with the cards, null without one
	protected ScrollLayoutWidget m_wScroll; //!< `Scroll`: reset to the top on every rebuild

	protected TBD_LobbyCatalog m_Catalog; //!< the kit source; may be null
	protected int m_iGround; //!< opaque ARGB of the inspector's own glass
	protected int m_iCardGround; //!< opaque ARGB of a section card's fill

	static const int CARD_GAP = 12; //!< px between section cards

	//! Resolve and paint the inspector frame, mount the scroll bar and show the empty state.
	//! @param root a mounted TBD_KitInspector.layout
	//! @param catalog the kit source; may be null
	//! @return false without a root
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
		TBD_UITheme.PaintOver(panelBorder, TBD_UITintColours.PanelBorder(TBD_EUITint.NEUTRAL), TBD_UITheme.Ground());
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

	//! Destroy the preview and scroll bar and forget the widgets and catalog.
	void Destroy()
	{
		DestroyPreview();
		if (m_ScrollBar)
			m_ScrollBar.Destroy();

		m_ScrollBar = null;
		m_wRoot = null;
		m_wCardsContent = null;
		m_Catalog = null;
	}

	//! Rebuild for a seat: header chips, preview and every non-empty section card.
	//! @param slot the seat; null shows the empty state
	//! @param squad the seat's squad for the callsign chip; may be null
	void Show(TBD_LobbySlotInfo slot, TBD_LobbySquadInfo squad)
	{
		DestroyPreview();
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

		MountPreview(kit);
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

	//! Scroll back to the top: a shorter stack under the previous kit's scroll offset shows nothing until the player scrolls.
	protected void ResetScroll()
	{
		if (m_wScroll)
			m_wScroll.SetSliderPos(0, 0);

		if (m_wCardsContent)
			m_wCardsContent.Update();
	}


	//! Mount one header chip with 6 px right padding.
	protected void AddHeaderChip(string text, TBD_EUITint tint, int ground)
	{
		TBD_ChipComponent chip = TBD_ChipComponent.Mount(m_wSlotChipsDock, text, tint, ground);
		if (!chip)
			return;

		chip.SetUppercase(false);
		AlignableSlot.SetPadding(chip.GetRootWidget(), 0, 0, 6, 0);
	}

	//! Mount the preview card and attach TBD_KitPreviewComponent showing `kit`; captions PREVIEW UNAVAILABLE when the component cannot attach.
	protected void MountPreview(TBD_KitInfo kit)
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

		// The mockup's dotted grid: the topo art at 20 % alpha behind the doll.
		ImageWidget grid = ImageWidget.Cast(preview.FindAnyWidget("GridImage"));
		if (TBD_UILayouts.LoadTexture(grid, TBD_UILayouts.HERO_TOPO))
			TBD_UITheme.PaintAlpha(grid, 0x3338BDF8);

		Widget caption = preview.FindAnyWidget("Label");
		m_Preview = TBD_KitPreviewComponent.Attach(preview.FindAnyWidget("Preview"), caption);
		if (m_Preview)
			m_Preview.Show(kit);
		else
			TBD_UITheme.Write(TextWidget.Cast(caption), "PREVIEW UNAVAILABLE");
	}

	//! Destroy the live preview, if any.
	protected void DestroyPreview()
	{
		if (m_Preview)
			m_Preview.Destroy();

		m_Preview = null;
	}

	//! Mount one titled section card, stretched, CARD_GAP below it.
	//! @return the card, or null when the layout fails
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

	//! The one-column stack inside `panel`'s body, with the mockup's inner padding.
	//! @return the stack, or null when the layout fails
	protected Widget CardBody(TBD_PanelComponent panel)
	{
		Widget body = panel.GetBodyDock();
		if (!body)
			return null;

		// The body dock is an overlay; a vertical layout inside it stacks the grids.
		Widget stack = TBD_UILayouts.CreateStretched(TBD_UILayouts.COLUMNS_2, body);
		// COLUMNS_2 is only borrowed for its stretched root; use ColumnA as a one-column stack and
		// collapse ColumnB; no single-stack layout exists.
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

	//! GEAR: four cells a row.
	protected void FillGear(TBD_KitInfo kit)
	{
		TBD_PanelComponent card = MountCard("Gear");
		if (!card)
			return;

		Widget body = CardBody(card);
		TBD_KitInspectorCells.FillCells(body, kit.m_aGear, 4, card.GetGround(), false);
	}

	//! A label-and-value grid card; skipped when `entries` is empty.
	protected void FillLabelGrid(string title, array<ref TBD_KitEntry> entries, int columns)
	{
		if (entries.IsEmpty())
			return;

		TBD_PanelComponent card = MountCard(title);
		if (!card)
			return;

		TBD_KitInspectorCells.FillCells(CardBody(card), entries, columns, card.GetGround(), false);
	}

	//! GRENADES and MEDICAL: an item-and-count grid card; skipped when `entries` is empty.
	protected void FillCountGrid(string title, array<ref TBD_KitEntry> entries, int columns)
	{
		if (entries.IsEmpty())
			return;

		TBD_PanelComponent card = MountCard(title);
		if (!card)
			return;

		TBD_KitInspectorCells.FillCells(CardBody(card), entries, columns, card.GetGround(), true);
	}

	//! WEAPONS: up to three weapon cards side by side; skipped without weapons.
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
			TBD_KitInspectorCells.MountWeapon(row.FindAnyWidget(TBD_KitInspectorCells.ColumnName(i)), kit.m_aWeapons[i], cardGround);
		}
	}
}
