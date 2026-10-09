/**
 * @file TBD_LoadoutWeaponPhase.c
 * @brief The weapon phase of a loadout application: mount optic, magazine and attachments.
 *
 * Role: resolves the primary weapon once the gear phase settles, spawns each authored optic,
 * magazine and attachment into that weapon's storage, polls until each is mounted, and swaps out
 * the weapon's own magazine once when the authored one cannot take the well.  Position: owned by
 * `TBD_LoadoutApplication`, which calls `Begin` after the gear phase and `FinishRest` when this
 * phase settles; reports through the application's `Fail` and `Degrade` ledger.
 * State: the pending weapon-borne items and the resolved primary of one application, on the
 * server.  Invariants: the magazine swap runs at most once; an item that will not mount is stowed
 * loose (degraded) or failed, never dropped silently; optic incumbents are never deleted.
 */

//! One weapon-mounted item (optic, magazine or attachment) awaiting its mount verify. These go
//! straight into the primary weapon's storage rather than through the equip calls.
class TBD_PendingWeaponItem
{
	string label; //!< `optic`, `magazine` or `attach`
	string resName; //!< the item ResourceName
	bool mountIssued; //!< true when `TrySpawnPrefabToStorage` accepted the spawn into the weapon
}

//! Mounts the weapon-borne items of one loadout application onto its primary weapon.
class TBD_LoadoutWeaponPhase : Managed
{
	//! Weapon-mounted items settle faster than worn garments, so the mount poll is 4 x 250 ms.
	protected const int WEAPON_TICK_MS = 250; //!< ms between mount-verify ticks
	protected const int WEAPON_MAX_ATTEMPTS = 4; //!< mount-verify ticks per round

	protected TBD_LoadoutApplication m_Application; //!< the owning application; weak, it owns this phase
	protected IEntity m_Character; //!< the body being dressed; null once the engine deletes it
	protected TBD_SlotLoadoutStruct m_Loadout; //!< the authored loadout; owned by the application
	protected string m_sTag; //!< log tag, for example `[TBD][Loadout][Slot]`
	protected string m_sLabel; //!< slot id or harness label named on every log line
	protected ref array<ref TBD_PendingWeaponItem> m_aWeaponPending = {}; //!< issued items not yet mounted
	protected IEntity m_PrimaryWeapon; //!< the weapon the items mount onto; null when none resolved
	protected bool m_bWeaponSwapRetried; //!< true once the one magazine swap retry has run

	//! Bind the phase to its application and read the body, loadout, tag and label.
	void TBD_LoadoutWeaponPhase(TBD_LoadoutApplication application)
	{
		m_Application = application;
		m_Character = application.GetCharacter();
		m_Loadout = application.GetLoadout();
		m_sTag = application.GetTag();
		m_sLabel = application.GetLabel();
	}

	//! Drop every pending weapon-borne item.
	void Cancel()
	{
		m_aWeaponPending.Clear();
	}

	//! The weapon the authored optic, magazine and attachments belong to: the carried weapon whose
	//! prefab matches `primaryRes` (so a body holding a pistol still gets its rifle fitted), else
	//! the weapon in hand.
	//! @param primaryRes the authored primary; empty goes straight to the weapon in hand
	//! @return the weapon, or null when the body has no weapon manager or no weapon
	protected IEntity ResolvePrimaryWeapon(string primaryRes)
	{
		BaseWeaponManagerComponent weaponMgr = BaseWeaponManagerComponent.Cast(
			m_Character.FindComponent(BaseWeaponManagerComponent));
		if (!weaponMgr)
			return null;

		if (!primaryRes.IsEmpty())
		{
			array<IEntity> weapons = {};
			weaponMgr.GetWeaponsList(weapons);
			foreach (IEntity weapon : weapons)
			{
				if (TBD_LoadoutInventoryUtil.PrefabOf(weapon) == primaryRes)
					return weapon;
			}
		}

		BaseWeaponComponent current = weaponMgr.GetCurrentWeapon();
		if (current)
			return current.GetOwner();

		return null;
	}

