// Generated from the authoritative gameplay field-selection policy.
class TBD_GameplayPolicy_medical_effects_0
{
	static void Apply(TBD_GameplaySelectionPolicy policy)
	{
		policy.AddClass("SCR_ConsumableBandage", "medical_effects");
		policy.AddRule("SCR_ConsumableBandage", "m_aDamageEffectsToLoad\tOBJECT_ARRAY", "retain_relationship", "medical_effects", false, "Native configuration used by medical effects");
		policy.AddRule("SCR_ConsumableBandage", "m_aCharacterLabels\tINTEGER_ARRAY|m_aGroupRoles\tINTEGER_ARRAY|m_bDeleteOnUse\tBOOLEAN|m_fApplyToOtherDuration\tSCALAR|m_fApplyToSelfDuration\tSCALAR|m_fItemAbsoluteRegenerationAmount\tSCALAR|m_fItemRegenerationDuration\tSCALAR|m_fItemRegenerationSpeed\tSCALAR|m_fRoleSpeedBonus\tSCALAR|m_fRoleSpeedPenalty\tSCALAR", "retain_value", "medical_effects", false, "Native configuration used by medical effects");
		policy.AddClass("SCR_ConsumableEffectBase", "medical_effects");
		policy.AddClass("SCR_ConsumableEffectHealthItems", "medical_effects");
		policy.AddClass("SCR_ConsumableItemComponent", "medical_effects");
		policy.AddRule("SCR_ConsumableItemComponent", "m_eAnimVariable\tINTEGER", "exclude", "excluded", false, "Presentation, authoring, lifecycle, replication, animation, audio, or cosmetic effect setting");
		policy.AddRule("SCR_ConsumableItemComponent", "m_ConsumableEffect\tOBJECT", "retain_relationship", "medical_effects", false, "Native configuration used by medical effects");
		policy.AddRule("SCR_ConsumableItemComponent", "Enabled\tBOOLEAN|m_bAlternativeModelOnAction\tBOOLEAN|m_bCanBeHeld\tBOOLEAN|m_bVisibleEquipped\tBOOLEAN|m_eUseMask\tINTEGER|m_fClothesOffsetStrength\tSCALAR|m_fWeaponNoFireTime\tSCALAR|m_vEquipmentSlotOffset\tVECTOR3", "retain_value", "medical_effects", false, "Native configuration used by medical effects");
		policy.AddRule("SCR_ConsumableItemComponent", "components\tOBJECT_ARRAY", "traverse_required_container", "medical_effects", false, "Native configuration used by medical effects");
		policy.AddClass("SCR_ConsumableMorphine", "medical_effects");
		policy.AddRule("SCR_ConsumableMorphine", "m_aDamageEffectsToLoad\tOBJECT_ARRAY", "retain_relationship", "medical_effects", false, "Native configuration used by medical effects");
		policy.AddRule("SCR_ConsumableMorphine", "m_aCharacterLabels\tINTEGER_ARRAY|m_aGroupRoles\tINTEGER_ARRAY|m_bDeleteOnUse\tBOOLEAN|m_fApplyToOtherDuration\tSCALAR|m_fApplyToSelfDuration\tSCALAR|m_fItemAbsoluteRegenerationAmount\tSCALAR|m_fItemRegenerationDuration\tSCALAR|m_fItemRegenerationSpeed\tSCALAR|m_fRoleSpeedBonus\tSCALAR|m_fRoleSpeedPenalty\tSCALAR", "retain_value", "medical_effects", false, "Native configuration used by medical effects");
		policy.AddClass("SCR_ConsumableSalineBag", "medical_effects");
		policy.AddRule("SCR_ConsumableSalineBag", "m_aDamageEffectsToLoad\tOBJECT_ARRAY", "retain_relationship", "medical_effects", false, "Native configuration used by medical effects");
		policy.AddRule("SCR_ConsumableSalineBag", "m_aCharacterLabels\tINTEGER_ARRAY|m_aGroupRoles\tINTEGER_ARRAY|m_bDeleteOnUse\tBOOLEAN|m_fApplyToOtherDuration\tSCALAR|m_fApplyToSelfDuration\tSCALAR|m_fItemAbsoluteRegenerationAmount\tSCALAR|m_fItemRegenerationDuration\tSCALAR|m_fItemRegenerationSpeed\tSCALAR|m_fRoleSpeedBonus\tSCALAR|m_fRoleSpeedPenalty\tSCALAR", "retain_value", "medical_effects", false, "Native configuration used by medical effects");
		policy.AddClass("SCR_ConsumableTourniquet", "medical_effects");
		policy.AddRule("SCR_ConsumableTourniquet", "m_aDamageEffectsToLoad\tOBJECT_ARRAY", "retain_relationship", "medical_effects", false, "Native configuration used by medical effects");
		policy.AddRule("SCR_ConsumableTourniquet", "m_aCharacterLabels\tINTEGER_ARRAY|m_aGroupRoles\tINTEGER_ARRAY|m_bDeleteOnUse\tBOOLEAN|m_fApplyToOtherDuration\tSCALAR|m_fApplyToSelfDuration\tSCALAR|m_fItemAbsoluteRegenerationAmount\tSCALAR|m_fItemRegenerationDuration\tSCALAR|m_fItemRegenerationSpeed\tSCALAR|m_fRoleSpeedBonus\tSCALAR|m_fRoleSpeedPenalty\tSCALAR", "retain_value", "medical_effects", false, "Native configuration used by medical effects");
	}
}
