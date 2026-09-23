use super::*;

const LOADER: &str = r#"
	static const int MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024;
	static bool LoadDocument(string data, string source)
	{
		if (!IsMissionBodyWithinCap(data))
		{
			TBD_Log.Error(TBD_Log.CH_MISSION, string.Format("too big %1 > %2", data.Length(), MISSION_FILE_MAX_BYTES));
			return false;
		}

		if (!ParseMissionJson(data))
			return false;

		return true;
	}

	protected static bool IsMissionBodyWithinCap(string data)
	{
		return data.Length() <= MISSION_FILE_MAX_BYTES;
	}

	protected static bool ParseMissionJson(string data)
	{
		return true;
	}
"#;

const VERIFICATION: &str =
    "\t\tif (document.Length() > TBD_MissionLoader.MISSION_FILE_MAX_BYTES)\n\t\t\treturn false;\n";
const CACHE: &str = "\t\tif (byteCount <= 0 || byteCount > TBD_MissionLoader.MISSION_FILE_MAX_BYTES)\n\t\t\treturn false;\n";

fn sources(loader: &str) -> Sources {
    Sources {
        loader: loader.to_string(),
        verification: VERIFICATION.to_string(),
        cache: CACHE.to_string(),
    }
}

#[test]
fn strip_c_comments_drops_line_and_block() {
    assert_eq!(strip_c_comments("a // x\nb"), "a \nb");
    assert_eq!(strip_c_comments("a /* x\ny */ b"), "a \n b");
}

#[test]
fn live_shaped_loader_passes_order_and_length() {
    assert!(assert_rest_size_gate(&sources(LOADER), "fixture").unwrap());
}

#[test]
fn every_red_arm_turns_the_live_shape_red() {
    for arm in [
        red1_strip_cap_call,
        red2_relocate_after_parse,
        red3_stub_return_true,
    ] {
        let perturbed = arm(LOADER).expect("the arm applies to the live shape");
        assert!(!assert_rest_size_gate(&sources(&perturbed), "red").unwrap());
    }
}

#[test]
fn a_cache_or_verification_without_the_ceiling_fails() {
    let mut uncapped_cache = sources(LOADER);
    uncapped_cache.cache = "\t\t// byteCount > TBD_MissionLoader.MISSION_FILE_MAX_BYTES\n".into();
    assert!(!assert_rest_size_gate(&uncapped_cache, "cache").unwrap());
    let mut unchecked = sources(LOADER);
    unchecked.verification = String::new();
    assert!(!assert_rest_size_gate(&unchecked, "verification").unwrap());
}
