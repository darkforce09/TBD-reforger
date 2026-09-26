/**
 * @file TBD_MissionUnconsumedKeyCheck.c
 * @brief Warns about authored mission keys this build drops or does not honour.
 *
 * Role: the unconsumed-key check of mission validation.  Position: called by
 * `TBD_MissionValidator.Run`; reads the held mission text (`TBD_MissionLoader.GetRawJson`) and a
 * second typed pass into `TBD_ValidatorSecondPassStruct`; writes WARNINGs into a
 * `TBD_MissionValidationFindings`. `cargo xtask ci schema-validate` requires one
 * `UNCONSUMED-WARN: <key>` marker and one `AddWarning` per key below.
 * State: none.  Invariants: a key warns only when something authored will not be honoured, never
 * on a value the framework already honours (`tickets: 0`, `respawn: "none"`, an empty list):
 *   * `environment`: presence of the block.
 *   * `settings`: `respawn` other than "none" (no respawn pool exists; vanilla respawn is stood
 *     down) and `nightVision: true` (no NVG seam); `spectatorPolicy` is applied.
 *   * `layers`: one or more aliases; decoration layers are not loaded.
 *   * `factions[].tickets`: a positive pool; 0 (or absent) is the schema's one-life, which is the
 *     framework's only mode.
 *   * `orbat.*.groups[].roles[].radio`: a non-empty list; net delivery is faction-wide
 *     (`TBD_RadioService.BuildForPlayer`), so role-scoped nets are not honoured.
 * A second pass that cannot parse falls back to presence probes, toward warning, never silence.
 */

//! Static unconsumed-key check.
class TBD_MissionUnconsumedKeyCheck
{
	//! Warn for each authored key this build drops or does not honour.
	//! @param findings receives the warnings
	//! @param mission the document; null does nothing
	static void CheckUnconsumedKeys(TBD_MissionValidationFindings findings, TBD_MissionDocumentStruct mission)
	{
		if (!mission)
			return;

		string json = TBD_MissionLoader.GetRawJson();
		if (json.IsEmpty())
			return;

		// UNCONSUMED-WARN: environment -- presence of the block.
		if (JsonAuthoredKey(json, "environment"))
		{
			findings.AddWarning("environment",
				"authored but TBD_MissionDocumentStruct does not model it -- weather and time of day are not applied at load");
		}

		// UNCONSUMED-WARN: settings -- values only: spectatorPolicy is applied at load, and the
		// block is always allocated, so presence proves nothing.
		TBD_MissionSettingsStruct settings = mission.settings;
		if (settings)
		{
			if (!settings.respawn.IsEmpty() && settings.respawn != "none")
			{
				findings.AddWarning("settings", string.Format(
					"respawn='%1' authored but the framework implements no respawn pool -- vanilla respawn is stood down (TBD_SCR_RespawnSystemComponent), one-life is the only mode, and the value is not honoured",
					settings.respawn));
			}

			if (settings.nightVision)
			{
				findings.AddWarning("settings",
					"nightVision=true authored but no NVG policy seam exists -- the value is not honoured");
			}
		}

		// entities[] is modeled + SpawnMissionEntities places resolvable rows, so it is consumed.

		TBD_ValidatorSecondPassStruct raw = ParseSecondPass();
		if (!raw)
		{
			// Fail-closed fallback: presence probes, each naming itself so a fallback finding is
			// distinguishable from the precise check.
			if (JsonAuthoredKey(json, "layers"))
			{
				findings.AddWarning("layers",
					"authored but decoration layers are not loaded -- the block is discarded on load (second-pass parse failed; presence probe)");
			}

			if (JsonAuthoredKey(json, "tickets"))
			{
				findings.AddWarning("factions.tickets",
					"authored but TBD_MissionFactionStruct does not model tickets -- respawn pools are not applied (second-pass parse failed; presence probe)");
			}

			if (JsonAuthoredKey(json, "radio"))
			{
				findings.AddWarning("orbat.roles.radio",
					"authored but TBD_MissionOrbatRoleStruct does not model radio -- role-scoped net assignment is not applied (second-pass parse failed; presence probe)");
			}

			return;
		}

		// UNCONSUMED-WARN: layers -- content: an absent key reads null and an authored [] drops
		// nothing; neither warns.
		if (raw.layers && raw.layers.Count() > 0)
		{
			findings.AddWarning("layers", string.Format(
				"%1 decoration layer alias(es) authored but decoration layers are not loaded -- the block is discarded on load",
				raw.layers.Count()));
		}

		// UNCONSUMED-WARN: tickets -- a positive pool only: 0 is the schema's one-life, which the
		// framework implements, and an absent key reads 0 too.
		if (raw.factions)
		{
			foreach (TBD_ValidatorRawFactionStruct rawFaction : raw.factions)
			{
				if (!rawFaction || rawFaction.tickets <= 0)
					continue;

				string factionName = rawFaction.key;
				if (factionName.IsEmpty())
					factionName = "(no key)";

				findings.AddWarning("factions.tickets", string.Format(
					"faction '%1' authors tickets=%2 but TBD_MissionFactionStruct does not model tickets -- no respawn pool will exist (an authored 0 would be one-life, which IS the implemented behaviour)",
					factionName, rawFaction.tickets));
			}
		}

		// UNCONSUMED-WARN: radio -- content: only a role naming at least one net authors something
		// faction-wide delivery can fail to honour.
		if (raw.orbat)
		{
			int rolesWithNets = 0;
			foreach (string factionKey, TBD_ValidatorRawOrbatFactionStruct side : raw.orbat)
			{
				if (!side || !side.groups)
					continue;

				foreach (TBD_ValidatorRawGroupStruct group : side.groups)
				{
					if (!group || !group.roles)
						continue;

					foreach (TBD_ValidatorRawRoleStruct role : group.roles)
					{
						if (role && role.radio && role.radio.Count() > 0)
							rolesWithNets++;
					}
				}
			}

			if (rolesWithNets > 0)
			{
				findings.AddWarning("orbat.roles.radio", string.Format(
					"%1 ORBAT role(s) author radio[] net ids but TBD_MissionOrbatRoleStruct does not model radio -- net delivery is faction-wide (TBD_RadioService.BuildForPlayer), so role-scoped assignments are not honoured",
					rolesWithNets));
			}
		}
	}

	//! Second typed pass over the held mission text for the values the primary document does not
	//! declare.
	//! @return the pass, or null when the text is missing or does not parse
	protected static TBD_ValidatorSecondPassStruct ParseSecondPass()
	{
		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (!ctx)
			return null;

		TBD_ValidatorSecondPassStruct doc = new TBD_ValidatorSecondPassStruct();
		if (!ctx.ReadValue("", doc))
			return null;

		return doc;
	}

	//! Whether `json` contains a `"key":` property assignment. Exact for this contract: `"radio":`
	//! cannot match `"radioPlan":`.
	//! @param json the mission text
	//! @param key the property name
	//! @return true when the key is assigned anywhere in the text
	protected static bool JsonAuthoredKey(string json, string key)
	{
		if (json.IsEmpty() || key.IsEmpty())
			return false;

		string needle = "\"" + key + "\":";
		return json.IndexOf(needle) >= 0;
	}
}
