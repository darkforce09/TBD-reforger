//! Briefing rebuild (2026-09-14) — Assets and Uniforms pages, one builder each for both sides:
//! the enemy page is the friendly page painted in the enemy tint (operator word), minus Locate.

//! `friendly_vehicles_panel`: per vehicle type a collapsible group (name · count) holding a
//! collapsed "Vehicle Info" (3D render + specs) and one collapsible row per vehicle (ammunition,
//! inventory grids, Locate on the friendly side).
class TBD_BriefingAssetsPage : TBD_BriefingPage
{
	protected bool m_bFriendly;

	void TBD_BriefingAssetsPage(bool friendly)
	{
		m_bFriendly = friendly;
	}

	override string Title()
	{
		if (m_bFriendly)
			return "Friendly Assets";

		return "Enemy Assets";
	}

	override string Icon() { return "directions_car"; }

	override int IconTint() { return SideInk(); }

	//------------------------------------------------------------------------------------------------
	protected int SideInk()
	{
		TBD_BriefingFaction faction = m_Catalog.GetFaction(m_bFriendly);
		if (!faction)
			return TBD_UITheme.PRIMARY_CONTAINER;

		return TBD_UITheme.FactionRowInk(faction.m_eTint);
	}

	override void AddBadges(Widget badgeDock)
	{
		TBD_BriefingFaction faction = m_Catalog.GetFaction(m_bFriendly);
		if (faction)
			AddBadge(badgeDock, faction.m_sRoleLabel, faction.m_eTint);

		AddBadge(badgeDock, m_Catalog.CountAssets(m_bFriendly).ToString(), CountTint(), false);
	}

	//------------------------------------------------------------------------------------------------
	protected TBD_EUITint CountTint()
	{
		if (m_bFriendly)
			return TBD_EUITint.PRIMARY;

		return TBD_EUITint.OPFOR;
	}

