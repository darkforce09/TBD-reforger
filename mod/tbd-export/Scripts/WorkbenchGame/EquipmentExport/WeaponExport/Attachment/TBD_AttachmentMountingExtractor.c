/**
 * Reads effective attachment mounting rules and individual nested attachment slots.
 * Prefab ancestry resolves property values; native type ancestry determines type compatibility.
 */

//! Reads mounting attributes without combining overridden ancestor configurations.
class TBD_AttachmentMountingExtractor
{
	//! Weapons that also attach to slots retain the same native mounting rules as other attachments.
	static string ExtractWeaponMounting(map<string, ref array<BaseContainer>> comps)
	{
		TBD_AttachmentMountingInfo mounting = new TBD_AttachmentMountingInfo();
		ExtractMounting(comps, mounting, string.Empty, string.Empty);
		if (mounting.m_sAttachmentType.IsEmpty()) return string.Empty;
		string json = "{\"attachment_type\":" + TBD_EquipmentExportJson.Quote(mounting.m_sAttachmentType);
		json += ",\"native_type_hierarchy\":" + TBD_EquipmentExportJson.Strings(mounting.m_aNativeTypeHierarchy);
		json += ",\"compatible_attachment_types\":" + TBD_EquipmentExportJson.Strings(mounting.m_aCompatibleAttachmentTypes);
		return json + ",\"obstructed_attachment_types\":" + TBD_EquipmentExportJson.Strings(mounting.m_aObstructedAttachmentTypes) + "}";
	}

	//! Read the effective attachment type, its native ancestors, and explicit restrictions.
	static void ExtractMounting(map<string, ref array<BaseContainer>> comps, TBD_AttachmentMountingInfo outMounting, string filePath, string category)
	{
		outMounting.m_sAttachmentType = string.Empty;
		outMounting.m_aCompatibleAttachmentTypes.Clear();
		outMounting.m_aNativeTypeHierarchy.Clear();
		outMounting.m_aObstructedAttachmentTypes.Clear();

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!IsNativeType(cls, "InventoryItemComponent") && !IsNativeType(cls, "WeaponAttachmentComponent"))
				continue;

