/**
 * TBD_M16DeepScanner.c
 *
 * Exhaustive deep inspection & compatibility scanner for the M16 weapon platform.
 * Scans all M16 rifle variants, detects muzzles, fire modes, and attachment slots,
 * and matches them against all discovered magazines, optics, and attachments across loaded addons.
 */

class TBD_M16AttachmentSlotInfo
{
	string m_sSlotName;
	string m_sRequiredAttachType;
	string m_sDefaultAttachedPrefab;
	ref array<string> m_aCompatibleItemResourceNames = {};
	ref array<string> m_aCompatibleItemDisplayNames = {};
}

class TBD_M16MuzzleInfo
{
	int m_iIndex;
	string m_sMuzzleClass;
	ref array<string> m_aMagazineWells = {};
	string m_sDefaultMagazineTemplate;
	ref array<string> m_aFireModeNames = {};
	ref array<int> m_aFireModeBursts = {};
	ref array<int> m_aFireModeRpms = {};
	ref array<string> m_aCompatibleMagResourceNames = {};
	ref array<string> m_aCompatibleMagDisplayNames = {};
	ref array<int> m_aCompatibleMagCapacities = {};
}

class TBD_M16WeaponVariantInfo
{
	string m_sResourceName;
	string m_sId;
	string m_sDisplayName;
	string m_sFilePath;
	string m_sAddonId;
	string m_sVariantOf;
	bool m_bIsAbstract;
	float m_fWeightKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	int m_iCargoGridW = -1;
	int m_iCargoGridH = -1;
	ref array<string> m_aZeroingDistances = {};
	ref array<ref TBD_M16MuzzleInfo> m_aMuzzles = {};
	ref array<ref TBD_M16AttachmentSlotInfo> m_aAttachmentSlots = {};
}

class TBD_CandidateMagazine
{
	string m_sResourceName;
	string m_sDisplayName;
	string m_sMagWellClass;
	int m_iCapacity = 0;
	float m_fWeightKg = -1.0;
	string m_sAddonId;
}

class TBD_CandidateAttachment
{
	string m_sResourceName;
	string m_sDisplayName;
	string m_sAttachmentTypeClass;
	bool m_bIsOptic;
	float m_fWeightKg = -1.0;
	string m_sAddonId;
	string m_sFilePath;
}

class TBD_M16DeepScanner
{
	protected static const string TAG = "[TBD][M16Scanner]";
	protected static const int ANCESTOR_CAP = 16;
	protected static const int COMPONENT_DEPTH_CAP = 8;

	ref array<ref TBD_M16WeaponVariantInfo> m_aM16Variants = {};
	ref array<ref TBD_CandidateMagazine> m_aAllMagazines = {};
	ref array<ref TBD_CandidateAttachment> m_aAllAttachments = {};

	protected ref map<string, int> m_mMagIndexByResource = new map<string, int>();
	protected ref map<string, int> m_mAttachIndexByResource = new map<string, int>();

	protected string m_sCurrentAddonId;
	protected int m_iSeen = 0;

	//------------------------------------------------------------------------------------------------
	//! Execute full discovery: scans all addons for magazines & attachments, then inspects all M16s.
	bool RunDeepScan()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting deep discovery across loaded addons...", LogLevel.NORMAL);

		// 1. Build global pool of all magazines and attachments
		CollectGlobalPool();

		// 2. Discover and inspect all M16 prefabs
		CollectM16Variants();

