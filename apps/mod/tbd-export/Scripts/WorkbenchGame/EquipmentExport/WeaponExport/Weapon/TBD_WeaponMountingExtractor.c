//------------------------------------------------------------------------------------------------
// TBD_WeaponMountingExtractor.c
//
// Reads the attachment slots a weapon declares: what each slot is called, the attachment type it
// requires, the prefab already fitted to it, and the types it blocks other slots from taking.
//
// Slots are deduplicated by name and required type, because the same slot is frequently declared
// again on a variant prefab. Obstruction lists are how the engine expresses that a fitted
// attachment denies a neighbouring slot.
//------------------------------------------------------------------------------------------------

class TBD_WeaponMountingExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract attachment slots, mount types, pre-attached prefabs, and obstruction rules.
	static void ExtractAttachmentSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_WeaponAttachmentSlotInfo> outSlots)
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

				// Resolve attachment type class from ancestor slot component if missing
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

				// Clean human-friendly slot name
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

				// Obstruction check: extract blocked attachment classes from SCR_WeaponAttachmentObstructionAttributes
				array<string> obstructedTypes = {};
				ExtractObstructions(slotComp, obstructedTypes);

				// Deduplicate and merge defaults
				bool duplicate = false;
				foreach (TBD_WeaponAttachmentSlotInfo existing : outSlots)
				{
					if (existing.m_sSlotName == slotName || (!typeClass.IsEmpty() && existing.m_sRequiredAttachmentType == typeClass))
					{
						duplicate = true;
						if (existing.m_sDefaultAttachedPrefab.IsEmpty() && !defAttach.IsEmpty())
							existing.m_sDefaultAttachedPrefab = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(defAttach);
						if (existing.m_sPivotId.IsEmpty() && !pivotId.IsEmpty())
							existing.m_sPivotId = pivotId;

						foreach (string obs : obstructedTypes)
						{
							if (existing.m_aObstructedAttachmentTypes.Find(obs) == -1)
								existing.m_aObstructedAttachmentTypes.Insert(obs);
						}
						break;
					}
				}

				if (!duplicate)
				{
					TBD_WeaponAttachmentSlotInfo s = new TBD_WeaponAttachmentSlotInfo();
					s.m_sSlotName = slotName;
					s.m_sPivotId = pivotId;
					s.m_sRequiredAttachmentType = typeClass;
					if (!defAttach.IsEmpty())
						s.m_sDefaultAttachedPrefab = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(defAttach);
					s.m_aObstructedAttachmentTypes = obstructedTypes;
					outSlots.Insert(s);
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static void ExtractObstructions(BaseContainer slotComp, notnull array<string> outObstructed)
	{
		BaseContainer cur = slotComp;
		while (cur)
		{
			BaseContainer customAttrs = cur.GetObject("CustomAttributes");
			if (customAttrs)
			{
				BaseContainerList attrList = customAttrs.GetObjectArray("m_aAttributes");
				if (attrList)
				{
					for (int i = 0, n = attrList.Count(); i < n; i++)
					{
						BaseContainer attr = attrList.Get(i);
						if (!attr || !attr.GetClassName().Contains("Obstruction"))
							continue;

						BaseContainerList obsList = attr.GetObjectArray("m_aObstructedAttachmentTypes");
						if (obsList)
						{
							for (int j = 0, jn = obsList.Count(); j < jn; j++)
							{
								BaseContainer obsObj = obsList.Get(j);
								if (obsObj)
								{
									string obsCls = obsObj.GetClassName();
									if (!obsCls.IsEmpty() && outObstructed.Find(obsCls) == -1)
										outObstructed.Insert(obsCls);
								}
							}
						}
					}
				}
			}
			cur = cur.GetAncestor();
		}
	}
}
