//------------------------------------------------------------------------------------------------
// TBD_HandguardMountingExtractor.c
//
// Reads how a handguard fits a weapon and what it carries in turn: the attachment type it
// presents, every type it is compatible with, the types it obstructs, and the rail slots it
// offers to further attachments.
//
// Nested slots matter more here than in any other domain - a rail handguard is the part that
// turns one attachment point into several, and each nested slot carries its own required type and
// any prefab already fitted to it.
//------------------------------------------------------------------------------------------------

class TBD_HandguardMountingExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract mounting relational keys directly from container data and genuine container ancestry:
	//!   - attachment_type: primary class name declared in AttachmentType / m_AttachmentType
	//!   - compatible_attachment_types: container ancestor classes and declared m_aCompatibleAttachmentTypes
	//!   - obstructed_attachment_types: classes from m_aObstructedAttachmentTypes
	//! ZERO hardcoded class checks: works dynamically for any mod.
	static void ExtractMounting(map<string, ref array<BaseContainer>> comps, TBD_HandguardMountingInfo outMounting)
	{
		string primaryType;
		ref array<string> compatTypes = {};
		ref array<string> obstructedTypes = {};

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("WeaponAttachmentComponent"))
				continue;

			foreach (BaseContainer inv : bucket)
			{
				BaseContainer cur = inv;
				while (cur)
				{
					BaseContainer attrs = cur.GetObject("Attributes");
					while (attrs)
					{
						BaseContainerList attrList = TBD_EquipmentDisplayAttributes.GetCustomAttributes(attrs);
						if (attrList)
						{
							for (int a = 0, an = attrList.Count(); a < an; a++)
							{
								BaseContainer attr = attrList.Get(a);
								if (!attr)
									continue;

								// Walk container ancestor chain to discover genuine attachment types and compatibility
								BaseContainer curAttr = attr;
								while (curAttr)
								{
									// 1. Primary AttachmentType and container ancestor classes
									BaseContainer typeObj = curAttr.GetObject("AttachmentType");
									if (!typeObj)
										typeObj = curAttr.GetObject("m_AttachmentType");

									if (typeObj)
									{
										string typeCls = typeObj.GetClassName();
										if (!typeCls.IsEmpty() && !typeCls.StartsWith("AttachmentCamouflage"))
										{
											if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType" || primaryType == "AttachmentBase" || primaryType == "AttachmentHandGuard")
												primaryType = typeCls;

											TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, typeCls);

											// Check if typeObj itself has an ancestor container
											BaseContainer ancType = typeObj.GetAncestor();
											while (ancType)
											{
												string ancCls = ancType.GetClassName();
												if (!ancCls.IsEmpty())
													TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, ancCls);
												ancType = ancType.GetAncestor();
											}
										}
									}

									// Read explicit compatible types if declared on container
									BaseContainerList cList = curAttr.GetObjectArray("m_aCompatibleAttachmentTypes");
									if (!cList)
										cList = curAttr.GetObjectArray("CompatibleAttachmentTypes");

									if (cList)
									{
										for (int c = 0, cn = cList.Count(); c < cn; c++)
										{
											BaseContainer ct = cList.Get(c);
											if (ct)
											{
												string ctCls = ct.GetClassName();
												if (!ctCls.IsEmpty())
													TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, ctCls);
											}
										}
									}

									// 2. Weapon attachment obstruction attributes
									BaseContainerList obsList = curAttr.GetObjectArray("m_aObstructedAttachmentTypes");
									if (!obsList)
										obsList = curAttr.GetObjectArray("ObstructedAttachmentTypes");

									if (obsList)
									{
										for (int o = 0, on = obsList.Count(); o < on; o++)
										{
											BaseContainer ot = obsList.Get(o);
											if (ot)
											{
												string otCls = ot.GetClassName();
												if (!otCls.IsEmpty())
													TBD_EquipmentComponentGraph.AddUniqueType(obstructedTypes, otCls);
											}
										}
									}

									curAttr = curAttr.GetAncestor();
								}
							}
						}

						attrs = attrs.GetAncestor();
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// Fallback check on non-slot components for direct AttachmentType if still empty
		if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType" || primaryType == "AttachmentHandGuard")
		{
			foreach (string compCls, array<BaseContainer> compBucket : comps)
			{
				if (compCls.EndsWith("AttachmentSlotComponent") || compCls.EndsWith("SlotManagerComponent"))
					continue;

				foreach (BaseContainer c : compBucket)
				{
					BaseContainer atObj = c.GetObject("AttachmentType");
					if (!atObj)
						atObj = c.GetObject("m_AttachmentType");

					if (atObj)
					{
						string cType = atObj.GetClassName();
						if (cType != "BaseAttachmentType" && !cType.IsEmpty() && !cType.StartsWith("AttachmentCamouflage"))
						{
							primaryType = cType;
							TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, cType);
							BaseContainer ancType = atObj.GetAncestor();
							while (ancType)
							{
								string ancCls = ancType.GetClassName();
								if (!ancCls.IsEmpty())
									TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, ancCls);
								ancType = ancType.GetAncestor();
							}
							break;
						}
					}
				}
				if (!primaryType.IsEmpty() && primaryType != "BaseAttachmentType" && primaryType != "AttachmentHandGuard")
					break;
			}
		}

		// Check script inheritance for base AttachmentHandGuard
		if (!primaryType.IsEmpty())
		{
			typename pt = primaryType.ToType();
			if (pt && pt != AttachmentHandGuard && pt.IsInherited(AttachmentHandGuard))
			{
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentHandGuard");
			}
		}

		// Ensure primary type is at index 0 of compatible types if present
		if (!primaryType.IsEmpty())
		{
			int idx = compatTypes.Find(primaryType);
			if (idx > 0)
			{
				compatTypes.Remove(idx);
				compatTypes.InsertAt(primaryType, 0);
			}
			else if (idx == -1)
			{
				compatTypes.InsertAt(primaryType, 0);
			}
		}

		outMounting.m_sAttachmentType = primaryType;
		outMounting.m_aCompatibleAttachmentTypes = compatTypes;
		outMounting.m_aObstructedAttachmentTypes = obstructedTypes;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract nested child attachment slots from AttachmentSlotComponent entries on modular handguards.
	//! Modular handguards host optics, underbarrel launchers, foregrips, bipods, and illuminators.
	static void ExtractNestedAttachmentSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_HandguardSlotInfo> outSlots)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("AttachmentSlotComponent"))
				continue;

			foreach (BaseContainer slotComp : bucket)
			{
				BaseContainer slotObj = slotComp.GetObject("AttachmentSlot");
				BaseContainer typeObj = slotComp.GetObject("AttachmentType");
				if (!typeObj)
					typeObj = slotComp.GetObject("m_AttachmentType");
				if (!typeObj && slotObj)
					typeObj = slotObj.GetObject("AttachmentType");

				// If typeObj was empty on child, resolve from ancestor component
				if (!typeObj)
				{
					BaseContainer curComp = slotComp.GetAncestor();
					while (curComp && !typeObj)
					{
						typeObj = curComp.GetObject("AttachmentType");
						if (!typeObj)
							typeObj = curComp.GetObject("m_AttachmentType");
						curComp = curComp.GetAncestor();
					}
				}

				string typeClass = "";
				if (typeObj)
					typeClass = typeObj.GetClassName();

				// Resolve exact slot name and pivot ID across slot container hierarchy
				string slotName = "";
				string pivotId = "";

				BaseContainer curSlot = slotObj;
				while (curSlot)
				{
					if (slotName.IsEmpty())
						slotName = curSlot.GetName();

					if (pivotId.IsEmpty())
						curSlot.Get("PivotID", pivotId);

					curSlot = curSlot.GetAncestor();
				}

				// If slot name is empty or engine base class name, fallback to pivot ID or type
				if (slotName.IsEmpty() || slotName.StartsWith("InventoryStorageSlot") || slotName.StartsWith("BaseAttachmentSlot") || slotName.StartsWith("SCR_WeaponAttachmentSlot"))
				{
					if (!pivotId.IsEmpty())
						slotName = pivotId;
					else if (!typeClass.IsEmpty())
						slotName = typeClass;
					else
						slotName = "Slot_Attachment";
				}

				// Introspect compatible attachment types accepted by this slot
				ref array<string> slotCompatTypes = {};
				if (typeObj)
				{
					if (!typeClass.IsEmpty())
					{
						TBD_EquipmentComponentGraph.AddUniqueType(slotCompatTypes, typeClass);

						BaseContainer ancType = typeObj.GetAncestor();
						while (ancType)
						{
							string ancCls = ancType.GetClassName();
							if (!ancCls.IsEmpty())
								TBD_EquipmentComponentGraph.AddUniqueType(slotCompatTypes, ancCls);
							ancType = ancType.GetAncestor();
						}
					}
				}

				// Check explicit compatible attachment types on slot container or component
				BaseContainer curCheck = slotComp;
				while (curCheck)
				{
					BaseContainerList cList = curCheck.GetObjectArray("m_aCompatibleAttachmentTypes");
					if (!cList)
						cList = curCheck.GetObjectArray("CompatibleAttachmentTypes");

					if (cList)
					{
						for (int c = 0, cn = cList.Count(); c < cn; c++)
						{
							BaseContainer ct = cList.Get(c);
							if (ct)
							{
								string ctCls = ct.GetClassName();
								if (!ctCls.IsEmpty())
									TBD_EquipmentComponentGraph.AddUniqueType(slotCompatTypes, ctCls);
							}
						}
					}
					curCheck = curCheck.GetAncestor();
				}

				if (slotObj)
				{
					BaseContainer curSlotCheck = slotObj;
					while (curSlotCheck)
					{
						BaseContainerList sList = curSlotCheck.GetObjectArray("m_aCompatibleAttachmentTypes");
						if (!sList)
							sList = curSlotCheck.GetObjectArray("CompatibleAttachmentTypes");

						if (sList)
						{
							for (int sc = 0, scn = sList.Count(); sc < scn; sc++)
							{
								BaseContainer sct = sList.Get(sc);
								if (sct)
								{
									string sctCls = sct.GetClassName();
									if (!sctCls.IsEmpty())
										TBD_EquipmentComponentGraph.AddUniqueType(slotCompatTypes, sctCls);
								}
							}
						}
						curSlotCheck = curSlotCheck.GetAncestor();
					}
				}

				// Deduplicate child slots across components (merge compatible types if same slot name)
				bool duplicate = false;
				foreach (TBD_HandguardSlotInfo existing : outSlots)
				{
					if (existing.m_sSlotName == slotName)
					{
						duplicate = true;
						foreach (string nct : slotCompatTypes)
						{
							TBD_EquipmentComponentGraph.AddUniqueType(existing.m_aCompatibleAttachmentTypes, nct);
						}
						break;
					}
				}

				if (!duplicate)
				{
					TBD_HandguardSlotInfo s = new TBD_HandguardSlotInfo();
					s.m_sSlotName = slotName;
					s.m_aCompatibleAttachmentTypes = slotCompatTypes;
					outSlots.Insert(s);
				}
			}
		}
	}
}
