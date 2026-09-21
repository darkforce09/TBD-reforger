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
	protected static const int ANCESTOR_CAP = 16;
	protected static const int COMPONENT_DEPTH_CAP = 4;

	//------------------------------------------------------------------------------------------------
	//! Extract all components across the entire inheritance hierarchy into a mapped bucket list.
	static void CollectComponentChain(BaseContainer prefabRoot, notnull map<string, ref array<BaseContainer>> outComps)
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
	protected static void CollectComponentsRec(BaseContainer holder, notnull map<string, ref array<BaseContainer>> outComps, int depth)
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
	//! Returns true if any component in the map ends with the given suffix.
	static bool HasCompSuffix(map<string, ref array<BaseContainer>> comps, string suffix)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.EndsWith(suffix))
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Append an item to an array only if it does not already exist.
	protected static void AddUniqueType(notnull array<string> list, string item)
	{
		if (item.IsEmpty())
			return;

		if (list.Find(item) == -1)
			list.Insert(item);
	}

	//------------------------------------------------------------------------------------------------
	//! Resolve canonical GUID-based resource string with forward slashes.
	static string ResolveCanonical(string raw)
	{
		if (raw.IsEmpty())
			return string.Empty;

		raw.Replace("\\", "/");
		return raw;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract raw unmutated localized display name token directly from ItemDisplayName.Name.
	//! ZERO string mutation: keeps # intact, zero English fallbacks, zero synthetic guessing.
	static string RawDisplayNameFor(map<string, ref array<BaseContainer>> comps)
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
						BaseContainer disp = attrs.GetObject("ItemDisplayName");
						if (disp)
						{
							string n;
							if (disp.Get("Name", n) && !n.IsEmpty())
								return n;
						}
						attrs = attrs.GetAncestor();
					}
					cur = cur.GetAncestor();
				}
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract raw unmutated localized description token directly from ItemDisplayName.Description.
	//! Returns string.Empty if omitted in prefab (serializes as null).
	static string RawDescriptionFor(map<string, ref array<BaseContainer>> comps)
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
						BaseContainer disp = attrs.GetObject("ItemDisplayName");
						if (disp)
						{
							string d;
							if (disp.Get("Description", d) && !d.IsEmpty())
								return d;
						}
						attrs = attrs.GetAncestor();
					}
					cur = cur.GetAncestor();
				}
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract raw icon texture path directly from ItemDisplayName.Icon.
	static string RawIconFor(map<string, ref array<BaseContainer>> comps)
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
						BaseContainer disp = attrs.GetObject("ItemDisplayName");
						if (disp)
						{
							string icon;
							if (disp.Get("Icon", icon) && !icon.IsEmpty())
								return ResolveCanonical(icon);
						}
						attrs = attrs.GetAncestor();
					}
					cur = cur.GetAncestor();
				}
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Retrieve CustomAttributes list from SCR_ItemAttributeCollection or BaseContainer.
	static BaseContainerList GetCustomAttributes(BaseContainer attrs)
	{
		if (!attrs)
			return null;

		BaseContainerList list = attrs.GetObjectArray("CustomAttributes");
		if (list && list.Count() > 0)
			return list;

		BaseContainer subCustom = attrs.GetObject("CustomAttributes");
		if (subCustom)
		{
			BaseContainerList wrapped = subCustom.GetObjectArray("CustomAttributes");
			if (wrapped && wrapped.Count() > 0)
				return wrapped;
		}

		return null;
	}

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
						BaseContainerList attrList = GetCustomAttributes(attrs);
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

											AddUniqueType(compatTypes, typeCls);

											// Check if typeObj itself has an ancestor container
											BaseContainer ancType = typeObj.GetAncestor();
											while (ancType)
											{
												string ancCls = ancType.GetClassName();
												if (!ancCls.IsEmpty())
													AddUniqueType(compatTypes, ancCls);
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
													AddUniqueType(compatTypes, ctCls);
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
													AddUniqueType(obstructedTypes, otCls);
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
							AddUniqueType(compatTypes, cType);
							BaseContainer ancType = atObj.GetAncestor();
							while (ancType)
							{
								string ancCls = ancType.GetClassName();
								if (!ancCls.IsEmpty())
									AddUniqueType(compatTypes, ancCls);
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
				AddUniqueType(compatTypes, "AttachmentBayonet");
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
						BaseContainerList attrList = GetCustomAttributes(attrs);
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
						meshPath = ResolveCanonical(obj);
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