	//------------------------------------------------------------------------------------------------
	override void Fill(Widget content)
	{
		TBD_BriefingFaction faction = m_Catalog.GetFaction(m_bFriendly);
		TBD_EUITint tint = TBD_EUITint.NEUTRAL;
		if (faction)
			tint = faction.m_eTint;

		foreach (TBD_AssetTypeInfo type : m_Catalog.GetAssets(m_bFriendly))
		{
			TBD_SectionComponent group = TBD_SectionComponent.Mount(content, type.m_sName, m_iGround);
			if (!group)
				continue;

			group.SetIcon("directions_car", SideInk());
			group.SetBadge(type.m_iCount.ToString(), CountTint());
			if (!m_bFriendly)
				group.SetTint(tint);
			group.SetExpanded(type.m_bExpanded);

			Widget body = group.GetBody();
			int bodyGround = group.GetBodyGround();

			if (type.HasInfo())
				AddVehicleInfo(body, type, bodyGround);

			foreach (TBD_AssetInstanceInfo instance : type.m_aInstances)
			{
				AddInstance(body, type, instance, bodyGround, tint);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void AddVehicleInfo(Widget parent, TBD_AssetTypeInfo type, int ground)
	{
		TBD_SectionComponent info = TBD_SectionComponent.Mount(parent, "Vehicle Info", ground);
		if (!info)
			return;

		info.SetExpanded(false);
		Widget body = info.GetBody();
		int infoGround = info.GetBodyGround();

		if (!type.m_sPrefab.IsEmpty())
		{
			Widget box = TBD_UILayouts.CreateStretched(TBD_UILayouts.BRIEFING_ASSET_PREVIEW, body);
			PaintPreviewBox(box, infoGround);
			TBD_KitPreviewComponent preview = AttachPreview(box);
			if (preview)
				preview.ShowVehicle(type.m_sPrefab);
		}

		if (!type.m_aWeapons.IsEmpty())
		{
			TBD_Caption.Mount(body, "Vehicle Weapons");
			foreach (string weapon : type.m_aWeapons)
			{
				AddRow(body, weapon, "", infoGround);
			}
		}

		TBD_Caption.Mount(body, "Mobility");
		if (!type.m_sSpeedRoad.IsEmpty())
			AddRow(body, "Speed (Road / Land)", type.m_sSpeedRoad, infoGround);
		if (!type.m_sAmphibious.IsEmpty())
			AddRow(body, "Amphibious", type.m_sAmphibious, infoGround);
		if (!type.m_sSpeedWater.IsEmpty())
			AddRow(body, "Water Speed (Amphibious)", type.m_sSpeedWater, infoGround);

		if (!type.m_sCrew.IsEmpty())
		{
			TBD_Caption.Mount(body, "Crew & Capacity");
			AddRow(body, "Crew", type.m_sCrew, infoGround);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void AddInstance(Widget parent, TBD_AssetTypeInfo type, TBD_AssetInstanceInfo instance, int ground, TBD_EUITint tint)
	{
		TBD_SectionComponent row = TBD_SectionComponent.Mount(parent, type.m_sName, ground);
		if (!row)
			return;

		row.SetBadge(instance.m_sCallsign, tint);
		row.SetExpanded(false);
		if (m_bFriendly && (instance.m_fX != 0 || instance.m_fZ != 0))
			AddLocate(row.GetActionDock(), instance.m_fX, instance.m_fZ, "Locate Vehicle");

		Widget body = row.GetBody();
		int rowGround = row.GetBodyGround();

		if (!instance.m_aAmmo.IsEmpty())
		{
			TBD_Caption.Mount(body, "Vehicle Ammunition");
			foreach (TBD_KitEntry ammo : instance.m_aAmmo)
			{
				AddRow(body, ammo.m_sLabel, ammo.m_sValue, rowGround);
			}
		}

		TBD_Caption.Mount(body, "Inventory");
		AddInventory(body, "Ammunition", instance.m_aInvAmmo, rowGround);
		AddInventory(body, "Weapons", instance.m_aInvWeapons, rowGround);
		AddInventory(body, "Grenades", instance.m_aInvGrenades, rowGround);
		AddInventory(body, "Medical", instance.m_aInvMedical, rowGround);
		AddInventory(body, "Miscellaneous", instance.m_aInvMisc, rowGround);
	}

	//------------------------------------------------------------------------------------------------
	protected void AddInventory(Widget parent, string title, array<ref TBD_KitEntry> entries, int ground)
	{
		if (!entries || entries.IsEmpty())
			return;

		TBD_Caption.Mount(parent, title);
		AddCellGrid(parent, entries, 3, ground, true);
	}
}

//! `visual_pid_uniforms_panel`: one card per faction component — the doll wearing that
//! component's rifleman prefab, its weapon chips and camo name.
class TBD_BriefingUniformsPage : TBD_BriefingPage
{
	protected bool m_bFriendly;

	void TBD_BriefingUniformsPage(bool friendly)
	{
		m_bFriendly = friendly;
	}

	override string Title()
	{
		if (m_bFriendly)
			return "Friendly Uniforms";

		return "Enemy Uniforms";
	}

	override string Icon() { return "checkroom"; }

	override int IconTint()
	{
		TBD_BriefingFaction faction = m_Catalog.GetFaction(m_bFriendly);
		if (!faction)
			return TBD_UITheme.PRIMARY_CONTAINER;

		return TBD_UITheme.FactionRowInk(faction.m_eTint);
	}

	override void AddBadges(Widget badgeDock)
	{
		TBD_BriefingFaction faction = m_Catalog.GetFaction(m_bFriendly);
		if (faction)
			AddBadge(badgeDock, faction.m_sRoleLabel, faction.m_eTint);
	}

	//------------------------------------------------------------------------------------------------
	override void Fill(Widget content)
	{
		TBD_BriefingFaction faction = m_Catalog.GetFaction(m_bFriendly);
		int borderTone = TBD_UITheme.KIT_CARD_BORDER;
		if (faction && !m_bFriendly)
			borderTone = TBD_UITheme.FactionRowBorder(faction.m_eTint);

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
