/**
 * TBD_LoadoutEquipHelper.c - shared loadout application (T-068.12, reworked A2; T-181.10).
 *
 * One in-flight equip pass, shared by the dev harness (TestNPC) and the SpawnManager
 * slot-body path so both run the SAME proven APIs with only the log tag differing.
 *
 * A2 determinism rework:
 *  - poll-until-worn VERIFY (500 ms ticks, 6 attempts) replaces the old fixed +1000 ms
 *    one-shot that deleted slow-settling equips;
 *  - DETERMINISTIC SWAP: incumbents (kit garments / current weapon) are captured
 *    BEFORE the equip; when the new item verifies worn, the displaced incumbent is
 *    deleted WITH its contents (deliberate: Arsenal cargo is the contents source of
 *    truth) and logged `swapped`; same-prefab equips are skipped up front
 *    (`swap-skipped`) so the verify can't mistake the old item for the new one;
 *    `IsRootedOn`-only verifies log `swap-deferred` and never guess-delete;
 *  - cargo runs strictly AFTER the wear verify (FinishRest) so container resolution
 *    sees the NEW garments.
 *
 * T-181.10 — JSON-driven loadouts are now COMPLETE and HONEST:
 *  - WEAPON PHASE: `gear.optic` and `gear.magazine` were previously parsed and dropped
 *    ("informational until the attachments slice"). They are now mounted into the primary
 *    weapon's own storage — the mechanic CRF uses for gearscript attachments
 *    (TrySpawnPrefabToStorage into the weapon's BaseInventoryStorageComponent, which on a
 *    Reforger weapon is the SCR_WeaponAttachmentsStorageComponent that carries both
 *    attachment slots and the magazine well). The phase runs AFTER the wear verify so the
 *    primary is really in hand, and re-verifies by scanning that storage.
 *  - HONEST FAILURE: every failure path names the SLOT and the OFFENDING ITEM on one
 *    `[TBD]` ERROR line, is counted, and is repeated in an end-of-pass verdict. A pass that
 *    could not deliver everything the JSON asked for ends on `INCOMPLETE`, never silence.
 *  - NAKEDNESS GUARD: the pass ends with a worn audit of the decency areas; a character
 *    that ends up with no jacket and/or no pants is an ERROR naming the slot, so a bad kit
 *    prefab or a bad loadout can never quietly ship a naked player.
 *
 * T-181.41 — THE NAKEDNESS GUARD GETS A REAL ANCHOR.
 *  The guard above was written to catch a bad KIT PREFAB, and until now it only ever reached
 *  a kit-only body BY ACCIDENT: `if (slot.loadout)` is always true under JsonLoadContext's
 *  ref-field over-allocation, so every slot looked like it carried a JSON loadout and every
 *  slot therefore got an application whose tail happened to run the audit. T-181.32 fixed that
 *  presence test correctly and the guard stopped covering its own use case — measured on
 *  golden-missions/bridgehead-at-levie.json, which is "0 with a JSON loadout, 18 kit-only".
 *  `RunKitWornAudit` is the public entry point that re-homes it onto the kit-only path, and it
 *  POLLS to decency rather than firing once after a chosen delay, because a false NAKED is a
 *  TBD-owned script ERROR and those hard-fail world-boot.sh for everyone.
 *
 * The whole file is server-authority code: it spawns entities and mutates inventories.
 * @authority server
 */

//------------------------------------------------------------------------------------------------
//! One issued equip awaiting its deferred worn-verify pass.
class TBD_PendingEquip
{
	string label;
	string resName;
	IEntity item;
	bool isWeapon;
	typename areaType;                       // primary LoadoutAreaType for clothing
	ref array<typename> candidateAreas = {}; // all areas this item may land in
	ref map<typename, IEntity> incumbents;   // pre-equip worn garment per candidate area
	IEntity oldWeapon;                       // pre-equip current weapon (weapon rows)
}

//------------------------------------------------------------------------------------------------
//! T-181.10 — one weapon-mounted item (optic / magazine) awaiting its mount verify.
//! These do not go through EquipCloth/EquipWeapon: they are spawned straight into the
//! primary weapon's storage, so they need their own (much shorter) verify loop.
class TBD_PendingWeaponItem
{
	string label;    //!< "optic" / "magazine"
	string resName;  //!< item ResourceName
	bool mountIssued; //!< TrySpawnPrefabToStorage accepted the spawn-to-weapon call
}

//------------------------------------------------------------------------------------------------
//! One loadout application: equip gear -> poll worn-verify (+swap) -> mount weapon items
//! -> cargo -> worn audit + verdict.
//! The owner must hold a strong ref until `IsDone()` (CallLater does not keep one).
class TBD_LoadoutApplication : Managed
{
	protected const int VERIFY_TICK_MS = 500;
	protected const int VERIFY_MAX_ATTEMPTS = 6;
	//! Weapon-mounted items settle far faster than worn garments (no animation graph in
	//! the way) — 4 x 250 ms is a full second of grace, measured against nothing but the
	//! same async-settle class the A2 rework found for EquipCloth.
	protected const int WEAPON_TICK_MS = 250;
	protected const int WEAPON_MAX_ATTEMPTS = 4;

