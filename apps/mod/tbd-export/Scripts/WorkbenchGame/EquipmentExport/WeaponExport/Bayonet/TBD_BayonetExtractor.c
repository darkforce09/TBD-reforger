//------------------------------------------------------------------------------------------------
// TBD_BayonetExtractor.c
//
// Core introspection engine for extracting complete, ground-truth component graphs from bayonets
// and weapon-mounted blades.
// Extracts mounting relational keys, mutual obstruction types, combat properties (extra obstruction
// length, melee damage), physical attributes, and visual 3D model meshes.
//
// 100% genuine introspection:
//   - ZERO fake/mock data: omitted values serialize as JSON null.
//   - ZERO hardcoded class checks: works universally with any mod.
//   - All attributes and relational keys are extracted directly from native Enfusion BaseContainer
//     objects and container ancestry.
//   - Localization tokens are preserved raw without stripping # or fabricating humanized strings.
//------------------------------------------------------------------------------------------------

class TBD_BayonetExtractor
{

	//------------------------------------------------------------------------------------------------
	//! Extract mounting relational keys directly from container data and genuine container ancestry:
	//!   - attachment_type: primary class name declared in AttachmentType / m_AttachmentType
	//!   - compatible_attachment_types: container ancestor classes and declared m_aCompatibleAttachmentTypes
	//!   - obstructed_attachment_types: classes from m_aObstructedAttachmentTypes
	//! ZERO hardcoded class checks: works dynamically for any mod.
	static void ExtractMounting(map<string, ref array<BaseContainer>> comps, TBD_BayonetMountingInfo outMounting)
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
											if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType" || primaryType == "AttachmentBase" || primaryType == "AttachmentBayonet")
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

