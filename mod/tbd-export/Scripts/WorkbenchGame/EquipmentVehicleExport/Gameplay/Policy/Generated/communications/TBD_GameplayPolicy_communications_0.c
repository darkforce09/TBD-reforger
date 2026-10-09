// Generated from the authoritative gameplay field-selection policy.
class TBD_GameplayPolicy_communications_0
{
	static void Apply(TBD_GameplaySelectionPolicy policy)
	{
		policy.AddClass("BaseRadioComponent", "communications");
		policy.AddRule("BaseRadioComponent", "Transceivers\tOBJECT_ARRAY", "retain_relationship", "communications", false, "Native configuration used by communications");
		policy.AddRule("BaseRadioComponent", "Editor radio\tBOOLEAN|Enabled\tBOOLEAN|Encryption key\tSTRING|Turned on\tBOOLEAN", "retain_value", "communications", false, "Native configuration used by communications");
		policy.AddRule("BaseRadioComponent", "components\tOBJECT_ARRAY", "traverse_required_container", "communications", false, "Native configuration used by communications");
		policy.AddClass("IntercomTransceiver", "communications");
		policy.AddRule("IntercomTransceiver", "ChannelFrequency\tINTEGER|Frequency resolution\tINTEGER|Max tunable frequency\tINTEGER|Min tunable frequency\tINTEGER|Transmitting Range\tSCALAR", "retain_value", "communications", false, "Native configuration used by communications");
		policy.AddClass("RadioTransceiver", "communications");
		policy.AddRule("RadioTransceiver", "ChannelFrequency\tINTEGER|Frequency resolution\tINTEGER|Max tunable frequency\tINTEGER|Min tunable frequency\tINTEGER|Transmitting Range\tSCALAR", "retain_value", "communications", false, "Native configuration used by communications");
		policy.AddClass("RelayTransceiver", "communications");
		policy.AddRule("RelayTransceiver", "Any frequency\tBOOLEAN|ChannelFrequency\tINTEGER|Frequency resolution\tINTEGER|Max tunable frequency\tINTEGER|Min tunable frequency\tINTEGER|Transmitting Range\tSCALAR", "retain_value", "communications", false, "Native configuration used by communications");
		policy.AddClass("SCR_CoverageRadioComponent", "communications");
		policy.AddRule("SCR_CoverageRadioComponent", "Transceivers\tOBJECT_ARRAY", "retain_relationship", "communications", false, "Native configuration used by communications");
		policy.AddRule("SCR_CoverageRadioComponent", "Editor radio\tBOOLEAN|Enabled\tBOOLEAN|Encryption key\tSTRING|Turned on\tBOOLEAN|m_bIsSource\tBOOLEAN", "retain_value", "communications", false, "Native configuration used by communications");
		policy.AddRule("SCR_CoverageRadioComponent", "components\tOBJECT_ARRAY", "traverse_required_container", "communications", false, "Native configuration used by communications");
		policy.AddClass("SCR_RadioComponent", "communications");
		policy.AddRule("SCR_RadioComponent", "m_eAnimVariable\tINTEGER", "exclude", "excluded", false, "Presentation, authoring, lifecycle, replication, animation, audio, or cosmetic effect setting");
		policy.AddRule("SCR_RadioComponent", "Enabled\tBOOLEAN|m_bCanBeHeld\tBOOLEAN|m_eUseMask\tINTEGER|m_fClothesOffsetStrength\tSCALAR|m_fWeaponNoFireTime\tSCALAR|m_iRadioCategory\tINTEGER|m_iRadioType\tINTEGER|m_vEquipmentSlotOffset\tVECTOR3", "retain_value", "communications", false, "Native configuration used by communications");
		policy.AddRule("SCR_RadioComponent", "components\tOBJECT_ARRAY", "traverse_required_container", "communications", false, "Native configuration used by communications");
		policy.AddClass("ScriptedRadioComponent", "communications");
	}
}
