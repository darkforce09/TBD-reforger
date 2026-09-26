/**
 * @file TBD_GadgetFlags.c
 * @brief Adds or withholds each player's map, compass, watch, GPS and radio from `slots[].gadgets`.
 *
 * Role: a second `JsonLoadContext` pass over the held mission JSON whose root declares only
 * `slots[]` gadgets, and the inventory edits that honour them after each player spawn.
 * Position: `Bind` runs from `TBD_MissionLoader` after a valid parse and hooks
 * `SCR_BaseGameMode.GetOnPlayerSpawned`; reads `TBD_MissionJsonPass.LoadRoot`,
 * `TBD_SpawnManager.GetAssignedSlot` and the body's gadget and inventory managers.
 * State: the static parsed rows keyed to the mission id, and the one-time spawn hook, server only.
 * Invariants: `gadgets` binds as `map<string, bool>`, so an omitted block (Count 0) keeps the
 * kit's gadgets and each flag's presence is `Find`; a flag authored true adds an item only when
 * the body has none of that type; apply runs `POST_LOADOUT_MS` after spawn so loadout cargo
 * cannot put a withheld gadget back.
 */

//! One `slots[]` row's gadget flags. Field names are the JSON keys.
//! @contract mission.schema.json#/$defs/slot
class TBD_GadgetFlagsSlotWireStruct
{
	string id; //!< `id`
	string uid; //!< `uid`, or empty
	//! Per-flag presence is Find(), not a null check.
	ref map<string, bool> gadgets; //!< `gadgets` (`$defs/gadgetFlags`); allocated empty when omitted
}

//! Root of the second parse. Declares `slots` and nothing else.
//! @contract mission.schema.json#/properties/slots
class TBD_GadgetFlagsDocStruct
{
	ref array<ref TBD_GadgetFlagsSlotWireStruct> slots; //!< `slots[]`
}

//! Server-side reader: bind slot.gadgets and apply after loadout on player spawn.
class TBD_GadgetFlags
{
	static const string CH = "Gadgets"; //!< log channel
	static const string KEY_MAP = "map"; //!< `gadgets.map`
	static const string KEY_COMPASS = "compass"; //!< `gadgets.compass`
	static const string KEY_WATCH = "watch"; //!< `gadgets.watch`
	static const string KEY_GPS = "gps"; //!< `gadgets.gps`
	static const string KEY_RADIO = "radio"; //!< `gadgets.radio`

	//! Kit-neutral defaults used only when the flag is authored true and the body has none.
	static const string PREFAB_MAP = "{7B6990100263E2B3}Prefabs/Items/Core/Map_Base.et"; //!< map added for `map` true
	static const string PREFAB_COMPASS = "{61D4F80E49BF9B12}Prefabs/Items/Equipment/Compass/Compass_SY183.et"; //!< compass added for `compass` true
	static const string PREFAB_WATCH = "{6FD6C96121905202}Prefabs/Items/Equipment/Watches/Watch_Vostok.et"; //!< watch added for `watch` true
	static const string PREFAB_RADIO = "{E1A5D4B878AA8980}Prefabs/Items/Equipment/Radios/Radio_R148.et"; //!< radio added for `radio` true

	//! Loadout cargo lands in VerifyTick (500 ms after Run). Apply after that so a cargo
	//! row cannot put a withheld gadget back.
	protected static const int POST_LOADOUT_MS = 800; //!< milliseconds from spawn to apply

	protected static ref array<ref TBD_GadgetFlagsSlotWireStruct> s_aSlots; //!< parsed `slots[]` rows; empty when none
	protected static bool s_bParsed; //!< `s_aSlots` is current for `s_sParsedForMission`
	protected static bool s_bArmed; //!< the spawn hook is installed
	protected static string s_sParsedForMission; //!< mission id `s_aSlots` was parsed for

	//! Parse the gadgets block and arm the spawn hook. Called from `TBD_MissionLoader` after a
	//! valid parse. A mission whose slots author no gadgets keeps every kit's gadgets.
	//! @authority server
	static void Bind()
	{
		s_bParsed = false;
		s_sParsedForMission = string.Empty;
		Parse();
		ArmSpawnHook();
	}

