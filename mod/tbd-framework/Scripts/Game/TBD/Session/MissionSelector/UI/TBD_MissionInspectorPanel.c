/**
 * @file TBD_MissionInspectorPanel.c
 * @brief The right column of the Mission Selector: the hero band and the four mission cards.
 *
 * Role: controller for `TBD_MissionInspector.layout`: paints the rounded glass panel and the hero
 * photo, binds title, tag, author and version to the selected mission, and hands the cards to
 * TBD_MissionInspectorCards.  Position: TBD_MissionSelectorScreen builds it on RightDock and calls
 * Show on each selection; data comes from TBD_MissionCatalog; GetOnVersionChanged reports the
 * version pick.
 * State: widget references, the chips, the version dropdown and the cards helper, owned by the
 * screen on the client.  Invariants: Show(null) shows the empty state and hides the hero content;
 * a missing widget is skipped, never fatal.
 *
 *   | PVP Test 1  [PVP]                     [v2.14.99 LATEST v]  |  hero
 *   | by Bohemia Interactive [AUTHOR]                            |
 *   | REQUIRED MODSET & MODS  [TBD CORE COMPETITIVE]  [6 SYNCED] |
 *   | MISSION SUMMARY                                  [SITREP]  |
 *   | ORBAT OVERVIEW                                 [48 SLOTS]  |
 *   | OBJECTIVES                                     [4 ACTIVE]  |
 */

//! Controller of the mission inspector column. Not a widget handler: the screen mounts the layout
//! and hands its root here.
class TBD_MissionInspectorPanel
{
	protected Widget m_wRoot; //!< the mounted inspector layout
	protected TextWidget m_wHeroTitle; //!< `HeroTitle`
	protected Widget m_wHeroTagDock; //!< `HeroTagDock`: the mode tag chip
	protected int m_iHeroGround; //!< composited hero band colour, ground for the tag chip, author chip and version pill
	protected ImageWidget m_wAuthorIcon; //!< `AuthorIcon`
	protected TextWidget m_wAuthorBy; //!< `AuthorBy`
	protected TextWidget m_wAuthorName; //!< `AuthorName`
	protected Widget m_wAuthorChipDock; //!< `AuthorChipDock`
	protected Widget m_wVersionDock; //!< `VersionDock`: the version dropdown
	protected Widget m_wHero; //!< `Hero`
	protected ImageWidget m_wHeroImage; //!< `HeroImage`: terrain satellite band or topo art
	protected ref TBD_UIScrollBar m_ScrollBar; //!< scroll bar of the card column
	protected Widget m_wCardsContent; //!< `CardsContent`: the card column
	protected Widget m_wEmptyState; //!< `EmptyState`, shown when no mission is selected
	protected ref TBD_MissionInspectorCards m_Cards; //!< the four cards; null before Build and after Destroy

	protected TBD_ChipComponent m_TagChip; //!< mode tag chip; mounted on the first Show
	protected TBD_ChipComponent m_AuthorChip; //!< AUTHOR chip
	protected TBD_DropdownComponent m_Version; //!< version dropdown

	protected TBD_MissionCatalog m_Catalog; //!< the catalog in force
	protected TBD_MissionSummary m_Mission; //!< the mission shown, or null

	protected ref ScriptInvoker m_OnVersionChanged; //!< (TBD_MissionInspectorPanel panel, int versionIndex)

