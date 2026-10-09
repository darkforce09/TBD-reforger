//------------------------------------------------------------------------------------------------
// TBD_RifleExtractor.c
//
// Reads what one rifle prefab intrinsically declares: its muzzles with their magazine wells,
// default magazine and fire modes; its attachment slots with the type each requires and any prefab
// already fitted; the zeroing distances its sights offer; and its mass and volume.
//
// Intrinsic only. Nothing here resolves a rifle against the magazine or attachment catalogs - the
// magazine well and required attachment type are exported as foreign keys for the platform to link
// against. Muzzle containers that are ancestors of another container in the same bucket are
// discarded, because a variant prefab re-declares the muzzle it inherits.
//------------------------------------------------------------------------------------------------

class TBD_RifleExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Introspection: MuzzleComponent extraction with ancestor deduplication
	static void ExtractMuzzles(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_RifleMuzzleInfo> outMuzzles)
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
						m.m_sDefaultMagazineTemplate = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(magTmpl);
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
	//! Introspection: AttachmentSlotComponent extraction with ancestor resolution & deduplication
	static void ExtractAttachmentSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_RifleAttachmentSlotInfo> outSlots)
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
							existing.m_sDefaultAttachedPrefab = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(defAttach);
						break;
					}
				}

				if (!duplicate)
				{
					TBD_RifleAttachmentSlotInfo s = new TBD_RifleAttachmentSlotInfo();
					s.m_sSlotName = slotName;
					s.m_sRequiredAttachmentType = typeClass;
					if (!defAttach.IsEmpty())
						s.m_sDefaultAttachedPrefab = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(defAttach);
					outSlots.Insert(s);
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	static void ExtractZeroingDistances(map<string, ref array<BaseContainer>> comps, notnull array<string> outZeroings)
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
}