			foreach (BaseContainer component : bucket)
			{
				BaseContainer attrs = component.GetObject("Attributes");
				BaseContainerList attributes = EffectiveCustomAttributes(attrs);
				if (attributes)
				{
					for (int i = 0; i < attributes.Count(); i++)
						ReadMountingAttribute(attributes.Get(i), outMounting);
				}
				ReadMountingAttribute(component, outMounting);
			}
		}

		// Some native components expose mounting attributes directly.
		if (outMounting.m_sAttachmentType.IsEmpty())
		{
			foreach (string compCls, array<BaseContainer> compBucket : comps)
			{
				if (IsNativeType(compCls, "AttachmentSlotComponent") || IsNativeType(compCls, "SlotManagerComponent") || IsNativeType(compCls, "WeaponComponent"))
					continue;
				foreach (BaseContainer candidate : compBucket)
				{
					ReadMountingAttribute(candidate, outMounting);
					if (!outMounting.m_sAttachmentType.IsEmpty())
						break;
				}
				if (!outMounting.m_sAttachmentType.IsEmpty())
					break;
			}
		}

		if (!outMounting.m_sAttachmentType.IsEmpty())
			BuildAttachmentAncestry(outMounting.m_sAttachmentType, outMounting.m_aNativeTypeHierarchy);
	}

	//! Enumerate the native attachment family, then retain only this type and its actual bases.
	protected static void BuildAttachmentAncestry(string primaryType, notnull array<string> hierarchy)
	{
		typename attachmentType = primaryType.ToType();
		if (!attachmentType)
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, "/mounting/native_type_hierarchy", "Cannot resolve native attachment type: " + primaryType);
			return;
		}
		hierarchy.Insert(primaryType);
		array<typename> candidates = {};
		typename family = BaseAttachmentType;
		family.GetInheritedTypes(candidates);
		candidates.Insert(family);
		foreach (typename candidate : candidates)
		{
			if (attachmentType.IsInherited(candidate))
				TBD_EquipmentComponentGraph.AddUniqueType(hierarchy, candidate.ToString());
		}
	}

	//! Preserve explicit empty custom-attribute lists instead of searching an ancestor.
	protected static BaseContainerList EffectiveCustomAttributes(BaseContainer attrs)
	{
		if (!attrs)
			return null;
		BaseContainerList direct = attrs.GetObjectArray("CustomAttributes");
		if (direct)
			return direct;
		BaseContainer wrapper = attrs.GetObject("CustomAttributes");
		if (!wrapper)
			return null;
		BaseContainerList wrapped = wrapper.GetObjectArray("m_aAttributes");
		if (wrapped)
			return wrapped;
		return wrapper.GetObjectArray("CustomAttributes");
	}

	//! A null attachment override remains null; another property name is used only if absent.
	protected static BaseContainer ReadAttachmentType(BaseContainer container)
	{
		if (!container)
			return null;
		BaseContainer attachment = container.GetObject("AttachmentType");
		if (attachment || container.IsVariableSet("AttachmentType"))
			return attachment;
		return container.GetObject("m_AttachmentType");
	}

	//! Preserve authored restrictions from a single effective attribute instance.
	protected static void ReadMountingAttribute(BaseContainer attribute, TBD_AttachmentMountingInfo output)
	{
		if (!attribute)
			return;
		BaseContainer attachment = ReadAttachmentType(attribute);
		if (attachment && output.m_sAttachmentType.IsEmpty())
			{
			output.m_sAttachmentType = attachment.GetClassName();
			TBD_EquipmentExportJson.RecordNativeField(attachment, "/mounting/attachment_type", "present", "BaseContainer.GetClassName", "Native AttachmentType configuration", "", "TYPENAME");
			TBD_EquipmentExportJson.RecordNativeField(attachment, "/mounting/native_type_hierarchy", "present", "TypeName.GetInheritedTypes + TypeName.IsInherited", "Native self and base attachment types", "", "TYPENAME_ARRAY");
		}
		ReadTypeList(attribute.GetObjectArray("m_aCompatibleAttachmentTypes"), output.m_aCompatibleAttachmentTypes);
		ReadTypeList(attribute.GetObjectArray("m_aObstructedAttachmentTypes"), output.m_aObstructedAttachmentTypes);
	}

	//! Read native type names in their configured order without fabricated class names.
	protected static void ReadTypeList(BaseContainerList configuredTypes, notnull array<string> output)
	{
		if (!configuredTypes)
			return;
		for (int i = 0; i < configuredTypes.Count(); i++)
		{
			BaseContainer configuredType = configuredTypes.Get(i);
			if (configuredType)
				TBD_EquipmentComponentGraph.AddUniqueType(output, configuredType.GetClassName());
		}
	}

	//! Match native inheritance without suffix-based component classification.
	protected static bool IsNativeType(string className, string baseName)
	{
		typename actualType = className.ToType();
		typename baseType = baseName.ToType();
		return actualType && baseType && actualType.IsInherited(baseType);
	}

	//! Keep every effective slot installation, including repeated accepted types.
	static void ExtractNestedAttachmentSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_AttachmentSlotInfo> outSlots, string outputPath = "/attachment_slots", bool weaponFormat = false)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!IsNativeType(cls, "AttachmentSlotComponent"))
				continue;
			foreach (BaseContainer slotComponent : bucket)
			{
				TBD_AttachmentSlotInfo slot = new TBD_AttachmentSlotInfo();
				slot.m_sInstanceId = TBD_EquipmentComponentGraph.InstanceId(slotComponent);
				slot.m_sSourceJson = TBD_EquipmentExportJson.Context(slotComponent);
				string location = outputPath + "/" + outSlots.Count().ToString();
				BaseContainer slotObject = slotComponent.GetObject("AttachmentSlot");
				BaseContainer attachmentType = ReadAttachmentType(slotComponent);
				if (!attachmentType && !slotComponent.IsVariableSet("AttachmentType") && !slotComponent.IsVariableSet("m_AttachmentType"))
					attachmentType = ReadAttachmentType(slotObject);
				if (attachmentType)
					{
					slot.m_sRequiredAttachmentType = attachmentType.GetClassName();
					string typePath = location + "/required_attachment_type";
					if (weaponFormat) typePath = location + "/accepted_types/0";
					TBD_EquipmentExportJson.RecordNativeField(attachmentType, typePath, "present", "BaseContainer.GetClassName", "Native slot AttachmentType configuration", "", "TYPENAME");
				}
				if (slotObject)
				{
					slot.m_sSlotName = slotObject.GetName();
					slotObject.Get("PivotID", slot.m_sPivotId);
					TBD_EquipmentExportJson.Field(slotObject, "PivotID", location + "/pivot_id");
					TBD_EquipmentExportJson.RecordNativeField(slotObject, location + "/slot_name", "present", "BaseContainer.GetName", "Native attachment slot name", "", "STRING");
					if (!slotObject.Get("Prefab", slot.m_sDefaultAttachedPrefab))
						slotObject.Get("m_sAttachment", slot.m_sDefaultAttachedPrefab);
					string defaultProperty = "Prefab";
					if (slotObject.GetVarIndex(defaultProperty) < 0) defaultProperty = "m_sAttachment";
					TBD_EquipmentExportJson.Field(slotObject, defaultProperty, location + "/default_attachment");
				}
				ReadTypeList(slotComponent.GetObjectArray("m_aObstructedAttachmentTypes"), slot.m_aObstructedAttachmentTypes);
				BaseContainerList attributes = EffectiveCustomAttributes(slotComponent);
				if (attributes)
				{
					for (int i = 0; i < attributes.Count(); i++)
					{
						BaseContainer attribute = attributes.Get(i);
						if (attribute)
							ReadTypeList(attribute.GetObjectArray("m_aObstructedAttachmentTypes"), slot.m_aObstructedAttachmentTypes);
					}
				}
				outSlots.Insert(slot);
			}
		}
	}
}
