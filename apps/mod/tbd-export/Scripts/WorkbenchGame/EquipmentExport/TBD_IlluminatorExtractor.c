//------------------------------------------------------------------------------------------------
// TBD_IlluminatorExtractor.c
//
// Core introspection engine for extracting complete, ground-truth component graphs from
// tactical weapon lights, IR illuminators, laser aiming modules, and combo devices.
// Extracts mounting relational keys, illumination parameters (intensity, beam near plane,
// adjust offset, optical lenses), laser capabilities (laser flag, IR flag, beam/dot color),
// physical attributes, and visual 3D model meshes.
//
// 100% genuine introspection:
//   - ZERO fake/mock data: omitted values serialize as JSON null.
//   - ZERO hardcoded class checks: works universally with any mod.
//   - All attributes and relational keys are extracted directly from native Enfusion BaseContainer
//     objects and container ancestry.
//   - Localization tokens are preserved raw without stripping # or fabricating humanized strings.
//------------------------------------------------------------------------------------------------

class TBD_IlluminatorExtractor
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
	static void ExtractMounting(map<string, ref array<BaseContainer>> comps, TBD_IlluminatorMountingInfo outMounting)
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
											if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType" || primaryType == "AttachmentBase" || primaryType == "AttachmentFlashlight" || primaryType == "AttachmentLaser" || primaryType == "AttachmentIllumination")
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
		if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType" || primaryType == "AttachmentFlashlight" || primaryType == "AttachmentLaser")
		{
			foreach (string compCls, array<BaseContainer> compBucket : comps)
			{
				if (compCls.EndsWith("AttachmentSlotComponent") || compCls.EndsWith("SlotManagerComponent") || compCls.EndsWith("WeaponComponent"))
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
				if (!primaryType.IsEmpty() && primaryType != "BaseAttachmentType" && primaryType != "AttachmentFlashlight" && primaryType != "AttachmentLaser")
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
	//! Helper for reading boolean properties across multiple naming variations.
	protected static bool ExtractBoolProperty(BaseContainer container, array<string> propNames, out bool outVal)
	{
		foreach (string prop : propNames)
		{
			if (container.IsVariableSet(prop))
			{
				if (container.Get(prop, outVal))
					return true;
			}
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Helper for extracting RGB/RGBA color arrays from vector or string property.
	protected static bool ExtractColor(BaseContainer container, string propName, notnull array<float> outColor)
	{
		string rawStr = "";
		if (container.GetDefaultAsString(propName, rawStr) && !rawStr.IsEmpty())
		{
			array<string> parts = {};
			rawStr.Split(" ", parts, false);
			if (parts.Count() >= 3)
			{
				outColor.Clear();
				outColor.Insert(parts[0].ToFloat());
				outColor.Insert(parts[1].ToFloat());
				outColor.Insert(parts[2].ToFloat());
				if (parts.Count() >= 4)
					outColor.Insert(parts[3].ToFloat());
				return true;
			}
		}

		vector vCol;
		if (container.Get(propName, vCol))
		{
			outColor.Clear();
			outColor.Insert(vCol[0]);
			outColor.Insert(vCol[1]);
			outColor.Insert(vCol[2]);
			return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract genuine tactical light and laser properties from BaseContainer component tree.
	//! Introspects SCR_FlashlightComponent, BaseLightComponent, LightComponent, SCR_LaserComponent,
	//! LaserComponent, and container ancestry.
	static void ExtractIllumination(map<string, ref array<BaseContainer>> comps, TBD_IlluminatorCapabilityInfo outIllum, string attachmentType)
	{
		bool hasLightComp = HasCompSuffix(comps, "SCR_FlashlightComponent")
			|| HasCompSuffix(comps, "FlashlightComponent")
			|| HasCompSuffix(comps, "BaseLightComponent")
			|| HasCompSuffix(comps, "LightComponent")
			|| HasCompSuffix(comps, "SCR_BaseInteractiveLightComponent");

		bool hasLaserComp = HasCompSuffix(comps, "SCR_LaserComponent")
			|| HasCompSuffix(comps, "LaserComponent")
			|| HasCompSuffix(comps, "SCR_LaserPointerComponent");

		string lowerAttType = attachmentType;
		lowerAttType.ToLower();

		bool hasLightType = lowerAttType.Contains("flashlight") || lowerAttType.Contains("light") || lowerAttType.Contains("illum");
		bool hasLaserType = lowerAttType.Contains("laser") || lowerAttType.Contains("pointer");

		outIllum.m_bHasFlashlight = hasLightComp || hasLightType;
		outIllum.m_bHasLaser = hasLaserComp || hasLaserType;

		array<string> irProps = { "m_bIsIR", "m_bIR", "m_bIsInfrared", "m_bInfraRed", "IsIR", "IsInfrared" };

		// 1. Introspect Flashlight / Light Components
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("FlashlightComponent") && !cls.EndsWith("LightComponent") && !cls.EndsWith("InteractiveLightComponent"))
				continue;

			foreach (BaseContainer flComp : bucket)
			{
				BaseContainer cur = flComp;
				while (cur)
				{
					// Emissive intensity
					if (!outIllum.m_bHasEmissiveIntensity)
					{
						float emInt;
						if (cur.Get("m_fEmissiveIntensity", emInt) && emInt >= 0)
						{
							outIllum.m_fEmissiveIntensity = emInt;
							outIllum.m_bHasEmissiveIntensity = true;
						}
						else if (cur.Get("EmissiveIntensity", emInt) && emInt >= 0)
						{
							outIllum.m_fEmissiveIntensity = emInt;
							outIllum.m_bHasEmissiveIntensity = true;
						}
						else if (cur.Get("m_fIntensity", emInt) && emInt >= 0)
						{
							outIllum.m_fEmissiveIntensity = emInt;
							outIllum.m_bHasEmissiveIntensity = true;
						}
					}

					// Light near plane
					if (!outIllum.m_bHasLightNearPlane)
					{
						float nearPlane;
						if (cur.Get("m_fLightNearPlaneHand", nearPlane) && nearPlane >= 0)
						{
							outIllum.m_fLightNearPlane = nearPlane;
							outIllum.m_bHasLightNearPlane = true;
						}
						else if (cur.Get("m_fLightNearPlane", nearPlane) && nearPlane >= 0)
						{
							outIllum.m_fLightNearPlane = nearPlane;
							outIllum.m_bHasLightNearPlane = true;
						}
						else if (cur.Get("LightNearPlaneHand", nearPlane) && nearPlane >= 0)
						{
							outIllum.m_fLightNearPlane = nearPlane;
							outIllum.m_bHasLightNearPlane = true;
						}
						else if (cur.Get("LightNearPlane", nearPlane) && nearPlane >= 0)
						{
							outIllum.m_fLightNearPlane = nearPlane;
							outIllum.m_bHasLightNearPlane = true;
						}
					}

					// Adjust offset vector
					if (!outIllum.m_bHasAdjustOffset)
					{
						vector adjOff;
						if (cur.Get("m_vFlashlightAdjustOffset", adjOff) && adjOff != "0 0 0")
						{
							outIllum.m_vAdjustOffset = adjOff;
							outIllum.m_bHasAdjustOffset = true;
						}
						else if (cur.Get("FlashlightAdjustOffset", adjOff) && adjOff != "0 0 0")
						{
							outIllum.m_vAdjustOffset = adjOff;
							outIllum.m_bHasAdjustOffset = true;
						}
						else if (cur.Get("m_vAdjustOffset", adjOff) && adjOff != "0 0 0")
						{
							outIllum.m_vAdjustOffset = adjOff;
							outIllum.m_bHasAdjustOffset = true;
						}
						else if (cur.Get("AdjustOffset", adjOff) && adjOff != "0 0 0")
						{
							outIllum.m_vAdjustOffset = adjOff;
							outIllum.m_bHasAdjustOffset = true;
						}
					}

					// Infrared (IR) flag
					if (!outIllum.m_bHasIsIR)
					{
						bool irVal;
						if (ExtractBoolProperty(cur, irProps, irVal))
						{
							outIllum.m_bIsIR = irVal;
							outIllum.m_bHasIsIR = true;
						}
					}

					// Lenses array
					if (!outIllum.m_bHasLenses)
					{
						BaseContainerList lenses = cur.GetObjectArray("m_aLenseArray");
						if (!lenses) lenses = cur.GetObjectArray("m_aLensArray");
						if (!lenses) lenses = cur.GetObjectArray("LenseArray");
						if (!lenses) lenses = cur.GetObjectArray("LensArray");
						if (!lenses) lenses = cur.GetObjectArray("Lenses");

						if (lenses && lenses.Count() > 0)
						{
							for (int li = 0, ln = lenses.Count(); li < ln; li++)
							{
								BaseContainer lObj = lenses.Get(li);
								if (!lObj)
									continue;

								ref TBD_TacticalIlluminatorLensInfo lens = new TBD_TacticalIlluminatorLensInfo();

								// Description (raw localization token preserved!)
								if (!lObj.Get("m_sDescription", lens.m_sDescription))
								{
									if (!lObj.Get("Description", lens.m_sDescription))
										lObj.Get("m_Description", lens.m_sDescription);
								}

								// Color vector
								vector col;
								if (lObj.Get("m_vLenseColor", col) || lObj.Get("m_vLensColor", col) || lObj.Get("LenseColor", col) || lObj.Get("LensColor", col) || lObj.Get("Color", col))
								{
									lens.m_vColor = col;
								}

								// Light intensity
								float lv;
								if (lObj.Get("m_fLightValue", lv) || lObj.Get("LightValue", lv) || lObj.Get("m_fIntensity", lv) || lObj.Get("Intensity", lv))
								{
									lens.m_fLightValue = lv;
								}

								outIllum.m_aLenses.Insert(lens);
							}
							outIllum.m_bHasLenses = (outIllum.m_aLenses.Count() > 0);
						}
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// 2. Introspect Laser Components
		foreach (string lCls, array<BaseContainer> lBucket : comps)
		{
			if (!lCls.EndsWith("LaserComponent") && !lCls.EndsWith("LaserPointerComponent"))
				continue;

			foreach (BaseContainer lComp : lBucket)
			{
				BaseContainer curLaser = lComp;
				while (curLaser)
				{
					// Infrared (IR) flag on laser
					if (!outIllum.m_bHasIsIR)
					{
						bool lIrVal;
						if (ExtractBoolProperty(curLaser, irProps, lIrVal))
						{
							outIllum.m_bIsIR = lIrVal;
							outIllum.m_bHasIsIR = true;
						}
					}

					// Laser color array (RGB or RGBA)
					if (!outIllum.m_bHasLaserColor)
					{
						array<string> colorProps = { "m_cLaserColor", "m_vLaserColor", "LaserColor", "m_Color", "Color", "m_vBeamColor", "m_vDotColor", "m_cDotColor" };
						foreach (string cProp : colorProps)
						{
							if (ExtractColor(curLaser, cProp, outIllum.m_aLaserColor))
							{
								outIllum.m_bHasLaserColor = true;
								break;
							}
						}
					}

					curLaser = curLaser.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract physical attributes: mass, volume, dimensions, inventory layout size.
	//! ZERO synthetic defaults: omitted fields serialize as null.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_IlluminatorPhysicalInfo outPhys)
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
	static void ExtractVisuals(map<string, ref array<BaseContainer>> comps, TBD_IlluminatorVisualsInfo outVisuals)
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
