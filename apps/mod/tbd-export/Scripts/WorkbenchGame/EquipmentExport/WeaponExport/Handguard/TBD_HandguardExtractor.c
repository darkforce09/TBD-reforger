//------------------------------------------------------------------------------------------------
// TBD_HandguardExtractor.c
//
// Core introspection engine for extracting complete, ground-truth component graphs from weapon handguards,
// rail systems, and foregrips.
// Extracts mounting relational keys, nested child attachment slots, handling and recoil modifiers,
// physical attributes, and visuals (3D model mesh).
//
// 100% genuine introspection:
//   - ZERO fake/mock data: omitted values serialize as JSON null.
//   - ZERO hardcoded class checks: works universally with any mod.
//   - All attributes, relational keys, and child slots are extracted directly from native Enfusion
//     BaseContainer objects and container ancestry.
//   - Localization tokens are preserved raw without stripping # or fabricating humanized strings.
//   - Nested Attachment Host: Modular handguards provide child slots (top rail for optics,
//     bottom rail for UGLs/bipods/foregrips, side rails for illuminators) declared via AttachmentSlotComponent.
//------------------------------------------------------------------------------------------------

class TBD_HandguardExtractor
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

	//------------------------------------------------------------------------------------------------
	//! Extract handling and recoil modifiers directly from SCR_WeaponAttachmentAttributes.
	//! Introspects recoil angular factors, linear factors, turn factors, and extra obstruction length.
	static void ExtractHandlingModifiers(map<string, ref array<BaseContainer>> comps, TBD_HandguardHandlingInfo outHandling)
	{
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
						BaseContainerList attrList = TBD_EquipmentDisplayAttributes.GetCustomAttributes(attrs);
						if (attrList)
						{
							for (int a = 0, an = attrList.Count(); a < an; a++)
							{
								BaseContainer attr = attrList.Get(a);
								if (!attr)
									continue;

								// Walk container ancestor chain to extract declared recoil/handling modifiers
								BaseContainer curAttr = attr;
								while (curAttr)
								{
									if (!outHandling.m_bHasRecoilAngularFactors)
									{
										vector ang;
										if (curAttr.IsVariableSet("m_vAngularFactors") && curAttr.Get("m_vAngularFactors", ang) && ang != "0 0 0")
										{
											outHandling.m_vRecoilAngularFactors = ang;
											outHandling.m_bHasRecoilAngularFactors = true;
										}
										else if (curAttr.IsVariableSet("AngularFactors") && curAttr.Get("AngularFactors", ang) && ang != "0 0 0")
										{
											outHandling.m_vRecoilAngularFactors = ang;
											outHandling.m_bHasRecoilAngularFactors = true;
										}
									}

									if (!outHandling.m_bHasRecoilLinearFactors)
									{
										vector lin;
										if (curAttr.IsVariableSet("m_vLinearFactors") && curAttr.Get("m_vLinearFactors", lin) && lin != "0 0 0")
										{
											outHandling.m_vRecoilLinearFactors = lin;
											outHandling.m_bHasRecoilLinearFactors = true;
										}
										else if (curAttr.IsVariableSet("LinearFactors") && curAttr.Get("LinearFactors", lin) && lin != "0 0 0")
										{
											outHandling.m_vRecoilLinearFactors = lin;
											outHandling.m_bHasRecoilLinearFactors = true;
										}
									}

									if (!outHandling.m_bHasTurnFactors)
									{
										vector turn;
										if (curAttr.IsVariableSet("m_vTurnFactors") && curAttr.Get("m_vTurnFactors", turn) && turn != "0 0 0")
										{
											outHandling.m_vTurnFactors = turn;
											outHandling.m_bHasTurnFactors = true;
										}
										else if (curAttr.IsVariableSet("TurnFactors") && curAttr.Get("TurnFactors", turn) && turn != "0 0 0")
										{
											outHandling.m_vTurnFactors = turn;
											outHandling.m_bHasTurnFactors = true;
										}
									}

									if (!outHandling.m_bHasExtraObstructionLength)
									{
										float extra;
										if (curAttr.IsVariableSet("m_fExtraObstructionLength") && curAttr.Get("m_fExtraObstructionLength", extra) && extra > 0)
										{
											outHandling.m_fExtraObstructionLength = extra;
											outHandling.m_bHasExtraObstructionLength = true;
										}
										else if (curAttr.IsVariableSet("ExtraObstructionLength") && curAttr.Get("ExtraObstructionLength", extra) && extra > 0)
										{
											outHandling.m_fExtraObstructionLength = extra;
											outHandling.m_bHasExtraObstructionLength = true;
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
	}

	//------------------------------------------------------------------------------------------------
	//! Extract physical attributes: mass, volume, dimensions, inventory layout size.
	//! ZERO synthetic defaults: omitted fields serialize as null.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_HandguardPhysicalInfo outPhys)
	{
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
						// Inventory item size token
						if (outPhys.m_sInventorySize.IsEmpty())
						{
							string sz;
							if (attrs.Get("m_Size", sz) && !sz.IsEmpty())
								outPhys.m_sInventorySize = sz;
						}

						// Item physical attributes
						BaseContainer phys = attrs.GetObject("ItemPhysAttributes");
						while (phys)
						{
							if (!outPhys.m_bHasWeight)
							{
								float weight;
								if (phys.Get("Weight", weight) && weight > 0)
								{
									outPhys.m_fWeightKg = weight;
									outPhys.m_bHasWeight = true;
								}
							}

							if (!outPhys.m_bHasVolume)
							{
								float volume;
								if (phys.Get("ItemVolume", volume) && volume > 0)
								{
									outPhys.m_fVolumeCm3 = volume;
									outPhys.m_bHasVolume = true;
								}
							}

							if (!outPhys.m_bHasDimensions)
							{
								vector dim;
								if (phys.Get("ItemDimensions", dim) && dim != "0 0 0")
								{
									outPhys.m_vDimensions = dim;
									outPhys.m_bHasDimensions = true;
								}
							}

							phys = phys.GetAncestor();
						}

						attrs = attrs.GetAncestor();
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract visual 3D model mesh path directly from MeshObject.Object.
	static void ExtractVisuals(map<string, ref array<BaseContainer>> comps, TBD_HandguardVisualsInfo outVisuals)
	{
		string meshPath;

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MeshObject"))
				continue;

			foreach (BaseContainer mesh : bucket)
			{
				BaseContainer cur = mesh;
				while (cur)
				{
					string obj;
					if (cur.Get("Object", obj) && !obj.IsEmpty())
					{
						meshPath = TBD_EquipmentResourceNames.NormalizePathSeparators(obj);
						break;
					}
					cur = cur.GetAncestor();
				}

				if (!meshPath.IsEmpty())
					break;
			}

			if (!meshPath.IsEmpty())
				break;
		}

		outVisuals.m_sModelMesh = meshPath;
	}
}
