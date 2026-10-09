/**
 * @file TBD_LoadoutApplication.c
 * @brief One loadout application: dress a body with a slot loadout and report the delivery.
 *
 * Role: runs the gear, weapon and cargo phases over one body in order, keeps the delivery ledger
 * (failed, blocking, degraded, shortfall) and writes the end-of-pass verdict; also runs the
 * kit-only worn audit for a slot that authors no loadout.  Position: created by
 * `TBD_SlotBodyMaterializer` (tag `[TBD][Loadout][Slot]`) and `TBD_LoadoutEquipComponent` (tag
 * `[TBD][Loadout][TestNPC]`); tracked and read by `TBD_SlotLoadoutSettle` and
 * `TBD_DeployExecutor`; owns `TBD_LoadoutGearPhase`, `TBD_LoadoutWeaponPhase`,
 * `TBD_LoadoutCargoPhase` and `TBD_LoadoutWornAudit`.
 * State: the ledger and the done flag of one pass, on the server; the phases hold the in-flight
 * items.  Invariants: every item that does not arrive is named on one log line and counted in
 * the verdict; a blocking failure logs at ERROR and nothing else does; the verdict is written
 * exactly once, after cargo and the worn audit; `CallLater` keeps no reference, so the owner
 * holds a strong one until `IsDone()`.
 */

//! One loadout application: gear, then weapon-borne items, then cargo, then the worn audit and
//! the verdict. The owner holds a strong reference until `IsDone()`.
class TBD_LoadoutApplication : Managed
{
	protected IEntity m_Character; //!< the body being dressed; null once the engine deletes it
	protected ref TBD_SlotLoadoutStruct m_Loadout; //!< the authored loadout; null runs nothing
	protected string m_sTag; //!< log tag: `[TBD][Loadout][Slot]` or `[TBD][Loadout][TestNPC]`
	protected string m_sLabel; //!< slot id or harness label named on every log line
	protected bool m_bDone; //!< true once the pass has finished, been cancelled or had nothing to do
	protected bool m_bAuditOnly; //!< true for a kit-only worn audit: no equip was issued

	protected ref TBD_LoadoutGearPhase m_GearPhase; //!< worn gear and weapon slots
	protected ref TBD_LoadoutWeaponPhase m_WeaponPhase; //!< optic, magazine and attachments on the primary
	protected ref TBD_LoadoutCargoPhase m_CargoPhase; //!< cargo rows into worn containers
	protected ref TBD_LoadoutWornAudit m_WornAudit; //!< the nakedness guard

	protected ref array<string> m_aFailures = {}; //!< `label=item (reason)` of every item that never reached the body
	protected ref array<string> m_aDegraded = {}; //!< `label=item (reason)` of every item on the body in the wrong place
	protected ref array<string> m_aBlocking = {}; //!< the subset of `m_aFailures` that leaves the slot unplayable
	protected ref array<string> m_aShortfallLabels = {}; //!< label of every non-blocking shortfall, one per occurrence
	protected int m_iGearRequested; //!< gear ResourceNames the loadout authors
	protected int m_iGearApplied; //!< gear items verified on the body or already there
	protected int m_iCargoRequested; //!< cargo units the loadout authors (rows with qty above 0)
	protected int m_iCargoInserted; //!< cargo units inserted somewhere on the body

	//! Prepare a pass over `character`; nothing runs until `Run` or `RunKitWornAudit`.
	//! @param character the body to dress
	//! @param loadout the authored slot loadout; null makes `Run` finish immediately
	//! @param tag the log tag
	//! @param label the slot id or harness label
	void TBD_LoadoutApplication(IEntity character, TBD_SlotLoadoutStruct loadout, string tag, string label)
	{
		m_Character = character;
		m_Loadout = loadout;
		m_sTag = tag;
		m_sLabel = label;
		m_GearPhase = new TBD_LoadoutGearPhase(this);
		m_WeaponPhase = new TBD_LoadoutWeaponPhase(this);
		m_CargoPhase = new TBD_LoadoutCargoPhase(this);
		m_WornAudit = new TBD_LoadoutWornAudit(this);
	}

	//! @return true once the pass has finished, been cancelled or had nothing to do
	bool IsDone()
	{
		return m_bDone;
	}

	//! @return the body this pass dresses; null once the engine has deleted it
	IEntity GetCharacter()
	{
		return m_Character;
	}

	//! @return the authored loadout the phases read
	TBD_SlotLoadoutStruct GetLoadout()
	{
		return m_Loadout;
	}

	//! @return the log tag every phase prefixes its lines with
	string GetTag()
	{
		return m_sTag;
	}

	//! Whether the finished pass delivered everything the loadout authors, in the place it
	//! authors it. A delivery verdict, not the spawn gate: `HasBlockingFailure` is the gate.
	//! @return true when nothing failed and nothing was degraded; meaningless before `IsDone()`
	bool IsComplete()
	{
		return m_aFailures.IsEmpty() && m_aDegraded.IsEmpty();
	}