	//! Hook `OnPlayerSpawned` on the game mode once; does nothing without a game mode.
	protected static void ArmSpawnHook()
	{
		if (s_bArmed)
			return;

		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
		if (!gm)
			return;

		gm.GetOnPlayerSpawned().Insert(OnPlayerSpawned);
		s_bArmed = true;
	}

	//! Spawn-notify sink. Dressing already ran in SpawnSlotBody; cargo verify is still in
	//! flight, so apply is deferred POST_LOADOUT_MS. Ignores clients and non-player ids.
	//! @authority server
	protected static void OnPlayerSpawned(int playerId, IEntity controlledEntity)
	{
		if (TBD_Authority.IsClient())
			return;
		if (playerId <= 0)
			return;

		GetGame().GetCallqueue().CallLater(ApplyForPlayer, POST_LOADOUT_MS, false, playerId);
	}

	//! Apply the flags of the player's assigned slot to the body the player controls; does nothing
	//! when either is missing.
	//! @authority server
	protected static void ApplyForPlayer(int playerId)
	{
		if (TBD_Authority.IsClient())
			return;

		PlayerManager pm = GetGame().GetPlayerManager();
		if (!pm)
			return;

		IEntity body = pm.GetPlayerControlledEntity(playerId);
		if (!body)
			return;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return;

		TBD_MissionSlotStruct slot = spawn.GetAssignedSlot(playerId);
		if (!slot)
			return;

		ApplyToBody(body, slot.Key(), slot.id);
	}

	//! Apply the flags authored for one slot to `body`. Does nothing when the slot authors none; a
	//! body without a gadget manager logs a WARNING.
	//! @param slotKey the slot's durable key (`uid`, else `id`)
	//! @param slotId the slot's derived `id`
	//! @authority server
	static void ApplyToBody(IEntity body, string slotKey, string slotId)
	{
		if (!body)
			return;
		if (!Parse())
			return;

		TBD_GadgetFlagsSlotWireStruct wire = FindSlot(slotKey, slotId);
		if (!wire)
			return;
		if (!wire.gadgets)
			return;
		if (wire.gadgets.Count() < 1)
			return;

		SCR_GadgetManagerComponent gadgets = SCR_GadgetManagerComponent.GetGadgetManager(body);
		if (!gadgets)
		{
			Print(string.Format("[TBD][%1] slot=%2 no gadget manager -- flags not applied", CH, slotKey), LogLevel.WARNING);
			return;
		}

		bool wantMap;
		bool compass;
		bool watch;
		bool gps;
		bool radio;

		if (wire.gadgets.Find(KEY_MAP, wantMap))
			ApplyOne(body, gadgets, KEY_MAP, wantMap, EGadgetType.MAP, PREFAB_MAP);
		if (wire.gadgets.Find(KEY_COMPASS, compass))
			ApplyOne(body, gadgets, KEY_COMPASS, compass, EGadgetType.COMPASS, PREFAB_COMPASS);
		if (wire.gadgets.Find(KEY_WATCH, watch))
			ApplyByNeedle(body, KEY_WATCH, watch, "Watch", PREFAB_WATCH);
		if (wire.gadgets.Find(KEY_GPS, gps))
			ApplyByNeedle(body, KEY_GPS, gps, "GPS", string.Empty);
		if (wire.gadgets.Find(KEY_RADIO, radio))
			ApplyRadio(body, gadgets, radio);
	}

	//! Ensure or remove one gadget type.
	protected static void ApplyOne(IEntity body, notnull SCR_GadgetManagerComponent gadgets, string key, bool want, EGadgetType type, string prefab)
	{
		if (want)
		{
			EnsureType(body, gadgets, key, type, prefab);
			return;
		}

		RemoveType(gadgets, key, type);
	}

	//! Watch and GPS have no EGadgetType member on this engine (compile-proved: WATCH
	//! is undefined). Match inventory prefab paths. GPS has no item in the TBD registry,
	//! so authored true cannot add one; authored false still withholds a GPS-named item.
	protected static void ApplyByNeedle(IEntity body, string key, bool want, string needle, string prefab)
	{
		if (want)
		{
			if (HasPrefabNeedle(body, needle))
				return;
			if (prefab.IsEmpty())
			{
				Print(string.Format("[TBD][%1] %2=true but this engine has no %2 item to add -- kit default kept", CH, key), LogLevel.WARNING);
				return;
			}
			EnsurePrefab(body, key, prefab);
			return;
		}

		RemoveByNeedle(body, key, needle);
	}