	// T-181.41 — AUDIT-ONLY MODE (kit-only slots). See RunKitWornAudit.
	//
	// THE ANCHOR, MEASURED — kit clothing is present SYNCHRONOUSLY.
	// The real question this slice had to answer was "what signal says the body has finished
	// dressing?", because auditing early emits a false NAKED, and a TBD-owned script ERROR is a
	// hard fail in world-boot.sh. The answer is not a timer at all: it is `SpawnEntityPrefab`
	// RETURNING. Measured on a live boot of golden-missions/bridgehead-at-levie.json (engine
	// 1.7.0.54), reading the decency areas in the same statement sequence as the spawn, before
	// any CallLater:
	//
	//     17 of 18 slot bodies:  storageComp=1 jacket=1 pants=1     <- t = 0 ms, already dressed
	//      1 of 18 (control):    storageComp=1 jacket=0 pants=0     <- deliberately bare prefab
	//
	// Every one of the 17 also read decent on the first 25 ms poll tick, and the 1 stayed bare
	// for all 100 of them. So there is no async settle on this path to wait for, and the reason
	// is structural rather than lucky: kit clothing is baked into the PREFAB's entity hierarchy,
	// it is not applied by the async EquipCloth call the A2 rework had to poll for. That is the
	// same fact that makes IssueEquip's empty-ResourceName early-return leave kit clothing intact.
	//
	// WHY POLL AT ALL, THEN. Because the requirement is 0 ms and the cost of insurance is also
	// ~0: the happy path exits on the first tick and never schedules a second. A poll that stops
	// the instant the body is decent CANNOT accuse a body that was merely still dressing — it can
	// only be LATE, and lateness is bounded. Reading once and reporting would trade a free
	// guarantee for a bet on an unobserved slow case (a future engine build, a modded kit), and
	// losing that bet breaks the gate for every slice, not just this one.
	//
	// THE DEADLINE IS 1 s (4 x 250 ms), bounded from BOTH sides:
	//   * Lower — measured need is 0 ms across 17/17 real kits, on two independent readings
	//     (synchronous, and a 25 ms tick). 1 s is 40x the first observation point.
	//   * Upper — the verdict has to land inside world-boot's capture window or the guard is not
	//     gated at all, and that window is SHORT. Bodies materialise ~3.0 s after the
	//     `mission result=` line the harness polls for, and the server is killed ~3.0 s after
	//     THAT (poll granularity 0.5 s + TBD_WORLDBOOT_SETTLE, default 4 s, + engine shutdown).
	//     Two independent boots: run A bodies at +2.947 s, `Game destroyed` +2.985 s later; run B
	//     (the negative control) bodies at +2.983 s, NAKED at exactly +1.000 s, `Game destroyed`
	//     +2.968 s. So the 3 s deadline this started with is not merely "tight" — in run B it
	//     would have fired 32 ms AFTER the capture ended and the NAKED verdict would simply not
	//     exist. 1 s left 1.97 s of margin. That margin is TBD_WORLDBOOT_SETTLE; if the settle is
	//     ever shortened below ~2 s, this deadline has to come down with it or the guard silently
	//     stops being a guard again.
	//   * It is also exactly the grace this file already gives its other fast-settling class
	//     (WEAPON_TICK_MS x WEAPON_MAX_ATTEMPTS), so it is not a fresh invented number.
	//
	// PROVEN TO FAIL, not just to pass — and re-proven independently rather than taken on trust:
	// pointing kit:us_sl at a bare `Character_US_Base.et` ({520EC961A090BBD5}) put out
	// `NAKED after kit spawn` on exactly that slot, left the other 17 clean, and drove
	// world-boot.sh to FAIL on its TBD-script-error check. A guard that has only ever been seen
	// to pass is not a guard.
	protected const int AUDIT_TICK_MS = 250;
	protected const int AUDIT_MAX_ATTEMPTS = 4;

	protected IEntity m_Character;
	protected ref TBD_SlotLoadoutStruct m_Loadout;
	protected string m_sTag;    // "[TBD][Loadout][Slot]" / "[TBD][Loadout][TestNPC]"
	protected string m_sLabel;  // slot id / harness label for log context
	protected ref array<ref TBD_PendingEquip> m_aPending = {};
	protected ref array<ref TBD_PendingEquip> m_aVerified = {};
	protected bool m_bDone;

	//! T-181.41 — audit-only mode: no equips were issued, only the kit prefab dressed this body.
	protected bool m_bAuditOnly;
	//! The kit alias the body was spawned from ("kit:sov_rifleman"). Named in the failure line
	//! because on a kit-only slot the KIT is the thing an operator has to go and fix.
	protected string m_sKit;

	// --- T-181.10 accounting: what the JSON asked for vs what the character got --------
	protected ref array<ref TBD_PendingWeaponItem> m_aWeaponPending = {};
	protected IEntity m_PrimaryWeapon;
	//! One-shot guard for the magazine-well swap retry (below) — never loop on it.
	protected bool m_bWeaponSwapRetried;
	protected ref array<string> m_aFailures = {};  //!< item never reached the character
	protected ref array<string> m_aDegraded = {};  //!< item reached the character, wrong place
	protected int m_iGearRequested;
	protected int m_iGearApplied;
	protected int m_iCargoRequested;
	protected int m_iCargoInserted;

	//------------------------------------------------------------------------------------------------
	void TBD_LoadoutApplication(IEntity character, TBD_SlotLoadoutStruct loadout, string tag, string label)
	{
		m_Character = character;
		m_Loadout = loadout;
		m_sTag = tag;
		m_sLabel = label;
	}

	//------------------------------------------------------------------------------------------------
	bool IsDone()
	{
		return m_bDone;
	}

