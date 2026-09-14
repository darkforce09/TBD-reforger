/**
 * TBD_LoadoutPreviewDresser.c - dress a PREVIEW character with a slot's exact kit (kit-preview pass,
 * 2026-09-13).
 *
 * The client twin of `TBD_LoadoutApplication` (TBD_LoadoutEquipHelper.c). Same inputs — a kit
 * prefab resolved through `TBD_Registry` plus the JSON `TBD_SlotLoadoutStruct` — same composition
 * rule (an authored gear slot REPLACES the kit's garment, an absent one KEEPS it, optic + magazine
 * mount into the primary), but on the `ItemPreviewManagerEntity`'s preview entity instead of the
 * spawned slot body. It is NOT a subclass of the application, on purpose:
 *  - the application spawns REPLICATED entities (`SpawnEntityPrefab`) into the game world; this
 *    spawns LOCAL ones (`SpawnEntityPrefabLocal`) into the preview entity's own world;
 *  - the application polls a worn-verify for 3 s, audits nakedness and hard-ERRORs — every one of
 *    those is right for a body a player will inhabit and wrong for a menu thumbnail. Here a miss
 *    is one WARNING line, once per reason, and the doll simply wears less;
 *  - `AttachEntity` on the storage slots is synchronous, so the doll is complete on return.
 *
 * The recipe is vanilla's `SCR_LoadoutPreviewComponent.SetPreviewedLoadout` (read from the pak
 * this pass): resolve the preview entity for the prefab, attach garments to
 * `EquipedLoadoutStorageComponent` slots and weapons to `EquipedWeaponStorageComponent` slots,
 * mount attachments through the weapon's `WeaponAttachmentsStorageComponent`, `SelectWeapon` the
 * one to hold, then `SetPreviewItem(widget, entity, attributes, true)` (the caller's job).
 *
 * THE CACHE RULE (MEASURED run 1: after the AT seat every seat carried the RPG).
 * `ResolvePreviewEntityForPrefab` hands back ONE entity per prefab for the life of the manager, so
 * every seat with the same kit prefab shares a doll, and `InventoryStorageSlot.GetSlotTemplate()`
 * is EMPTY for the baked weapons — there is nothing on the slot to fall back to. So the first time a
 * prefab is resolved, BEFORE anything is dressed, its BASELINE is recorded: the prefab of what every
 * garment slot and weapon slot holds ("" = the kit leaves it empty). Every pass then computes, per
 * slot, DESIRED = authored prefab, else the baseline, and: equal → keep; different → swap; desired
 * empty with a baseline on record → CLEAR the slot. A kit-only seat after a fully authored one
 * therefore puts every kit garment back and takes the launcher off. Weapon slots also remember
 * whether the last pass AUTHORED them: an authored primary is always respawned (it may carry the
 * previous seat's optic), and a baseline restore over an authored weapon respawns too.
 *
 * ATTACHMENTS (MEASURED run 1: optic and magazine ended up on the muzzle). `CanStoreItem` on the
 * attachment storage accepts anything in a free slot; the typed check is the slot's own
 * `AttachmentSlotComponent.CanSetAttachment` (the storage slot's `GetParentContainer()`). The
 * magazine well is not always an attachment slot, so the magazine falls back to the route the
 * server pass uses — `SCR_InventoryStorageManagerComponent.TrySpawnPrefabToStorage` into the
 * weapon storage — and is verified by scanning the storage.
 *
 * Client-side, menu-time code: no authority, no replication, no wire.
 */

//! What a kit prefab's preview entity held before any loadout touched it (see the header).
class TBD_PreviewBaseline
{
	ref map<int, string> m_mGarments;       //!< loadout slot id -> prefab ("" = empty)
	ref map<int, string> m_mWeapons;        //!< engine weapon slot index -> prefab ("" = empty)
	ref map<int, bool> m_mWeaponAuthored;   //!< weapon slot index -> last pass put an AUTHORED weapon there

