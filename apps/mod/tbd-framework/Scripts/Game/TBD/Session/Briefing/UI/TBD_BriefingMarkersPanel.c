/**
 * @file TBD_BriefingMarkersPanel.c
 * @brief The Briefing screen's Markers mode panel: a plan dropdown and a Load Plan button.
 *
 * Role: builds `markers_panel` from TBD_BriefingCatalog's plans; Load Plan logs the chosen plan id.
 * Position: TBD_BriefingScreen builds it in CenterDock in MARKERS mode and destroys it on mode change.
 * State: the panel widgets, dropdown and button on the client, owned by the screen.
 * Invariants: no plan store exists, so Load Plan changes nothing; the panel shares CenterDock with the topic
 * navigation and removes its own widget.
 */

//! The Markers mode panel.
class TBD_BriefingMarkersPanel : Managed
{
	protected Widget m_wRoot; //!< the panel root
	protected TBD_DropdownComponent m_Plans; //!< the plan dropdown; item tags are plan indices
	protected TBD_UIButton m_Load; //!< the Load Plan button
	protected TBD_BriefingCatalog m_Catalog; //!< the plans source

	//! Build the panel into `dock`: caption, plan dropdown (first plan selected) and Load Plan.
	//! @param dock the dock to build into
	//! @param catalog the plans source
	//! @param overlayHost the host for the dropdown's open menu
	//! @return false when the dock, catalog or layout is missing
	bool Build(Widget dock, TBD_BriefingCatalog catalog, Widget overlayHost)
	{
		m_Catalog = catalog;
		if (!dock || !catalog)
			return false;

		m_wRoot = TBD_UILayouts.Create(TBD_UILayouts.BRIEFING_MARKERS_PANEL, dock);
		if (!m_wRoot)
			return false;

		Widget border = m_wRoot.FindAnyWidget("PanelBorder");
		Widget background = m_wRoot.FindAnyWidget("PanelBG");
		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UITheme.PaintOver(border, TBD_UITheme.PanelBorder(TBD_EUITint.NEUTRAL), TBD_UITheme.Ground());
		TBD_UITheme.PaintOver(background, TBD_UITheme.PANEL_FILL, TBD_UITheme.Ground());
		int ground = TBD_UITheme.PanelGround();

		array<ref TBD_PlanInfo> plans = catalog.GetPlans();
		TBD_Caption.Mount(m_wRoot.FindAnyWidget("CaptionDock"), "Tactical Plan", string.Format("%1 Available", plans.Count()));

		string first = "No plans";
		if (!plans.IsEmpty())
			first = plans[0].m_sLabel;

		m_Plans = TBD_DropdownComponent.Mount(m_wRoot.FindAnyWidget("PlanDropdownDock"), overlayHost, first, false);
		if (m_Plans)
		{
			m_Plans.SetGround(ground);
			m_Plans.SetMenuTitle("Tactical Plan");
			array<ref TBD_DropdownItem> items = {};
			foreach (int i, TBD_PlanInfo plan : plans)
			{
				items.Insert(new TBD_DropdownItem(plan.m_sLabel, i));
			}
			m_Plans.SetItems(items);
			if (!plans.IsEmpty())
				m_Plans.SetSelectedTag(0);
		}

		Widget buttonRoot = TBD_UILayouts.CreateStretched(TBD_UILayouts.BUTTON, m_wRoot.FindAnyWidget("LoadButtonDock"));
		if (buttonRoot)
			m_Load = TBD_UIButton.Cast(buttonRoot.FindHandler(TBD_UIButton));

		if (m_Load)
		{
			m_Load.SetGround(ground);
			m_Load.SetLabel("Load Plan");
			m_Load.SetPrimary(true);
			m_Load.GetOnActivate().Insert(OnLoad);
		}

		return true;
	}

	//! Unbind the button, close the dropdown and remove the panel widget.
	void Destroy()
	{
		if (m_Load)
			m_Load.GetOnActivate().Remove(OnLoad);

		if (m_Plans)
			m_Plans.Close();

		// The panel shares CenterDock with the topic nav, so it removes its own widget.
		if (m_wRoot)
			m_wRoot.RemoveFromHierarchy();

		m_Load = null;
		m_Plans = null;
		m_wRoot = null;
		m_Catalog = null;
	}

	//! Log the selected plan id, or `none`.
	//! @param button the Load Plan button
	protected void OnLoad(TBD_UIButton button)
	{
		string id = "none";
		if (m_Plans && m_Catalog)
		{
			int tag = m_Plans.GetSelectedTag();
			array<ref TBD_PlanInfo> plans = m_Catalog.GetPlans();
			if (tag >= 0 && tag < plans.Count())
				id = plans[tag].m_sId;
		}

		Print(string.Format("[TBD][briefing] load plan %1 (no-op: no plan store yet)", id));
	}
}
