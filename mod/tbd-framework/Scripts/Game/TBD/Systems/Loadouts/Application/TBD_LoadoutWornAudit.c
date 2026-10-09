/**
 * @file TBD_LoadoutWornAudit.c
 * @brief The nakedness guard: reports a loadout body that wears no jacket or no pants.
 *
 * Role: reads the decency areas (jacket, pants, boots) of a body once after a loadout pass, or
 * polls them after a kit-only spawn, and writes one audit line.  Position: owned by
 * `TBD_LoadoutApplication`, which calls `AuditWorn` from its tail and `Begin` from
 * `RunKitWornAudit`; the kit-only path ends the application through `MarkDone`.
 * State: the kit alias of a kit-only audit, on the server.  Invariants: a naked or half-dressed
 * body is always an ERROR naming the slot; the kit-only poll reports success the first tick the
 * body is decent, so it never accuses a body that is still dressing.
 */

//! Audits what a loadout body is wearing.
class TBD_LoadoutWornAudit : Managed
{
	//! Kit-only poll period. Kit clothing is part of the prefab's entity hierarchy, so a kit body
	//! reads decent on the first tick; the 1 s deadline (4 x 250 ms) bounds how long a naked body
	//! goes unreported and stays inside the `world-boot.sh` capture window, which ends about 3 s
	//! after the bodies spawn (`TBD_WORLDBOOT_SETTLE`).
	protected const int AUDIT_TICK_MS = 250; //!< ms between kit-only audit reads
	protected const int AUDIT_MAX_ATTEMPTS = 4; //!< kit-only reads before the audit reports

	protected TBD_LoadoutApplication m_Application; //!< the owning application; weak, it owns this audit
	protected IEntity m_Character; //!< the audited body; null once the engine deletes it
	protected string m_sTag; //!< log tag, for example `[TBD][Loadout][Slot]`
	protected string m_sLabel; //!< slot id or harness label named on every log line
	protected string m_sKit; //!< kit alias of a kit-only audit, for example `kit:sov_rifleman`; empty otherwise

	//! Bind the audit to its application and read the body, tag and label.
	void TBD_LoadoutWornAudit(TBD_LoadoutApplication application)
	{
		m_Application = application;
		m_Character = application.GetCharacter();
		m_sTag = application.GetTag();
		m_sLabel = application.GetLabel();
	}

	//! Read the decency areas once, after a loadout pass. The gear verify and the weapon phase have
	//! settled by then, so no poll is needed. Writes the audit line; an ERROR when the body is naked
	//! or half-dressed, or has no character storage.
	//! @authority server
	void AuditWorn()
	{
		SCR_CharacterInventoryStorageComponent charStorage = SCR_CharacterInventoryStorageComponent.Cast(
			m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));
		if (!charStorage)
		{
			Print(string.Format("%1 slot=%2 worn-audit SKIPPED -- character has no SCR_CharacterInventoryStorageComponent", m_sTag, m_sLabel), LogLevel.ERROR);
			return;
		}

		ReportWornAudit(charStorage, "after loadout pass", "kit prefab and JSON loadout both left the body bare");
	}

	//! Write the audit line for one reading: OK, `NAKED` (ERROR) or `HALF-DRESSED` (ERROR).
	//! @param charStorage the body's character storage
	//! @param context when the reading was taken
	//! @param cause who is responsible for a bare body
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
			Print(string.Format("%1 slot=%2 NAKED %3 -- no jacket and no pants worn (%4)",
				m_sTag, m_sLabel, context, cause), LogLevel.ERROR);
			return;
		}

		Print(string.Format("%1 slot=%2 HALF-DRESSED %3 -- jacket=%4 pants=%5 boots=%6 (%7)",
			m_sTag, m_sLabel, context, jacket, pants, boots, cause), LogLevel.ERROR);
	}

	//! Start the kit-only audit: poll the decency areas every `AUDIT_TICK_MS`.
	//! @param kit the kit alias the body was spawned from, named on every line
	//! @authority server
	void Begin(string kit)
	{
		m_sKit = kit;
		GetGame().GetCallqueue().CallLater(AuditTick, AUDIT_TICK_MS, false, 1);
	}

	//! One kit-only poll. Reports OK, with the attempt it settled on, the first time jacket and pants
	//! are worn; retries until `AUDIT_MAX_ATTEMPTS`; then reports the finding and ends the pass. A
	//! deleted body cancels the application instead.
	//! @param attempt 1-based poll number
	//! @authority server
	protected void AuditTick(int attempt)
	{
		if (!m_Application || m_Application.IsDone())
			return;
		if (!m_Character)
		{
			// Body deleted between ticks: a stand-down, not a nakedness finding.
			m_Application.Cancel("body superseded");
			return;
		}

		SCR_CharacterInventoryStorageComponent charStorage = SCR_CharacterInventoryStorageComponent.Cast(
			m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));

		// A body with no character storage yet is retried like any other not-yet-decent state and
		// reported only at the deadline.
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
			m_Application.MarkDone();
			return;
		}

		if (attempt < AUDIT_MAX_ATTEMPTS)
		{
			GetGame().GetCallqueue().CallLater(AuditTick, AUDIT_TICK_MS, false, attempt + 1);
			return;
		}

		// Deadline reached and still not decent.
		if (!charStorage)
		{
			Print(string.Format("%1 slot=%2 worn-audit SKIPPED -- character has no SCR_CharacterInventoryStorageComponent after %3 ms (kit %4)",
				m_sTag, m_sLabel, AUDIT_MAX_ATTEMPTS * AUDIT_TICK_MS, m_sKit), LogLevel.ERROR);
			m_Application.MarkDone();
			return;
		}

		ReportWornAudit(charStorage,
			string.Format("after kit spawn (%1 ms, no JSON loadout authored)", AUDIT_MAX_ATTEMPTS * AUDIT_TICK_MS),
			string.Format("kit %1 dressed the body itself and this is what it produced -- fix the kit prefab or author a loadout", m_sKit));
		m_Application.MarkDone();
	}
}
