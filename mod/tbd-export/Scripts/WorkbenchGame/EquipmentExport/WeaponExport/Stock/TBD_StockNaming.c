//------------------------------------------------------------------------------------------------
// TBD_StockNaming.c
//
// Reads the player-facing strings a buttstock declares - display name, description, inventory icon.
//
// Core's TBD_EquipmentDisplayAttributes searches the InventoryItemComponent ancestry for an
// ItemDisplayName node. This domain searches wider: it accepts UIInfo in place of ItemDisplayName,
// probes UIInfo directly on each component, and makes a second pass over every component that is
// not a slot. Buttstock prefabs frequently declare their strings outside the node the shared
// reader looks at, which is why this domain keeps its own.
//------------------------------------------------------------------------------------------------

class TBD_StockNaming
{
	//------------------------------------------------------------------------------------------------
	//! Extract raw unmutated localized display name token directly from ItemDisplayName.Name or UIInfo.Name.
	//! ZERO string mutation: keeps # intact, zero English fallbacks, zero synthetic guessing.
	static string RawDisplayNameFor(map<string, ref array<BaseContainer>> comps)
	{
		// 1. Primary: InventoryItemComponent -> Attributes -> ItemDisplayName / UIInfo
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;

			foreach (BaseContainer inv : bucket)
			{
				BaseContainer cur = inv;
				while (cur)
				{
					BaseContainer attrs = cur.GetObject("Attributes");
					while (attrs)
					{
						BaseContainer disp = attrs.GetObject("ItemDisplayName");
						if (!disp)
							disp = attrs.GetObject("UIInfo");

						if (disp)
						{
							string n;
							if (disp.Get("Name", n) && !n.IsEmpty())
								return n;
						}
						attrs = attrs.GetAncestor();
					}

					BaseContainer ui = cur.GetObject("UIInfo");
					if (ui)
					{
						string n2;
						if (ui.Get("Name", n2) && !n2.IsEmpty())
							return n2;
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// 2. Secondary: Any component with UIInfo or Attributes.ItemDisplayName / UIInfo
		foreach (string compCls, array<BaseContainer> compBucket : comps)
		{
			if (compCls.EndsWith("AttachmentSlotComponent") || compCls.EndsWith("SlotManagerComponent"))
				continue;

			foreach (BaseContainer c : compBucket)
			{
				BaseContainer curC = c;
				while (curC)
				{
					BaseContainer cAttrs = curC.GetObject("Attributes");
					while (cAttrs)
					{
						BaseContainer cDisp = cAttrs.GetObject("ItemDisplayName");
						if (!cDisp)
							cDisp = cAttrs.GetObject("UIInfo");
						if (cDisp)
						{
							string cn;
							if (cDisp.Get("Name", cn) && !cn.IsEmpty())
								return cn;
						}
						cAttrs = cAttrs.GetAncestor();
					}

					BaseContainer cUi = curC.GetObject("UIInfo");
					if (cUi)
					{
						string uin;
						if (cUi.Get("Name", uin) && !uin.IsEmpty())
							return uin;
					}

					curC = curC.GetAncestor();
				}
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract raw unmutated localized description token directly from ItemDisplayName.Description or UIInfo.Description.
	//! Returns string.Empty if omitted in prefab (serializes as null).
	static string RawDescriptionFor(map<string, ref array<BaseContainer>> comps)
	{
		// 1. Primary: InventoryItemComponent -> Attributes -> ItemDisplayName / UIInfo
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;

			foreach (BaseContainer inv : bucket)
			{
				BaseContainer cur = inv;
				while (cur)
				{
					BaseContainer attrs = cur.GetObject("Attributes");
					while (attrs)
					{
						BaseContainer disp = attrs.GetObject("ItemDisplayName");
						if (!disp)
							disp = attrs.GetObject("UIInfo");

						if (disp)
						{
							string d;
							if (disp.Get("Description", d) && !d.IsEmpty())
								return d;
						}
						attrs = attrs.GetAncestor();
					}

					BaseContainer ui = cur.GetObject("UIInfo");
					if (ui)
					{
						string d2;
						if (ui.Get("Description", d2) && !d2.IsEmpty())
							return d2;
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// 2. Secondary: Any component with UIInfo or Attributes.ItemDisplayName / UIInfo
		foreach (string compCls, array<BaseContainer> compBucket : comps)
		{
			if (compCls.EndsWith("AttachmentSlotComponent") || compCls.EndsWith("SlotManagerComponent"))
				continue;

			foreach (BaseContainer c : compBucket)
			{
				BaseContainer curC = c;
				while (curC)
				{
					BaseContainer cAttrs = curC.GetObject("Attributes");
					while (cAttrs)
					{
						BaseContainer cDisp = cAttrs.GetObject("ItemDisplayName");
						if (!cDisp)
							cDisp = cAttrs.GetObject("UIInfo");
						if (cDisp)
						{
							string cd;
							if (cDisp.Get("Description", cd) && !cd.IsEmpty())
								return cd;
						}
						cAttrs = cAttrs.GetAncestor();
					}

					BaseContainer cUi = curC.GetObject("UIInfo");
					if (cUi)
					{
						string uid;
						if (cUi.Get("Description", uid) && !uid.IsEmpty())
							return uid;
					}

					curC = curC.GetAncestor();
				}
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract raw icon texture path directly from ItemDisplayName.Icon or UIInfo.Icon.
	static string RawIconFor(map<string, ref array<BaseContainer>> comps)
	{
		// 1. Primary: InventoryItemComponent -> Attributes -> ItemDisplayName / UIInfo
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;

			foreach (BaseContainer inv : bucket)
			{
				BaseContainer cur = inv;
				while (cur)
				{
					BaseContainer attrs = cur.GetObject("Attributes");
					while (attrs)
					{
						BaseContainer disp = attrs.GetObject("ItemDisplayName");
						if (!disp)
							disp = attrs.GetObject("UIInfo");

						if (disp)
						{
							string icon;
							if (disp.Get("Icon", icon) && !icon.IsEmpty())
								return TBD_EquipmentResourceNames.NormalizePathSeparators(icon);
						}
						attrs = attrs.GetAncestor();
					}

					BaseContainer ui = cur.GetObject("UIInfo");
					if (ui)
					{
						string icon2;
						if (ui.Get("Icon", icon2) && !icon2.IsEmpty())
							return TBD_EquipmentResourceNames.NormalizePathSeparators(icon2);
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// 2. Secondary: Any component with UIInfo or Attributes.ItemDisplayName / UIInfo
		foreach (string compCls, array<BaseContainer> compBucket : comps)
		{
			if (compCls.EndsWith("AttachmentSlotComponent") || compCls.EndsWith("SlotManagerComponent"))
				continue;

			foreach (BaseContainer c : compBucket)
			{
				BaseContainer curC = c;
				while (curC)
				{
					BaseContainer cAttrs = curC.GetObject("Attributes");
					while (cAttrs)
					{
						BaseContainer cDisp = cAttrs.GetObject("ItemDisplayName");
						if (!cDisp)
							cDisp = cAttrs.GetObject("UIInfo");
						if (cDisp)
						{
							string cIcon;
							if (cDisp.Get("Icon", cIcon) && !cIcon.IsEmpty())
								return TBD_EquipmentResourceNames.NormalizePathSeparators(cIcon);
						}
						cAttrs = cAttrs.GetAncestor();
					}

					BaseContainer cUi = curC.GetObject("UIInfo");
					if (cUi)
					{
						string uiIcon;
						if (cUi.Get("Icon", uiIcon) && !uiIcon.IsEmpty())
							return TBD_EquipmentResourceNames.NormalizePathSeparators(uiIcon);
					}

					curC = curC.GetAncestor();
				}
			}
		}

		return string.Empty;
	}
}
