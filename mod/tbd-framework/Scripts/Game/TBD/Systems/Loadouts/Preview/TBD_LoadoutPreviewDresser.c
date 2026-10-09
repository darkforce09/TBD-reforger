/**
 * @file TBD_LoadoutPreviewDresser.c
 * @brief Dresses the lobby kit-preview doll with a slot's exact kit and loadout.
 *
 * Role: the client counterpart of `TBD_LoadoutApplication`: the same inputs (a kit prefab
 * resolved through `TBD_Registry` and the slot's `TBD_SlotLoadoutStruct`) and the same
 * composition rule (an authored gear field replaces the kit's garment, an absent one keeps it,
 * optic and magazine mount on the primary), applied to the `ItemPreviewManagerEntity` preview
 * entity with local spawns and synchronous `AttachEntity`. It follows vanilla
 * `SCR_LoadoutPreviewComponent.SetPreviewedLoadout`: resolve the preview entity, attach garments
 * to `EquipedLoadoutStorageComponent` slots and weapons to `EquipedWeaponStorageComponent` slots,
 * mount through `TBD_LoadoutPreviewMount`, select the weapon to hold.  Position: called by
 * `TBD_KitPreviewComponent`, which hands the entity to `SetPreviewItem`.
 * State: one `TBD_PreviewBaseline` per kit prefab for the session, on the client.
 * Invariants: `ResolvePreviewEntityForPrefab` returns one shared entity per prefab and baked weapons have no
 * slot template, so the prefab's untouched state is recorded before its first dressing and every
 * pass computes each slot as authored, else baseline: equal keeps, different swaps, empty
 * baseline clears. An authored weapon is always respawned so a previous seat's attachments never
 * linger. A miss is one WARNING per reason, never an ERROR; no authority, replication or wire.
 */

//! What a kit prefab's preview entity holds before any loadout touches it.
class TBD_PreviewBaseline
{
	ref map<int, string> m_mGarments;       //!< loadout slot id -> prefab ("" = empty)
	ref map<int, string> m_mWeapons;        //!< engine weapon slot index -> prefab ("" = empty)
	ref map<int, bool> m_mWeaponAuthored;   //!< weapon slot index -> last pass put an AUTHORED weapon there

	//! Create the empty maps.
	void TBD_PreviewBaseline()
	{
		m_mGarments = new map<int, string>();
		m_mWeapons = new map<int, string>();
		m_mWeaponAuthored = new map<int, bool>();
	}
}

//! Stateless dresser for the kit-preview doll; the baselines are session-static.
class TBD_LoadoutPreviewDresser
{
	//! Vanilla's manager prefab (`SCR_LoadoutPreviewComponent.m_sPreviewManager`); TBD worlds place none.
	static const ResourceName PREVIEW_MANAGER_PREFAB = "{9F18C476AB860F3B}Prefabs/World/Game/ItemPreviewManager.et"; //!< spawned locally on the first preview when absent

	static const int WEAPON_SLOT_PRIMARY = 0; //!< engine weapon slot, same ids as `TBD_LoadoutGearPhase.Begin`
	static const int WEAPON_SLOT_LAUNCHER = 1; //!< engine weapon slot: the second untyped long slot
	static const int WEAPON_SLOT_HANDGUN = 2; //!< engine weapon slot: sidearm
	static const int WEAPON_SLOT_THROWABLE = 3; //!< engine weapon slot: grenade

	static const string LOG_CHANNEL = "lobby"; //!< `TBD_Log` channel of every preview warning
	static const string WARN_PREFIX = "kit preview: "; //!< text every preview warning starts with

	protected static ref map<string, ref TBD_PreviewBaseline> s_mBaselines; //!< kit prefab -> its recorded baseline; null until first use

	//! The world's ItemPreviewManager, spawned locally when the world has none (vanilla fallback).
	static ItemPreviewManagerEntity GetOrSpawnManager()
	{
		ChimeraWorld world = ChimeraWorld.CastFrom(GetGame().GetWorld());
		if (!world)
			return null;

		ItemPreviewManagerEntity manager = world.GetItemPreviewManager();
		if (manager)
			return manager;

		Resource res = Resource.Load(PREVIEW_MANAGER_PREFAB);
		if (res && res.IsValid())
			GetGame().SpawnEntityPrefabLocal(res, world);

		manager = world.GetItemPreviewManager();
		if (manager)
			Print("[TBD][lobby] kit preview: spawned ItemPreviewManager locally (the world places none)");
		else
			TBD_WarnOnce.Warn(LOG_CHANNEL, "manager", WARN_PREFIX + "the world has no ItemPreviewManager and the local spawn failed");

		return manager;
	}