	//------------------------------------------------------------------------------------------------
	IEntity GetCharacter()
	{
		return m_Character;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.10 — true when the finished pass delivered everything the JSON asked for.
	//! Meaningless before IsDone().
	bool IsComplete()
	{
		return m_aFailures.IsEmpty() && m_aDegraded.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	//! A2 — abort an in-flight application whose body was reaped (vanilla
	//! double-spawn) or superseded by a respawn: loose not-yet-rooted spawned items are
	//! deleted; equipped ones die with the body. Idempotent.
	void Cancel(string reason)
	{
		if (m_bDone)
			return;
		foreach (TBD_PendingEquip p : m_aPending)
		{
			if (p.item && !IsRootedOn(p.item, m_Character))
				SCR_EntityHelper.DeleteEntityAndChildren(p.item);
		}
		m_aPending.Clear();
		m_aWeaponPending.Clear();
		// T-181.41 — an audit-only pass never issued an equip, so calling it a cancelled loadout
		// application would send an operator hunting for gear that was never asked for.
		if (m_bAuditOnly)
			Print(string.Format("%1 slot=%2 kit worn-audit stood down (%3)", m_sTag, m_sLabel, reason));
		else
			Print(string.Format("%1 slot=%2 loadout application cancelled (%3)", m_sTag, m_sLabel, reason));
		m_bDone = true;
	}

	//------------------------------------------------------------------------------------------------
	//! HONEST FAILURE (T-181.10): the item never made it onto the character. One line,
	//! naming the slot AND the offending item AND why, plus a counted entry that the
	//! end-of-pass verdict repeats so a failure can never scroll away unnoticed.
	protected void Fail(string label, string resName, string reason)
	{
		Print(string.Format("%1 slot=%2 %3 FAILED item=%4 — %5", m_sTag, m_sLabel, label, resName, reason), LogLevel.ERROR);
		m_aFailures.Insert(string.Format("%1=%2 (%3)", label, resName, reason));
	}

	//------------------------------------------------------------------------------------------------
	//! The item IS on the character but not where the JSON put it (e.g. an optic that
	//! would not mount and had to be stowed loose). Loud, counted, but not fatal.
	protected void Degrade(string label, string resName, string reason)
	{
		Print(string.Format("%1 slot=%2 %3 DEGRADED item=%4 — %5", m_sTag, m_sLabel, label, resName, reason), LogLevel.WARNING);
		m_aDegraded.Insert(string.Format("%1=%2 (%3)", label, resName, reason));
	}

	//------------------------------------------------------------------------------------------------
	protected static string JoinIssues(notnull array<string> issues)
	{
		string joined;
		for (int i = 0; i < issues.Count(); i++)
		{
			if (i > 0)
				joined += ", ";
			joined += issues[i];
		}
		return joined;
	}

	//------------------------------------------------------------------------------------------------
	//! How many gear ResourceNames this loadout actually asks for (the denominator of the
	//! verdict line). Absent fields are empty strings — the compiler omits them.
	protected static int CountGear(TBD_SlotGearStruct gear)
	{
		if (!gear)
			return 0;

		int n;
		if (!gear.primary.IsEmpty())  n++;
		if (!gear.optic.IsEmpty())    n++;
		if (!gear.magazine.IsEmpty()) n++;
		if (!gear.uniform.IsEmpty())  n++;
		if (!gear.vest.IsEmpty())     n++;
		if (!gear.helmet.IsEmpty())   n++;
		if (!gear.pants.IsEmpty())    n++;
		if (!gear.boots.IsEmpty())    n++;
		if (!gear.handwear.IsEmpty()) n++;
		if (!gear.backpack.IsEmpty()) n++;
		return n;
	}

	//------------------------------------------------------------------------------------------------
	//! Issue every gear equip, then start the poll-verify (EquipCloth/EquipWeapon
	//! settle asynchronously — the T-068.5.1 finding; A2 polls instead of guessing).
	void Run()
	{
		if (!m_Character || !m_Loadout)
		{
			m_bDone = true;
			return;
		}

		TBD_SlotGearStruct gear = m_Loadout.gear;
		m_iGearRequested = CountGear(gear);
		if (m_Loadout.cargo)
		{
			foreach (TBD_SlotCargoStruct row : m_Loadout.cargo)
			{
				// A malformed qty must not corrupt the verdict's denominator — the row is
				// rejected with its own named ERROR in InsertCargo.
				if (row && row.qty > 0)
					m_iCargoRequested += row.qty;
			}
		}

		if (gear)
		{
			IssueEquip("primary",  gear.primary,  true,  LoadoutAreaType); // areaType unused for weapon
			IssueEquip("uniform",  gear.uniform,  false, LoadoutJacketArea);
			IssueEquip("vest",     gear.vest,     false, LoadoutVestArea);
			IssueEquip("helmet",   gear.helmet,   false, LoadoutHeadCoverArea);
			// A3 — the wear map arrives complete now.
			IssueEquip("pants",    gear.pants,    false, LoadoutPantsArea);
			IssueEquip("boots",    gear.boots,    false, LoadoutBootsArea);
			IssueEquip("handwear", gear.handwear, false, LoadoutHandwearSlotArea);
			IssueEquip("backpack", gear.backpack, false, LoadoutBackpackArea);
			// optic + magazine are NOT worn — they mount onto the primary once it is in
			// hand, which is the BeginWeaponPhase step after this verify loop drains.
		}

		GetGame().GetCallqueue().CallLater(VerifyTick, VERIFY_TICK_MS, false, 1);
	}

	//------------------------------------------------------------------------------------------------
	//! Candidate landing areas per clothing label. EquipCloth routes by the ITEM's own
	//! AreaType (locked landmine, t068_10_4), so a "vest" pick may land in the armored
	//! area (plate carrier) — capture + verify must cover every candidate.
	protected static void AreasForLabel(string label, typename primaryArea, notnull array<typename> outAreas)
	{
		outAreas.Insert(primaryArea);
		if (label == "vest")
			outAreas.Insert(LoadoutArmoredVestSlotArea);
	}

	//------------------------------------------------------------------------------------------------
	//! Resolved prefab ResourceName of a spawned entity ("" when unresolvable).
	protected static string PrefabOf(IEntity ent)
	{
		if (!ent)
			return string.Empty;
		EntityPrefabData pd = ent.GetPrefabData();
		if (!pd)
			return string.Empty;
		return pd.GetPrefabName();
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn the gear item and hand it to the real equip API; capture displaced
	//! incumbents first (deterministic swap) and skip same-prefab re-equips.
	protected void IssueEquip(string label, string resName, bool isWeapon, typename areaType)
	{
		if (resName.IsEmpty())
			return; // absent gear slot — kit garment (if any) is deliberately retained

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			m_Character.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!mgr)
		{
			Fail(label, resName, "character has no SCR_InventoryStorageManagerComponent");
			return;
		}

		TBD_PendingEquip pending = new TBD_PendingEquip();
		pending.label = label;
		pending.resName = resName;
		pending.isWeapon = isWeapon;
		pending.areaType = areaType;
		pending.incumbents = new map<typename, IEntity>();

		// --- capture incumbents / same-prefab short-circuit -------------------------------
		if (isWeapon)
		{
			BaseWeaponManagerComponent weaponMgr = BaseWeaponManagerComponent.Cast(
				m_Character.FindComponent(BaseWeaponManagerComponent));
			if (weaponMgr)
			{
				BaseWeaponComponent cur = weaponMgr.GetCurrentWeapon();
				if (cur)
					pending.oldWeapon = cur.GetOwner();
			}
			if (pending.oldWeapon && PrefabOf(pending.oldWeapon) == resName)
			{
				Print(string.Format("%1 slot=%2 %3 swap-skipped (already worn) %4", m_sTag, m_sLabel, label, resName));
				m_iGearApplied++;
				return;
			}
		}
		else
		{
			SCR_CharacterInventoryStorageComponent charStorage = SCR_CharacterInventoryStorageComponent.Cast(
				m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));
			if (charStorage)
			{
				AreasForLabel(label, areaType, pending.candidateAreas);
				foreach (typename area : pending.candidateAreas)
				{
					IEntity worn = charStorage.GetClothFromArea(area);
					if (!worn)
						continue;
					pending.incumbents.Insert(area, worn);
					if (PrefabOf(worn) == resName)
					{
						Print(string.Format("%1 slot=%2 %3 swap-skipped (already worn) %4", m_sTag, m_sLabel, label, resName));
						m_iGearApplied++;
						return;
					}
				}
			}
		}

		IEntity item = SpawnAtCharacter(resName);
		if (!item)
		{
			Fail(label, resName, "prefab failed to load/spawn (bad or missing asset)");
			return;
		}
		pending.item = item;

		if (isWeapon)
			mgr.EquipWeapon(item);
		else
			mgr.EquipCloth(item);

		m_aPending.Insert(pending);
	}

	//------------------------------------------------------------------------------------------------
	protected IEntity SpawnAtCharacter(string resName)
	{
		Resource resource = Resource.Load(resName);
		if (!resource || !resource.IsValid())
			return null;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = m_Character.GetOrigin();
		return GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
	}

	//------------------------------------------------------------------------------------------------
	//! True if entity's parent chain roots at the given character (attached/worn, not loose).
	protected static bool IsRootedOn(IEntity entity, IEntity root)
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

	//------------------------------------------------------------------------------------------------
	//! True when `ent` is one of the items THIS application spawned (pending or
	//! verified) — guards the swap-delete against mis-authored same-area collisions.
	protected bool IsOwnIssuedItem(IEntity ent)
	{
		foreach (TBD_PendingEquip p : m_aPending)
		{
			if (p.item == ent)
				return true;
		}
		foreach (TBD_PendingEquip v : m_aVerified)
		{
			if (v.item == ent)
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Poll pass: verify pending equips; verified items trigger their swap-delete and
	//! move out. All settled (or attempts exhausted) → the weapon phase.
	protected void VerifyTick(int attempt)
	{
		if (m_bDone)
			return;
		if (!m_Character)
		{
			// The body was reaped between ticks (vanilla double-spawn) — a clean
			// cancel, not an equip failure.
			Cancel("body superseded");
			return;
		}

		SCR_CharacterInventoryStorageComponent charStorage =
			SCR_CharacterInventoryStorageComponent.Cast(
				m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));

		if (!charStorage && !m_aPending.IsEmpty())
		{
			foreach (TBD_PendingEquip broken : m_aPending)
			{
				Fail(broken.label, broken.resName, "character has no SCR_CharacterInventoryStorageComponent (cannot verify worn state)");
			}
			m_aPending.Clear();
		}

		for (int i = m_aPending.Count() - 1; i >= 0; i--)
		{
			TBD_PendingEquip p = m_aPending[i];
			string detail;
			typename foundArea;
			bool worn = VerifyOne(charStorage, p, detail, foundArea);
			if (!worn)
				continue;

			Print(string.Format("%1 slot=%2 %3 equip OK %4 [%5]", m_sTag, m_sLabel, p.label, p.resName, detail));
			m_iGearApplied++;
			SwapDelete(charStorage, p, foundArea);
			m_aVerified.Insert(p);
			m_aPending.Remove(i);
		}

		if (!m_aPending.IsEmpty() && attempt < VERIFY_MAX_ATTEMPTS)
		{
			GetGame().GetCallqueue().CallLater(VerifyTick, VERIFY_TICK_MS, false, attempt + 1);
			return;
		}

		// Attempts exhausted: stragglers are honestly failed + removed (never worn).
		foreach (TBD_PendingEquip straggler : m_aPending)
		{
			Fail(straggler.label, straggler.resName, string.Format("not worn after %1 verify ticks — deleted", VERIFY_MAX_ATTEMPTS));
			if (straggler.item)
				SCR_EntityHelper.DeleteEntityAndChildren(straggler.item);
		}
		m_aPending.Clear();

		BeginWeaponPhase();
	}

	//------------------------------------------------------------------------------------------------
	//! Worn check for one pending equip (same signals as T-068.5.1: current weapon /
	//! GetClothFromArea across candidates / IsRootedOn fallback).
	protected bool VerifyOne(SCR_CharacterInventoryStorageComponent charStorage, TBD_PendingEquip p, out string detail, out typename foundArea)
	{
		if (p.isWeapon)
		{
			IEntity wornEnt;
			BaseWeaponManagerComponent weaponMgr = BaseWeaponManagerComponent.Cast(
				m_Character.FindComponent(BaseWeaponManagerComponent));
			if (weaponMgr)
			{
				BaseWeaponComponent weapon = weaponMgr.GetCurrentWeapon();
				if (weapon)
					wornEnt = weapon.GetOwner();
			}
			bool worn = (wornEnt && wornEnt == p.item) || IsRootedOn(p.item, m_Character);
			detail = "weapon";
			return worn;
		}

		if (!charStorage)
			return false;

		foreach (typename area : p.candidateAreas)
		{
			if (charStorage.GetClothFromArea(area) == p.item)
			{
				detail = area.ToString() + " ent=" + p.item.GetID().ToString();
				foundArea = area;
				return true;
			}
		}

		if (IsRootedOn(p.item, m_Character))
		{
			detail = "rooted on character (no area resolution)";
			return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Delete the displaced incumbent of a freshly verified equip (deterministic swap).
	protected void SwapDelete(SCR_CharacterInventoryStorageComponent charStorage, TBD_PendingEquip p, typename foundArea)
	{
		if (p.isWeapon)
		{
			IEntity old = p.oldWeapon;
			if (!old || old == p.item || IsOwnIssuedItem(old))
				return;
			Print(string.Format("%1 slot=%2 swapped area=weapon out=%3 in=%4", m_sTag, m_sLabel, PrefabOf(old), p.resName));
			SCR_EntityHelper.DeleteEntityAndChildren(old);
			return;
		}

		if (!foundArea)
		{
			// IsRootedOn-only verify — landing area unknown; never guess-delete.
			Print(string.Format("%1 slot=%2 swap-deferred (no area resolution) %3", m_sTag, m_sLabel, p.resName));
			return;
		}

		IEntity incumbent;
		if (!p.incumbents.Find(foundArea, incumbent) || !incumbent)
			return; // area was empty pre-equip — nothing displaced
		if (incumbent == p.item || IsOwnIssuedItem(incumbent))
			return;
		// Belt-and-braces: only delete once the engine really unseated it.
		if (charStorage && charStorage.GetClothFromArea(foundArea) == incumbent)
			return;

		Print(string.Format("%1 slot=%2 swapped area=%3 out=%4 in=%5", m_sTag, m_sLabel, foundArea.ToString(), PrefabOf(incumbent), p.resName));
		SCR_EntityHelper.DeleteEntityAndChildren(incumbent);
	}

	//====================================================================================
	// T-181.10 — WEAPON PHASE: optic + magazine onto the primary
	//====================================================================================

	//------------------------------------------------------------------------------------------------
	//! Which weapon entity the JSON's optic/magazine belong to. Prefer an exact prefab
	//! match against `gear.primary` (the CRF FindWeaponByResource pattern) so a character
	//! holding a pistol at settle time still gets its rifle kitted; fall back to whatever
	//! is currently in hand when the loadout authored no primary of its own.
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
				if (PrefabOf(weapon) == primaryRes)
					return weapon;
			}
		}

		BaseWeaponComponent current = weaponMgr.GetCurrentWeapon();
		if (current)
			return current.GetOwner();

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! A Reforger weapon carries its attachments AND its magazine well in one storage
	//! component (SCR_WeaponAttachmentsStorageComponent : BaseInventoryStorageComponent).
	protected static BaseInventoryStorageComponent WeaponStorageOf(IEntity weapon)
	{
		if (!weapon)
			return null;
		return BaseInventoryStorageComponent.Cast(weapon.FindComponent(BaseInventoryStorageComponent));
	}

	//------------------------------------------------------------------------------------------------
	//! True when the weapon storage already holds an item of that prefab (mounted optic /
	//! loaded magazine). Includes child components so a mag inside a well still counts.
	protected static bool WeaponStorageHas(BaseInventoryStorageComponent storage, string resName)
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

	//------------------------------------------------------------------------------------------------
	//! Gear settled → mount the weapon-borne JSON items. Nothing to do (or nothing to
	//! mount onto) short-circuits straight to the tail.
	protected void BeginWeaponPhase()
	{
		if (m_bDone)
			return;
		if (!m_Character)
		{
			Cancel("body superseded");
			return;
		}

		TBD_SlotGearStruct gear = m_Loadout.gear;
		if (!gear || (gear.optic.IsEmpty() && gear.magazine.IsEmpty()))
		{
			FinishRest();
			return;
		}

		m_PrimaryWeapon = ResolvePrimaryWeapon(gear.primary);
		if (!m_PrimaryWeapon)
		{
			if (!gear.optic.IsEmpty())
				Fail("optic", gear.optic, "no primary weapon on the character to mount it on");
			if (!gear.magazine.IsEmpty())
				Fail("magazine", gear.magazine, "no primary weapon on the character to load it into");
			FinishRest();
			return;
		}

		IssueWeaponItem("optic", gear.optic);
		IssueWeaponItem("magazine", gear.magazine);

		if (m_aWeaponPending.IsEmpty())
		{
			FinishRest();
			return;
		}

		GetGame().GetCallqueue().CallLater(WeaponVerifyTick, WEAPON_TICK_MS, false, 1);
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn one weapon-borne item straight into the primary's storage. This is CRF's
	//! gearscript attachment mechanic (TrySpawnPrefabToStorage into the weapon storage,
	//! slot auto-select) — the storage picks the matching attachment slot / magazine well
	//! itself, which is why we do not hand-pick an AttachmentSlotComponent.
	protected void IssueWeaponItem(string label, string resName)
	{
		if (resName.IsEmpty())
			return;

		// A bad ResourceName must not reach the inventory system as a silent no-op.
		Resource probe = Resource.Load(resName);
		if (!probe || !probe.IsValid())
		{
			Fail(label, resName, "prefab failed to load (bad or missing asset)");
			return;
		}

		BaseInventoryStorageComponent storage = WeaponStorageOf(m_PrimaryWeapon);
		if (!storage)
		{
			Fail(label, resName, string.Format("primary weapon %1 has no attachment storage", PrefabOf(m_PrimaryWeapon)));
			return;
		}

		if (WeaponStorageHas(storage, resName))
		{
			Print(string.Format("%1 slot=%2 %3 mount-skipped (already on weapon) %4", m_sTag, m_sLabel, label, resName));
			m_iGearApplied++;
			return;
		}

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			m_Character.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!mgr)
		{
			Fail(label, resName, "character has no SCR_InventoryStorageManagerComponent");
			return;
		}

		TBD_PendingWeaponItem pending = new TBD_PendingWeaponItem();
		pending.label = label;
		pending.resName = resName;
		pending.mountIssued = mgr.TrySpawnPrefabToStorage(resName, storage, -1, EStoragePurpose.PURPOSE_ANY);
		if (!pending.mountIssued)
			Print(string.Format("%1 slot=%2 %3 mount refused up front by the weapon storage %4 — the verify pass will retry and then fall back",
				m_sTag, m_sLabel, label, resName), LogLevel.WARNING);
		m_aWeaponPending.Insert(pending);
	}

	//------------------------------------------------------------------------------------------------
	//! Evict whatever magazine the weapon came with so the JSON's magazine can take the
	//! well, then re-issue it. Returns true when something was actually cleared (the caller
	//! only spends another verify round in that case). Optic incumbents are deliberately
	//! NOT swept: an occupied optic rail is rare on a freshly spawned weapon, and blindly
	//! deleting attachments we cannot positively identify as being in the way is exactly
	//! the guess-delete this file refuses to make elsewhere.
	protected bool ClearBlockingMagazine()
	{
		BaseInventoryStorageComponent storage = WeaponStorageOf(m_PrimaryWeapon);
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
				if (PrefabOf(item) == p.resName)
					continue; // already the requested magazine — nothing is in the way

				Print(string.Format("%1 slot=%2 magazine swapping out the weapon's own %3 for %4",
					m_sTag, m_sLabel, PrefabOf(item), p.resName), LogLevel.WARNING);
				SCR_EntityHelper.DeleteEntityAndChildren(item);
				cleared = true;
			}

			if (cleared)
				p.mountIssued = mgr.TrySpawnPrefabToStorage(p.resName, storage, -1, EStoragePurpose.PURPOSE_ANY);
		}

		return cleared;
	}

