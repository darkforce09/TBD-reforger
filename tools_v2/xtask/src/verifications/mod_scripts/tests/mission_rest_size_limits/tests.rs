use super::*;

#[test]
fn strip_c_comments_drops_line_and_block() {
    assert_eq!(strip_c_comments("a // x\nb"), "a \nb");
    assert_eq!(strip_c_comments("a /* x\ny */ b"), "a \n b");
}

#[test]
fn live_shaped_helper_passes_order_and_length() {
    let src = r#"
	protected static const int MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024;
	protected static void OnBackendFetchSuccess(RestCallback cb)
	{
		if (!IsMissionBodyWithinCap(data))
		{
			Print(string.Format("too big %1 > %2", data.Length(), MISSION_FILE_MAX_BYTES), LogLevel.ERROR);
			return;
		}
		if (!ParseMissionJson(data))
		{
			return;
		}
	}
	protected static void OnBackendFetchError(RestCallback cb)
	{
	}
	protected static bool IsMissionBodyWithinCap(string data)
	{
		return data.Length() <= MISSION_FILE_MAX_BYTES;
	}
	protected static bool ParseMissionJson(string data)
	{
		return true;
	}
	protected static bool LoadFromProfileFile(string missionId)
	{
		if (fileSize > MISSION_FILE_MAX_BYTES)
		{
			return false;
		}
		return true;
	}
"#;
    assert!(assert_rest_size_gate(src, "fixture").unwrap());
}

#[test]
fn return_true_stub_fails() {
    let src = r#"
	protected static const int MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024;
	protected static void OnBackendFetchSuccess(RestCallback cb)
	{
		if (!IsMissionBodyWithinCap(data))
		{
			return;
		}
		if (!ParseMissionJson(data))
		{
			return;
		}
	}
	protected static void OnBackendFetchError(RestCallback cb)
	{
	}
	protected static bool IsMissionBodyWithinCap(string data)
	{
		return true;
	}
	protected static bool ParseMissionJson(string data)
	{
		return true;
	}
	protected static bool LoadFromProfileFile(string missionId)
	{
		if (fileSize > MISSION_FILE_MAX_BYTES)
		{
			return false;
		}
		return true;
	}
"#;
    assert!(!assert_rest_size_gate(src, "stub").unwrap());
}
