/**
 * @file TBD_BriefingPage.c
 * @brief The base of every Briefing topic page: fill panel, scroll list, chips, previews and Locate.
 *
 * Role: mounts a TBD_PanelFill (title, icon, badge chips) with a TBD_ScrollList body that a subclass
 * fills from TBD_BriefingCatalog with Common primitives; pages that are not one panel override
 * `UsesPanel()` and mount into the dock.  Position: TBD_BriefingNav.CreatePage creates a subclass;
 * TBD_BriefingScreen builds and destroys it; Locate buttons pan through TBD_BriefingMapLauncher.
 * State: the page's widgets, previews and Locate buttons on the client, owned by the screen.
 * Invariants: every 3D preview it attaches is destroyed in `Destroy` before its widget is cleared;
 * every Locate handler is unbound in `Destroy`; the screen reference is weak.
 */

//! One Briefing topic page; subclasses supply the title, icon, badges and content.
class TBD_BriefingPage : Managed
{
	protected TBD_BriefingScreen m_Screen; //!< weak -- the screen owns the page
	protected TBD_BriefingCatalog m_Catalog; //!< the data the page draws
	protected TBD_LobbyCatalog m_Lobby; //!< the lobby data, for ORBAT
	protected Widget m_wRoot; //!< the page root widget
	protected TBD_PanelComponent m_Panel; //!< the fill panel; null when UsesPanel() is false
	protected ref TBD_ScrollList m_List; //!< the panel's scroll list
	protected ref array<ref TBD_KitPreviewComponent> m_aPreviews; //!< 3D previews to destroy with the page
	protected ref array<TBD_UIButton> m_aLocateButtons; //!< Locate buttons, parallel to m_aLocateX and m_aLocateZ
	protected ref array<float> m_aLocateX; //!< world X per Locate button, metres
	protected ref array<float> m_aLocateZ; //!< world Z per Locate button, metres
	protected int m_iGround; //!< ARGB ground colour under the content

	static const int CELL_HEIGHT = 46; //!< stat cell height, pixels

	//! Build the page into `dock`: the fill panel and scroll list, or the raw dock when
	//! `UsesPanel()` is false, then `Fill`.
	//! @param screen the owning screen
	//! @param dock the page column
	//! @param catalog the briefing data
	//! @param lobby the lobby data
	//! @return false when the dock, catalog, layout, panel or list is missing
	bool Build(TBD_BriefingScreen screen, Widget dock, TBD_BriefingCatalog catalog, TBD_LobbyCatalog lobby)
	{
		m_Screen = screen;
		m_Catalog = catalog;
		m_Lobby = lobby;
		m_aPreviews = {};
		m_aLocateButtons = {};
		m_aLocateX = {};
		m_aLocateZ = {};
		if (!dock || !catalog)
			return false;

		if (!UsesPanel())
		{
			m_iGround = TBD_UITheme.Ground();
			Fill(dock);
			return true;
		}

		m_wRoot = TBD_UILayouts.Create(TBD_UILayouts.PANEL_FILL, dock);
		if (!m_wRoot)
			return false;

		m_Panel = TBD_PanelComponent.Cast(m_wRoot.FindHandler(TBD_PanelComponent));
		if (!m_Panel)
			return false;

		m_Panel.SetTitle(Title());
		m_Panel.SetIcon(Icon());
		m_Panel.SetIconTint(IconTint());
		m_iGround = m_Panel.GetGround();
		AddBadges(m_Panel.GetBadgeDock());

		m_List = TBD_ScrollList.Mount(m_Panel.GetBodyDock(), m_iGround, 12);
		if (!m_List)
			return false;

		Fill(m_List.GetContent());
		m_List.ResetScroll();
		return true;
	}

	//! Destroy the previews, unbind the Locate buttons and the list, and drop every reference.
	void Destroy()
	{
		if (m_aPreviews)
		{
			foreach (TBD_KitPreviewComponent preview : m_aPreviews)
			{
				if (preview)
					preview.Destroy();
			}
			m_aPreviews.Clear();
		}

		if (m_aLocateButtons)
		{
			foreach (TBD_UIButton button : m_aLocateButtons)
			{
				if (button)
					button.GetOnActivate().Remove(OnLocate);
			}
			m_aLocateButtons.Clear();
		}

		if (m_List)
			m_List.Destroy();

		m_List = null;
		m_Panel = null;
		m_wRoot = null;
		m_Screen = null;
		m_Catalog = null;
		m_Lobby = null;
	}