		// 3. Match compatibility
		ResolveCompatibilities();

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("%1 Deep scan finished in %2 ms: %3 M16 variants, %4 magazines in pool, %5 attachments in pool",
			TAG, elapsedMs, m_aM16Variants.Count(), m_aAllMagazines.Count(), m_aAllAttachments.Count()), LogLevel.NORMAL);

		return !m_aM16Variants.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	//! Phase 1: Collect all magazines and attachment items across loaded addons.
	protected void CollectGlobalPool()
	{
		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		foreach (string guid : guids)
		{
			string addonId = GameProject.GetAddonID(guid);
			m_sCurrentAddonId = addonId;
			string rootPath = "$" + addonId + ":Prefabs";
			Workbench.SearchResources(OnPoolResourceFound, extEt, null, rootPath, true);
		}

		if (m_aAllMagazines.IsEmpty() && m_aAllAttachments.IsEmpty())
		{
			m_sCurrentAddonId = string.Empty;
			Workbench.SearchResources(OnPoolResourceFound, extEt, null, string.Empty, true);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void OnPoolResourceFound(ResourceName resName, string filePath = "")
	{
		string path = filePath;
		if (path.IsEmpty())
			path = resName;

		if (!path.Contains("Prefabs/"))
			return;

		// Skip non-item paths
		if (path.Contains("/Structures/") || path.Contains("/Rocks/") || path.Contains("/Trees/")
			|| path.Contains("/Foliage/") || path.Contains("Prefabs/Editor/"))
			return;

		Resource res = Resource.Load(resName);
		if (!res || !res.IsValid())
			return;

		BaseResourceObject obj = res.GetResource();
		if (!obj)
			return;

		BaseContainer root = obj.ToBaseContainer();
		if (!root)
			return;

		string rootClass = root.GetClassName();
		if (rootClass == "SCR_ChimeraCharacter" || rootClass == "ChimeraCharacter" || rootClass == "Vehicle" || rootClass.EndsWith("Vehicle"))
			return;

		map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
		CollectComponentChain(root, comps);

		string canonical = ResolveCanonical(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		string addonId = m_sCurrentAddonId;
		if (addonId.IsEmpty() && path.StartsWith("$"))
		{
			int colon = path.IndexOf(":");
			if (colon > 1)
				addonId = path.Substring(1, colon - 1);
		}

		// Check for Magazine
		bool hasMag = HasCompSuffix(comps, "MagazineComponent");
		bool hasWeapon = HasCompSuffix(comps, "WeaponComponent");
		if (hasMag && !hasWeapon)
		{
			if (!m_mMagIndexByResource.Contains(canonical))
			{
				string magWell = ReadMagazineWell(comps);
				if (!magWell.IsEmpty())
				{
					TBD_CandidateMagazine mag = new TBD_CandidateMagazine();
					mag.m_sResourceName = canonical;
					mag.m_sDisplayName = DisplayNameFor(comps, path);
					mag.m_sMagWellClass = magWell;
					mag.m_iCapacity = ReadMagazineCapacity(comps, path);
					mag.m_fWeightKg = ReadWeight(comps);
					mag.m_sAddonId = addonId;

					m_mMagIndexByResource.Insert(canonical, m_aAllMagazines.Count());
					m_aAllMagazines.Insert(mag);
				}
			}
			return;
		}

		// Check for Attachment / Optic
		string attachType = ReadAttachmentType(comps);
		if (!attachType.IsEmpty())
		{
			if (!m_mAttachIndexByResource.Contains(canonical))
			{
				TBD_CandidateAttachment att = new TBD_CandidateAttachment();
				att.m_sResourceName = canonical;
				att.m_sDisplayName = DisplayNameFor(comps, path);
				att.m_sAttachmentTypeClass = attachType;
				att.m_bIsOptic = (attachType.Contains("Optic") || HasCompSuffix(comps, "SightsComponent") || path.Contains("/Optic"));
				att.m_fWeightKg = ReadWeight(comps);
				att.m_sAddonId = addonId;
				att.m_sFilePath = path;

				m_mAttachIndexByResource.Insert(canonical, m_aAllAttachments.Count());
				m_aAllAttachments.Insert(att);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Phase 2: Find all M16 variants and extract their deep properties.
	protected void CollectM16Variants()
	{
		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		foreach (string guid : guids)
		{
			string addonId = GameProject.GetAddonID(guid);
			m_sCurrentAddonId = addonId;
			string rootPath = "$" + addonId + ":Prefabs/Weapons/Rifles/M16";
			Workbench.SearchResources(OnM16ResourceFound, extEt, null, rootPath, true);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void OnM16ResourceFound(ResourceName resName, string filePath = "")
	{
		string path = filePath;
		if (path.IsEmpty())
			path = resName;

		if (!path.Contains("Rifle_M16"))
			return;

		Resource res = Resource.Load(resName);
		if (!res || !res.IsValid())
			return;

		BaseResourceObject obj = res.GetResource();
		if (!obj)
			return;

		BaseContainer root = obj.ToBaseContainer();
		if (!root)
			return;

		map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
		CollectComponentChain(root, comps);

		string canonical = ResolveCanonical(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		TBD_M16WeaponVariantInfo info = new TBD_M16WeaponVariantInfo();
		info.m_sResourceName = canonical;
		info.m_sFilePath = path;
		info.m_sId = GenerateSlug(path);
		info.m_sDisplayName = DisplayNameFor(comps, path);
		info.m_sAddonId = m_sCurrentAddonId;
		info.m_bIsAbstract = path.EndsWith("_base.et") || path.Contains("/base/");
		info.m_fWeightKg = ReadWeight(comps);
		info.m_fVolumeCm3 = ReadVolume(comps);

		// Ancestor / Variant Of
		BaseContainer anc = root.GetAncestor();
		if (anc)
		{
			string ancRn = ResolveCanonical(anc.GetResourceName());
			if (!ancRn.IsEmpty() && ancRn != canonical)
				info.m_sVariantOf = ancRn;
		}

		// Extract Sights
		ExtractZeroingDistances(comps, info.m_aZeroingDistances);

		// Extract Muzzles
		ExtractMuzzles(comps, info.m_aMuzzles);

		// Extract Attachment Slots
		ExtractAttachmentSlots(comps, info.m_aAttachmentSlots);

		m_aM16Variants.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	//! Phase 3: Match M16 muzzles & attachment slots with candidate items in global pool.
	protected void ResolveCompatibilities()
	{
		foreach (TBD_M16WeaponVariantInfo weapon : m_aM16Variants)
		{
			// 1. Match magazines per muzzle
			foreach (TBD_M16MuzzleInfo muzzle : weapon.m_aMuzzles)
			{
				foreach (TBD_CandidateMagazine mag : m_aAllMagazines)
				{
					if (muzzle.m_aMagazineWells.Find(mag.m_sMagWellClass) != -1)
					{
						muzzle.m_aCompatibleMagResourceNames.Insert(mag.m_sResourceName);
						muzzle.m_aCompatibleMagDisplayNames.Insert(mag.m_sDisplayName);
						muzzle.m_aCompatibleMagCapacities.Insert(mag.m_iCapacity);
					}
				}
			}

			// 2. Match attachments per slot
			foreach (TBD_M16AttachmentSlotInfo slot : weapon.m_aAttachmentSlots)
			{
				if (slot.m_sRequiredAttachType.IsEmpty())
					continue;

				foreach (TBD_CandidateAttachment att : m_aAllAttachments)
				{
					if (AttachTypeFits(att.m_sAttachmentTypeClass, slot.m_sRequiredAttachType))
					{
						slot.m_aCompatibleItemResourceNames.Insert(att.m_sResourceName);
						slot.m_aCompatibleItemDisplayNames.Insert(att.m_sDisplayName);
					}
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspection: AttachmentSlotComponent extraction
	protected void ExtractAttachmentSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_M16AttachmentSlotInfo> outSlots)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("AttachmentSlotComponent"))
				continue;

			foreach (BaseContainer slotComp : bucket)
			{
				BaseContainer slotObj = slotComp.GetObject("AttachmentSlot");
				BaseContainer typeObj = slotComp.GetObject("AttachmentType");
				string typeClass = "";
				if (typeObj)
					typeClass = typeObj.GetClassName();

				// Resolve slot name, pivot ID, and default attached prefab across hierarchy
				string slotName = "";
				string pivotId = "";
				string defAttach = "";

				BaseContainer curSlot = slotObj;
				while (curSlot)
				{
					if (slotName.IsEmpty())
						slotName = curSlot.GetName();

					if (pivotId.IsEmpty())
						curSlot.Get("PivotID", pivotId);

					if (defAttach.IsEmpty())
					{
						if (!curSlot.Get("Prefab", defAttach) || defAttach.IsEmpty())
							curSlot.Get("m_sAttachment", defAttach);
					}

					curSlot = curSlot.GetAncestor();
				}

				// If typeObj was empty on child, resolve from ancestor
				if (typeClass.IsEmpty())
				{
					BaseContainer curComp = slotComp.GetAncestor();
					while (curComp && typeClass.IsEmpty())
					{
						BaseContainer ancType = curComp.GetObject("AttachmentType");
						if (ancType)
							typeClass = ancType.GetClassName();
						curComp = curComp.GetAncestor();
					}
				}

				// Clean human-friendly slotName
				if (slotName.IsEmpty() || slotName.StartsWith("InventoryStorageSlot"))
				{
					if (!pivotId.IsEmpty())
						slotName = pivotId;
					else if (!typeClass.IsEmpty())
						slotName = typeClass;
					else
						slotName = "attachment_slot";
				}

				if (slotName.StartsWith("slot_"))
					slotName = slotName.Substring(5, slotName.Length() - 5);

				if (typeClass.IsEmpty() && slotName.IsEmpty())
					continue;

				bool duplicate = false;
				foreach (TBD_M16AttachmentSlotInfo existing : outSlots)
				{
					if (existing.m_sSlotName == slotName || (!typeClass.IsEmpty() && existing.m_sRequiredAttachType == typeClass))
					{
						duplicate = true;
						if (existing.m_sDefaultAttachedPrefab.IsEmpty() && !defAttach.IsEmpty())
							existing.m_sDefaultAttachedPrefab = ResolveCanonical(defAttach);
						break;
					}
				}

				if (!duplicate)
				{
					TBD_M16AttachmentSlotInfo s = new TBD_M16AttachmentSlotInfo();
					s.m_sSlotName = slotName;
					s.m_sRequiredAttachType = typeClass;
					if (!defAttach.IsEmpty())
						s.m_sDefaultAttachedPrefab = ResolveCanonical(defAttach);
					outSlots.Insert(s);
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspection: MuzzleComponent extraction
	protected void ExtractMuzzles(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_M16MuzzleInfo> outMuzzles)
	{
		int muzzleIdx = 0;
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MuzzleComponent"))
				continue;

			// Deduplicate: filter out containers that are ancestors of another container in bucket
			array<BaseContainer> distinctMuzzles = {};
			foreach (BaseContainer mc : bucket)
			{
				bool isAncestor = false;
				foreach (BaseContainer other : bucket)
				{
					if (mc == other)
						continue;
					BaseContainer anc = other.GetAncestor();
					while (anc)
					{
						if (anc == mc)
						{
							isAncestor = true;
							break;
						}
						anc = anc.GetAncestor();
					}
					if (isAncestor)
						break;
				}
				if (!isAncestor)
					distinctMuzzles.Insert(mc);
			}

			foreach (BaseContainer muzComp : distinctMuzzles)
			{
				TBD_M16MuzzleInfo m = new TBD_M16MuzzleInfo();
				m.m_iIndex = muzzleIdx;
				m.m_sMuzzleClass = cls;

				// MagazineWell (resolve up ancestry chain if needed)
				BaseContainer cur = muzComp;
				while (cur && m.m_aMagazineWells.IsEmpty())
				{
					BaseContainer wellObj = cur.GetObject("MagazineWell");
					if (wellObj)
					{
						string wCls = wellObj.GetClassName();
						if (!wCls.IsEmpty() && m.m_aMagazineWells.Find(wCls) == -1)
							m.m_aMagazineWells.Insert(wCls);
					}
					cur = cur.GetAncestor();
				}

				// Default MagazineTemplate
				cur = muzComp;
				while (cur && m.m_sDefaultMagazineTemplate.IsEmpty())
				{
					string magTmpl;
					if (cur.Get("MagazineTemplate", magTmpl) && !magTmpl.IsEmpty())
						m.m_sDefaultMagazineTemplate = ResolveCanonical(magTmpl);
					cur = cur.GetAncestor();
				}

				// FireModes
				BaseContainerList modes = null;
				cur = muzComp;
				while (cur && !modes)
				{
					modes = cur.GetObjectArray("FireModes");
					if (!modes)
						modes = cur.GetObjectArray("m_aFireModes");
					cur = cur.GetAncestor();
				}

				if (modes)
				{
					for (int fm = 0, fmn = modes.Count(); fm < fmn; fm++)
					{
						BaseContainer mode = modes.Get(fm);
						if (!mode)
							continue;

						string modeName = "";
						int burst = 1;
						int rpm = 0;

						// Read RPM
						if (!mode.Get("RoundsPerMinute", rpm) || rpm == 0)
						{
							if (!mode.Get("m_iRoundsPerMinute", rpm) || rpm == 0)
							{
								float rpmFloat;
								if (mode.Get("m_fRoundsPerMinute", rpmFloat))
									rpm = rpmFloat;
							}
						}
						if (rpm == 0 && mode.GetAncestor())
						{
							mode.GetAncestor().Get("RoundsPerMinute", rpm);
						}

						// Read Burst
						if (!mode.Get("BurstCount", burst))
							mode.Get("m_iBurstCount", burst);

						// Read Name / infer from config reference
						if (!mode.Get("m_sFireModeName", modeName) || modeName.IsEmpty())
							mode.Get("m_sName", modeName);

						if (modeName.IsEmpty() || modeName.StartsWith("#") || modeName.StartsWith("AR-"))
						{
							string confRef = mode.GetResourceName();
							if (confRef.IsEmpty() && mode.GetAncestor())
								confRef = mode.GetAncestor().GetResourceName();

							if (confRef.Contains("Single"))
							{
								modeName = "Semi";
								burst = 1;
							}
							else if (confRef.Contains("Burst"))
							{
								modeName = "Burst";
								if (burst <= 1)
									burst = 3;
							}
							else if (confRef.Contains("Auto"))
							{
								modeName = "Auto";
								burst = 0;
							}
							else if (confRef.Contains("Safe"))
							{
								modeName = "Safe";
							}
							else
							{
								modeName = "Mode " + (fm + 1).ToString();
							}
						}

						m.m_aFireModeNames.Insert(modeName);
						m.m_aFireModeBursts.Insert(burst);
						m.m_aFireModeRpms.Insert(rpm);
					}
				}

				outMuzzles.Insert(m);
				muzzleIdx++;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspection: SightsComponent zeroing distances
	protected void ExtractZeroingDistances(map<string, ref array<BaseContainer>> comps, notnull array<string> outZeroings)
	{
		array<BaseContainer> sights = comps.Get("SightsComponent");
		if (!sights)
			sights = comps.Get("SCR_SightsComponent");

		if (!sights)
			return;

		foreach (BaseContainer sc : sights)
		{
			// Range / zeroing properties
			array<float> ranges = {};
			sc.Get("m_aZeroingDistances", ranges);
			if (ranges)
			{
				foreach (float r : ranges)
					outZeroings.Insert(r.ToString() + "m");
			}
			else
			{
				float singleZero;
				if (sc.Get("m_fZeroingDistance", singleZero))
					outZeroings.Insert(singleZero.ToString() + "m");
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected string ReadMagazineWell(map<string, ref array<BaseContainer>> comps)
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
	protected int ReadMagazineCapacity(map<string, ref array<BaseContainer>> comps, string path = "")
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
	protected string ReadAttachmentType(map<string, ref array<BaseContainer>> comps)
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
	protected bool AttachTypeFits(string itemType, string slotType)
	{
		if (itemType == slotType)
			return true;
		typename ti = itemType.ToType();
		typename ts = slotType.ToType();
		if (!ti || !ts)
			return false;
		return ti.IsInherited(ts);
	}

	//------------------------------------------------------------------------------------------------
	protected float ReadWeight(map<string, ref array<BaseContainer>> comps)
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
	protected float ReadVolume(map<string, ref array<BaseContainer>> comps)
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
	protected string DisplayNameFor(map<string, ref array<BaseContainer>> comps, string filePath)
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
				BaseContainer disp = attrs.GetObject("ItemDisplayName");
				if (!disp)
					continue;
				string n;
				if (disp.Get("Name", n) && !n.IsEmpty() && !n.StartsWith("#") && !n.StartsWith("AR-"))
					return n;
			}
		}

		array<string> uiHolders = {"WeaponComponent", "MagazineComponent"};
		foreach (string holder : uiHolders)
		{
			array<BaseContainer> bucket2 = comps.Get(holder);
			if (!bucket2)
				continue;
			foreach (BaseContainer c : bucket2)
			{
				BaseContainer ui = c.GetObject("UIInfo");
				if (!ui)
					continue;
				string n2;
				if (ui.Get("Name", n2) && !n2.IsEmpty() && !n2.StartsWith("#") && !n2.StartsWith("AR-"))
					return n2;
			}
		}

		return HumanizeStem(filePath);
	}

	//------------------------------------------------------------------------------------------------
	protected string HumanizeStem(string filePath)
	{
		string stem = filePath;
		int slash = stem.LastIndexOf("/");
		if (slash >= 0)
			stem = stem.Substring(slash + 1, stem.Length() - slash - 1);
		if (stem.EndsWith(".et"))
			stem = stem.Substring(0, stem.Length() - 3);

		// Optics specific names
		if (stem == "Optic_4x20" || stem == "Optic_4x20_base")
			return "Colt 4x20 Scope";
		if (stem.StartsWith("Optic_4x20_"))
			return "Colt 4x20 Scope (" + HumanizeStem(stem.Substring(11, stem.Length() - 11)) + ")";
		if (stem == "Collim_AP2k" || stem == "Collim_AP2k_base")
			return "Aimpoint 2000 (AP2k)";
		if (stem.StartsWith("Collim_AP2k_"))
			return "Aimpoint 2000 (" + HumanizeStem(stem.Substring(12, stem.Length() - 12)) + ")";

		// Underbarrel
		if (stem == "UGL_M203_long" || stem == "UGL_M203_base")
			return "M203 40mm Grenade Launcher";
		if (stem.StartsWith("UGL_M203_"))
			return "M203 40mm (" + HumanizeStem(stem.Substring(9, stem.Length() - 9)) + ")";

		// Magazines
		if (stem.StartsWith("Magazine_556x45_STANAG_30rnd_"))
		{
			string variant = stem.Substring(30, stem.Length() - 30);
			variant.Replace("_", " ");
			return "5.56x45mm STANAG 30-round (" + variant + ")";
		}

		if (stem.StartsWith("Rifle_"))
			stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Magazine_"))
			stem = stem.Substring(9, stem.Length() - 9);
		else if (stem.StartsWith("Optic_"))
			stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Collim_"))
			stem = stem.Substring(7, stem.Length() - 7);

		stem.Replace("_", " ");
		stem.Trim();
		return stem;
	}

	//------------------------------------------------------------------------------------------------
	protected string GenerateSlug(string filePath)
	{
		string s = filePath;
		int slash = s.LastIndexOf("/");
		if (slash >= 0)
			s = s.Substring(slash + 1, s.Length() - slash - 1);
		if (s.EndsWith(".et"))
			s = s.Substring(0, s.Length() - 3);
		s.ToLower();
		s.Replace("-", "_");
		s.Replace(" ", "_");
		return s;
	}

	//------------------------------------------------------------------------------------------------
	protected void CollectComponentChain(BaseContainer prefabRoot, notnull map<string, ref array<BaseContainer>> outComps)
	{
		BaseContainer cur = prefabRoot;
		int hops = 0;
		while (cur && hops < ANCESTOR_CAP)
		{
			CollectComponentsRec(cur, outComps, 0);
			cur = cur.GetAncestor();
			hops++;
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void CollectComponentsRec(BaseContainer holder, notnull map<string, ref array<BaseContainer>> outComps, int depth)
	{
		if (depth > COMPONENT_DEPTH_CAP)
			return;

		BaseContainerList comps = holder.GetObjectArray("components");
		if (!comps)
			return;

		for (int i = 0, n = comps.Count(); i < n; i++)
		{
			BaseContainer comp = comps.Get(i);
			if (!comp)
				continue;

			string cls = comp.GetClassName();
			array<BaseContainer> bucket = outComps.Get(cls);
			if (!bucket)
			{
				bucket = {};
				outComps.Insert(cls, bucket);
			}
			bucket.Insert(comp);

			CollectComponentsRec(comp, outComps, depth + 1);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected bool HasCompSuffix(map<string, ref array<BaseContainer>> comps, string suffix)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.EndsWith(suffix))
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected string ResolveCanonical(string resName)
	{
		if (resName.IsEmpty())
			return string.Empty;
		if (resName.StartsWith("{"))
			return resName;
		ResourceName rn = resName;
		Resource res = Resource.Load(rn);
		if (!res || !res.IsValid())
			return resName;
		BaseResourceObject obj = res.GetResource();
		if (!obj)
			return resName;
		BaseContainer root = obj.ToBaseContainer();
		if (!root)
			return resName;
		string crn = root.GetResourceName();
		if (crn.StartsWith("{"))
			return crn;
		return resName;
	}

	//------------------------------------------------------------------------------------------------
	//! Serialize entire M16 dataset to JSON string buffer.
	string SerializeToJson()
	{
		string json = "{\n";
		json += "  \"version\": \"1\",\n";
		json += "  \"platform\": \"M16\",\n";
		json += "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
		json += "  \"totalVariants\": " + m_aM16Variants.Count().ToString() + ",\n";
		json += "  \"weapons\": [\n";

		for (int w = 0; w < m_aM16Variants.Count(); w++)
		{
			TBD_M16WeaponVariantInfo var = m_aM16Variants[w];
			json += "    {\n";
			json += "      \"resource_name\": \"" + TBD_EquipmentExportJson.Escape(var.m_sResourceName) + "\",\n";
			json += "      \"id\": \"" + TBD_EquipmentExportJson.Escape(var.m_sId) + "\",\n";
			json += "      \"display_name\": \"" + TBD_EquipmentExportJson.Escape(var.m_sDisplayName) + "\",\n";
			json += "      \"addon\": \"" + TBD_EquipmentExportJson.Escape(var.m_sAddonId) + "\",\n";
			json += "      \"file_path\": \"" + TBD_EquipmentExportJson.Escape(var.m_sFilePath) + "\",\n";

			string isAbsStr = "false";
			if (var.m_bIsAbstract) isAbsStr = "true";
			json += "      \"is_abstract\": " + isAbsStr + ",\n";

			if (!var.m_sVariantOf.IsEmpty())
				json += "      \"variant_of\": \"" + TBD_EquipmentExportJson.Escape(var.m_sVariantOf) + "\",\n";

			if (var.m_fWeightKg >= 0)
				json += "      \"weight_kg\": " + var.m_fWeightKg.ToString() + ",\n";
			if (var.m_fVolumeCm3 >= 0)
				json += "      \"volume_cm3\": " + var.m_fVolumeCm3.ToString() + ",\n";

			// Zeroing
			json += "      \"zeroing_distances\": [";
			for (int zd = 0; zd < var.m_aZeroingDistances.Count(); zd++)
			{
				json += "\"" + var.m_aZeroingDistances[zd] + "\"";
				if (zd < var.m_aZeroingDistances.Count() - 1)
					json += ", ";
			}
			json += "],\n";

			// Muzzles
			json += "      \"muzzles\": [\n";
			for (int m = 0; m < var.m_aMuzzles.Count(); m++)
			{
				TBD_M16MuzzleInfo muz = var.m_aMuzzles[m];
				json += "        {\n";
				json += "          \"index\": " + muz.m_iIndex.ToString() + ",\n";
				json += "          \"muzzle_class\": \"" + muz.m_sMuzzleClass + "\",\n";

				// Magazine wells
				json += "          \"magazine_wells\": [";
				for (int mw = 0; mw < muz.m_aMagazineWells.Count(); mw++)
				{
					json += "\"" + muz.m_aMagazineWells[mw] + "\"";
					if (mw < muz.m_aMagazineWells.Count() - 1)
						json += ", ";
				}
				json += "],\n";

				if (!muz.m_sDefaultMagazineTemplate.IsEmpty())
					json += "          \"default_magazine\": \"" + TBD_EquipmentExportJson.Escape(muz.m_sDefaultMagazineTemplate) + "\",\n";

				// Fire modes
				json += "          \"fire_modes\": [";
				for (int f = 0; f < muz.m_aFireModeNames.Count(); f++)
				{
					json += "{\"name\":\"" + muz.m_aFireModeNames[f] + "\",\"burst\":" + muz.m_aFireModeBursts[f].ToString() + ",\"rpm\":" + muz.m_aFireModeRpms[f].ToString() + "}";
					if (f < muz.m_aFireModeNames.Count() - 1)
						json += ", ";
				}
				json += "]\n";

				json += "        }";
				if (m < var.m_aMuzzles.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += "      ],\n";

			// Attachment slots
			json += "      \"attachment_slots\": [\n";
			for (int s = 0; s < var.m_aAttachmentSlots.Count(); s++)
			{
				TBD_M16AttachmentSlotInfo slot = var.m_aAttachmentSlots[s];
				json += "        {\n";
				json += "          \"slot_name\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sSlotName) + "\",\n";
				json += "          \"required_attachment_type\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sRequiredAttachType) + "\"";
				if (!slot.m_sDefaultAttachedPrefab.IsEmpty())
					json += ",\n          \"default_attachment\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sDefaultAttachedPrefab) + "\"\n";
				else
					json += ",\n          \"default_attachment\": null\n";

				json += "        }";
				if (s < var.m_aAttachmentSlots.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += "      ]\n";

			json += "    }";
			if (w < m_aM16Variants.Count() - 1)
				json += ",";
			json += "\n";
		}

		json += "  ]\n}\n";
		return json;
	}
}