	//! Mount the authored weapon-borne items once the gear phase has settled. With nothing to mount,
	//! or no primary to mount onto (each item then fails by name), the application's tail runs at
	//! once; otherwise the mount poll starts.
	//! @authority server
	void Begin()
	{
		if (!m_Application || m_Application.IsDone())
			return;
		if (!m_Character)
		{
			m_Application.Cancel("body superseded");
			return;
		}

		TBD_SlotGearStruct gear = m_Loadout.gear;
		bool hasAttach = false;
		if (gear && gear.attachments && gear.attachments.Count() > 0)
			hasAttach = true;
		if (!gear || (gear.optic.IsEmpty() && gear.magazine.IsEmpty() && !hasAttach))
		{
			m_Application.FinishRest();
			return;
		}

		m_PrimaryWeapon = ResolvePrimaryWeapon(gear.primary);
		if (!m_PrimaryWeapon)
		{
			if (!gear.optic.IsEmpty())
				m_Application.Fail("optic", gear.optic, "no primary weapon on the character to mount it on");
			if (!gear.magazine.IsEmpty())
				m_Application.Fail("magazine", gear.magazine, "no primary weapon on the character to load it into");
			if (gear.attachments)
			{
				foreach (string attachRes : gear.attachments)
					m_Application.Fail("attach", attachRes, "no primary weapon on the character to mount it on");
			}
			m_Application.FinishRest();
			return;
		}

		IssueWeaponItem("optic", gear.optic);
		IssueWeaponItem("magazine", gear.magazine);
		if (gear.attachments)
		{
			foreach (string attachRes : gear.attachments)
				IssueWeaponItem("attach", attachRes);
		}

		if (m_aWeaponPending.IsEmpty())
		{
			m_Application.FinishRest();
			return;
		}

		GetGame().GetCallqueue().CallLater(WeaponVerifyTick, WEAPON_TICK_MS, false, 1);
	}

	//! Write the per-attachment line `[TBD][Equip] attach=<res> result=<ok|failed>`.
	//! @param ok true writes `ok`, false writes `failed`
	static void LogAttachResult(string resName, bool ok)
	{
		string result = "failed";
		if (ok)
			result = "ok";
		Print(string.Format("[TBD][Equip] attach=%1 result=%2", resName, result));
	}

	//! Spawn one weapon-borne item straight into the primary's storage with
	//! `TrySpawnPrefabToStorage`; the storage selects the matching attachment slot or magazine well.
	//! A prefab that does not load is blocking; a primary without attachment storage is not.
	//! @param label `optic`, `magazine` or `attach`
	//! @param resName the item; empty issues nothing
	//! @authority server
	protected void IssueWeaponItem(string label, string resName)
	{
		if (resName.IsEmpty())
			return;

		// A bad ResourceName fails by name instead of reaching the inventory as a no-op.
		Resource probe = Resource.Load(resName);
		if (!probe || !probe.IsValid())
		{
			m_Application.Fail(label, resName, "prefab failed to load (bad or missing asset)");
			return;
		}

		BaseInventoryStorageComponent storage = TBD_LoadoutInventoryUtil.WeaponStorageOf(m_PrimaryWeapon);
		if (!storage)
		{
			// Non-blocking: the body is armed and dressed; the weapon has no rail or well for this.
			m_Application.Fail(label, resName, string.Format("primary weapon %1 has no attachment storage", TBD_LoadoutInventoryUtil.PrefabOf(m_PrimaryWeapon)), false);
			return;
		}

		if (TBD_LoadoutInventoryUtil.WeaponStorageHas(storage, resName))
		{
			Print(string.Format("%1 slot=%2 %3 mount-skipped (already on weapon) %4", m_sTag, m_sLabel, label, resName));
			if (label == "attach")
				LogAttachResult(resName, true);
			m_Application.CountGearApplied();
			return;
		}

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			m_Character.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!mgr)
		{
			m_Application.Fail(label, resName, "character has no SCR_InventoryStorageManagerComponent");
			return;
		}

