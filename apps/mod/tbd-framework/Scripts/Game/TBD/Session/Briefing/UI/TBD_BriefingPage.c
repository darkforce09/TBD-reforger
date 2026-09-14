//! Briefing rebuild (2026-09-14) — one topic page of the Briefing screen.
//!
//! A page is a `TBD_PanelFill` (title · icon · badge chips) whose body is a `TBD_ScrollList` the
//! subclass fills from `TBD_BriefingCatalog` with Common primitives (`TBD_Section`,
//! `TBD_NumberedCard`, `TBD_KeyValueRow`, `TBD_Caption`, `STAT_CELL` grids). Pages that are not a
//! single panel (ORBAT) override `UsesPanel()` and mount their own layout into the dock.
//!
//! Every 3D preview a page attaches is tracked here and destroyed with the page (the Pass-6
//! rule: a `TBD_KitPreviewComponent` must be destroyed before its widget is cleared). Locate
//! buttons pan the map through `TBD_BriefingScreen.LocateOnMap`.
class TBD_BriefingPage : Managed
{
	protected TBD_BriefingScreen m_Screen; //!< weak — the screen owns the page
	protected TBD_BriefingCatalog m_Catalog;
	protected TBD_LobbyCatalog m_Lobby;
	protected Widget m_wRoot;
	protected TBD_PanelComponent m_Panel;
	protected ref TBD_ScrollList m_List;
	protected ref array<ref TBD_KitPreviewComponent> m_aPreviews;
	protected ref array<TBD_UIButton> m_aLocateButtons;
	protected ref array<float> m_aLocateX;
	protected ref array<float> m_aLocateZ;
	protected int m_iGround;

	static const int CELL_HEIGHT = 46;

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	// ── Subclass hooks ──────────────────────────────────────────────────────────────────────

	string Title()  { return "Page"; }
	string Icon()   { return ""; }
	//! Header icon ink; faction pages return their side's ink.
	int IconTint() { return TBD_UITheme.PRIMARY_CONTAINER; }
	bool UsesPanel() { return true; }
	//! Chips after the title (faction role, counts).
	void AddBadges(Widget badgeDock) {}
	//! Fill `content` (the scroll list's column, or the raw dock when UsesPanel() is false).
	void Fill(Widget content) {}

	// ── Helpers for subclasses ──────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	//! Attach a doll / vehicle preview to a mounted preview box (`Preview` + `Label`), tracked.
	protected TBD_KitPreviewComponent AttachPreview(Widget box)
	{
		if (!box)
			return null;

		TBD_KitPreviewComponent preview = TBD_KitPreviewComponent.Attach(box.FindAnyWidget("Preview"), box.FindAnyWidget("Label"));
		if (preview)
			m_aPreviews.Insert(preview);

		return preview;
	}

	//------------------------------------------------------------------------------------------------
	//! Paint a preview box (`PreviewBorder`/`PreviewBG` or `Border`/`Background`, `GridImage`, `Label`).
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

	//------------------------------------------------------------------------------------------------
	//! A quiet "Locate" button that pans the map to (x, z). Mounted into `dock`.
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

	//------------------------------------------------------------------------------------------------
	protected void OnLocate(TBD_UIButton button)
	{
		int index = m_aLocateButtons.Find(button);
		if (index < 0 || !m_Screen)
			return;

		m_Screen.LocateOnMap(m_aLocateX[index], m_aLocateZ[index]);
	}

	//------------------------------------------------------------------------------------------------
	//! One key / value row over `ground`, mono value.
	protected TBD_KeyValueRowComponent AddRow(Widget parent, string key, string value, int ground, TBD_EUITint valueTint = TBD_EUITint.NEUTRAL)
	{
		TBD_KeyValueRowComponent row = TBD_KeyValueRowComponent.Mount(parent);
		if (!row)
			return null;

		row.SetGround(ground);
		row.Set(key, value, valueTint);
		return row;
	}

	//------------------------------------------------------------------------------------------------
	//! Grid of `STAT_CELL`s, `columns` per row. `countOnly` = "Bandages  x4" cells.
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
	//! One `STAT_CELL`: label / value (+ amber count). `entry.m_eTint` SUCCESS / WARNING tints the value.
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
				TBD_UITheme.Paint(value, TBD_UITheme.ChipInk(entry.m_eTint));
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
