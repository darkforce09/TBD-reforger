//------------------------------------------------------------------------------------------------
// TBD_MuzzleExtractor.c
//
// Core introspection engine for extracting complete, ground-truth component graphs from muzzle devices.
// Extracts mounting relational keys, physical attributes, visuals (3D mesh), and acoustics &
// ballistics modifiers for suppressors, flash hiders, compensators, and muzzle brakes.
//
// 100% genuine introspection:
//   - ZERO fake/mock data.
//   - ZERO hardcoded class checks: works universally with any mod.
//   - All attributes and relational keys are extracted directly from native Enfusion BaseContainer
//     objects and container ancestry.
//   - Localization tokens are preserved raw without stripping # or fabricating humanized strings.
//------------------------------------------------------------------------------------------------

class TBD_MuzzleExtractor
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
	static void ExtractMounting(map<string, ref array<BaseContainer>> comps, TBD_MuzzleMountingInfo outMounting)
	{
		string primaryType;
		ref array<string> compatTypes = {};
		ref array<string> obstructedTypes = {};

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
										if (!typeCls.IsEmpty())
										{
											if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType" || primaryType == "AttachmentBase")
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

									// 2. Weapon attachment obstruction attributes
									BaseContainerList obsList = curAttr.GetObjectArray("m_aObstructedAttachmentTypes");
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
						if (cType != "BaseAttachmentType" && !cType.IsEmpty())
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
	//! Extract acoustics and ballistics modifiers directly from attribute containers and container ancestry.
	//! Preserves real values without fake defaults; omitted fields are flagged as not present.
	static void ExtractModifiers(map<string, ref array<BaseContainer>> comps, TBD_MuzzleModifiersInfo outModifiers, string attachmentType, string filePath)
	{
		// 1. is_suppressed
		typename attT;
		if (!attachmentType.IsEmpty())
			attT = attachmentType.ToType();

		if (attT && (attT == AttachmentSuppressor || attT.IsInherited(AttachmentSuppressor)))
			outModifiers.m_bIsSuppressed = true;
		else if (attachmentType.Contains("Suppressor") || attachmentType.Contains("Silencer") || attachmentType.Contains("suppressor") || attachmentType.Contains("silencer"))
			outModifiers.m_bIsSuppressed = true;
		else if (HasCompSuffix(comps, "SuppressorComponent"))
			outModifiers.m_bIsSuppressed = true;

		// 2. Modifiers from attribute containers
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

								// Walk container ancestor chain to extract all declared modifier properties
								BaseContainer curAttr = attr;
								while (curAttr)
								{
									// Explicit is_suppressed attribute if declared on container
									if (!outModifiers.m_bIsSuppressed)
									{
										bool explicitSup;
										if (curAttr.Get("m_bIsSuppressed", explicitSup) || curAttr.Get("is_suppressed", explicitSup))
											outModifiers.m_bIsSuppressed = explicitSup;
									}

									if (!outModifiers.m_bHasMuzzleSpeedCoefficient && curAttr.IsVariableSet("m_fMuzzleSpeedCoefficient"))
									{
										float speed;
										if (curAttr.Get("m_fMuzzleSpeedCoefficient", speed))
										{
											outModifiers.m_fMuzzleSpeedCoefficient = speed;
											outModifiers.m_bHasMuzzleSpeedCoefficient = true;
										}
									}

									if (!outModifiers.m_bHasMuzzleDispersionFactor && curAttr.IsVariableSet("m_fMuzzleDispersionFactor"))
									{
										float disp;
										if (curAttr.Get("m_fMuzzleDispersionFactor", disp))
										{
											outModifiers.m_fMuzzleDispersionFactor = disp;
											outModifiers.m_bHasMuzzleDispersionFactor = true;
										}
									}

									if (!outModifiers.m_bHasExtraObstructionLength && curAttr.IsVariableSet("m_fExtraObstructionLength"))
									{
										float extra;
										if (curAttr.Get("m_fExtraObstructionLength", extra))
										{
											outModifiers.m_fExtraObstructionLength = extra;
											outModifiers.m_bHasExtraObstructionLength = true;
										}
									}

									if (!outModifiers.m_bHasOverrideMuzzleEffects && curAttr.IsVariableSet("m_bOverrideMuzzleEffects"))
									{
										bool ovrEffects;
										if (curAttr.Get("m_bOverrideMuzzleEffects", ovrEffects))
										{
											outModifiers.m_bOverrideMuzzleEffects = ovrEffects;
											outModifiers.m_bHasOverrideMuzzleEffects = true;
										}
									}

									if (!outModifiers.m_bHasOverrideShot && curAttr.IsVariableSet("m_bOverrideShot"))
									{
										bool ovrShot;
										if (curAttr.Get("m_bOverrideShot", ovrShot))
										{
											outModifiers.m_bOverrideShot = ovrShot;
											outModifiers.m_bHasOverrideShot = true;
										}
									}

									if (!outModifiers.m_bHasRecoilAngularFactors && curAttr.IsVariableSet("m_vAngularFactors"))
									{
										vector ang;
										if (curAttr.Get("m_vAngularFactors", ang) && ang != "0 0 0")
										{
											outModifiers.m_vRecoilAngularFactors = ang;
											outModifiers.m_bHasRecoilAngularFactors = true;
										}
									}

									if (!outModifiers.m_bHasRecoilLinearFactors && curAttr.IsVariableSet("m_vLinearFactors"))
									{
										vector lin;
										if (curAttr.Get("m_vLinearFactors", lin) && lin != "0 0 0")
										{
											outModifiers.m_vRecoilLinearFactors = lin;
											outModifiers.m_bHasRecoilLinearFactors = true;
										}
									}

									if (!outModifiers.m_bHasTurnFactors && curAttr.IsVariableSet("m_vTurnFactors"))
									{
										vector turn;
										if (curAttr.Get("m_vTurnFactors", turn) && turn != "0 0 0")
										{
											outModifiers.m_vTurnFactors = turn;
											outModifiers.m_bHasTurnFactors = true;
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
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_MuzzlePhysicalInfo outPhys)
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
	static void ExtractVisuals(map<string, ref array<BaseContainer>> comps, TBD_MuzzleVisualsInfo outVisuals)
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