	//! @return the panel title
	string Title()  { return "Page"; }
	//! @return the header icon key; empty for none
	string Icon()   { return ""; }
	//! @return the header icon ink; faction pages return their side's ink
	int IconTint() { return TBD_UITheme.PRIMARY_CONTAINER; }
	//! @return true when the page lives in the fill panel; false to mount into the raw dock
	bool UsesPanel() { return true; }
	//! Add chips after the title, such as the faction role and counts.
	//! @param badgeDock the header chip dock
	void AddBadges(Widget badgeDock) {}
	//! Fill the page.
	//! @param content the scroll list's column, or the raw dock when UsesPanel() is false
	void Fill(Widget content) {}


	//! Mount one header chip.
	//! @param dock the header chip dock
	//! @param text the chip text
	//! @param tint the chip tint
	//! @param pill round the chip ends
	//! @return the chip, or null when it could not mount
	protected TBD_ChipComponent AddBadge(Widget dock, string text, TBD_EUITint tint, bool pill = true)
	{
		int headerGround = TBD_UITheme.Over(TBD_UITheme.PANEL_HEADER_FILL, m_iGround);
		TBD_ChipComponent chip = TBD_ChipComponent.Mount(dock, text, tint, headerGround);
		if (chip)
		{
			chip.SetPill(pill);
			AlignableSlot.SetPadding(chip.GetRootWidget(), 8, 0, 0, 0);
		}

		return chip;
	}

	//! Attach a doll or vehicle preview to a mounted preview box (`Preview`, `Label`) and track it.
	//! @param box the preview box
	//! @return the preview, or null when the box is missing or the attach fails
	protected TBD_KitPreviewComponent AttachPreview(Widget box)
	{
		if (!box)
			return null;

		TBD_KitPreviewComponent preview = TBD_KitPreviewComponent.Attach(box.FindAnyWidget("Preview"), box.FindAnyWidget("Label"));
		if (preview)
			m_aPreviews.Insert(preview);

		return preview;
	}

