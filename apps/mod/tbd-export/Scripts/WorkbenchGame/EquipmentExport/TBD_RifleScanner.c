//------------------------------------------------------------------------------------------------
// TBD_RifleScanner.c
//
// Generic deep scanner for all Rifle weapons across loaded addons in Arma Reforger Workbench.
// Extracts pure intrinsic properties: muzzles, magazine wells, fire modes, attachment slots,
// required attachment types, pre-attached prefabs, zeroing distances, mass, and volume.
//
// Does NOT embed pre-resolved cross-product compatibility arrays (compatible_magazines or
// compatible_attachments); exports pure foreign keys (magazine_wells, required_attachment_type)
// so the platform / website can dynamically link them with ammunition and attachment catalogs.
//------------------------------------------------------------------------------------------------

class TBD_RifleFireModeInfo
{
	string m_sName;
	int m_iBurstCount = 1;
	int m_iRoundsPerMinute = 0;
}

class TBD_RifleMuzzleInfo
{
	int m_iIndex = 0;
	string m_sMuzzleClass;
	ref array<string> m_aMagazineWells = {};
	string m_sDefaultMagazineTemplate;
	ref array<ref TBD_RifleFireModeInfo> m_aFireModes = {};
}

class TBD_RifleAttachmentSlotInfo
{
	string m_sSlotName;
	string m_sRequiredAttachmentType;
	string m_sDefaultAttachedPrefab;
}

class TBD_RifleWeaponInfo
{
	string m_sResourceName;
	string m_sId;
	string m_sDisplayName;
	string m_sFamily;
	string m_sAddonId;
	string m_sFilePath;
	bool m_bIsAbstract = false;
	string m_sVariantOf;
	float m_fWeightKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	ref array<string> m_aZeroingDistances = {};
	ref array<ref TBD_RifleMuzzleInfo> m_aMuzzles = {};
	ref array<ref TBD_RifleAttachmentSlotInfo> m_aAttachmentSlots = {};
}

class TBD_RifleScanner
{
	protected static const int ANCESTOR_CAP = 16;
	protected static const int COMPONENT_DEPTH_CAP = 4;

	protected ref TBD_EquipmentExportConfig m_Config;
	protected ref array<ref TBD_RifleWeaponInfo> m_aRifles = {};
	protected string m_sCurrentAddonId;