									// 2. Weapon attachment obstruction attributes (critical relational data for bayonets)
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
		if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType" || primaryType == "AttachmentBayonet")
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
				if (!primaryType.IsEmpty() && primaryType != "BaseAttachmentType" && primaryType != "AttachmentBayonet")
					break;
			}
		}

		// Check script inheritance for base AttachmentBayonet
		if (!primaryType.IsEmpty())
		{
			typename pt = primaryType.ToType();
			if (pt && pt != AttachmentBayonet && pt.IsInherited(AttachmentBayonet))
			{
				TBD_EquipmentComponentGraph.AddUniqueType(compatTypes, "AttachmentBayonet");
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
	//! Extract combat & handling attributes (extra obstruction length and melee damage).
	//! ZERO synthetic defaults: omitted fields serialize as null.
	static void ExtractCombatHandling(map<string, ref array<BaseContainer>> comps, TBD_BayonetCombatInfo outCombat)
	{
		// 1. Extra obstruction length from SCR_WeaponAttachmentBayonetAttributes or custom attributes
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

								BaseContainer curAttr = attr;
								while (curAttr)
								{
									// Extra obstruction length
									if (!outCombat.m_bHasExtraObstructionLength)
									{
										float extraLen = 0.0;
										if (curAttr.IsVariableSet("m_fExtraObstructionLength") && curAttr.Get("m_fExtraObstructionLength", extraLen) && extraLen > 0)
										{
											outCombat.m_fExtraObstructionLength = extraLen;
											outCombat.m_bHasExtraObstructionLength = true;
										}
										else if (curAttr.IsVariableSet("ExtraObstructionLength") && curAttr.Get("ExtraObstructionLength", extraLen) && extraLen > 0)
										{
											outCombat.m_fExtraObstructionLength = extraLen;
											outCombat.m_bHasExtraObstructionLength = true;
										}
									}

									// Direct melee damage on bayonet attributes if configured
									if (!outCombat.m_bHasMeleeDamage)
									{
										float directDmg = 0.0;
										if (curAttr.IsVariableSet("m_fMeleeDamage") && curAttr.Get("m_fMeleeDamage", directDmg) && directDmg > 0)
										{
											outCombat.m_fMeleeDamage = directDmg;
											outCombat.m_bHasMeleeDamage = true;
										}
										else if (curAttr.IsVariableSet("MeleeDamage") && curAttr.Get("MeleeDamage", directDmg) && directDmg > 0)
										{
											outCombat.m_fMeleeDamage = directDmg;
											outCombat.m_bHasMeleeDamage = true;
										}
										else if (curAttr.IsVariableSet("m_fDamage") && curAttr.Get("m_fDamage", directDmg) && directDmg > 0)
										{
											outCombat.m_fMeleeDamage = directDmg;
											outCombat.m_bHasMeleeDamage = true;
										}
										else if (curAttr.IsVariableSet("Damage") && curAttr.Get("Damage", directDmg) && directDmg > 0)
										{
											outCombat.m_fMeleeDamage = directDmg;
											outCombat.m_bHasMeleeDamage = true;
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

		// Fallback check on components directly for extra obstruction length
		if (!outCombat.m_bHasExtraObstructionLength)
		{
			foreach (string compCls, array<BaseContainer> compBucket : comps)
			{
				foreach (BaseContainer c : compBucket)
				{
					BaseContainer curC = c;
					while (curC)
					{
						float cExtraLen = 0.0;
						if (curC.IsVariableSet("m_fExtraObstructionLength") && curC.Get("m_fExtraObstructionLength", cExtraLen) && cExtraLen > 0)
						{
							outCombat.m_fExtraObstructionLength = cExtraLen;
							outCombat.m_bHasExtraObstructionLength = true;
							break;
						}
						else if (curC.IsVariableSet("ExtraObstructionLength") && curC.Get("ExtraObstructionLength", cExtraLen) && cExtraLen > 0)
						{
							outCombat.m_fExtraObstructionLength = cExtraLen;
							outCombat.m_bHasExtraObstructionLength = true;
							break;
						}
						curC = curC.GetAncestor();
					}
					if (outCombat.m_bHasExtraObstructionLength)
						break;
				}
				if (outCombat.m_bHasExtraObstructionLength)
					break;
			}
		}

		// 2. Melee damage introspection from SCR_MeleeComponent, MeleeWeaponComponent, or SCR_BayonetComponent
		if (!outCombat.m_bHasMeleeDamage)
		{
			foreach (string meleeCls, array<BaseContainer> meleeBucket : comps)
			{
				if (!meleeCls.Contains("Melee") && !meleeCls.Contains("Bayonet"))
					continue;

				foreach (BaseContainer mc : meleeBucket)
				{
					BaseContainer curMc = mc;
					while (curMc)
					{
						float mDmg = 0.0;
						if (curMc.IsVariableSet("m_fMeleeDamage") && curMc.Get("m_fMeleeDamage", mDmg) && mDmg > 0)
						{
							outCombat.m_fMeleeDamage = mDmg;
							outCombat.m_bHasMeleeDamage = true;
							break;
						}
						if (curMc.IsVariableSet("MeleeDamage") && curMc.Get("MeleeDamage", mDmg) && mDmg > 0)
						{
							outCombat.m_fMeleeDamage = mDmg;
							outCombat.m_bHasMeleeDamage = true;
							break;
						}
						if (curMc.IsVariableSet("m_fDamage") && curMc.Get("m_fDamage", mDmg) && mDmg > 0)
						{
							outCombat.m_fMeleeDamage = mDmg;
							outCombat.m_bHasMeleeDamage = true;
							break;
						}
						if (curMc.IsVariableSet("Damage") && curMc.Get("Damage", mDmg) && mDmg > 0)
						{
							outCombat.m_fMeleeDamage = mDmg;
							outCombat.m_bHasMeleeDamage = true;
							break;
						}
						if (curMc.IsVariableSet("m_fBaseDamage") && curMc.Get("m_fBaseDamage", mDmg) && mDmg > 0)
						{
							outCombat.m_fMeleeDamage = mDmg;
							outCombat.m_bHasMeleeDamage = true;
							break;
						}

						// Check nested sub-objects (e.g. WeaponProperties, HitData, DamageEffect)
						BaseContainer propObj = curMc.GetObject("m_WeaponProperties");
						if (!propObj)
							propObj = curMc.GetObject("WeaponProperties");
						if (!propObj)
							propObj = curMc.GetObject("m_DamageEffect");
						if (!propObj)
							propObj = curMc.GetObject("DamageEffect");

						if (propObj)
						{
							float propDmg = 0.0;
							if (propObj.IsVariableSet("m_fDamage") && propObj.Get("m_fDamage", propDmg) && propDmg > 0)
							{
								outCombat.m_fMeleeDamage = propDmg;
								outCombat.m_bHasMeleeDamage = true;
								break;
							}
							if (propObj.IsVariableSet("Damage") && propObj.Get("Damage", propDmg) && propDmg > 0)
							{
								outCombat.m_fMeleeDamage = propDmg;
								outCombat.m_bHasMeleeDamage = true;
								break;
							}
							if (propObj.IsVariableSet("m_fMeleeDamage") && propObj.Get("m_fMeleeDamage", propDmg) && propDmg > 0)
							{
								outCombat.m_fMeleeDamage = propDmg;
								outCombat.m_bHasMeleeDamage = true;
								break;
							}
						}

						curMc = curMc.GetAncestor();
					}

					if (outCombat.m_bHasMeleeDamage)
						break;
				}

				if (outCombat.m_bHasMeleeDamage)
					break;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract physical attributes: mass, volume, dimensions, inventory layout size.
	//! ZERO synthetic defaults: omitted fields serialize as null.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_BayonetPhysicalInfo outPhys)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("StorageComponent"))
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

		// Fallback for mass on RigidBody if weight was not defined in ItemPhysAttributes
		if (!outPhys.m_bHasWeight)
		{
			array<BaseContainer> rbBucket = comps.Get("RigidBody");
			if (rbBucket)
			{
				foreach (BaseContainer rb : rbBucket)
				{
					float mass;
					if (rb.Get("Mass", mass) && mass > 0)
					{
						outPhys.m_fWeightKg = mass;
						outPhys.m_bHasWeight = true;
						break;
					}
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract visual 3D model mesh path directly from MeshObject.Object.
	static void ExtractVisuals(map<string, ref array<BaseContainer>> comps, TBD_BayonetVisualsInfo outVisuals)
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