	//! Resolve the preview entity for `basePrefab` and make it wear `loadout` (null = kit-only).
	//! Returns the entity to hand to `SetPreviewItem`, or null with `failReason` set.
	static IEntity Dress(ItemPreviewManagerEntity manager, ResourceName basePrefab, TBD_SlotLoadoutStruct loadout, out string failReason)
	{
		failReason = string.Empty;
		if (!manager)
		{
			failReason = "no preview manager";
			return null;
		}

		if (basePrefab.IsEmpty())
		{
			failReason = "kit prefab unresolved";
			return null;
		}

		IEntity ent = manager.ResolvePreviewEntityForPrefab(basePrefab);
		if (!ent)
		{
			failReason = "no preview entity for " + basePrefab;
			TBD_WarnOnce.Warn(LOG_CHANNEL, basePrefab, WARN_PREFIX + failReason);
			return null;
		}

		EquipedLoadoutStorageComponent loadoutStorage = EquipedLoadoutStorageComponent.Cast(ent.FindComponent(EquipedLoadoutStorageComponent));
		EquipedWeaponStorageComponent weaponStorage = EquipedWeaponStorageComponent.Cast(ent.FindComponent(EquipedWeaponStorageComponent));
		TBD_PreviewBaseline baseline = BaselineFor(basePrefab, loadoutStorage, weaponStorage);

		TBD_SlotGearStruct gear;
		if (loadout)
			gear = loadout.gear;

		string uniform, vest, helmet, pants, boots, handwear, backpack;
		string primary, launcher, handgun, throwable, optic, magazine;
		if (gear)
		{
			uniform = gear.uniform;
			vest = gear.vest;
			helmet = gear.helmet;
			pants = gear.pants;
			boots = gear.boots;
			handwear = gear.handwear;
			backpack = gear.backpack;
			primary = gear.primary;
			launcher = gear.launcher;
			handgun = gear.handgun;
			throwable = gear.throwable;
			optic = gear.optic;
			magazine = gear.magazine;
		}

		if (loadoutStorage)
		{
			DressArea(ent, loadoutStorage, baseline, "uniform",  uniform,  LoadoutJacketArea,       typename.Empty);
			int vestSlot = DressArea(ent, loadoutStorage, baseline, "vest", vest, LoadoutVestArea, LoadoutArmoredVestSlotArea);
			// The armored area is the vest's alternate landing slot; when the vest did not land there
			// it is restored on its own, or a previous seat's plate carrier would linger.
			InventoryStorageSlot armored = SlotForArea(loadoutStorage, LoadoutArmoredVestSlotArea);
			if (armored && armored.GetID() != vestSlot)
				DressArea(ent, loadoutStorage, baseline, "armored vest", string.Empty, LoadoutArmoredVestSlotArea, typename.Empty);
			DressArea(ent, loadoutStorage, baseline, "helmet",   helmet,   LoadoutHeadCoverArea,    typename.Empty);
			DressArea(ent, loadoutStorage, baseline, "pants",    pants,    LoadoutPantsArea,        typename.Empty);
			DressArea(ent, loadoutStorage, baseline, "boots",    boots,    LoadoutBootsArea,        typename.Empty);
			DressArea(ent, loadoutStorage, baseline, "handwear", handwear, LoadoutHandwearSlotArea, typename.Empty);
			DressArea(ent, loadoutStorage, baseline, "backpack", backpack, LoadoutBackpackArea,     typename.Empty);
		}
		else
		{
			TBD_WarnOnce.Warn(LOG_CHANNEL, "loadoutStorage:" + basePrefab, WARN_PREFIX + "preview entity has no EquipedLoadoutStorageComponent -- garments not dressed");
		}

		IEntity primaryWeapon;
		if (weaponStorage)
		{
			primaryWeapon = DressWeapon(ent, weaponStorage, baseline, "primary", primary, WEAPON_SLOT_PRIMARY);
			DressWeapon(ent, weaponStorage, baseline, "launcher",  launcher,  WEAPON_SLOT_LAUNCHER);
			DressWeapon(ent, weaponStorage, baseline, "handgun",   handgun,   WEAPON_SLOT_HANDGUN);
			DressWeapon(ent, weaponStorage, baseline, "throwable", throwable, WEAPON_SLOT_THROWABLE);
		}
		else
		{
			TBD_WarnOnce.Warn(LOG_CHANNEL, "weaponStorage:" + basePrefab, WARN_PREFIX + "preview entity has no EquipedWeaponStorageComponent -- weapons not dressed");
		}

		if (primaryWeapon && !primary.IsEmpty())
		{
			TBD_LoadoutPreviewMount.Mount(ent, primaryWeapon, "optic", optic);
			TBD_LoadoutPreviewMount.Mount(ent, primaryWeapon, "magazine", magazine);
		}

		TBD_LoadoutPreviewMount.HoldWeapon(ent, primaryWeapon);
		return ent;
	}

