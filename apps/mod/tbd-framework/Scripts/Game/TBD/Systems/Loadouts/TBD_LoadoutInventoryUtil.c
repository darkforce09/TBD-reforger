/**
 * @file TBD_LoadoutInventoryUtil.c
 * @brief Inventory and prefab queries shared by the loadout equip, preview and gadget code.
 *
 * Role: prefab names of spawned items, the gear count of a slot loadout, clothing landing areas,
 * parent-chain tests and weapon storage lookups.  Position: called by
 * `TBD_LoadoutEquipHelper`, `TBD_LoadoutPreviewDresser` and `TBD_GadgetFlags`; reads engine
 * entities and `TBD_SlotGearStruct`.
 * State: none.  Invariants: every query accepts null and answers the empty or false value; none
 * spawns, moves or deletes an entity.
 */

//! Stateless loadout inventory queries.
class TBD_LoadoutInventoryUtil
{
	//! The prefab ResourceName `ent` was spawned from.
	//! @return the prefab name, or empty for null or an entity without prefab data
	static string PrefabOf(IEntity ent)
	{
		if (!ent)
			return string.Empty;

		EntityPrefabData prefabData = ent.GetPrefabData();
		if (!prefabData)
			return string.Empty;

		return prefabData.GetPrefabName();
	}

	//! How many gear ResourceNames `gear` asks for: every non-empty weapon, magazine, optic,
	//! clothing and backpack field plus each attachment. The denominator of the equip verdict.
	//! @return the count; 0 for null
	static int CountGear(TBD_SlotGearStruct gear)
	{
		if (!gear)
			return 0;

		int n;
		if (!gear.primary.IsEmpty())   n++;
		if (!gear.optic.IsEmpty())     n++;
		if (!gear.magazine.IsEmpty())  n++;
		if (gear.attachments)
			n += gear.attachments.Count();
		if (!gear.launcher.IsEmpty())  n++;
		if (!gear.handgun.IsEmpty())   n++;
		if (!gear.throwable.IsEmpty()) n++;
		if (!gear.uniform.IsEmpty())  n++;
		if (!gear.vest.IsEmpty())     n++;
		if (!gear.helmet.IsEmpty())   n++;
		if (!gear.pants.IsEmpty())    n++;
		if (!gear.boots.IsEmpty())    n++;
		if (!gear.handwear.IsEmpty()) n++;
		if (!gear.backpack.IsEmpty()) n++;
		return n;
	}

	//! The slot areas a clothing item of `label` may land in. Equipping routes by the item's own
	//! area type, so a `vest` may land in the armored vest area as well as its primary area.
	//! @param label the loadout clothing label
	//! @param primaryArea the area the label maps to
	//! @param outAreas receives `primaryArea`, then any alternative
	static void AreasForLabel(string label, typename primaryArea, notnull array<typename> outAreas)
	{
		outAreas.Insert(primaryArea);
		if (label == "vest")
			outAreas.Insert(LoadoutArmoredVestSlotArea);
	}

	//! Whether `entity`'s parent chain reaches `root` (worn or attached, not lying loose).
	//! @return true when `entity` is `root` or a descendant of it; false for a null `entity`
	static bool IsRootedOn(IEntity entity, IEntity root)
	{
		IEntity cur = entity;
		while (cur)
		{
			if (cur == root)
				return true;

			cur = cur.GetParent();
		}

		return false;
	}

	//! The storage component that holds a weapon's attachments and magazine well.
	//! @return the weapon's `BaseInventoryStorageComponent`, or null for null or none
	static BaseInventoryStorageComponent WeaponStorageOf(IEntity weapon)
	{
		if (!weapon)
			return null;

		return BaseInventoryStorageComponent.Cast(weapon.FindComponent(BaseInventoryStorageComponent));
	}

	//! Whether `storage` already holds an item of prefab `resName`, nested items included (a
	//! magazine inside a well counts).
	//! @return true on the first match; false for null storage
	static bool WeaponStorageHas(BaseInventoryStorageComponent storage, string resName)
	{
		if (!storage)
			return false;

		array<IEntity> items = {};
		storage.GetAll(items, true);
		foreach (IEntity item : items)
		{
			if (PrefabOf(item) == resName)
				return true;
		}

		return false;
	}
}
