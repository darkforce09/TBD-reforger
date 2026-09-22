//------------------------------------------------------------------------------------------------
// TBD_AttachmentMountingExtractor.c
//
// Reads how an attachment fits a weapon: the attachment type it presents, every type it is
// compatible with, the types it obstructs, and the slots it offers to further attachments of its
// own.
//
// Compatibility is widened by walking the attachment type's inheritance chain, because a prefab
// declaring AttachmentOpticsRIS1913 also fits anything accepting its ancestors. Nested slots are
// what make a rail carry an optic that carries a magnifier.
//------------------------------------------------------------------------------------------------

class TBD_AttachmentMountingExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract mounting details: attachment_type, compatible_attachment_types, obstructed_attachment_types.
	static void ExtractMounting(map<string, ref array<BaseContainer>> comps, TBD_AttachmentMountingInfo outMounting, string filePath, string category)
	{
		string primaryType = "";
		array<string> compatTypes = {};
		array<string> obstructedTypes = {};

		// Inspect InventoryItemComponent -> Attributes -> CustomAttributes
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
						BaseContainerList attrList = TBD_AttachmentCustomAttributes.GetCustomAttributes(attrs);
						if (attrList)
						{
							for (int a = 0, an = attrList.Count(); a < an; a++)
							{
								BaseContainer attr = attrList.Get(a);
								if (!attr)
									continue;

								string attrCls = attr.GetClassName();

								// Weapon attachment attributes
								if (attrCls.Contains("AttachmentAttributes") || attrCls.Contains("WeaponAttachment"))
								{
									BaseContainer typeObj = attr.GetObject("AttachmentType");
									if (!typeObj)
										typeObj = attr.GetObject("m_AttachmentType");

									if (typeObj)
									{
										string rawTypeCls = typeObj.GetClassName();
										if (primaryType.IsEmpty() || primaryType == "AttachmentBase" || primaryType == "BaseAttachmentType")
										{
											primaryType = rawTypeCls;

											// Walk typeObj ancestor chain for full inheritance hierarchy
											BaseContainer ancType = typeObj.GetAncestor();
											while (ancType)
											{
												string ancCls = ancType.GetClassName();
												if (!ancCls.IsEmpty() && ancCls != "BaseAttachmentType" && ancCls != "AttachmentBase")
													TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, ancCls);
												ancType = ancType.GetAncestor();
											}
										}
									}

									// Read explicit compatible types if declared
									BaseContainerList cList = attr.GetObjectArray("m_aCompatibleAttachmentTypes");
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
								}

								// Weapon attachment obstruction attributes
								if (attrCls.Contains("ObstructionAttributes") || attrCls.Contains("WeaponAttachmentObstruction"))
								{
									BaseContainerList obsList = attr.GetObjectArray("m_aObstructedAttachmentTypes");
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
								}
							}
						}

						attrs = attrs.GetAncestor();
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// Direct check on non-slot components for AttachmentType if still empty
		if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType")
		{
			foreach (string compCls, array<BaseContainer> compBucket : comps)
			{
				if (compCls.EndsWith("AttachmentSlotComponent") || compCls.EndsWith("SlotManagerComponent") || compCls.EndsWith("WeaponComponent"))
					continue;

				foreach (BaseContainer c : compBucket)
				{
					BaseContainer atObj = c.GetObject("AttachmentType");
					if (atObj)
					{
						string cType = atObj.GetClassName();
						if (cType != "BaseAttachmentType")
						{
							primaryType = cType;
							break;
						}
					}
				}
				if (!primaryType.IsEmpty() && primaryType != "BaseAttachmentType")
					break;
			}
		}

		// Ensure primary type is present in compatible types
		if (!primaryType.IsEmpty() && compatTypes.Find(primaryType) == -1)
			compatTypes.InsertAt(primaryType, 0);

		// Resolve genuine inheritance chain via EnScript reflection
		BuildAttachmentAncestry(primaryType, compatTypes);

		outMounting.m_sAttachmentType = primaryType;
		outMounting.m_aCompatibleAttachmentTypes = compatTypes;
		outMounting.m_aObstructedAttachmentTypes = obstructedTypes;
	}

	//------------------------------------------------------------------------------------------------
	//! Resolve genuine inheritance hierarchy via EnScript reflection.
	protected static void BuildAttachmentAncestry(string primaryType, notnull array<string> compatTypes)
	{
		if (primaryType.IsEmpty())
			return;

		typename t = primaryType.ToType();
		if (!t)
			return;

		if (t != BaseAttachmentType && t.IsInherited(BaseAttachmentType))
		{
			if (t != AttachmentMuzzle && t.IsInherited(AttachmentMuzzle))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentMuzzle");
			if (t != AttachmentSuppressor && t.IsInherited(AttachmentSuppressor))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentSuppressor");
			if (t != AttachmentFlashHider && t.IsInherited(AttachmentFlashHider))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentFlashHider");
			if (t != AttachmentBayonet && t.IsInherited(AttachmentBayonet))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentBayonet");
			if (t != AttachmentStock && t.IsInherited(AttachmentStock))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentStock");
			if (t != AttachmentHandGuard && t.IsInherited(AttachmentHandGuard))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentHandGuard");
			if (t != AttachmentUnderBarrel && t.IsInherited(AttachmentUnderBarrel))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentUnderBarrel");
			if (t != AttachmentOptics && t.IsInherited(AttachmentOptics))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentOptics");
			if (t != AttachmentOpticsDovetail && t.IsInherited(AttachmentOpticsDovetail))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentOpticsDovetail");
			if (t != AttachmentOpticsRIS1913 && t.IsInherited(AttachmentOpticsRIS1913))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentOpticsRIS1913");
			if (t != AttachmentCamouflage && t.IsInherited(AttachmentCamouflage))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentCamouflage");
			if (t != AttachmentCamouflageOptics && t.IsInherited(AttachmentCamouflageOptics))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentCamouflageOptics");
			if (t != AttachmentRIS1913 && t.IsInherited(AttachmentRIS1913))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentRIS1913");
			if (t != AttachmentUnderbarrelRIS1913 && t.IsInherited(AttachmentUnderbarrelRIS1913))
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentUnderbarrelRIS1913");

			TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "BaseAttachmentType");
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract nested child attachment slots from AttachmentSlotComponent entries.
	static void ExtractNestedAttachmentSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_AttachmentSlotInfo> outSlots)
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

				if (typeClass.IsEmpty() && slotObj)
				{
					BaseContainer curS = slotObj;
					while (curS && typeClass.IsEmpty())
					{
						BaseContainer stObj = curS.GetObject("AttachmentType");
						if (stObj)
							typeClass = stObj.GetClassName();
						curS = curS.GetAncestor();
					}
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
						slotName = "nested_slot";
				}

				if (slotName.StartsWith("slot_"))
					slotName = slotName.Substring(5, slotName.Length() - 5);

				if (typeClass.IsEmpty() && slotName.IsEmpty())
					continue;

				// Obstruction list
				array<string> obstructedTypes = {};
				BaseContainer curObst = slotComp;
				while (curObst)
				{
					BaseContainerList attrList = TBD_AttachmentCustomAttributes.GetCustomAttributes(curObst);
					if (attrList)
					{
						for (int oi = 0, on = attrList.Count(); oi < on; oi++)
						{
							BaseContainer oa = attrList.Get(oi);
							if (!oa)
								continue;

							if (oa.GetClassName().Contains("ObstructionAttributes") || oa.GetClassName().Contains("WeaponAttachmentObstruction"))
							{
								BaseContainerList otList = oa.GetObjectArray("m_aObstructedAttachmentTypes");
								if (otList)
								{
									for (int oti = 0, otn = otList.Count(); oti < otn; oti++)
									{
										BaseContainer obc = otList.Get(oti);
										if (obc)
										{
											string ocName = obc.GetClassName();
											if (!ocName.IsEmpty() && obstructedTypes.Find(ocName) == -1)
												obstructedTypes.Insert(ocName);
										}
									}
								}
							}
						}
					}
					curObst = curObst.GetAncestor();
				}

				// Deduplicate
				bool duplicate = false;
				foreach (TBD_AttachmentSlotInfo existing : outSlots)
				{
					if (existing.m_sSlotName == slotName || (!typeClass.IsEmpty() && existing.m_sRequiredAttachmentType == typeClass))
					{
						duplicate = true;
						if (existing.m_sDefaultAttachedPrefab.IsEmpty() && !defAttach.IsEmpty())
							existing.m_sDefaultAttachedPrefab = TBD_EquipmentResourceNames.NormalizePathSeparators(defAttach);
						if (existing.m_sPivotId.IsEmpty() && !pivotId.IsEmpty())
							existing.m_sPivotId = pivotId;
						break;
					}
				}

				if (!duplicate)
				{
					TBD_AttachmentSlotInfo s = new TBD_AttachmentSlotInfo();
					s.m_sSlotName = slotName;
					s.m_sPivotId = pivotId;
					s.m_sRequiredAttachmentType = typeClass;
					if (!defAttach.IsEmpty())
						s.m_sDefaultAttachedPrefab = TBD_EquipmentResourceNames.NormalizePathSeparators(defAttach);
					s.m_aObstructedAttachmentTypes = obstructedTypes;
					outSlots.Insert(s);
				}
			}
		}
	}
}