	//! Paint a preview box (`PreviewBorder`/`PreviewBG` or `Border`/`Background`, `GridImage`, `Label`).
	//! @param box the preview box; null does nothing
	//! @param ground the ARGB ground colour under it
	protected void PaintPreviewBox(Widget box, int ground)
	{
		if (!box)
			return;

		Widget border = box.FindAnyWidget("PreviewBorder");
		if (!border)
			border = box.FindAnyWidget("Border");
		Widget background = box.FindAnyWidget("PreviewBG");
		if (!background)
			background = box.FindAnyWidget("Background");

		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_TAG);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_TAG - 1);
		TBD_UITheme.PaintOver(border, TBD_UITheme.KIT_WEAPON_BORDER, ground);
		TBD_UITheme.PaintOver(background, TBD_UITheme.KIT_WEAPON_FILL, ground);
		TBD_UITheme.Paint(box.FindAnyWidget("Label"), TBD_UITheme.DIM_INK);

		ImageWidget grid = ImageWidget.Cast(box.FindAnyWidget("GridImage"));
		if (TBD_UILayouts.LoadTexture(grid, TBD_UILayouts.HERO_TOPO))
			TBD_UITheme.PaintAlpha(grid, 0x3338BDF8);
	}

	//! Mount a quiet Locate button into `dock` that pans the map to (x, z).
	//! @param dock the dock to mount into
	//! @param x world X, metres
	//! @param z world Z, metres
	//! @param label the button text
	//! @return the button, or null when it could not mount
	protected TBD_UIButton AddLocate(Widget dock, float x, float z, string label = "Locate")
	{
		if (!dock)
			return null;

		Widget w = TBD_UILayouts.Create(TBD_UILayouts.BUTTON, dock);
		if (!w)
			return null;

		TBD_UIButton button = TBD_UIButton.Cast(w.FindHandler(TBD_UIButton));
		if (!button)
			return null;

		button.SetLabel(label);
		button.SetPrimary(false);
		button.GetOnActivate().Insert(OnLocate);
		m_aLocateButtons.Insert(button);
		m_aLocateX.Insert(x);
		m_aLocateZ.Insert(z);
		return button;
	}

	//! Pan the map to the activated button's position through the screen's map launcher.
	//! @param button the activated Locate button
	protected void OnLocate(TBD_UIButton button)
	{
		int index = m_aLocateButtons.Find(button);
		if (index < 0 || !m_Screen)
			return;

		TBD_BriefingMapLauncher launcher = m_Screen.GetMapLauncher();
		if (launcher)
			launcher.LocateOnMap(m_aLocateX[index], m_aLocateZ[index]);
	}

	//! Mount one key-value row with a mono value.
	//! @param parent the column to mount into
	//! @param key the key text
	//! @param value the value text
	//! @param ground the ARGB ground colour under the row
	//! @param valueTint the value tint
	//! @return the row, or null when it could not mount
	protected TBD_KeyValueRowComponent AddRow(Widget parent, string key, string value, int ground, TBD_EUITint valueTint = TBD_EUITint.NEUTRAL)
	{
		TBD_KeyValueRowComponent row = TBD_KeyValueRowComponent.Mount(parent);
		if (!row)
			return null;

		row.SetGround(ground);
		row.Set(key, value, valueTint);
		return row;
	}

	//! Mount a grid of `STAT_CELL`s, `columns` per row; an empty list mounts nothing.
	//! @param parent the column to mount into
	//! @param entries the cells
	//! @param columns cells per row: 2, 3, else 4
	//! @param ground the ARGB ground colour under the grid
	//! @param countOnly draw `label  x<count>` cells instead of label and value
	protected void AddCellGrid(Widget parent, array<ref TBD_KitEntry> entries, int columns, int ground, bool countOnly)
	{
		if (!parent || !entries || entries.IsEmpty())
			return;

		ResourceName rowLayout = TBD_UILayouts.COLUMNS_4;
		if (columns == 2)
			rowLayout = TBD_UILayouts.COLUMNS_2;
		else if (columns == 3)
			rowLayout = TBD_UILayouts.COLUMNS_3;

		Widget row;
		int inRow = columns;
		foreach (TBD_KitEntry entry : entries)
		{
			if (inRow >= columns)
			{
				row = TBD_UILayouts.CreateStretched(rowLayout, parent);
				inRow = 0;
			}

			if (!row)
				return;

			Widget column = row.FindAnyWidget(ColumnName(inRow));
			inRow++;
			AddCell(column, entry, ground, countOnly);
		}
	}

	//! @param index the column index
	//! @return `ColumnA` to `ColumnC` for 0 to 2, else `ColumnD`
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

	//! Mount one `STAT_CELL`: label and value, plus an amber count when positive; a non-neutral
	//! `entry.m_eTint` tints the value.
	//! @param column the column to mount into; null does nothing
	//! @param entry the cell data; null does nothing
	//! @param ground the ARGB ground colour under the cell
	//! @param countOnly draw the label alone, in surface ink
	protected void AddCell(Widget column, TBD_KitEntry entry, int ground, bool countOnly)
	{
		if (!column || !entry)
			return;

		Widget cell = TBD_UILayouts.CreateStretched(TBD_UILayouts.STAT_CELL, column);
		if (!cell)
			return;

		Widget border = cell.FindAnyWidget("Border");
		Widget background = cell.FindAnyWidget("Background");
		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_TAG);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_TAG - 1);
		TBD_UITheme.PaintOver(border, TBD_UITheme.KIT_CELL_BORDER, ground);
		TBD_UITheme.PaintOver(background, TBD_UITheme.KIT_CELL_FILL, ground);

		TextWidget label = TextWidget.Cast(cell.FindAnyWidget("Label"));
		TextWidget value = TextWidget.Cast(cell.FindAnyWidget("Value"));
		TextWidget count = TextWidget.Cast(cell.FindAnyWidget("Count"));

		if (countOnly)
		{
			TBD_UITheme.Write(label, entry.m_sLabel);
			TBD_UITheme.Paint(label, TBD_UITheme.ON_SURFACE);
			TBD_UITheme.Show(value, false);
		}
		else
		{
			string shown = entry.m_sLabel;
			shown.ToUpper();
			TBD_UITheme.Write(label, shown);
			TBD_UITheme.Paint(label, TBD_UITheme.MUTED_INK);
			TBD_UITheme.Write(value, entry.m_sValue);
			TBD_UITheme.Show(value, true);
			if (entry.m_eTint == TBD_EUITint.NEUTRAL)
				TBD_UITheme.Paint(value, TBD_UITheme.ON_SURFACE);
			else
				TBD_UITheme.Paint(value, TBD_UITintColours.ChipInk(entry.m_eTint));
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
}