	//! Ensure a handheld or backpack radio, or remove both.
	protected static void ApplyRadio(IEntity body, notnull SCR_GadgetManagerComponent gadgets, bool want)
	{
		if (want)
		{
			IEntity handheld = gadgets.GetGadgetByType(EGadgetType.RADIO);
			if (handheld)
				return;
			IEntity backpack = gadgets.GetGadgetByType(EGadgetType.RADIO_BACKPACK);
			if (backpack)
				return;
			EnsurePrefab(body, KEY_RADIO, PREFAB_RADIO);
			return;
		}

		RemoveType(gadgets, KEY_RADIO, EGadgetType.RADIO);
		RemoveType(gadgets, KEY_RADIO, EGadgetType.RADIO_BACKPACK);
	}

	//! Add `prefab` unless the body already carries a gadget of `type`.
	protected static void EnsureType(IEntity body, notnull SCR_GadgetManagerComponent gadgets, string key, EGadgetType type, string prefab)
	{
		IEntity existing = gadgets.GetGadgetByType(type);
		if (existing)
			return;
		EnsurePrefab(body, key, prefab);
	}

	//! Spawn `prefab` and insert it into the body's inventory; an empty prefab, a failed spawn or a
	//! refused insert logs a WARNING and discards the item.
	protected static void EnsurePrefab(IEntity body, string key, string prefab)
	{
		if (prefab.IsEmpty())
		{
			Print(string.Format("[TBD][%1] slot body wants %2=true but no prefab is known -- not added", CH, key), LogLevel.WARNING);
			return;
		}

		IEntity item = SpawnItem(body, prefab);
		if (!item)
		{
			Print(string.Format("[TBD][%1] %2=true prefab failed to spawn %3", CH, key, prefab), LogLevel.WARNING);
			return;
		}

		SCR_InventoryStorageManagerComponent inv = SCR_InventoryStorageManagerComponent.Cast(
			body.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!inv)
		{
			Print(string.Format("[TBD][%1] %2=true no inventory manager -- spawned item discarded", CH, key), LogLevel.WARNING);
			SCR_EntityHelper.DeleteEntityAndChildren(item);
			return;
		}

		if (inv.CanInsertItem(item))
		{
			if (inv.TryInsertItem(item))
			{
				Print(string.Format("[TBD][%1] added %2", CH, key));
				return;
			}
		}

		Print(string.Format("[TBD][%1] %2=true insert refused -- item discarded", CH, key), LogLevel.WARNING);
		SCR_EntityHelper.DeleteEntityAndChildren(item);
	}

	//! Delete every gadget of `type` the body carries and log the count.
	protected static void RemoveType(notnull SCR_GadgetManagerComponent gadgets, string key, EGadgetType type)
	{
		array<SCR_GadgetComponent> found = gadgets.GetGadgetsByType(type);
		if (!found)
			return;
		if (found.Count() < 1)
			return;

		int removed = 0;
		foreach (SCR_GadgetComponent gadget : found)
		{
			if (!gadget)
				continue;
			IEntity item = gadget.GetOwner();
			if (!item)
				continue;
			SCR_EntityHelper.DeleteEntityAndChildren(item);
			removed++;
		}

		if (removed > 0)
			Print(string.Format("[TBD][%1] withheld %2 count=%3", CH, key, removed));
	}

	//! Whether any inventory item's prefab path matches `needle`.
	protected static bool HasPrefabNeedle(IEntity body, string needle)
	{
		array<IEntity> items = CollectItems(body);
		if (!items)
			return false;

		foreach (IEntity item : items)
		{
			if (PrefabMatchesNeedle(item, needle))
				return true;
		}
		return false;
	}