	//! The prefab's untouched state, recorded once per prefab. Null when nothing could be read (the
	//! entity had not populated its slots yet) -- callers then fall back to the slot template.
	protected static TBD_PreviewBaseline BaselineFor(ResourceName basePrefab, EquipedLoadoutStorageComponent loadoutStorage, EquipedWeaponStorageComponent weaponStorage)
	{
		if (!s_mBaselines)
			s_mBaselines = new map<string, ref TBD_PreviewBaseline>();

		TBD_PreviewBaseline known;
		if (s_mBaselines.Find(basePrefab, known))
			return known;

		TBD_PreviewBaseline baseline = new TBD_PreviewBaseline();
		int populated;
		int i;
		if (loadoutStorage)
		{
			for (i = 0; i < loadoutStorage.GetSlotsCount(); i++)
			{
				InventoryStorageSlot slot = loadoutStorage.GetSlot(i);
				if (!slot)
					continue;

				string prefab = TBD_LoadoutInventoryUtil.PrefabOf(slot.GetAttachedEntity());
				baseline.m_mGarments.Insert(slot.GetID(), prefab);
				if (!prefab.IsEmpty())
					populated++;
			}
		}

		if (weaponStorage)
		{
			for (i = 0; i < weaponStorage.GetSlotsCount(); i++)
			{
				InventoryStorageSlot slot = weaponStorage.GetSlot(i);
				if (!slot)
					continue;

				string prefab = TBD_LoadoutInventoryUtil.PrefabOf(slot.GetAttachedEntity());
				baseline.m_mWeapons.Insert(i, prefab);
				if (!prefab.IsEmpty())
					populated++;
			}
		}

		if (populated == 0)
		{
			TBD_WarnOnce.Warn(LOG_CHANNEL, "baseline:" + basePrefab, WARN_PREFIX + "preview entity held nothing when first resolved -- kit-only restore falls back to slot templates for " + basePrefab);
			return null;
		}

		s_mBaselines.Insert(basePrefab, baseline);
		return baseline;
	}

	//! One garment area. `altArea` is the second candidate the server pass also accepts (a plate
	//! carrier authored as "vest" lands in the armored area). Returns the slot id the item sits in
	//! after the pass (-1 when the area has no slot).
	protected static int DressArea(IEntity ent, EquipedLoadoutStorageComponent storage, TBD_PreviewBaseline baseline, string label, string authored, typename area, typename altArea)
	{
		InventoryStorageSlot slot = SlotForArea(storage, area);
		InventoryStorageSlot altSlot;
		if (altArea)
			altSlot = SlotForArea(storage, altArea);

		if (!slot)
		{
			slot = altSlot;
			altSlot = null;
		}

		if (!slot)
		{
			if (!authored.IsEmpty())
				TBD_WarnOnce.Warn(LOG_CHANNEL, label + ":" + authored, WARN_PREFIX + string.Format("preview character has no %1 slot for %2", label, authored));
			return -1;
		}

		bool knownEmpty;
		string desired = Desired(authored, baseline, slot, knownEmpty);
		IEntity current = slot.GetAttachedEntity();

		if (desired.IsEmpty())
		{
			// Nothing authored, and the kit leaves this slot empty: take a previous seat's item off.
			if (knownEmpty && current)
			{
				slot.DetachEntity();
				SCR_EntityHelper.DeleteEntityAndChildren(current);
			}

			return slot.GetID();
		}

		if (TBD_LoadoutInventoryUtil.PrefabOf(current) == desired)
			return slot.GetID();

		IEntity item = SpawnLocal(desired, ent);
		if (!item)
		{
			TBD_WarnOnce.Warn(LOG_CHANNEL, label + ":" + desired, WARN_PREFIX + string.Format("%1 prefab failed to load: %2", label, desired));
			return slot.GetID();
		}

		// The alt slot wins only when it is the one that accepts the item (plate carrier case).
		if (altSlot && !storage.CanStoreItem(item, slot.GetID()) && storage.CanStoreItem(item, altSlot.GetID()))
		{
			slot = altSlot;
			current = slot.GetAttachedEntity();
			if (TBD_LoadoutInventoryUtil.PrefabOf(current) == desired)
			{
				SCR_EntityHelper.DeleteEntityAndChildren(item);
				return slot.GetID();
			}
		}

		if (current)
		{
			slot.DetachEntity();
			SCR_EntityHelper.DeleteEntityAndChildren(current);
		}

		slot.AttachEntity(item);
		return slot.GetID();
	}

