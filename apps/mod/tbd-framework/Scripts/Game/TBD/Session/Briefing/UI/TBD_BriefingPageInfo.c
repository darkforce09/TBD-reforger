//! Briefing rebuild (2026-09-14) — the information pages: Objectives, Rules, Background, Parameters.
//! Each is a `TBD_BriefingPage` built from Common primitives; see the base class.

//! `objectives_panel`: time-limit bar, numbered objective cards with a stat grid and Locate.
class TBD_BriefingObjectivesPage : TBD_BriefingPage
{
	override string Title() { return "Mission Objectives"; }
	override string Icon()  { return "target"; }

	override void Fill(Widget content)
	{
		TBD_Caption.Mount(content, "Directives", m_Catalog.GetCaptureSummary());
		TBD_KeyValueRowComponent limit = AddRow(content, "Time Limit", m_Catalog.GetTimeLimit(), m_iGround);
		if (limit)
			limit.SetIcon("timer", TBD_UITheme.MUTED_INK);

		TBD_Caption.Mount(content, "Objectives", string.Format("%1 total", m_Catalog.GetObjectives().Count()));
		foreach (TBD_ObjectiveInfo objective : m_Catalog.GetObjectives())
		{
			TBD_NumberedCardComponent card = TBD_NumberedCardComponent.Mount(content, objective.m_iIndex, objective.m_sTitle, m_iGround);
			if (!card)
				continue;

			card.SetChip(objective.m_sRoleLabel, TBD_EUITint.NEUTRAL);
			int cardGround = card.GetGround();

			array<ref TBD_KitEntry> stats = {};
			stats.Insert(new TBD_KitEntry("Type", objective.m_sType));
			stats.Insert(new TBD_KitEntry("Capture Time", objective.m_sCaptureTime));
			AddCellGrid(card.GetBodyDock(), stats, 2, cardGround, false);

			array<ref TBD_KitEntry> retake = {};
			retake.Insert(new TBD_KitEntry("Retake", objective.m_sRetake, 0, TBD_EUITint.SUCCESS));
			AddCellGrid(card.GetBodyDock(), retake, 1, cardGround, false);

			AddLocate(card.GetFooterDock(), objective.m_fX, objective.m_fZ);
		}
	}
}

//! `rules_panel`: two collapsible groups of numbered rules.
class TBD_BriefingRulesPage : TBD_BriefingPage
{
	override string Title() { return "Rules"; }
	override string Icon()  { return "warning"; }

	override void Fill(Widget content)
	{
		foreach (TBD_RuleGroup group : m_Catalog.GetRuleGroups())
		{
			TBD_SectionComponent section = TBD_SectionComponent.Mount(content, group.m_sTitle, m_iGround);
			if (!section)
				continue;

			section.SetIcon("assignment");
			section.SetExpanded(group.m_bOpen);
			int bodyGround = section.GetBodyGround();
			int number = 1;
			foreach (TBD_RuleInfo rule : group.m_aRules)
			{
				TBD_NumberedCardComponent card = TBD_NumberedCardComponent.Mount(section.GetBody(), number, rule.m_sTitle, bodyGround);
				if (card)
					card.SetBody(rule.m_sBody);
				number++;
			}
		}
	}
}

//! `lore_panel`: the situation, one inset paragraph each.
class TBD_BriefingBackgroundPage : TBD_BriefingPage
{
	override string Title() { return "Background"; }
	override string Icon()  { return "description"; }

	override void Fill(Widget content)
	{
		foreach (string paragraph : m_Catalog.GetLore())
		{
			Widget inset = TBD_UILayouts.CreateStretched(TBD_UILayouts.INSET_TEXT, content);
			if (!inset)
				continue;

			AlignableSlot.SetPadding(inset, 0, 0, 0, 10);
			Widget border = inset.FindAnyWidget("InsetBorder");
			Widget background = inset.FindAnyWidget("InsetBG");
			TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_ROW);
			TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_ROW - 1);
			TBD_UITheme.PaintOver(border, TBD_UITheme.KIT_CARD_BORDER, m_iGround);
			TBD_UITheme.PaintOver(background, TBD_UITheme.INSET_FILL, m_iGround);

			TextWidget body = TextWidget.Cast(inset.FindAnyWidget("Body"));
			TBD_UITheme.Write(body, paragraph);
			TBD_UITheme.Paint(body, TBD_UITheme.ON_SURFACE);
		}
	}
}

//! `parameters_panel`: icon · label · mono value rows.
class TBD_BriefingParametersPage : TBD_BriefingPage
{
	override string Title() { return "Parameters"; }
	override string Icon()  { return "tune"; }

	override void Fill(Widget content)
	{
		foreach (TBD_ParamInfo param : m_Catalog.GetParams())
		{
			TBD_KeyValueRowComponent row = AddRow(content, param.m_sLabel, param.m_sValue, m_iGround, param.m_eTint);
			if (row)
				row.SetIcon(param.m_sIcon, TBD_UITheme.MUTED_INK);
		}
	}
}