	//! Delete every inventory item whose prefab path matches `needle` and log the count.
	protected static void RemoveByNeedle(IEntity body, string key, string needle)
	{
		array<IEntity> items = CollectItems(body);
		if (!items)
			return;

		int removed = 0;
		foreach (IEntity item : items)
		{
			if (!PrefabMatchesNeedle(item, needle))
				continue;
			SCR_EntityHelper.DeleteEntityAndChildren(item);
			removed++;
		}

		if (removed > 0)
			Print(string.Format("[TBD][%1] withheld %2 count=%3", CH, key, removed));
	}

	//! Whether the item's prefab path contains `needle` (`GPS` also matches `Gps`).
	protected static bool PrefabMatchesNeedle(IEntity item, string needle)
	{
		if (!item)
			return false;
		if (needle.IsEmpty())
			return false;

		string prefab = TBD_LoadoutInventoryUtil.PrefabOf(item);
		if (prefab.Contains(needle))
			return true;
		if (needle == "GPS" && prefab.Contains("Gps"))
			return true;
		return false;
	}

	//! Every item in the body's inventory, or null without an inventory manager.
	protected static array<IEntity> CollectItems(IEntity body)
	{
		if (!body)
			return null;

		SCR_InventoryStorageManagerComponent inv = SCR_InventoryStorageManagerComponent.Cast(
			body.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!inv)
			return null;

		array<IEntity> items = new array<IEntity>();
		inv.GetItems(items);
		return items;
	}

	//! Spawn `prefab` at the body's transform.
	//! @return the item, or null when the prefab does not load
	protected static IEntity SpawnItem(IEntity body, string prefab)
	{
		Resource resource = Resource.Load(prefab);
		if (!resource)
			return null;
		if (!resource.IsValid())
			return null;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		body.GetTransform(params.Transform);
		return GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
	}

	//! Parse the gadgets pass into `s_aSlots` once per mission id.
	//! @return false when no mission text is held; true otherwise, with an ERROR line and no rows
	//! when the text or its root does not read
	protected static bool Parse()
	{
		string missionId = TBD_MissionLoader.GetMissionId();
		if (s_bParsed && missionId == s_sParsedForMission)
			return true;

		s_aSlots = new array<ref TBD_GadgetFlagsSlotWireStruct>();

		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (outcome == TBD_EMissionJsonPassOutcome.NO_DOCUMENT)
			return false;

		if (!ctx)
		{
			Print(string.Format("[TBD][%1] the mission document did not parse as JSON on the gadgets pass -- kit defaults kept", CH), LogLevel.ERROR);
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		TBD_GadgetFlagsDocStruct doc = new TBD_GadgetFlagsDocStruct();
		if (!ctx.ReadValue("", doc))
		{
			Print(string.Format("[TBD][%1] the mission document parsed but its root would not read on the gadgets pass -- kit defaults kept", CH), LogLevel.ERROR);
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		if (doc.slots)
			s_aSlots = doc.slots;

		int authored = CountAuthored();
		s_bParsed = true;
		s_sParsedForMission = missionId;
		Print(string.Format("[TBD][%1] bound slots=%2 withFlags=%3", CH, s_aSlots.Count(), authored));
		return true;
	}

	//! How many parsed rows author at least one flag.
	protected static int CountAuthored()
	{
		if (!s_aSlots)
			return 0;

		int n = 0;
		foreach (TBD_GadgetFlagsSlotWireStruct row : s_aSlots)
		{
			if (!row)
				continue;
			if (!row.gadgets)
				continue;
			if (row.gadgets.Count() < 1)
				continue;
			n++;
		}
		return n;
	}

	//! The parsed row of a slot: by `uid` equal to `slotKey` first, else by `id`.
	//! @return the row, or null
	protected static TBD_GadgetFlagsSlotWireStruct FindSlot(string slotKey, string slotId)
	{
		if (!s_aSlots)
			return null;

		foreach (TBD_GadgetFlagsSlotWireStruct row : s_aSlots)
		{
			if (!row)
				continue;
			if (slotKey.IsEmpty())
				continue;
			if (row.uid.IsEmpty())
				continue;
			if (row.uid == slotKey)
				return row;
		}

		foreach (TBD_GadgetFlagsSlotWireStruct row2 : s_aSlots)
		{
			if (!row2)
				continue;
			if (slotId.IsEmpty())
				continue;
			if (row2.id == slotId)
				return row2;
		}

		return null;
	}
}