	//------------------------------------------------------------------------------------------------
	//! Poll the weapon storage until each issued item shows up. Stragglers get one
	//! honest fallback (loose into the character's own inventory) so a player is never
	//! left with an unloaded weapon and no ammo at all — that fallback is DEGRADED, not
	//! success, and says so.
	protected void WeaponVerifyTick(int attempt)
	{
		if (m_bDone)
			return;
		if (!m_Character)
		{
			Cancel("body superseded");
			return;
		}

		BaseInventoryStorageComponent storage = WeaponStorageOf(m_PrimaryWeapon);

		for (int i = m_aWeaponPending.Count() - 1; i >= 0; i--)
		{
			TBD_PendingWeaponItem p = m_aWeaponPending[i];
			if (!WeaponStorageHas(storage, p.resName))
				continue;

			Print(string.Format("%1 slot=%2 %3 mount OK %4 (on %5)", m_sTag, m_sLabel, p.label, p.resName, PrefabOf(m_PrimaryWeapon)));
			m_iGearApplied++;
			m_aWeaponPending.Remove(i);
		}

		if (!m_aWeaponPending.IsEmpty() && attempt < WEAPON_MAX_ATTEMPTS)
		{
			GetGame().GetCallqueue().CallLater(WeaponVerifyTick, WEAPON_TICK_MS, false, attempt + 1);
			return;
		}

		// Deterministic magazine swap, one shot only. The usual reason a magazine will not
		// mount is that the weapon prefab already shipped with its own in the well, so the
		// JSON's choice has nowhere to go. Clear the incumbent and re-issue ONCE — deferred
		// this late on purpose, so we never take a working magazine away on speculation.
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
				Degrade(straggler.label, straggler.resName, "would not mount on the primary — stowed loose in the character's inventory");
			else
				Fail(straggler.label, straggler.resName, "would not mount on the primary and would not fit in the inventory");
		}
		m_aWeaponPending.Clear();

