/** Reads each effective attachment slot without merging installations that accept the same type. */
class TBD_WeaponMountingExtractor
{
	static void ExtractAttachmentSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_WeaponAttachmentSlotInfo> outSlots)
	{
		array<ref TBD_AttachmentSlotInfo> slots = {};
		TBD_AttachmentMountingExtractor.ExtractNestedAttachmentSlots(comps, slots, "/attachment_slots", true);
		foreach (TBD_AttachmentSlotInfo source : slots)
		{
			TBD_WeaponAttachmentSlotInfo slot = new TBD_WeaponAttachmentSlotInfo();
			slot.m_sInstanceId = source.m_sInstanceId;
			slot.m_sSlotName = source.m_sSlotName;
			slot.m_sPivotId = source.m_sPivotId;
			slot.m_sRequiredAttachmentType = source.m_sRequiredAttachmentType;
			slot.m_sDefaultAttachedPrefab = source.m_sDefaultAttachedPrefab;
			slot.m_aObstructedAttachmentTypes = source.m_aObstructedAttachmentTypes;
			string json = TBD_EquipmentExportJson.Member("{}", "source", source.m_sSourceJson);
			json = TBD_EquipmentExportJson.Member(json, "instance_id", TBD_EquipmentExportJson.Quote(slot.m_sInstanceId));
			json = TBD_EquipmentExportJson.Member(json, "slot_name", TBD_EquipmentExportJson.Quote(slot.m_sSlotName));
			json = TBD_EquipmentExportJson.Member(json, "pivot_id", TBD_EquipmentExportJson.Quote(slot.m_sPivotId));
			array<string> accepted = {};
			if (!slot.m_sRequiredAttachmentType.IsEmpty()) accepted.Insert(slot.m_sRequiredAttachmentType);
			json = TBD_EquipmentExportJson.Member(json, "accepted_types", TBD_EquipmentExportJson.Strings(accepted));
			json = TBD_EquipmentExportJson.Member(json, "default_attachment", TBD_EquipmentExportJson.Quote(slot.m_sDefaultAttachedPrefab));
			json = TBD_EquipmentExportJson.Member(json, "exclusions", TBD_EquipmentExportJson.Strings(slot.m_aObstructedAttachmentTypes));
			slot.m_sSourceJson = json;
			outSlots.Insert(slot);
		}
	}
}
