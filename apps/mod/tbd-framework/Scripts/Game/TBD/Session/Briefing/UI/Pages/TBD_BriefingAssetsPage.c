/**
 * @file TBD_BriefingAssetsPage.c
 * @brief The Assets page for either side: vehicle types, their specifications and each vehicle.
 *
 * Role: draws one collapsible group per vehicle type with a collapsed Vehicle Info section (3D
 * preview and specifications) and one collapsible row per vehicle (ammunition, inventory grids).  Position: TBD_BriefingNav.CreatePage creates it; TBD_BriefingScreen builds it into the page column.
 * State: which side it shows, fixed at construction.  Invariants: the enemy page is the friendly
 * builder in the enemy tint, without Locate; Locate appears only for a vehicle with a position.
 */

//! `friendly_vehicles_panel`: per vehicle type a collapsible group (name - count) holding a
//! collapsed "Vehicle Info" (3D render + specs) and one collapsible row per vehicle (ammunition,
//! inventory grids, Locate on the friendly side).
class TBD_BriefingAssetsPage : TBD_BriefingPage
{
	protected bool m_bFriendly; //!< true for the reader's side

	//! @param friendly true for the reader's side
	void TBD_BriefingAssetsPage(bool friendly)
	{
		m_bFriendly = friendly;
	}

	//! @return the panel title
	override string Title()
	{
		if (m_bFriendly)
			return "Friendly Assets";

		return "Enemy Assets";
	}

	//! @return the header icon key
	override string Icon() { return "directions_car"; }

	//! @return the side's row ink, or the primary container ink without a faction
	override int IconTint() { return SideInk(); }

	//! @return the side's row ink, or the primary container ink without a faction
	protected int SideInk()
	{
		TBD_BriefingFaction faction = m_Catalog.GetFaction(m_bFriendly);
		if (!faction)
			return TBD_UITheme.PRIMARY_CONTAINER;

		return TBD_UITheme.FactionRowInk(faction.m_eTint);
	}

	//! Add the side's role chip, and on the Assets page the vehicle count.
	//! @param badgeDock the header chip dock
	override void AddBadges(Widget badgeDock)
	{
		TBD_BriefingFaction faction = m_Catalog.GetFaction(m_bFriendly);
		if (faction)
			AddBadge(badgeDock, faction.m_sRoleLabel, faction.m_eTint);

		AddBadge(badgeDock, m_Catalog.CountAssets(m_bFriendly).ToString(), CountTint(), false);
	}

	//! @return PRIMARY for the reader's side, OPFOR for the other
	protected TBD_EUITint CountTint()
	{
		if (m_bFriendly)
			return TBD_EUITint.PRIMARY;

		return TBD_EUITint.OPFOR;
	}

	//! Fill the page with one group per vehicle type of the side.
	//! @param content the scroll list column
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

	//! Add the collapsed Vehicle Info section: preview, weapons, mobility and crew.
	//! @param parent the group body
	//! @param type the vehicle type
	//! @param ground the ARGB ground colour under the section
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

	//! Add one collapsed vehicle row with its ammunition and inventory.
	//! @param parent the group body
	//! @param type the vehicle type
	//! @param instance the vehicle
	//! @param ground the ARGB ground colour under the row
	//! @param tint the side tint for the callsign badge
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

	//! Add one captioned inventory grid; an empty list adds nothing.
	//! @param parent the row body
	//! @param title the caption
	//! @param entries the items
	//! @param ground the ARGB ground colour under the grid
	protected void AddInventory(Widget parent, string title, array<ref TBD_KitEntry> entries, int ground)
	{
		if (!entries || entries.IsEmpty())
			return;

		TBD_Caption.Mount(parent, title);
		AddCellGrid(parent, entries, 3, ground, true);
	}
}