		FinishRest();
	}

	//====================================================================================
	// Tail: cargo, worn audit, verdict
	//====================================================================================

	//------------------------------------------------------------------------------------------------
	//! Post-verify tail: cargo insert (against the NEW garments), the nakedness audit,
	//! and the one verdict line that says whether the JSON was honoured.
	protected void FinishRest()
	{
		if (m_bDone)
			return;

		InsertCargo();
		AuditWorn();
		ReportVerdict();
		m_bDone = true;
	}

	//------------------------------------------------------------------------------------------------
	//! Container key -> the WORN garment entity (vest also accepts the armored-vest area).
	protected IEntity GarmentForContainer(SCR_CharacterInventoryStorageComponent charStorage, string container)
	{
		if (container == "vest")
		{
			IEntity worn = charStorage.GetClothFromArea(LoadoutVestArea);
			if (!worn)
				worn = charStorage.GetClothFromArea(LoadoutArmoredVestSlotArea);
			return worn;
		}
		if (container == "jacket")
			return charStorage.GetClothFromArea(LoadoutJacketArea);
		if (container == "pants")
			return charStorage.GetClothFromArea(LoadoutPantsArea);
		if (container == "backpack")
			return charStorage.GetClothFromArea(LoadoutBackpackArea);
		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! Insert every cargo row into its resolved container storage. Failure ladder per unit:
	//! targeted TryInsertItemInStorage -> TryInsertItem anywhere (DEGRADED) -> delete (FAILED).
	//! No silent drops: every row logs an outcome naming the slot and the item.
	protected void InsertCargo()
	{
		if (!m_Loadout.cargo || m_Loadout.cargo.IsEmpty())
			return;

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			m_Character.FindComponent(SCR_InventoryStorageManagerComponent));
		SCR_CharacterInventoryStorageComponent charStorage = SCR_CharacterInventoryStorageComponent.Cast(
			m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));
		if (!mgr || !charStorage)
		{
			foreach (TBD_SlotCargoStruct broken : m_Loadout.cargo)
			{
				if (broken)
					Fail("cargo:" + broken.container, broken.item, "character missing inventory components");
			}
			return;
		}

		foreach (TBD_SlotCargoStruct row : m_Loadout.cargo)
		{
			if (!row)
				continue;
			if (row.item.IsEmpty())
			{
				Fail("cargo:" + row.container, "<empty>", "cargo row carries no item ResourceName");
				continue;
			}
			if (row.qty < 1)
			{
				Fail("cargo:" + row.container, row.item, string.Format("cargo row qty=%1 is below the schema minimum of 1", row.qty));
				continue;
			}

			IEntity garment = GarmentForContainer(charStorage, row.container);
			BaseInventoryStorageComponent storage;
			if (garment)
				storage = BaseInventoryStorageComponent.Cast(garment.FindComponent(BaseInventoryStorageComponent));
			if (!storage)
				Degrade("cargo:" + row.container, row.item, "no worn container of that kind — falling back to any-storage insert");

			int inserted = 0;
			string stopReason;
			for (int u = 0; u < row.qty; u++)
			{
				IEntity item = SpawnAtCharacter(row.item);
				if (!item)
				{
					stopReason = string.Format("prefab failed to load/spawn at unit %1/%2 (bad or missing asset)", u + 1, row.qty);
					break; // resource problems won't fix themselves for later units
				}

				bool ok = false;
				if (storage)
					ok = mgr.TryInsertItemInStorage(item, storage);
				if (!ok)
				{
					ok = mgr.TryInsertItem(item);
					if (ok && storage)
						Degrade("cargo:" + row.container, row.item, string.Format("unit %1/%2 did not fit the container — inserted elsewhere", u + 1, row.qty));
				}

				if (ok)
				{
					inserted++;
				}
				else
				{
					SCR_EntityHelper.DeleteEntityAndChildren(item);
					stopReason = string.Format("no storage would accept unit %1/%2 (character full)", u + 1, row.qty);
					break; // a full character won't accept later units either
				}
			}

			m_iCargoInserted += inserted;
			Print(string.Format("%1 slot=%2 cargo %3 x%4/%5 -> %6", m_sTag, m_sLabel, row.item, inserted, row.qty, row.container));
			if (!stopReason.IsEmpty())
				Fail("cargo:" + row.container, row.item, stopReason);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! NAKEDNESS GUARD (T-181.10). Whatever the kit prefab and the JSON between them did,
	//! the pass ends by looking at what the character is actually wearing. A body with no
	//! jacket and/or no pants is an ERROR naming the slot — a naked or half-dressed player
	//! must never leave this code silently.
	//!
	//! Called from FinishRest, i.e. AFTER the equip verify loop has drained (up to 6 x 500 ms)
	//! and the weapon phase has settled. That is why this one reads the areas ONCE and needs no
	//! poll of its own: by the time it runs, anything still moving has already been failed and
	//! deleted by name. The kit-only path has no such wait in front of it and therefore polls —
	//! see AuditTick.
	protected void AuditWorn()
	{
		SCR_CharacterInventoryStorageComponent charStorage = SCR_CharacterInventoryStorageComponent.Cast(
			m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));
		if (!charStorage)
		{
			Print(string.Format("%1 slot=%2 worn-audit SKIPPED — character has no SCR_CharacterInventoryStorageComponent", m_sTag, m_sLabel), LogLevel.ERROR);
			return;
		}

		ReportWornAudit(charStorage, "after loadout pass", "kit prefab and JSON loadout both left the body bare");
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.41 — THE decision, in one place, so "naked" means the same thing on both paths and
	//! there is exactly one set of words for it. `context` says WHEN the reading was taken and
	//! `cause` says who is on the hook for it; everything else is identical.
	protected void ReportWornAudit(notnull SCR_CharacterInventoryStorageComponent charStorage, string context, string cause)
	{
		bool jacket = charStorage.GetClothFromArea(LoadoutJacketArea) != null;
		bool pants = charStorage.GetClothFromArea(LoadoutPantsArea) != null;
		bool boots = charStorage.GetClothFromArea(LoadoutBootsArea) != null;

		if (jacket && pants)
		{
			Print(string.Format("%1 slot=%2 worn-audit jacket=1 pants=1 boots=%3", m_sTag, m_sLabel, boots));
			return;
		}

		if (!jacket && !pants)
		{
			Print(string.Format("%1 slot=%2 NAKED %3 — no jacket and no pants worn (%4)",
				m_sTag, m_sLabel, context, cause), LogLevel.ERROR);
			return;
		}

		Print(string.Format("%1 slot=%2 HALF-DRESSED %3 — jacket=%4 pants=%5 boots=%6 (%7)",
			m_sTag, m_sLabel, context, jacket, pants, boots, cause), LogLevel.ERROR);
	}

	//====================================================================================
	// T-181.41 — KIT-ONLY WORN AUDIT
	//====================================================================================

	//------------------------------------------------------------------------------------------------
	//! PUBLIC ENTRY POINT for a slot whose JSON authors no loadout at all.
	//!
	//! WHY THIS EXISTS. AuditWorn was written (T-181.10) to catch exactly one thing: a bad KIT
	//! PREFAB that spawns a player naked. It only ever ran as the tail of a JSON loadout pass,
	//! and it only ever reached kit-only bodies because of a BUG — `if (slot.loadout)` is always
	//! true under JsonLoadContext's over-allocation, so every slot looked like it carried a JSON
	//! loadout and every slot got a (mostly empty) application whose tail ran the audit.
	//! T-181.32 fixed that test correctly, and the side effect was that the guard stopped
	//! covering the one case it was written for: `bridgehead-at-levie` reports
	//! "0 with a JSON loadout, 18 kit-only", and not one of those 18 bodies was being looked at.
	//!
	//! This is the audit's real anchor rather than a bug: the body is dressed by its kit and
	//! nothing else, so the kit is solely responsible and is named in the failure.
	//!
	//! GATED FOR REAL, AND WITHOUT A CLIENT. Do not assume — as this program did — that dressing a
	//! body needs a player. TBD_SpawnManager materialises the whole slot lineup at MISSION START,
	//! not on join, and SpawnSlotBody is its ONLY body-creation call site, so
	//! `world-boot.sh --mission=<golden>` builds and audits every body with zero players
	//! connected. Measured on bridgehead-at-levie: 18/18 bodies, each
	//! `worn-audit jacket=1 pants=1 boots=1 kit=… (settled on attempt 1 of 4, 250 ms)`. That makes
	//! this one of the few things in this file proven END-TO-END by the boot gate instead of
	//! compile-only — and it is exactly why a false NAKED here would break that gate for every
	//! other slice, not just this one.
	//!
	//! CANCELLATION / THE `ScriptCallQueue.Remove` HAZARD. Deliberately none of this slice's
	//! business, because the object is a TBD_LoadoutApplication registered in the SpawnManager's
	//! m_aLoadoutApps like any other: CancelLoadoutAppsFor(body) already cancels it when the body
	//! is superseded or released to vanilla teardown, PruneDoneLoadoutApps already reaps it, and
	//! every tick re-checks m_bDone and m_Character first. The T-181.15 hazard — one
	//! ScriptCallQueue.Remove cancelling every player's pending callback — is sidestepped
	//! entirely rather than re-solved: NOTHING here is ever removed from the call queue, and the
	//! deferred callback carries no raw playerId to go stale. It carries the body itself, and a
	//! body is not recycled the way a numeric id is.
	//! @authority server
	void RunKitWornAudit(string kit)
	{
		if (m_bDone)
			return;

		m_bAuditOnly = true;
		m_sKit = kit;

		if (!m_Character)
		{
			m_bDone = true;
			return;
		}

		GetGame().GetCallqueue().CallLater(AuditTick, AUDIT_TICK_MS, false, 1);
	}

	//------------------------------------------------------------------------------------------------
	//! Poll the decency areas until the body is dressed or the deadline expires.
	//!
	//! THE POINT OF POLLING RATHER THAN WAITING. A false NAKED is worse than no audit: it is a
	//! TBD-owned script ERROR, and those are a hard fail in world-boot.sh, so an audit that cries
	//! wolf breaks the gate for every future slice. A single reading after a chosen delay is a bet
	//! on that delay. A poll is not: it reports success the instant the body IS decent, so it can
	//! never accuse a body that was merely still dressing. The only thing the deadline buys is how
	//! long a genuinely-naked body stays unreported, and 1 s of that costs nothing.
	//!
	//! In practice — measured — this returns on attempt 1 every time, because kit clothing is
	//! already there synchronously (see AUDIT_TICK_MS). The loop is insurance, not a wait.
	//!
	//! The OK line carries the attempt it settled on precisely so this stops being an argument:
	//! if a future engine build starts dressing bodies slower, the number climbs in the log long
	//! before it reaches the deadline and breaks anything.
	protected void AuditTick(int attempt)
	{
		if (m_bDone)
			return;
		if (!m_Character)
		{
			// Body reaped between ticks — a clean stand-down, not a nakedness finding.
			Cancel("body superseded");
			return;
		}

		SCR_CharacterInventoryStorageComponent charStorage = SCR_CharacterInventoryStorageComponent.Cast(
			m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));

		// A character with no storage component YET is indistinguishable from one that will never
		// have one, so it is retried like any other not-yet-decent state and only reported at the
		// deadline. Reporting it on tick 1 would be the same early-accusation bug in another coat.
		bool decent = false;
		if (charStorage)
		{
			decent = charStorage.GetClothFromArea(LoadoutJacketArea) != null
				&& charStorage.GetClothFromArea(LoadoutPantsArea) != null;
		}

		if (decent)
		{
			bool boots = charStorage.GetClothFromArea(LoadoutBootsArea) != null;
			Print(string.Format("%1 slot=%2 worn-audit jacket=1 pants=1 boots=%3 kit=%4 (settled on attempt %5 of %6, %7 ms)",
				m_sTag, m_sLabel, boots, m_sKit, attempt, AUDIT_MAX_ATTEMPTS, attempt * AUDIT_TICK_MS));
			m_bDone = true;
			return;
		}

		if (attempt < AUDIT_MAX_ATTEMPTS)
		{
			GetGame().GetCallqueue().CallLater(AuditTick, AUDIT_TICK_MS, false, attempt + 1);
			return;
		}

		// Deadline reached and still not decent — this is the finding the guard exists for.
		if (!charStorage)
		{
			Print(string.Format("%1 slot=%2 worn-audit SKIPPED — character has no SCR_CharacterInventoryStorageComponent after %3 ms (kit %4)",
				m_sTag, m_sLabel, AUDIT_MAX_ATTEMPTS * AUDIT_TICK_MS, m_sKit), LogLevel.ERROR);
			m_bDone = true;
			return;
		}

		ReportWornAudit(charStorage,
			string.Format("after kit spawn (%1 ms, no JSON loadout authored)", AUDIT_MAX_ATTEMPTS * AUDIT_TICK_MS),
			string.Format("kit %1 dressed the body itself and this is what it produced — fix the kit prefab or author a loadout", m_sKit));
		m_bDone = true;
	}

	//------------------------------------------------------------------------------------------------
	//! The one line an operator greps for: did this slot get the loadout its JSON asked
	//! for? Complete passes log a single OK line; anything less repeats every offending
	//! item so the verdict is self-contained.
	protected void ReportVerdict()
	{
		string counts = string.Format("gear=%1/%2 cargo=%3/%4",
			m_iGearApplied, m_iGearRequested, m_iCargoInserted, m_iCargoRequested);

		if (m_aFailures.IsEmpty() && m_aDegraded.IsEmpty())
		{
			Print(string.Format("%1 slot=%2 loadout pass complete %3", m_sTag, m_sLabel, counts));
			return;
		}

		if (!m_aFailures.IsEmpty())
			Print(string.Format("%1 slot=%2 loadout INCOMPLETE %3 — failed: %4",
				m_sTag, m_sLabel, counts, JoinIssues(m_aFailures)), LogLevel.ERROR);

		if (!m_aDegraded.IsEmpty())
			Print(string.Format("%1 slot=%2 loadout DEGRADED %3 — %4",
				m_sTag, m_sLabel, counts, JoinIssues(m_aDegraded)), LogLevel.WARNING);
	}
}