	//------------------------------------------------------------------------------------------------
	void TBD_RifleScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	int RunScan()
	{
		m_aRifles.Clear();

		Print("[TBD][RifleExport] Starting generic deep scan across all rifles...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		foreach (string guid : guids)
		{
			string addonId = GameProject.GetAddonID(guid);
			m_sCurrentAddonId = addonId;
			string rootPath = "$" + addonId + ":Prefabs/Weapons/Rifles";
			Workbench.SearchResources(OnRifleResourceFound, extEt, null, rootPath, true);
		}

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("[TBD][RifleExport] Completed in %1 ms. Discovered %2 rifle prefabs.", elapsedMs, m_aRifles.Count()), LogLevel.NORMAL);

		return m_aRifles.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnRifleResourceFound(ResourceName resName, string filePath = "")
	{
		string path = filePath;
		if (path.IsEmpty())
			path = resName;

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

		// Must carry WeaponComponent
		if (!HasCompSuffix(comps, "WeaponComponent"))
			return;

		string canonical = ResolveCanonical(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		TBD_RifleWeaponInfo info = new TBD_RifleWeaponInfo();
		info.m_sResourceName = canonical;
		info.m_sFilePath = path;
		info.m_sId = GenerateSlug(path);
		info.m_sDisplayName = DisplayNameFor(comps, path);
		info.m_sFamily = ExtractFamily(path);
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

		// Sights zeroing distances
		ExtractZeroingDistances(comps, info.m_aZeroingDistances);

		// Intrinsic Muzzles
		ExtractMuzzles(comps, info.m_aMuzzles);

		// Intrinsic Attachment Slots
		ExtractAttachmentSlots(comps, info.m_aAttachmentSlots);

		m_aRifles.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	//! Introspection: AttachmentSlotComponent extraction with ancestor resolution & deduplication
	protected void ExtractAttachmentSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_RifleAttachmentSlotInfo> outSlots)
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
				foreach (TBD_RifleAttachmentSlotInfo existing : outSlots)
				{
					if (existing.m_sSlotName == slotName || (!typeClass.IsEmpty() && existing.m_sRequiredAttachmentType == typeClass))
					{
						duplicate = true;
						if (existing.m_sDefaultAttachedPrefab.IsEmpty() && !defAttach.IsEmpty())
							existing.m_sDefaultAttachedPrefab = ResolveCanonical(defAttach);
						break;
					}
				}

				if (!duplicate)
				{
					TBD_RifleAttachmentSlotInfo s = new TBD_RifleAttachmentSlotInfo();
					s.m_sSlotName = slotName;
					s.m_sRequiredAttachmentType = typeClass;
					if (!defAttach.IsEmpty())
						s.m_sDefaultAttachedPrefab = ResolveCanonical(defAttach);
					outSlots.Insert(s);
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspection: MuzzleComponent extraction with ancestor deduplication
	protected void ExtractMuzzles(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_RifleMuzzleInfo> outMuzzles)
	{
		int muzzleIdx = 0;
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MuzzleComponent"))
				continue;

			// Filter out containers that are ancestors of another container in bucket
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
				TBD_RifleMuzzleInfo m = new TBD_RifleMuzzleInfo();
				m.m_iIndex = muzzleIdx;
				m.m_sMuzzleClass = cls;

				// MagazineWell
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

						TBD_RifleFireModeInfo fmInfo = new TBD_RifleFireModeInfo();

						// RPM
						int rpm = 0;
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
							mode.GetAncestor().Get("RoundsPerMinute", rpm);
						fmInfo.m_iRoundsPerMinute = rpm;

						// Burst & MaxBurst
						int burst = 1;
						if (!mode.Get("BurstCount", burst))
							mode.Get("m_iBurstCount", burst);

						int maxBurst = 1;
						if (!mode.Get("MaxBurst", maxBurst) && mode.GetAncestor())
							mode.GetAncestor().Get("MaxBurst", maxBurst);
						if (maxBurst != 1)
							burst = maxBurst;
						fmInfo.m_iBurstCount = burst;

						// Name: read UIName first
						string modeName = "";
						if (!mode.Get("UIName", modeName) || modeName.IsEmpty())
						{
							if (mode.GetAncestor())
								mode.GetAncestor().Get("UIName", modeName);
						}

						if (modeName.IsEmpty())
						{
							if (!mode.Get("m_sFireModeName", modeName) || modeName.IsEmpty())
								mode.Get("m_sName", modeName);
						}

						if (modeName.IsEmpty() || modeName.StartsWith("#") || modeName.StartsWith("AR-"))
						{
							string confRef = mode.GetResourceName();
							if (confRef.IsEmpty() && mode.GetAncestor())
								confRef = mode.GetAncestor().GetResourceName();

							if (confRef.Contains("Single") || maxBurst == 1)
							{
								modeName = "Semi";
								fmInfo.m_iBurstCount = 1;
							}
							else if (confRef.Contains("Burst") || maxBurst > 1)
							{
								modeName = "Burst";
								if (fmInfo.m_iBurstCount <= 1)
									fmInfo.m_iBurstCount = 3;
							}
							else if (confRef.Contains("Auto") || maxBurst == -1)
							{
								modeName = "Full Auto";
								fmInfo.m_iBurstCount = 0;
							}
							else if (confRef.Contains("Safe") || rpm == 0)
							{
								modeName = "Safe";
								fmInfo.m_iBurstCount = 0;
							}
							else
							{
								modeName = "Mode " + (fm + 1).ToString();
							}
						}

						fmInfo.m_sName = modeName;
						m.m_aFireModes.Insert(fmInfo);
					}
				}

				outMuzzles.Insert(m);
				muzzleIdx++;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void ExtractZeroingDistances(map<string, ref array<BaseContainer>> comps, notnull array<string> outZeroings)
	{
		array<BaseContainer> sights = comps.Get("SightsComponent");
		if (!sights)
			sights = comps.Get("SCR_SightsComponent");

		if (!sights)
			return;

		foreach (BaseContainer sc : sights)
		{
			BaseContainer cur = sc;
			while (cur && outZeroings.IsEmpty())
			{
				BaseContainerList sightRanges = cur.GetObjectArray("SightsRanges");
				if (sightRanges)
				{
					for (int i = 0, n = sightRanges.Count(); i < n; i++)
					{
						BaseContainer sri = sightRanges.Get(i);
						if (!sri) continue;
						string rStr = "";
						if (sri.Get("Range", rStr) && !rStr.IsEmpty())
						{
							array<string> parts = {};
							rStr.Split(" ", parts, false);
							if (!parts.IsEmpty())
							{
								string lastPart = parts[parts.Count() - 1];
								int dist = lastPart.ToInt();
								if (dist > 0)
									outZeroings.Insert(dist.ToString() + "m");
								else
									outZeroings.Insert(lastPart + "m");
							}
						}
					}
				}

				if (outZeroings.IsEmpty())
				{
					array<float> ranges = {};
					cur.Get("m_aZeroingDistances", ranges);
					if (ranges)
					{
						foreach (float r : ranges)
							outZeroings.Insert(r.ToString() + "m");
					}
					else
					{
						float singleZero;
						if (cur.Get("m_fZeroingDistance", singleZero))
							outZeroings.Insert(singleZero.ToString() + "m");
					}
				}

				cur = cur.GetAncestor();
			}
		}
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
				if (disp.Get("Name", n) && !n.IsEmpty())
				{
					string cleaned = CleanLocalizationToken(n);
					if (!cleaned.IsEmpty())
						return cleaned;
				}
			}
		}

		array<BaseContainer> weaponComps = comps.Get("WeaponComponent");
		if (weaponComps)
		{
			foreach (BaseContainer wc : weaponComps)
			{
				BaseContainer ui = wc.GetObject("UIInfo");
				if (!ui)
					continue;
				string n2;
				if (ui.Get("Name", n2) && !n2.IsEmpty())
				{
					string cleaned2 = CleanLocalizationToken(n2);
					if (!cleaned2.IsEmpty())
						return cleaned2;
				}
			}
		}

		return HumanizeStem(filePath);
	}