	//! The spawn-boundary predicate: whether this body is unplayable. True only for a blocking
	//! failure (an authored asset that does not exist, a storage or weapon slot the body lacks, a
	//! garment that would not go on). A full vest or an optic that would not seat is a shortfall.
	//! @return true when any blocking failure was recorded; meaningless before `IsDone()`
	bool HasBlockingFailure()
	{
		return !m_aBlocking.IsEmpty();
	}

	//! Whether the pass delivered a playable body that differs from what was authored.
	//! @return true when the pass is not complete and nothing blocking failed
	bool HasShortfall()
	{
		return !IsComplete() && !HasBlockingFailure();
	}

	//! @return every blocking failure, comma-joined, for the spawn-boundary refusal banner
	string BlockingSummary()
	{
		return JoinIssues(m_aBlocking);
	}

	//! The shortfall as a one-glance brief, for example `cargo:vest x6, optic x1`: one entry per
	//! label with its count. The per-item detail stays on the console lines; the brief feeds a
	//! session summary and a short chat reply. Built from the recorded labels, not by parsing the
	//! issue strings.
	//! @return the brief; empty when there is no shortfall
	string ShortfallBrief()
	{
		array<string> labels = {};
		array<int> counts = {};
		foreach (string label : m_aShortfallLabels)
		{
			int at = labels.Find(label);
			if (at >= 0)
			{
				counts[at] = counts[at] + 1;
				continue;
			}
			labels.Insert(label);
			counts.Insert(1);
		}

		string brief;
		for (int i = 0; i < labels.Count(); i++)
		{
			if (i > 0)
				brief += ", ";
			brief += string.Format("%1 x%2", labels[i], counts[i]);
		}
		return brief;
	}

	//! @return the slot id or harness label this pass runs for
	string GetLabel()
	{
		return m_sLabel;
	}

	//! Abort an in-flight pass whose body was deleted (engine double-spawn) or superseded by a
	//! respawn. Loose spawned items not yet on the body are deleted; equipped ones go with the body.
	//! Idempotent: a finished pass ignores the call.
	//! @param reason written on the cancellation line
	//! @authority server
	void Cancel(string reason)
	{
		if (m_bDone)
			return;
		m_GearPhase.Cancel();
		m_WeaponPhase.Cancel();
		// A kit-only audit issued no equip, so it stands down rather than reporting a cancelled loadout.
		if (m_bAuditOnly)
			Print(string.Format("%1 slot=%2 kit worn-audit stood down (%3)", m_sTag, m_sLabel, reason));
		else
			Print(string.Format("%1 slot=%2 loadout application cancelled (%3)", m_sTag, m_sLabel, reason));
		m_bDone = true;
	}

	//! Record an item that never reached the body: one log line naming the slot, the item and the
	//! reason, and an entry the verdict repeats.
	//!
	//! Blocking means the slot is unplayable until an operator changes something: the mission names
	//! a prefab that does not load, the body lacks the storage or weapon slot the row needs, or a
	//! garment would not go on. Non-blocking means the item exists but the body had no room or no
	//! rail for it; the body is dressed and armed and carries less than authored. The default is
	//! blocking, so an unconsidered call site fails closed.
	//!
	//! The log level follows `blocking`: a blocking failure is an ERROR (the session will not open,
	//! and `world-boot.sh` fails on any TBD-owned `SCRIPT (E)`), a non-blocking one a WARNING.
	//! @param label the loadout row label (`vest`, `optic`, `cargo:backpack`)
	//! @param resName the item ResourceName
	//! @param reason why it did not arrive
	//! @param blocking whether this makes the slot unplayable
	void Fail(string label, string resName, string reason, bool blocking = true)
	{
		if (label == "attach")
			TBD_LoadoutWeaponPhase.LogAttachResult(resName, false);
		string entry = string.Format("%1=%2 (%3)", label, resName, reason);
		m_aFailures.Insert(entry);

		if (blocking)
		{
			m_aBlocking.Insert(entry);
			Print(string.Format("%1 slot=%2 %3 FAILED item=%4 -- %5", m_sTag, m_sLabel, label, resName, reason), LogLevel.ERROR);
			return;
		}

		m_aShortfallLabels.Insert(label);
		Print(string.Format("%1 slot=%2 %3 NOT DELIVERED item=%4 -- %5 -- the slot is still playable, so this does NOT refuse the session",
			m_sTag, m_sLabel, label, resName, reason), LogLevel.WARNING);
	}

	//! Record an item that is on the body but not where the loadout puts it (an optic stowed loose,
	//! cargo in another container). A WARNING and a shortfall entry; never blocking, because every
	//! degraded item is on the body.
	//! @param label the loadout row label
	//! @param resName the item ResourceName
	//! @param reason where it ended up and why
	void Degrade(string label, string resName, string reason)
	{
		if (label == "attach")
			TBD_LoadoutWeaponPhase.LogAttachResult(resName, false);
		Print(string.Format("%1 slot=%2 %3 DEGRADED item=%4 -- %5", m_sTag, m_sLabel, label, resName, reason), LogLevel.WARNING);
		m_aDegraded.Insert(string.Format("%1=%2 (%3)", label, resName, reason));
		m_aShortfallLabels.Insert(label);
	}

