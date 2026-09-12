//! T-139 - lobby kit preview. Icon grid of the selected seat's 13 gear fields.
//!
//! Hosted by TBD_LobbyScreen beside the slot list. No network: Refresh reads a
//! TBD_MissionSlotStruct the lobby already has locally (TBD_MissionLoader on this
//! machine). Empty gear fields are skipped. Missing registry icons become a letter
//! glyph - the cell widget is never null.
class TBD_LoadoutPreview : ScriptedWidgetComponent
{
	static const ResourceName LAYOUT = TBD_UILayouts.LOADOUT_PREVIEW;

	protected static const int CELL_COUNT = 13;

	protected Widget m_wRoot;
	protected TextWidget m_wTitle;
	protected TextWidget m_wEmpty;
	protected Widget m_wGrid;
	protected ref array<Widget> m_aCells;
	protected ref array<ImageWidget> m_aIcons;
	protected ref array<TextWidget> m_aGlyphs;

	//------------------------------------------------------------------------------------------------
	//! Instantiate under `parent` (the lobby PanelFrame). Returns null on a dead workspace.
	static Widget CreateUnder(Widget parent)
	{
		if (!parent)
			return null;

		Widget created = TBD_UILayouts.Create(LAYOUT, parent);
		if (created)
			return created;

		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace)
			return null;