	//------------------------------------------------------------------------------------------------
	protected string CleanLocalizationToken(string token)
	{
		if (!token.StartsWith("#") && !token.StartsWith("AR-"))
			return token;

		string s = token;
		if (s.StartsWith("#"))
			s = s.Substring(1, s.Length() - 1);
		if (s.StartsWith("AR-"))
			s = s.Substring(3, s.Length() - 3);

		if (s.StartsWith("Weapon_"))
			s = s.Substring(7, s.Length() - 7);
		else if (s.StartsWith("Item_"))
			s = s.Substring(5, s.Length() - 5);
		else if (s.StartsWith("Magazine_"))
			s = s.Substring(9, s.Length() - 9);

		if (s.EndsWith("_Name"))
			s = s.Substring(0, s.Length() - 5);

		s.Replace("_", " ");
		s.Trim();
		return s;
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

		if (stem.StartsWith("Rifle_"))
			stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Weapon_"))
			stem = stem.Substring(7, stem.Length() - 7);

		stem.Replace("_", " ");
		stem.Trim();
		return stem;
	}

	//------------------------------------------------------------------------------------------------
	protected string ExtractFamily(string filePath)
	{
		int riflesIdx = filePath.IndexOf("Weapons/Rifles/");
		if (riflesIdx >= 0)
		{
			string sub = filePath.Substring(riflesIdx + 15, filePath.Length() - riflesIdx - 15);
			int slash = sub.IndexOf("/");
			if (slash > 0)
				return sub.Substring(0, slash);
			return sub;
		}
		return "Rifle";
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
	//! Serialize entire rifles dataset to normalized JSON
	string SerializeToJson()
	{
		string json = "{\n";
		json += "  \"version\": \"1\",\n";
		json += "  \"category\": \"rifles\",\n";
		json += "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
		json += "  \"totalCount\": " + m_aRifles.Count().ToString() + ",\n";
		json += "  \"rifles\": [\n";

		for (int r = 0; r < m_aRifles.Count(); r++)
		{
			TBD_RifleWeaponInfo rif = m_aRifles[r];
			json += "    {\n";
			json += "      \"resource_name\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sResourceName) + "\",\n";
			json += "      \"id\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sId) + "\",\n";
			json += "      \"display_name\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sDisplayName) + "\",\n";
			json += "      \"family\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sFamily) + "\",\n";
			json += "      \"addon\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sAddonId) + "\",\n";
			json += "      \"file_path\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sFilePath) + "\",\n";

			string isAbsStr = "false";
			if (rif.m_bIsAbstract) isAbsStr = "true";
			json += "      \"is_abstract\": " + isAbsStr + ",\n";

			if (!rif.m_sVariantOf.IsEmpty())
				json += "      \"variant_of\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sVariantOf) + "\",\n";

			if (rif.m_fWeightKg >= 0)
				json += "      \"weight_kg\": " + rif.m_fWeightKg.ToString() + ",\n";
			if (rif.m_fVolumeCm3 >= 0)
				json += "      \"volume_cm3\": " + rif.m_fVolumeCm3.ToString() + ",\n";

			// Zeroing
			json += "      \"zeroing_distances\": [";
			for (int zd = 0; zd < rif.m_aZeroingDistances.Count(); zd++)
			{
				json += "\"" + rif.m_aZeroingDistances[zd] + "\"";
				if (zd < rif.m_aZeroingDistances.Count() - 1)
					json += ", ";
			}
			json += "],\n";

			// Muzzles
			json += "      \"muzzles\": [\n";
			for (int m = 0; m < rif.m_aMuzzles.Count(); m++)
			{
				TBD_RifleMuzzleInfo muz = rif.m_aMuzzles[m];
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
				for (int f = 0; f < muz.m_aFireModes.Count(); f++)
				{
					TBD_RifleFireModeInfo fmi = muz.m_aFireModes[f];
					json += "{\"name\":\"" + fmi.m_sName + "\",\"burst\":" + fmi.m_iBurstCount.ToString() + ",\"rpm\":" + fmi.m_iRoundsPerMinute.ToString() + "}";
					if (f < muz.m_aFireModes.Count() - 1)
						json += ", ";
				}
				json += "]\n";

				json += "        }";
				if (m < rif.m_aMuzzles.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += "      ],\n";

			// Attachment slots
			json += "      \"attachment_slots\": [\n";
			for (int s = 0; s < rif.m_aAttachmentSlots.Count(); s++)
			{
				TBD_RifleAttachmentSlotInfo slot = rif.m_aAttachmentSlots[s];
				json += "        {\n";
				json += "          \"slot_name\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sSlotName) + "\",\n";
				json += "          \"required_attachment_type\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sRequiredAttachmentType) + "\"";

				if (!slot.m_sDefaultAttachedPrefab.IsEmpty())
					json += ",\n          \"default_attachment\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sDefaultAttachedPrefab) + "\"\n";
				else
					json += ",\n          \"default_attachment\": null\n";

				json += "        }";
				if (s < rif.m_aAttachmentSlots.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += "      ]\n";

			json += "    }";
			if (r < m_aRifles.Count() - 1)
				json += ",";
			json += "\n";
		}

		json += "  ]\n}\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	int GetCount()
	{
		return m_aRifles.Count();
	}
}