	void TBD_PreviewBaseline()
	{
		m_mGarments = new map<int, string>();
		m_mWeapons = new map<int, string>();
		m_mWeaponAuthored = new map<int, bool>();
	}
}

class TBD_LoadoutPreviewDresser
{
	//! Vanilla's manager prefab (`SCR_LoadoutPreviewComponent.m_sPreviewManager`); TBD worlds place
	//! none, so the local spawn below runs on the first preview of a session.
	static const ResourceName PREVIEW_MANAGER_PREFAB = "{9F18C476AB860F3B}Prefabs/World/Game/ItemPreviewManager.et";

	//! Engine weapon slot indices — byte-identical to TBD_LoadoutApplication.Run / arsenal_rules.rs.
	static const int WEAPON_SLOT_PRIMARY = 0;
	static const int WEAPON_SLOT_LAUNCHER = 1;
	static const int WEAPON_SLOT_HANDGUN = 2;
	static const int WEAPON_SLOT_THROWABLE = 3;

	protected static ref array<string> s_aWarned;
	protected static ref map<string, ref TBD_PreviewBaseline> s_mBaselines;

	//------------------------------------------------------------------------------------------------
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
			WarnOnce("manager", "the world has no ItemPreviewManager and the local spawn failed");

		return manager;
	}

	//------------------------------------------------------------------------------------------------
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
			WarnOnce(basePrefab, failReason);
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

		// --- garments: desired = authored, else the baseline, else the slot template -----------
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
			WarnOnce("loadoutStorage:" + basePrefab, "preview entity has no EquipedLoadoutStorageComponent — garments not dressed");
		}

		// --- weapons ---------------------------------------------------------------------------
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
			WarnOnce("weaponStorage:" + basePrefab, "preview entity has no EquipedWeaponStorageComponent — weapons not dressed");
		}

		// --- optic + magazine ride the primary, as on the server ---------------------------
		if (primaryWeapon && !primary.IsEmpty())
		{
			Mount(ent, primaryWeapon, "optic", optic);
			Mount(ent, primaryWeapon, "magazine", magazine);
		}

		HoldWeapon(ent, primaryWeapon);
		return ent;
	}

	// ── baseline ────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! The prefab's untouched state, recorded once per prefab. Null when nothing could be read (the
	//! entity had not populated its slots yet) — callers then fall back to the slot template.
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

				string prefab = PrefabOf(slot.GetAttachedEntity());
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

				string prefab = PrefabOf(slot.GetAttachedEntity());
				baseline.m_mWeapons.Insert(i, prefab);
				if (!prefab.IsEmpty())
					populated++;
			}
		}

		if (populated == 0)
		{
			WarnOnce("baseline:" + basePrefab, "preview entity held nothing when first resolved — kit-only restore falls back to slot templates for " + basePrefab);
			return null;
		}

		s_mBaselines.Insert(basePrefab, baseline);
		return baseline;
	}

	// ── garments ────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
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
				WarnOnce(label + ":" + authored, string.Format("preview character has no %1 slot for %2", label, authored));
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

		if (PrefabOf(current) == desired)
			return slot.GetID();

		IEntity item = SpawnLocal(desired, ent);
		if (!item)
		{
			WarnOnce(label + ":" + desired, string.Format("%1 prefab failed to load: %2", label, desired));
			return slot.GetID();
		}

		// The alt slot wins only when it is the one that accepts the item (plate carrier case).
		if (altSlot && !storage.CanStoreItem(item, slot.GetID()) && storage.CanStoreItem(item, altSlot.GetID()))
		{
			slot = altSlot;
			current = slot.GetAttachedEntity();
			if (PrefabOf(current) == desired)
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

	//------------------------------------------------------------------------------------------------
	//! authored, else the baseline (which may say "empty" — `knownEmpty`), else the slot template.
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

	// ── weapons ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! One engine weapon slot. Returns the weapon now in the slot (authored, baseline or untouched).
	protected static IEntity DressWeapon(IEntity ent, EquipedWeaponStorageComponent storage, TBD_PreviewBaseline baseline, string label, string authored, int slotIndex)
	{
		if (slotIndex < 0 || slotIndex >= storage.GetSlotsCount())
		{
			if (!authored.IsEmpty())
				WarnOnce(label + ":slot", string.Format("preview character has no engine weapon slot %1 for %2", slotIndex, label));
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
		if (authored.IsEmpty() && !wasAuthored && PrefabOf(current) == desired)
			return current;

		IEntity item = SpawnLocal(desired, ent);
		if (!item)
		{
			WarnOnce(label + ":" + desired, string.Format("%1 prefab failed to load: %2", label, desired));
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

	//------------------------------------------------------------------------------------------------
	protected static void MarkAuthored(TBD_PreviewBaseline baseline, int slotIndex, bool authored)
	{
		if (baseline)
			baseline.m_mWeaponAuthored.Set(slotIndex, authored);
	}

	// ── attachments ─────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Mount an optic / magazine onto the primary: the attachment slot whose own type accepts it;
	//! the magazine additionally falls back to the inventory manager (the server pass's route).
	protected static void Mount(IEntity ent, IEntity weapon, string label, string prefab)
	{
		if (prefab.IsEmpty())
			return;

		WeaponAttachmentsStorageComponent storage = WeaponAttachmentsStorageComponent.Cast(weapon.FindComponent(WeaponAttachmentsStorageComponent));
		if (!storage)
		{
			WarnOnce(label + ":storage", string.Format("primary has no WeaponAttachmentsStorageComponent — %1 not mounted", label));
			return;
		}

		if (StorageHas(storage, prefab))
			return;

		IEntity item = SpawnLocal(prefab, ent);
		if (!item)
		{
			WarnOnce(label + ":" + prefab, string.Format("%1 prefab failed to load: %2", label, prefab));
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

		WarnOnce(label + ":fit:" + prefab, string.Format("no slot on the primary accepts %1 %2 — the weapon shows its own", label, prefab));
	}

	//------------------------------------------------------------------------------------------------
	protected static bool StorageHas(BaseInventoryStorageComponent storage, string prefab)
	{
		int count = storage.GetSlotsCount();
		int i;
		for (i = 0; i < count; i++)
		{
			InventoryStorageSlot slot = storage.GetSlot(i);
			if (slot && PrefabOf(slot.GetAttachedEntity()) == prefab)
				return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Put `weapon` in the doll's hands (vanilla: SelectWeapon on the slot that holds it).
	protected static void HoldWeapon(IEntity ent, IEntity weapon)
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

	// ── helpers ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected static InventoryStorageSlot SlotForArea(EquipedLoadoutStorageComponent storage, typename area)
	{
		LoadoutSlotInfo info = storage.GetSlotFromArea(area);
		if (!info)
			return null;

		return storage.GetSlot(info.GetID());
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn a prefab locally in the preview entity's own world (never the game world).
	protected static IEntity SpawnLocal(string resName, IEntity previewEntity)
	{
		Resource resource = Resource.Load(resName);
		if (!resource || !resource.IsValid())
			return null;

		return GetGame().SpawnEntityPrefabLocal(resource, previewEntity.GetWorld());
	}

	//------------------------------------------------------------------------------------------------
	//! Prefab ResourceName of an entity ("" for null / unresolvable).
	protected static string PrefabOf(IEntity ent)
	{
		if (!ent)
			return string.Empty;

		EntityPrefabData data = ent.GetPrefabData();
		if (!data)
			return string.Empty;

		return data.GetPrefabName();
	}

	//------------------------------------------------------------------------------------------------
	//! One WARNING per reason per session — a menu must never spam or ERROR.
	protected static void WarnOnce(string key, string message)
	{
		if (!s_aWarned)
			s_aWarned = {};

		if (s_aWarned.Contains(key))
			return;

		s_aWarned.Insert(key);
		Print("[TBD][lobby] kit preview: " + message, LogLevel.WARNING);
	}
}