		return workspace.CreateWidgets(LAYOUT, parent);
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);

		m_wRoot = w;
		m_wTitle = TextWidget.Cast(w.FindAnyWidget("PreviewTitle"));
		m_wEmpty = TextWidget.Cast(w.FindAnyWidget("PreviewEmpty"));
		m_wGrid = w.FindAnyWidget("IconGrid");

		TBD_UITheme.Paint(w.FindAnyWidget("PreviewPanel"), TBD_UITheme.SURFACE_CONTAINER);
		TBD_UITheme.Paint(m_wTitle, TBD_UITheme.ON_SURFACE);
		TBD_UITheme.Paint(m_wEmpty, TBD_UITheme.ON_SURFACE_VARIANT);

		m_aCells = {};
		m_aIcons = {};
		m_aGlyphs = {};

		int i;
		for (i = 0; i < CELL_COUNT; i++)
		{
			string cellName = string.Format("Cell%1", i);
			string iconName = string.Format("Icon%1", i);
			string glyphName = string.Format("Glyph%1", i);
			m_aCells.Insert(w.FindAnyWidget(cellName));
			m_aIcons.Insert(ImageWidget.Cast(w.FindAnyWidget(iconName)));
			m_aGlyphs.Insert(TextWidget.Cast(w.FindAnyWidget(glyphName)));
		}

		TBD_UITheme.Write(m_wTitle, "KIT");
		Refresh(null, -1);
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		if (m_aCells)
			m_aCells.Clear();
		if (m_aIcons)
			m_aIcons.Clear();
		if (m_aGlyphs)
			m_aGlyphs.Clear();

		m_wRoot = null;
		m_wTitle = null;
		m_wEmpty = null;
		m_wGrid = null;

		super.HandlerDeattached(w);
	}

	//------------------------------------------------------------------------------------------------
	//! Rebuild the icon grid from the slot's 13 gear ResourceNames. `slotIndex` is the lobby
	//! row index (or -1 when nothing is selected). Logs once per refresh for the boot checklist.
	void Refresh(TBD_MissionSlotStruct slot, int slotIndex)
	{
		int items = 0;
		TBD_SlotGearStruct gear;
		if (slot && slot.loadout)
			gear = slot.loadout.gear;

		if (gear)
		{
			items = BindGear(gear, items, "Primary", gear.primary);
			items = BindGear(gear, items, "Launcher", gear.launcher);
			items = BindGear(gear, items, "Handgun", gear.handgun);
			items = BindGear(gear, items, "Throwable", gear.throwable);
			items = BindGear(gear, items, "Optic", gear.optic);
			items = BindGear(gear, items, "Magazine", gear.magazine);
			items = BindGear(gear, items, "Uniform", gear.uniform);
			items = BindGear(gear, items, "Vest", gear.vest);
			items = BindGear(gear, items, "Helmet", gear.helmet);
			items = BindGear(gear, items, "Pants", gear.pants);
			items = BindGear(gear, items, "Boots", gear.boots);
			items = BindGear(gear, items, "Gloves", gear.handwear);
			items = BindGear(gear, items, "Backpack", gear.backpack);
		}

		HideFrom(items);

		bool empty = items == 0;
		TBD_UITheme.Show(m_wEmpty, empty);
		TBD_UITheme.Show(m_wGrid, !empty);
		if (empty)
			TBD_UITheme.Write(m_wEmpty, EmptyCaption(slot));

		Print(string.Format("[TBD][LoadoutPreview] slot=%1 items=%2", slotIndex, items), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	protected int BindGear(TBD_SlotGearStruct gear, int items, string label, string resource)
	{
		if (!gear)
			return items;
		if (resource.IsEmpty())
			return items;
		if (items >= CELL_COUNT)
			return items;

		BindCell(items, label, resource);
		return items + 1;
	}

	//------------------------------------------------------------------------------------------------
	protected void BindCell(int index, string label, string resource)
	{
		Widget cell = m_aCells[index];
		TBD_UITheme.Show(cell, true);

		ImageWidget icon = m_aIcons[index];
		TextWidget glyph = m_aGlyphs[index];

		ResourceName iconPath = IconPathFor(resource);
		bool painted = false;
		if (icon && !iconPath.IsEmpty())
			painted = icon.LoadImageTexture(0, iconPath);

		if (painted)
		{
			TBD_UITheme.Paint(icon, TBD_UITheme.ON_SURFACE);
			TBD_UITheme.Show(icon, true);
			TBD_UITheme.Show(glyph, false);
			return;
		}

		TBD_UITheme.Show(icon, false);
		TBD_UITheme.Show(glyph, true);
		TBD_UITheme.Paint(glyph, TBD_UITheme.ON_SURFACE);
		TBD_UITheme.Write(glyph, GlyphFor(label));
	}

	//------------------------------------------------------------------------------------------------
	protected void HideFrom(int used)
	{
		int i;
		for (i = used; i < CELL_COUNT; i++)
		{
			TBD_UITheme.Show(m_aCells[i], false);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected string EmptyCaption(TBD_MissionSlotStruct slot)
	{
		if (!slot)
			return "Hover a seat to preview its kit.";
		return "No gear on this seat.";
	}

	//------------------------------------------------------------------------------------------------
	protected string GlyphFor(string label)
	{
		if (label.IsEmpty())
			return "?";
		return label.Substring(0, 1);
	}

	//------------------------------------------------------------------------------------------------
	//! Registry icon path: UIInfo on the prefab the gear ResourceName already names.
	//! Empty when the prefab has no icon - BindCell then shows the letter glyph.
	protected ResourceName IconPathFor(string resource)
	{
		if (resource.IsEmpty())
			return string.Empty;

		Resource prefab = Resource.Load(resource);
		if (!prefab)
			return string.Empty;
		if (!prefab.IsValid())
			return string.Empty;

		IEntityComponentSource src = SCR_BaseContainerTools.FindComponentSource(prefab, InventoryItemComponent);
		if (!src)
			src = SCR_BaseContainerTools.FindComponentSource(prefab, WeaponComponent);
		if (!src)
			src = SCR_BaseContainerTools.FindComponentSource(prefab, MagazineComponent);
		if (!src)
			return string.Empty;

		ResourceName icon = IconFromContainer(src.GetObject("Attributes"));
		if (!icon.IsEmpty())
			return icon;

		icon = IconFromContainer(src.GetObject("UIInfo"));
		if (!icon.IsEmpty())
			return icon;

		return IconFromContainer(src.GetObject("m_UIInfo"));
	}

	//------------------------------------------------------------------------------------------------
	protected ResourceName IconFromContainer(BaseContainer container)
	{
		if (!container)
			return string.Empty;

		BaseContainer disp = container.GetObject("ItemDisplayName");
		if (disp)
		{
			ResourceName nested = IconFromUiInfo(disp);
			if (!nested.IsEmpty())
				return nested;
		}

		return IconFromUiInfo(container);
	}

	//------------------------------------------------------------------------------------------------
	protected ResourceName IconFromUiInfo(BaseContainer ui)
	{
		if (!ui)
			return string.Empty;

		ResourceName icon;
		if (ui.Get("Icon", icon) && !icon.IsEmpty())
			return icon;
		if (ui.Get("m_Image", icon) && !icon.IsEmpty())
			return icon;

		return string.Empty;
	}
}
