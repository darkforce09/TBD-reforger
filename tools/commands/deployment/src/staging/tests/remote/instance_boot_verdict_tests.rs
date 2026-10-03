//! The boot progress probe, and which log folder counts as the new boot's.
use super::*;
use crate::staging::config::tests::base;

fn seen(instance: u16, registered: bool, folder: &str) -> BootProgress {
    BootProgress {
        instance,
        registered,
        newest_log_folder: folder.to_string(),
    }
}

#[test]
fn the_probe_reports_every_instance_in_one_round_trip() {
    let p = boot_progress_payload(&base().fleet.instances());
    assert!(p.starts_with("set -u\nfor n in 1 2 3 4 5; do\n"));
    assert!(p.contains(
        "ls -1d \"$HOME/tbd/fleet/instance-$n/profile/logs/logs_\"* 2>/dev/null | tail -1 || true"
    ));
    assert!(p.contains("grep -qF 'Server registered with address:' \"$newest/console.log\""));
    assert!(p.contains("printf '%s %s %s\\n' \"$n\" \"$registered\" \"$newest\""));
}

#[test]
fn probe_lines_parse_and_a_bad_line_is_skipped() {
    let text = "1 1 /home/deploy/tbd/fleet/instance-1/profile/logs/logs_2026-09-29_10-00-00\n\
                2 0 \n\
                garbage\n\
                3 0 /home/deploy/tbd/fleet/instance-3/profile/logs/logs_2026-09-29_10-00-01\n";
    assert_eq!(
        parse_boot_progress(text),
        [
            seen(
                1,
                true,
                "/home/deploy/tbd/fleet/instance-1/profile/logs/logs_2026-09-29_10-00-00"
            ),
            seen(2, false, ""),
            seen(
                3,
                false,
                "/home/deploy/tbd/fleet/instance-3/profile/logs/logs_2026-09-29_10-00-01"
            ),
        ]
    );
}

/// The previous boot's log also says "Server registered": the folder that was newest before the
/// restart never counts as the new boot's, so a server that did not come back cannot pass on it.
#[test]
fn the_log_folder_newest_before_the_restart_never_counts() {
    let before = [
        seen(1, true, "/p/logs/logs_2026-09-29_09-00-00"),
        seen(2, false, ""),
    ];
    assert_eq!(
        new_log_folder(&before, &seen(1, true, "/p/logs/logs_2026-09-29_09-00-00")),
        None,
        "the stale folder of the previous boot"
    );
    assert_eq!(
        new_log_folder(&before, &seen(1, true, "/p/logs/logs_2026-09-29_10-00-00")).as_deref(),
        Some("/p/logs/logs_2026-09-29_10-00-00")
    );
    assert_eq!(
        new_log_folder(&before, &seen(2, false, "")),
        None,
        "no folder yet"
    );
    assert_eq!(
        new_log_folder(&before, &seen(2, false, "/q/logs/logs_2026-09-29_10-00-00")).as_deref(),
        Some("/q/logs/logs_2026-09-29_10-00-00"),
        "an instance's first boot"
    );
    assert_eq!(
        new_log_folder(&[], &seen(3, true, "/r/logs/logs_1")).as_deref(),
        Some("/r/logs/logs_1"),
        "an instance the before probe did not see"
    );
}