	//! authored, else the baseline (which may say "empty" -- `knownEmpty`), else the slot template.
	protected static string Desired(string authored, TBD_PreviewBaseline baseline, InventoryStorageSlot slot, out bool knownEmpty)
	{
		knownEmpty = false;
		if (!authored.IsEmpty())
			return authored;

		string fromBaseline;
		if (baseline && baseline.m_mGarments.Find(slot.GetID(), fromBaseline))
		{
			knownEmpty = fromBaseline.IsEmpty();
			return fromBaseline;
		}

		return slot.GetSlotTemplate();
	}

	//! One engine weapon slot. Returns the weapon now in the slot (authored, baseline or untouched).
	protected static IEntity DressWeapon(IEntity ent, EquipedWeaponStorageComponent storage, TBD_PreviewBaseline baseline, string label, string authored, int slotIndex)
	{
		if (slotIndex < 0 || slotIndex >= storage.GetSlotsCount())
		{
			if (!authored.IsEmpty())
				TBD_WarnOnce.Warn(LOG_CHANNEL, label + ":slot", WARN_PREFIX + string.Format("preview character has no engine weapon slot %1 for %2", slotIndex, label));
			return null;
		}

		InventoryStorageSlot slot = storage.GetSlot(slotIndex);
		if (!slot)
			return null;

		IEntity current = slot.GetAttachedEntity();
		bool wasAuthored;
		if (baseline)
			baseline.m_mWeaponAuthored.Find(slotIndex, wasAuthored);

		string desired = authored;
		bool knownEmpty;
		if (desired.IsEmpty() && baseline)
		{
			string fromBaseline;
			if (baseline.m_mWeapons.Find(slotIndex, fromBaseline))
			{
				desired = fromBaseline;
				knownEmpty = desired.IsEmpty();
			}
		}

		if (desired.IsEmpty() && !knownEmpty)
			desired = slot.GetSlotTemplate();

		if (desired.IsEmpty())
		{
			if (knownEmpty && current)
			{
				slot.DetachEntity();
				SCR_EntityHelper.DeleteEntityAndChildren(current);
				current = null;
			}

			MarkAuthored(baseline, slotIndex, false);
			return current;
		}

		// A baseline weapon that is already the right prefab and was never authored over stays;
		// anything authored is respawned so the previous seat's attachments cannot linger.
		if (authored.IsEmpty() && !wasAuthored && TBD_LoadoutInventoryUtil.PrefabOf(current) == desired)
			return current;

		IEntity item = SpawnLocal(desired, ent);
		if (!item)
		{
			TBD_WarnOnce.Warn(LOG_CHANNEL, label + ":" + desired, WARN_PREFIX + string.Format("%1 prefab failed to load: %2", label, desired));
			return current;
		}

		if (current)
		{
			slot.DetachEntity();
			SCR_EntityHelper.DeleteEntityAndChildren(current);
		}

		slot.AttachEntity(item);
		MarkAuthored(baseline, slotIndex, !authored.IsEmpty());
		return item;
	}

	//! Record whether the last pass put an authored weapon in `slotIndex`; no-op without a baseline.
	protected static void MarkAuthored(TBD_PreviewBaseline baseline, int slotIndex, bool authored)
	{
		if (baseline)
			baseline.m_mWeaponAuthored.Set(slotIndex, authored);
	}

	//! The loadout storage slot for `area`.
	//! @return the slot, or null when the body has no slot for that area
	protected static InventoryStorageSlot SlotForArea(EquipedLoadoutStorageComponent storage, typename area)
	{
		LoadoutSlotInfo info = storage.GetSlotFromArea(area);
		if (!info)
			return null;

		return storage.GetSlot(info.GetID());
	}

	//! Spawn a prefab locally in the preview entity's own world (never the game world).
	static IEntity SpawnLocal(string resName, IEntity previewEntity)
	{
		Resource resource = Resource.Load(resName);
		if (!resource || !resource.IsValid())
			return null;

		return GetGame().SpawnEntityPrefabLocal(resource, previewEntity.GetWorld());
	}

}
