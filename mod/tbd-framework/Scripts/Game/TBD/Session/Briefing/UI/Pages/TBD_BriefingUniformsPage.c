/**
 * @file TBD_BriefingUniformsPage.c
 * @brief The Uniforms page for either side: one card per faction component.
 *
 * Role: draws a card per uniform with a 3D doll wearing the rifleman prefab, its weapon chips
 * and camouflage name.  Position: TBD_BriefingNav.CreatePage creates it; TBD_BriefingScreen builds it into the page column.
 * State: which side it shows, fixed at construction.  Invariants: the enemy page is the friendly
 * builder with the enemy border tone; previews are destroyed with the page.
 */

//! `visual_pid_uniforms_panel`: one card per faction component -- the doll wearing that
//! component's rifleman prefab, its weapon chips and camo name.
class TBD_BriefingUniformsPage : TBD_BriefingPage
{
	protected bool m_bFriendly; //!< true for the reader's side

	//! @param friendly true for the reader's side
	void TBD_BriefingUniformsPage(bool friendly)
	{
		m_bFriendly = friendly;
	}

	//! @return the panel title
	override string Title()
	{
		if (m_bFriendly)
			return "Friendly Uniforms";

		return "Enemy Uniforms";
	}

	//! @return the header icon key
	override string Icon() { return "checkroom"; }

	//! @return the side's row ink, or the primary container ink without a faction
	override int IconTint()
	{
		TBD_BriefingFaction faction = m_Catalog.GetFaction(m_bFriendly);
		if (!faction)
			return TBD_UITheme.PRIMARY_CONTAINER;

		return TBD_UITintColours.FactionRowInk(faction.m_eTint);
	}

	//! Add the side's role chip, and on the Assets page the vehicle count.
	//! @param badgeDock the header chip dock
	override void AddBadges(Widget badgeDock)
	{
		TBD_BriefingFaction faction = m_Catalog.GetFaction(m_bFriendly);
		if (faction)
			AddBadge(badgeDock, faction.m_sRoleLabel, faction.m_eTint);
	}

	//! Fill the page with one card per uniform of the side.
	//! @param content the scroll list column
	override void Fill(Widget content)
	{
		TBD_BriefingFaction faction = m_Catalog.GetFaction(m_bFriendly);
		int borderTone = TBD_UITheme.KIT_CARD_BORDER;
		if (faction && !m_bFriendly)
			borderTone = TBD_UITintColours.FactionRowBorder(faction.m_eTint);

		foreach (TBD_UniformInfo uniform : m_Catalog.GetUniforms(m_bFriendly))
		{
			Widget card = TBD_UILayouts.CreateStretched(TBD_UILayouts.BRIEFING_UNIFORM_CARD, content);
			if (!card)
				continue;

			Widget border = card.FindAnyWidget("Border");
			Widget background = card.FindAnyWidget("Background");
			TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_ROW);
			TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_ROW - 1);
			TBD_UITheme.PaintOver(border, borderTone, m_iGround);
			TBD_UITheme.PaintOver(background, TBD_UITheme.KIT_CARD_FILL, m_iGround);
			int cardGround = TBD_UITheme.Over(TBD_UITheme.KIT_CARD_FILL, m_iGround);

			TextWidget name = TextWidget.Cast(card.FindAnyWidget("Name"));
			TBD_UITheme.Write(name, uniform.m_sName);
			TBD_UITheme.Paint(name, TBD_UITheme.BRIGHT_INK);
			TBD_UITheme.PaintOver(card.FindAnyWidget("HeaderRule"), TBD_UITheme.KIT_CARD_BORDER, cardGround);

			PaintPreviewBox(card, cardGround);
			TBD_KitPreviewComponent preview = AttachPreview(card);
			if (preview)
				preview.ShowPrefab(uniform.Prefab(), null, uniform.m_sName);

			Widget chips = card.FindAnyWidget("ChipsDock");
			foreach (string chipText : uniform.m_aChips)
			{
				TBD_ChipComponent chip = TBD_ChipComponent.Mount(chips, chipText, TBD_EUITint.NEUTRAL, cardGround);
				if (chip)
					AlignableSlot.SetPadding(chip.GetRootWidget(), 0, 0, 6, 0);
			}

			TextWidget camo = TextWidget.Cast(card.FindAnyWidget("CamoLabel"));
			TBD_UITheme.Write(camo, uniform.m_sCamo);
			TBD_UITheme.Paint(camo, TBD_UITheme.MUTED_INK);
		}
	}
}