		TBD_PendingWeaponItem pending = new TBD_PendingWeaponItem();
		pending.label = label;
		pending.resName = resName;
		pending.mountIssued = mgr.TrySpawnPrefabToStorage(resName, storage, -1, EStoragePurpose.PURPOSE_ANY);
		// A weapon prefab usually spawns with its own magazine in the well, so the first mount of the
		// authored magazine is expected to be declined. `ClearBlockingMagazine` evicts that magazine
		// and the verify tick re-issues it. The expected refusal logs at normal level so a WARNING in
		// this sequence is always a real fault; a mount that still fails is reported by the verify
		// tick at WARNING or ERROR.
		if (!pending.mountIssued)
			Print(string.Format("%1 slot=%2 %3 first mount attempt declined by the weapon storage %4 -- EXPECTED, not an error: the verify pass clears the weapon's own incumbent and re-issues",
				m_sTag, m_sLabel, label, resName));
		m_aWeaponPending.Insert(pending);
	}

	//! Evict any magazine in the weapon that is not the authored one and re-issue the authored
	//! magazine. Optic incumbents are never evicted: an occupied rail cannot be proven to be in the
	//! way, and nothing is deleted on a guess.
	//! @return true when a magazine was evicted, so another verify round is worth running
	protected bool ClearBlockingMagazine()
	{
		BaseInventoryStorageComponent storage = TBD_LoadoutInventoryUtil.WeaponStorageOf(m_PrimaryWeapon);
		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			m_Character.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!storage || !mgr)
			return false;

		bool cleared = false;
		foreach (TBD_PendingWeaponItem p : m_aWeaponPending)
		{
			if (p.label != "magazine")
				continue;

			array<IEntity> items = {};
			storage.GetAll(items, true);
			foreach (IEntity item : items)
			{
				if (!item.FindComponent(BaseMagazineComponent))
					continue;
				if (TBD_LoadoutInventoryUtil.PrefabOf(item) == p.resName)
					continue; // already the authored magazine

				// The intended swap: the weapon's own magazine gives way to the authored one. Logged at
				// normal level; the verify tick reports a swap that still fails.
				Print(string.Format("%1 slot=%2 magazine deterministic swap: evicting the weapon's own %3 so the mission's %4 can take the well -- intended, not a problem",
					m_sTag, m_sLabel, TBD_LoadoutInventoryUtil.PrefabOf(item), p.resName));
				SCR_EntityHelper.DeleteEntityAndChildren(item);
				cleared = true;
			}

			if (cleared)
				p.mountIssued = mgr.TrySpawnPrefabToStorage(p.resName, storage, -1, EStoragePurpose.PURPOSE_ANY);
		}

		return cleared;
	}

	//! One mount-verify tick. When the attempts run out with items still pending, the magazine swap
	//! runs once and starts a new round. Items still pending after that are stowed loose in the body's
	//! inventory (degraded) or, when the body is full, failed as non-blocking; then the application's
	//! tail runs. A deleted body cancels the pass.
	//! @param attempt 1-based tick number
	//! @authority server
	protected void WeaponVerifyTick(int attempt)
	{
		if (!m_Application || m_Application.IsDone())
			return;
		if (!m_Character)
		{
			m_Application.Cancel("body superseded");
			return;
		}

		BaseInventoryStorageComponent storage = TBD_LoadoutInventoryUtil.WeaponStorageOf(m_PrimaryWeapon);

		for (int i = m_aWeaponPending.Count() - 1; i >= 0; i--)
		{
			TBD_PendingWeaponItem p = m_aWeaponPending[i];
			if (!TBD_LoadoutInventoryUtil.WeaponStorageHas(storage, p.resName))
				continue;

			Print(string.Format("%1 slot=%2 %3 mount OK %4 (on %5)", m_sTag, m_sLabel, p.label, p.resName, TBD_LoadoutInventoryUtil.PrefabOf(m_PrimaryWeapon)));
			if (p.label == "attach")
				LogAttachResult(p.resName, true);
			m_Application.CountGearApplied();
			m_aWeaponPending.Remove(i);
		}

		if (!m_aWeaponPending.IsEmpty() && attempt < WEAPON_MAX_ATTEMPTS)
		{
			GetGame().GetCallqueue().CallLater(WeaponVerifyTick, WEAPON_TICK_MS, false, attempt + 1);
			return;
		}

		// The magazine swap, once, and only after the normal attempts: a working magazine is never
		// evicted on speculation.
		if (!m_aWeaponPending.IsEmpty() && !m_bWeaponSwapRetried)
		{
			m_bWeaponSwapRetried = true;
			if (ClearBlockingMagazine())
			{
				GetGame().GetCallqueue().CallLater(WeaponVerifyTick, WEAPON_TICK_MS, false, 1);
				return;
			}
		}

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			m_Character.FindComponent(SCR_InventoryStorageManagerComponent));

		foreach (TBD_PendingWeaponItem straggler : m_aWeaponPending)
		{
			bool stowed = false;
			if (mgr)
				stowed = mgr.TrySpawnPrefabToStorage(straggler.resName, null, -1, EStoragePurpose.PURPOSE_ANY);

			if (stowed)
				m_Application.Degrade(straggler.label, straggler.resName, "would not mount on the primary -- stowed loose in the character's inventory");
			else
				// Non-blocking: the item exists but neither mounts nor fits; the body keeps its
				// weapon and clothes.
				m_Application.Fail(straggler.label, straggler.resName, "would not mount on the primary and would not fit in the inventory", false);
		}
		m_aWeaponPending.Clear();

		m_Application.FinishRest();
	}
}
