/**
 * TBD_RegistryItemsExportPlugin.c - T-150 universal mod-agnostic registry + compat export.
 *
 * Workbench plugin: scans EVERY loaded addon's Prefabs tree (GameProject.GetLoadedAddons +
 * Workbench.SearchResources — no curated path/GUID lists; T-068.1's BuildCuratedRows()
 * allowlist is retired), classifies prefabs into registry-items v2 kinds by component
 * introspection (TBD_RegistryScan.c), derives engine-only compat edges (magazine wells,
 * attachment slot types, vehicle weapon slot chains, character loadout slots), and writes:
 *   $profile:TBD_RegistryItems.json   (registry-items.schema.json, envelope v2)
 *   $profile:TBD_RegistryCompat.json  (registry-compat.schema.json, envelope v1)
 * Compat is written last and doubles as the run-complete sentinel.
 *
 * Adding a Workshop mod to the Workbench project and re-running this plugin includes that
 * mod's items/edges with zero code changes — the export is a function of the loaded addons.
 *
 * Run: Workbench > Plugins > TBD > "Export TBD Registry Items"
 *   (or NetAPI: wb_execute_action menuPath "Plugins,TBD,Export TBD Registry Items").
 * Then copy the two $profile: files to
 *   packages/tbd-schema/registry/registry-items.workbench.json
 *   packages/tbd-schema/registry/registry-compat.workbench.json
 *
 * @contract registry-items.schema.json#/
 * @contract registry-compat.schema.json#/
 */

[WorkbenchPluginAttribute(name: "Export TBD Registry Items", description: "Scan all loaded addons and write registry-items + registry-compat JSON (T-150 universal export).", category: "TBD")]
class TBD_RegistryItemsExportPlugin : WorkbenchPlugin
{
	protected static const string OUT_ITEMS = "$profile:TBD_RegistryItems.json";
	protected static const string OUT_COMPAT = "$profile:TBD_RegistryCompat.json";
	protected static const string MODPACK_ID = "00000000-0000-4000-a000-000000000001";
	protected static const string ITEMS_VERSION = "2";
	protected static const string COMPAT_VERSION = "1";
	protected static const string TAG = "[TBD][RegistryExport]";
	protected static const int FLUSH = 8000;

	// ref: a weak FileHandle member is collected right after Open() returns (locals hold
	// strong refs implicitly; members do not) — first Write then fails with wrote=0.
	protected ref FileHandle m_Handle;
	protected string m_sBuffer;
	protected bool m_bWriteOk;

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		int startMs = System.GetTickCount();

		TBD_RegistryScanner scanner = new TBD_RegistryScanner(TAG);
		if (!scanner.ScanLoadedAddons())
		{
			Print(TAG + " FAIL: prefab enumeration returned nothing — no files written (check Workbench.SearchResources / loaded addons).", LogLevel.ERROR);
			return;
		}

		if (scanner.m_Items.IsEmpty())
		{
			Print(TAG + " FAIL: zero items classified — refusing to write an empty registry (schema minItems 1).", LogLevel.ERROR);
			return;
		}

		scanner.DeriveEdges();

		// Kind histogram AFTER DeriveEdges — it reclassifies turret-referenced weapons to
		// vehicle_weapon ('other' count is a mandatory verify-log stat).
		map<string, int> kindCounts = new map<string, int>();
		foreach (TBD_RegistryScanItem it : scanner.m_Items)
		{
			int c;
			if (!kindCounts.Find(it.kind, c))
				c = 0;
			kindCounts.Set(it.kind, c + 1);
		}

		if (!WriteItems(scanner))
			return;

		if (scanner.m_Edges.IsEmpty())
		{
			Print(TAG + " ERROR: zero compat edges derived — compat file NOT written (schema minItems 1). Check magazine wells / attachment types / vehicle weapon chains in the loaded set.", LogLevel.ERROR);
		}
		else if (!WriteCompat(scanner))
		{
			return;
		}

