/**
 * @file TBD_LoadoutPreviewMount.c
 * @brief Mounts optic and magazine onto a kit-preview weapon and puts the weapon in hand.
 *
 * Role: the weapon half of dressing a lobby kit-preview doll: typed attachment-slot mounting with
 * a magazine-well fallback, and weapon selection.  Position: called by
 * `TBD_LoadoutPreviewDresser.Dress` on the client; spawns through
 * `TBD_LoadoutPreviewDresser.SpawnLocal` into the preview world.
 * State: none.  Invariants: only an attachment slot whose own `CanSetAttachment` accepts the item
 * receives it; a miss is one WARNING per reason, never an ERROR.
 */

//! Stateless weapon mounting for the kit-preview doll.
class TBD_LoadoutPreviewMount
{
	//! Mount an optic / magazine onto the primary: the attachment slot whose own type accepts it;
	//! the magazine additionally falls back to the inventory manager (the server pass's route).
	static void Mount(IEntity ent, IEntity weapon, string label, string prefab)
	{
		if (prefab.IsEmpty())
			return;

		WeaponAttachmentsStorageComponent storage = WeaponAttachmentsStorageComponent.Cast(weapon.FindComponent(WeaponAttachmentsStorageComponent));
		if (!storage)
		{
			TBD_WarnOnce.Warn(TBD_LoadoutPreviewDresser.LOG_CHANNEL, label + ":storage", TBD_LoadoutPreviewDresser.WARN_PREFIX + string.Format("primary has no WeaponAttachmentsStorageComponent -- %1 not mounted", label));
			return;
		}

		if (StorageHas(storage, prefab))
			return;

		IEntity item = TBD_LoadoutPreviewDresser.SpawnLocal(prefab, ent);
		if (!item)
		{
			TBD_WarnOnce.Warn(TBD_LoadoutPreviewDresser.LOG_CHANNEL, label + ":" + prefab, TBD_LoadoutPreviewDresser.WARN_PREFIX + string.Format("%1 prefab failed to load: %2", label, prefab));
			return;
		}

		int count = storage.GetSlotsCount();
		int i;
		for (i = 0; i < count; i++)
		{
			InventoryStorageSlot slot = storage.GetSlot(i);
			if (!slot)
				continue;

			AttachmentSlotComponent attachmentSlot = AttachmentSlotComponent.Cast(slot.GetParentContainer());
			if (!attachmentSlot || !attachmentSlot.CanSetAttachment(item))
				continue;

			IEntity occupant = slot.GetAttachedEntity();
			if (occupant)
			{
				slot.DetachEntity();
				SCR_EntityHelper.DeleteEntityAndChildren(occupant);
			}

			slot.AttachEntity(item);
			return;
		}

		SCR_EntityHelper.DeleteEntityAndChildren(item);

		// No typed attachment slot took it. The magazine well is reached the way the server pass
		// reaches it: through the character's inventory manager into the weapon storage.
		SCR_InventoryStorageManagerComponent manager = SCR_InventoryStorageManagerComponent.Cast(ent.FindComponent(SCR_InventoryStorageManagerComponent));
		if (manager && manager.TrySpawnPrefabToStorage(prefab, storage, -1, EStoragePurpose.PURPOSE_ANY) && StorageHas(storage, prefab))
			return;

		TBD_WarnOnce.Warn(TBD_LoadoutPreviewDresser.LOG_CHANNEL, label + ":fit:" + prefab, TBD_LoadoutPreviewDresser.WARN_PREFIX + string.Format("no slot on the primary accepts %1 %2 -- the weapon shows its own", label, prefab));
	}

	//! Whether any slot of `storage` holds an item of prefab `prefab`.
	//! @return true on the first match
	protected static bool StorageHas(BaseInventoryStorageComponent storage, string prefab)
	{
		int count = storage.GetSlotsCount();
		int i;
		for (i = 0; i < count; i++)
		{
			InventoryStorageSlot slot = storage.GetSlot(i);
			if (slot && TBD_LoadoutInventoryUtil.PrefabOf(slot.GetAttachedEntity()) == prefab)
				return true;
		}

		return false;
	}

	//! Put `weapon` in the doll's hands (vanilla: SelectWeapon on the slot that holds it).
	static void HoldWeapon(IEntity ent, IEntity weapon)
	{
		if (!weapon)
			return;

		BaseWeaponManagerComponent weaponManager = BaseWeaponManagerComponent.Cast(ent.FindComponent(BaseWeaponManagerComponent));
		if (!weaponManager)
			return;

		array<WeaponSlotComponent> slots = {};
		weaponManager.GetWeaponsSlots(slots);
		foreach (WeaponSlotComponent weaponSlot : slots)
		{
			if (weaponSlot.GetWeaponEntity() == weapon)
			{
				weaponManager.SelectWeapon(weaponSlot);
				return;
			}
		}
	}
}