	//! Find the inspector widgets, paint the panel and hero, mount the chips, the version dropdown
	//! and the cards, then show the empty state.
	//! @param root a mounted `TBD_MissionInspector.layout`
	//! @param overlayHost where the version dropdown opens its menu
	//! @param catalog the catalog in force
	//! @return false when root is null
	bool Build(Widget root, Widget overlayHost, TBD_MissionCatalog catalog)
	{
		m_Catalog = catalog;
		m_wRoot = root;
		if (!root)
			return false;

		m_wHero = root.FindAnyWidget("Hero");
		m_wHeroTitle = TextWidget.Cast(root.FindAnyWidget("HeroTitle"));
		m_wHeroTagDock = root.FindAnyWidget("HeroTagDock");
		m_wAuthorIcon = ImageWidget.Cast(root.FindAnyWidget("AuthorIcon"));
		m_wAuthorBy = TextWidget.Cast(root.FindAnyWidget("AuthorBy"));
		m_wAuthorName = TextWidget.Cast(root.FindAnyWidget("AuthorName"));
		m_wAuthorChipDock = root.FindAnyWidget("AuthorChipDock");
		m_wVersionDock = root.FindAnyWidget("VersionDock");
		m_wCardsContent = root.FindAnyWidget("CardsContent");
		m_wEmptyState = root.FindAnyWidget("EmptyState");

		// The inspector is a rounded glass panel on the backdrop. The hero is a PHOTO (terrain
		// satellite, or the topo art) under a dim and a bottom fade. A photo cannot be clipped to
		// a round corner, so two layers of inverse-disc masks fake it: inside the hero, r11 quarters
		// painted in the BORDER colour (the ring), and on the root, r12 quarters painted in the
		// BACKDROP colour (outside the panel). Photo inside r11, border r11-r12, backdrop beyond.
		Widget panelBorder = root.FindAnyWidget("PanelBorder");
		Widget panelBG = root.FindAnyWidget("PanelBG");
		TBD_UILayouts.MountRounded(panelBorder, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(panelBG, TBD_UITheme.RADIUS_PANEL - 1);

		int ground = TBD_UITheme.PanelGround();
		int borderColour = TBD_UITheme.Over(TBD_UITintColours.PanelBorder(TBD_EUITint.NEUTRAL), TBD_UITheme.Ground());
		TBD_UITheme.PaintOver(panelBorder, TBD_UITintColours.PanelBorder(TBD_EUITint.NEUTRAL), TBD_UITheme.Ground());
		TBD_UITheme.PaintOver(panelBG, TBD_UITheme.PANEL_FILL, TBD_UITheme.Ground());

		m_wHeroImage = ImageWidget.Cast(root.FindAnyWidget("HeroImage"));
		TBD_UITheme.PaintAlpha(root.FindAnyWidget("HeroDim"), TBD_UITheme.HERO_DIM);
		ImageWidget fade = ImageWidget.Cast(root.FindAnyWidget("HeroFade"));
		if (TBD_UILayouts.LoadTexture(fade, TBD_UILayouts.FADE_DOWN))
			TBD_UITheme.Paint(fade, TBD_UITheme.HERO_FADE);

		MountCornerMask(root, "HeroMaskTLImg", borderColour);
		MountCornerMask(root, "HeroMaskTRImg", borderColour);
		MountCornerMask(root, "PanelMaskTLImg", TBD_UITheme.Ground());
		MountCornerMask(root, "PanelMaskTRImg", TBD_UITheme.Ground());

		// Chips and the version pill sit at the bottom of the hero, where the fade is solid.
		m_iHeroGround = TBD_UITheme.HERO_FADE;

		m_ScrollBar = TBD_UIScrollBar.Mount(root.FindAnyWidget("ScrollBarDock"), ScrollLayoutWidget.Cast(root.FindAnyWidget("Scroll")), m_wCardsContent, ground);
		TBD_UITheme.Paint(m_wHeroTitle, TBD_UITheme.BRIGHT_INK);
		TBD_UITheme.Paint(m_wAuthorBy, TBD_UITheme.MUTED_INK);
		TBD_UITheme.Paint(m_wAuthorName, TBD_UITheme.ON_SURFACE);
		TBD_UITheme.Paint(m_wEmptyState, TBD_UITheme.MUTED_INK);

		if (m_wAuthorIcon)
		{
			TBD_UIIcons.Load(m_wAuthorIcon, "person");
			TBD_UITheme.Paint(m_wAuthorIcon, TBD_UITheme.MUTED_INK);
		}

		m_AuthorChip = TBD_ChipComponent.Mount(m_wAuthorChipDock, "AUTHOR", TBD_EUITint.NEUTRAL, m_iHeroGround);

		m_Version = TBD_DropdownComponent.Mount(m_wVersionDock, overlayHost, "Version", false);
		if (m_Version)
		{
			m_Version.SetMenuTitle("Select mission version");
			m_Version.SetGround(m_iHeroGround);
			m_Version.GetOnChanged().Insert(OnVersionChanged);
			AlignableSlot.SetHorizontalAlign(m_Version.GetRootWidget(), LayoutHorizontalAlign.Right);
		}

		m_Cards = new TBD_MissionInspectorCards(m_wCardsContent);
		Show(null);
		return true;
	}

	//! Load one quarter of the inverse disc into a corner mask, painted in the colour behind the hero
	//! there, which fakes a rounded corner on the photo.
	//! @param root the inspector root
	//! @param name the mask widget
	//! @param opaqueArgb the colour behind that corner
	protected void MountCornerMask(Widget root, string name, int opaqueArgb)
	{
		ImageWidget mask = ImageWidget.Cast(root.FindAnyWidget(name));
		if (TBD_UILayouts.LoadTexture(mask, TBD_UILayouts.CORNER_DISC_INV))
			TBD_UITheme.Paint(mask, opaqueArgb);
	}

	//! Show the terrain's satellite band when it has one, else the topo art at reduced alpha.
	//! @param terrainKey the mission's terrain
	protected void ShowHeroImage(string terrainKey)
	{
		ResourceName texture = TBD_UILayouts.HERO_TOPO;
		bool art = true;

		TBD_TerrainInfo terrain = m_Catalog.GetTerrain(terrainKey);
		if (terrain && !terrain.m_sHeroImage.IsEmpty())
		{
			texture = terrain.m_sHeroImage;
			art = texture == TBD_UILayouts.HERO_TOPO;
		}

		if (!TBD_UILayouts.LoadTexture(m_wHeroImage, texture))
			return;

		if (art)
			TBD_UITheme.PaintAlpha(m_wHeroImage, TBD_UITheme.HERO_ART_INK);
		else
			TBD_UITheme.Paint(m_wHeroImage, TBD_UITheme.BRIGHT_INK);
	}

	//! Release the scroll bar, the version dropdown and the cards before the screen closes.
	void Destroy()
	{
		if (m_ScrollBar)
			m_ScrollBar.Destroy();

		m_ScrollBar = null;
		if (m_Version)
		{
			m_Version.GetOnChanged().Remove(OnVersionChanged);
			m_Version.Close();
		}

		m_Version = null;
		m_wRoot = null;
		m_wCardsContent = null;
		m_Cards = null;
		m_Mission = null;
	}

	//! Rebind the hero and the cards to `mission`.
	//! @param mission the mission to show; null shows the empty state
	void Show(TBD_MissionSummary mission)
	{
		m_Mission = mission;

		bool has = mission != null;
		TBD_UITheme.Show(m_wEmptyState, !has);
		TBD_UITheme.Show(m_wCardsContent, has);
		TBD_UITheme.Show(m_wHeroTitle, has);
		TBD_UITheme.Show(m_wHeroTagDock, has);
		TBD_UITheme.Show(m_wAuthorBy, has);
		TBD_UITheme.Show(m_wAuthorName, has);
		TBD_UITheme.Show(m_wAuthorChipDock, has);
		TBD_UITheme.Show(m_wVersionDock, has);
		TBD_UITheme.Show(m_wAuthorIcon, has);
		TBD_UITheme.Show(m_wHeroImage, has);

		if (!has)
			return;

		TBD_UITheme.Write(m_wHeroTitle, mission.m_sTitle);
		ShowHeroImage(mission.m_sTerrainKey);
		TBD_UITheme.Write(m_wAuthorName, mission.m_sAuthor);

		if (!m_TagChip)
			m_TagChip = TBD_ChipComponent.Mount(m_wHeroTagDock, m_Catalog.ModeLabel(mission.m_sTag), m_Catalog.ModeTint(mission.m_sTag), m_iHeroGround);
		else
			m_TagChip.Set(m_Catalog.ModeLabel(mission.m_sTag), m_Catalog.ModeTint(mission.m_sTag));

		FillVersions(mission);
		if (m_Cards)
			m_Cards.Fill(mission);
	}

	//! @return the index into the mission's version list, -1 when none
	int GetSelectedVersionIndex()
	{
		if (!m_Version)
			return -1;

		return m_Version.GetSelectedTag();
	}

	//! @return the label of the chosen version ("v2.14.99"), empty when none
	string GetSelectedVersionLabel()
	{
		if (!m_Mission)
			return string.Empty;

		int index = GetSelectedVersionIndex();
		if (index < 0 || index >= m_Mission.m_aVersions.Count())
			return string.Empty;

		return m_Mission.m_aVersions[index].m_sLabel;
	}

	//! @return the invoker raised with (TBD_MissionInspectorPanel panel, int versionIndex) on a version pick
	ScriptInvoker GetOnVersionChanged()
	{
		if (!m_OnVersionChanged)
			m_OnVersionChanged = new ScriptInvoker();

		return m_OnVersionChanged;
	}

	//! Fill the version dropdown from the mission and select the first version.
	protected void FillVersions(TBD_MissionSummary mission)
	{
		if (!m_Version)
			return;

		array<ref TBD_DropdownItem> items = {};
		for (int i = 0; i < mission.m_aVersions.Count(); i++)
		{
			TBD_MissionVersion version = mission.m_aVersions[i];
			items.Insert(new TBD_DropdownItem(version.m_sLabel, i, version.m_sBadge));
		}

		m_Version.SetSelectedTag(-1);
		m_Version.SetItems(items);
		if (!items.IsEmpty())
			m_Version.SetSelectedTag(0);
	}

	//! Forward a version pick to GetOnVersionChanged.
	protected void OnVersionChanged(TBD_DropdownComponent dropdown, int tag)
	{
		if (m_OnVersionChanged)
			m_OnVersionChanged.Invoke(this, tag);
	}
}