		int elapsed = System.GetTickCount() - startMs;
		foreach (string kind, int kc : kindCounts)
			Print(string.Format("%1 kind %2 = %3", TAG, kind, kc));
		foreach (string et, int ec : scanner.m_EdgeHistogram)
			Print(string.Format("%1 edge %2 = %3", TAG, et, ec));
		Print(string.Format("%1 scan stats: seen=%2 skippedDeny=%3 noSignal=%4 failedLoad=%5 droppedEndpoints=%6",
			TAG, scanner.m_iSeen, scanner.m_iSkippedDeny, scanner.m_iSkippedNoSignal, scanner.m_iFailedLoad, scanner.m_iDroppedEndpoints));
		Print(string.Format("%1 DONE items=%2 edges=%3 addons=%4 elapsedMs=%5",
			TAG, scanner.m_Items.Count(), scanner.m_Edges.Count(), scanner.m_Addons.Count(), elapsed));
	}

	//------------------------------------------------------------------------------------------------
	protected bool WriteItems(TBD_RegistryScanner scanner)
	{
		if (!Open(OUT_ITEMS))
			return false;

		Emit("{\n");
		Emit("  \"registryItemsVersion\": \"" + ITEMS_VERSION + "\",\n");
		Emit("  \"modpackId\": \"" + MODPACK_ID + "\",\n");
		Emit("  \"generatedAt\": \"" + IsoNowUtc() + "\",\n");
		EmitAddons(scanner);
		Emit("  \"items\": [\n");

		int written = 0;
		foreach (TBD_RegistryScanItem it : scanner.m_Items)
		{
			if (written > 0)
				Emit(",\n");
			Emit("    {\n");
			Emit("      \"resource_name\": \"" + TBD_ExportJson.Escape(it.resourceName) + "\",\n");
			Emit("      \"display_name\": \"" + TBD_ExportJson.Escape(it.displayName) + "\",\n");
			Emit("      \"category\": \"" + TBD_ExportJson.Escape(it.category) + "\",\n");
			Emit("      \"kind\": \"" + TBD_ExportJson.Escape(it.kind) + "\"\n");
			Emit("    }");
			written++;
		}

		Emit("\n  ]\n}\n");
		if (!Close(OUT_ITEMS))
			return false;

		Print(string.Format("%1 Wrote %2 items to %3", TAG, written, OUT_ITEMS));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected bool WriteCompat(TBD_RegistryScanner scanner)
	{
		if (!Open(OUT_COMPAT))
			return false;

		Emit("{\n");
		Emit("  \"registryCompatVersion\": \"" + COMPAT_VERSION + "\",\n");
		Emit("  \"modpackId\": \"" + MODPACK_ID + "\",\n");
		Emit("  \"generatedAt\": \"" + IsoNowUtc() + "\",\n");
		EmitAddons(scanner);
		Emit("  \"edges\": [\n");

		int written = 0;
		foreach (TBD_RegistryEdge edge : scanner.m_Edges)
		{
			if (written > 0)
				Emit(",\n");
			Emit("    {\n");
			Emit("      \"from_node\": \"" + TBD_ExportJson.Escape(edge.fromNode) + "\",\n");
			Emit("      \"to_node\": \"" + TBD_ExportJson.Escape(edge.toNode) + "\",\n");
			Emit("      \"edge_type\": \"" + TBD_ExportJson.Escape(edge.edgeType) + "\",\n");
			Emit("      \"evidence\": \"" + TBD_ExportJson.Escape(edge.evidence) + "\"\n");
			Emit("    }");
			written++;
		}

		Emit("\n  ]\n}\n");
		if (!Close(OUT_COMPAT))
			return false;

		Print(string.Format("%1 Wrote %2 edges to %3", TAG, written, OUT_COMPAT));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected void EmitAddons(TBD_RegistryScanner scanner)
	{
		Emit("  \"addons\": [\n");
		int written = 0;
		foreach (TBD_RegistryAddonInfo addon : scanner.m_Addons)
		{
			if (written > 0)
				Emit(",\n");
			string vanillaStr = "false";
			if (addon.isVanilla)
				vanillaStr = "true";
			Emit("    { \"guid\": \"" + TBD_ExportJson.Escape(addon.guid)
				+ "\", \"name\": \"" + TBD_ExportJson.Escape(addon.id)
				+ "\", \"title\": \"" + TBD_ExportJson.Escape(addon.title)
				+ "\", \"vanilla\": " + vanillaStr + " }");
			written++;
		}
		Emit("\n  ],\n");
	}

	//------------------------------------------------------------------------------------------------
	protected bool Open(string path)
	{
		m_sBuffer = string.Empty;
		m_bWriteOk = true;
		m_Handle = FileIO.OpenFile(path, FileMode.WRITE);
		if (!m_Handle)
		{
			Print(TAG + " Could not open " + path + " for write", LogLevel.ERROR);
			return false;
		}
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected void Emit(string chunk)
	{
		if (!m_bWriteOk)
			return;
		m_sBuffer += chunk;
		if (m_sBuffer.Length() >= FLUSH)
		{
			if (!TBD_ExportJson.Write(m_Handle, m_sBuffer, TAG))
				m_bWriteOk = false;
			m_sBuffer = string.Empty;
		}
	}

	//------------------------------------------------------------------------------------------------
	protected bool Close(string path)
	{
		if (m_bWriteOk && !m_sBuffer.IsEmpty())
		{
			if (!TBD_ExportJson.Write(m_Handle, m_sBuffer, TAG))
				m_bWriteOk = false;
		}
		m_sBuffer = string.Empty;
		m_Handle.Close();

		if (!m_bWriteOk)
		{
			FileIO.DeleteFile(path);
			Print(TAG + " write failed — deleted partial " + path, LogLevel.ERROR);
			return false;
		}
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected string IsoNowUtc()
	{
		int y, mo, d, h, mi, s;
		System.GetYearMonthDayUTC(y, mo, d);
		System.GetHourMinuteSecondUTC(h, mi, s);
		return string.Format("%1-%2-%3T%4:%5:%6Z", y, Pad2(mo), Pad2(d), Pad2(h), Pad2(mi), Pad2(s));
	}

	//------------------------------------------------------------------------------------------------
	protected string Pad2(int v)
	{
		if (v < 10)
			return "0" + v.ToString();
		return v.ToString();
	}
}
