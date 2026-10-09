//------------------------------------------------------------------------------------------------
// TBD_M16Extractor.c
//
// Reads what one M16 variant intrinsically declares: its muzzles with their magazine wells,
// default magazine and fire modes; its attachment slots with the type each requires; and the
// zeroing distances its sights offer.
//
// This fills the variant's own carrier. Matching it against the magazine and attachment pool is a
// separate step, and the values read here are what that match is made on.
//------------------------------------------------------------------------------------------------

class TBD_M16Extractor
{
	//------------------------------------------------------------------------------------------------
	//! Introspection: MuzzleComponent extraction
	static void ExtractMuzzles(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_M16MuzzleInfo> outMuzzles)
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
	//! Introspection: AttachmentSlotComponent extraction
	static void ExtractAttachmentSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_M16AttachmentSlotInfo> outSlots)
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
							existing.m_sDefaultAttachedPrefab = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(defAttach);
						break;
					}
				}

				if (!duplicate)
				{
					TBD_M16AttachmentSlotInfo s = new TBD_M16AttachmentSlotInfo();
					s.m_sSlotName = slotName;
					s.m_sRequiredAttachType = typeClass;
					if (!defAttach.IsEmpty())
						s.m_sDefaultAttachedPrefab = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(defAttach);
					outSlots.Insert(s);
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspection: SightsComponent zeroing distances
	static void ExtractZeroingDistances(map<string, ref array<BaseContainer>> comps, notnull array<string> outZeroings)
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
}
