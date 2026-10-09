class TBD_SourceExportEnvironment
{
	static string Metadata(string value, string reason)
	{
		if (value.IsEmpty()) return "{\"status\":\"unavailable\",\"value\":null,\"reason\":" + TBD_SourceExportJson.Quote(reason) + "}";
		return "{\"status\":\"present\",\"value\":" + TBD_SourceExportJson.Quote(value) + ",\"reason\":null}";
	}

	static string Capture()
	{
		array<string> addons = {};
		GameProject.GetLoadedAddons(addons);
		array<string> entries = {};
		foreach (string guid : addons)
		{
			string entry = "{\"guid\":" + TBD_SourceExportJson.Quote(guid);
			entry += ",\"addon_id\":" + TBD_SourceExportJson.Quote(GameProject.GetAddonID(guid));
			entry += ",\"title\":" + TBD_SourceExportJson.Quote(GameProject.GetAddonTitle(guid));
			entry += ",\"version\":" + Metadata("", "GameProject does not expose loaded addon versions") + "}";
			entries.Insert(entry);
		}
		string build;
		if (GetGame()) build = GetGame().GetBuildVersion();
		string revision;
		FileHandle revisionFile = FileIO.OpenFile("$TBD_Export:exporter_revision.txt", FileMode.READ);
		if (revisionFile) { revisionFile.ReadLine(revision); revisionFile.Close(); }
		string json = "{\"game_build\":" + Metadata(build, "Game.GetBuildVersion returned no value");
		json += ",\"exporter_revision\":" + Metadata(revision, "No exporter build stamp is installed");
		json += ",\"addon_load_order\":[" + TBD_SourceExportJson.Join(entries) + "]";
		json += ",\"settings\":{\"source_only\":true,\"locale\":\"en_us\",\"max_container_depth\":128,\"max_nodes_per_resource\":100000";
		json += ",\"installation_identity\":\"native_container_id_with_structural_context_when_available\",\"addon_order_method\":\"GameProject.GetLoadedAddons\"}}";
		return json;
	}

	static string Timestamp()
	{
		int year, month, day, hour, minute, second;
		System.GetYearMonthDayUTC(year, month, day);
		System.GetHourMinuteSecondUTC(hour, minute, second);
		return string.Format("%1-%2-%3T%4:%5:%6Z", year, Pad(month), Pad(day), Pad(hour), Pad(minute), Pad(second));
	}

	protected static string Pad(int number)
	{
		if (number < 10) return "0" + number.ToString();
		return number.ToString();
	}
}
