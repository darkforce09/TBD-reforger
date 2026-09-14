//! Briefing rebuild (2026-09-14) — `markers_panel`: a plan dropdown and a Load Plan button.
//! There is no plan store yet (operator word), so Load Plan logs the intent and nothing else.
class TBD_BriefingMarkersPanel : Managed
{
	protected Widget m_wRoot;
	protected TBD_DropdownComponent m_Plans;
	protected TBD_UIButton m_Load;
	protected TBD_BriefingCatalog m_Catalog;

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
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
