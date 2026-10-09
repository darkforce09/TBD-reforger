//------------------------------------------------------------------------------------------------
// TBD_M16CandidateExtractor.c
//
// Reads the magazines and attachments that make up the pool an M16 variant is matched against:
// the magazine well a magazine fits, how many rounds it holds, the attachment type an attachment
// presents, and the mass and volume of either.
//
// AttachTypeFits is the rule the match is made on - an attachment fits a slot when its declared
// type equals the slot's required type or inherits from it. This domain resolves compatibility
// itself rather than exporting foreign keys, which is the one place the arsenal inverts the
// contract the other eleven domains follow.
//------------------------------------------------------------------------------------------------

class TBD_M16CandidateExtractor
{
	//------------------------------------------------------------------------------------------------
	static string ReadMagazineWell(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MagazineComponent"))
				continue;
			foreach (BaseContainer c : bucket)
			{
				BaseContainer w = c.GetObject("MagazineWell");
				if (w)
					return w.GetClassName();
			}
		}
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	static int ReadMagazineCapacity(map<string, ref array<BaseContainer>> comps, string path = "")
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MagazineComponent"))
				continue;
			foreach (BaseContainer c : bucket)
			{
				int cap = 0;
				if (c.Get("MaxAmmo", cap) && cap > 0)
					return cap;
				if (c.Get("m_iMaxAmmo", cap) && cap > 0)
					return cap;
				if (c.Get("m_iAmmoCount", cap) && cap > 0)
					return cap;
				if (c.Get("m_iCapacity", cap) && cap > 0)
					return cap;

				BaseContainer anc = c.GetAncestor();
				while (anc)
				{
					if (anc.Get("MaxAmmo", cap) && cap > 0)
						return cap;
					anc = anc.GetAncestor();
				}
			}
		}

		// Fallback: parse capacity from filename stem e.g. "30rnd", "20rnd", "100rnd"
		if (!path.IsEmpty())
		{
			int rndIdx = path.IndexOf("rnd");
			if (rndIdx > 0)
			{
				int start = rndIdx - 1;
				while (start >= 0)
				{
					string ch = path.Substring(start, 1);
					if (ch != "0" && ch != "1" && ch != "2" && ch != "3" && ch != "4" && ch != "5" && ch != "6" && ch != "7" && ch != "8" && ch != "9")
						break;
					start--;
				}
				start++;
				if (start < rndIdx)
				{
					string numStr = path.Substring(start, rndIdx - start);
					int parsed = numStr.ToInt();
					if (parsed > 0)
						return parsed;
				}
			}
		}

		return 0;
	}

	//------------------------------------------------------------------------------------------------
	static string ReadAttachmentType(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;
			foreach (BaseContainer inv : bucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainerList custom = attrs.GetObjectArray("CustomAttributes");
				if (!custom)
					continue;
				for (int i = 0, n = custom.Count(); i < n; i++)
				{
					BaseContainer ca = custom.Get(i);
					if (!ca || ca.GetClassName() != "WeaponAttachmentAttributes")
						continue;
					BaseContainer t = ca.GetObject("AttachmentType");
					if (t)
						return t.GetClassName();
				}
			}
		}
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	static float ReadWeight(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("StorageComponent"))
				continue;
			foreach (BaseContainer c : bucket)
			{
				BaseContainer attrs = c.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer phys = attrs.GetObject("ItemPhysAttributes");
				if (phys)
				{
					float w;
					if (phys.Get("Weight", w) && w >= 0)
						return w;
				}
			}
		}
		return -1.0;
	}

	//------------------------------------------------------------------------------------------------
	static float ReadVolume(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("StorageComponent"))
				continue;
			foreach (BaseContainer c : bucket)
			{
				BaseContainer attrs = c.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer phys = attrs.GetObject("ItemPhysAttributes");
				if (phys)
				{
					float v;
					if (phys.Get("ItemVolume", v) && v >= 0)
						return v;
				}
			}
		}
		return -1.0;
	}

	//------------------------------------------------------------------------------------------------
	static bool AttachTypeFits(string itemType, string slotType)
	{
		if (itemType == slotType)
			return true;
		typename ti = itemType.ToType();
		typename ts = slotType.ToType();
		if (!ti || !ts)
			return false;
		return ti.IsInherited(ts);
	}
}