	//! @return `issues` joined with `, `
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

	//! Start the pass: count what the loadout authors, then issue the gear phase. The equip calls
	//! settle asynchronously, so the gear phase polls before the later phases run.
	//! @authority server
	void Run()
	{
		if (!m_Character || !m_Loadout)
		{
			m_bDone = true;
			return;
		}

		TBD_SlotGearStruct gear = m_Loadout.gear;
		m_iGearRequested = TBD_LoadoutInventoryUtil.CountGear(gear);
		if (m_Loadout.cargo)
		{
			foreach (TBD_SlotCargoStruct row : m_Loadout.cargo)
			{
				// A malformed qty must not corrupt the verdict's denominator -- the row is
				// rejected with its own named ERROR by the cargo phase.
				if (row && row.qty > 0)
					m_iCargoRequested += row.qty;
			}
		}

		m_GearPhase.Begin(gear);
	}

	//! Start the kit-only worn audit for a slot whose mission authors no loadout: the kit prefab
	//! alone dressed the body, so the audit names the kit on failure. The owner tracks and cancels
	//! this pass like any other application.
	//! @param kit the kit alias the body was spawned from
	//! @authority server
	void RunKitWornAudit(string kit)
	{
		if (m_bDone)
			return;

		m_bAuditOnly = true;
		if (!m_Character)
		{
			m_bDone = true;
			return;
		}

		m_WornAudit.Begin(kit);
	}

	//! Called by the gear phase once every equip is verified or failed: start the weapon phase.
	//! @authority server
	void OnGearSettled()
	{
		m_WeaponPhase.Begin();
	}

	//! Count one gear item as delivered.
	void CountGearApplied()
	{
		m_iGearApplied++;
	}

	//! Count `units` cargo units as inserted.
	void CountCargoInserted(int units)
	{
		m_iCargoInserted += units;
	}

	//! End the pass without a verdict; the kit-only audit calls this when it has reported.
	void MarkDone()
	{
		m_bDone = true;
	}

	//! Spawn `resName` at the body's origin in the game world.
	//! @return the entity, or null when the prefab does not load
	//! @authority server
	IEntity SpawnAtCharacter(string resName)
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

	//! The tail after the weapon phase: cargo into the garments now worn, the worn audit, then the
	//! verdict. Runs once.
	//! @authority server
	void FinishRest()
	{
		if (m_bDone)
			return;

		m_CargoPhase.InsertCargo();
		m_WornAudit.AuditWorn();
		ReportVerdict();
		m_bDone = true;
	}

	//! The verdict line: `loadout pass complete` when everything arrived; otherwise `REFUSED` at
	//! ERROR for a blocking failure (the spawn boundary keeps the session in LOADING) or `SHORTFALL`
	//! at WARNING for a playable body carrying less than authored, followed by the itemised
	//! `loadout INCOMPLETE` and `loadout DEGRADED` lines.
	protected void ReportVerdict()
	{
		string counts = string.Format("gear=%1/%2 cargo=%3/%4",
			m_iGearApplied, m_iGearRequested, m_iCargoInserted, m_iCargoRequested);

		if (IsComplete())
		{
			Print(string.Format("%1 slot=%2 loadout pass complete %3", m_sTag, m_sLabel, counts));
			return;
		}

		if (HasBlockingFailure())
		{
			Print(string.Format("%1 slot=%2 loadout delivery REFUSED %3 -- this slot is UNPLAYABLE and the session will stay in LOADING: %4",
				m_sTag, m_sLabel, counts, JoinIssues(m_aBlocking)), LogLevel.ERROR);
		}
		else
		{
			// Everything that matters arrived, some of it elsewhere or in smaller quantity.
			Print(string.Format("%1 slot=%2 loadout SHORTFALL %3 -- the slot is playable and the session is NOT refused; it carries less/elsewhere than authored -- fix the mission or the kit",
				m_sTag, m_sLabel, counts), LogLevel.WARNING);
		}

		// The itemised lines always follow, so `loadout INCOMPLETE` and `loadout DEGRADED` are
		// present for every incomplete pass (the two-client playtest runbook greps them).
		if (!m_aFailures.IsEmpty())
		{
			if (HasBlockingFailure())
				Print(string.Format("%1 slot=%2 loadout INCOMPLETE %3 -- failed: %4",
					m_sTag, m_sLabel, counts, JoinIssues(m_aFailures)), LogLevel.ERROR);
			else
				Print(string.Format("%1 slot=%2 loadout INCOMPLETE %3 -- not delivered: %4",
					m_sTag, m_sLabel, counts, JoinIssues(m_aFailures)), LogLevel.WARNING);
		}

		if (!m_aDegraded.IsEmpty())
			Print(string.Format("%1 slot=%2 loadout DEGRADED %3 -- %4",
				m_sTag, m_sLabel, counts, JoinIssues(m_aDegraded)), LogLevel.WARNING);
	}
}
