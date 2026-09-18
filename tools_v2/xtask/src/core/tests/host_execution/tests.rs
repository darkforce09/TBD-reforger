use super::*;

fn on_metal() -> Host {
    Host::new(None, false)
}

#[test]
fn on_the_metal_commands_run_directly_and_need_no_bridge() {
    let h = on_metal();
    assert!(h.require_host(), "no bridge is needed outside a container");
    assert_eq!(h.argv(&["echo", "hi"]), vec!["echo", "hi"]);
    assert_eq!(h.capture(&["echo", "hi"]).unwrap(), "hi\n");
    // The loud form runs it for real and hands back the raw rc.
    assert_eq!(h.run(&["true"]), 0);
    assert_eq!(h.run(&["false"]), 1);
}

#[test]
fn in_a_container_the_bridge_is_prepended() {
    let h = Host::new(Some("distrobox-host-exec".into()), true);
    assert_eq!(
        h.argv(&["kill", "-9", "--", "-42"]),
        vec!["distrobox-host-exec", "kill", "-9", "--", "-42"]
    );
    assert!(h.require_host(), "a real bridge passes preflight");
}

#[test]
fn bridge_is_never_used_on_the_metal() {
    // THE 126 TRAP — see the module docs. `distrobox-host-exec` is installed on the HOST too,
    // where it refuses with exit 126 ("You must run distrobox-host-exec inside a container!").
    // So `which` finding it says nothing about which side we are on, and a `Host` that resolved
    // a bridge while NOT containerised must still run everything bare. Measured 2026-07-26: the
    // regression cost 10/10 red steps and an hour chasing a phantom.
    let h = Host::new(Some("distrobox-host-exec".into()), false);
    assert!(h.has_bridge(), "the binary really is on PATH on the host");
    assert!(!h.is_in_container());
    assert_eq!(
        h.argv(&["echo", "hi"]),
        vec!["echo", "hi"],
        "a bridge found on the metal must NOT be prepended"
    );
    // And it really executes, rather than being refused for lack of a bridge it does not need.
    assert_eq!(h.capture(&["echo", "hi"]).unwrap(), "hi\n");
    assert!(h.require_host());
}

#[test]
fn a_containerised_host_with_no_bridge_is_rc_127_and_silent() {
    // hostrun's `return 127` path. The capture must be None — NOT an empty success, which is
    // what let a bridge failure read as "the process is dead" (T-608).
    let h = Host::new(None, true);
    assert!(h.capture(&["echo", "hi"]).is_none());
    assert_eq!(h.capture_trimmed(&["echo", "hi"]), "");
    // …and `require_host` refuses it early, which is that function's entire job.
    assert!(!h.require_host());
}

#[test]
fn a_containerised_host_with_no_bridge_refuses_loudly_with_rc_127() {
    // The other half of the same state: `run` is the form that does NOT swallow stderr, so it
    // owes the operator the real diagnosis instead of a linker/GLIBC error.
    let h = Host::new(None, true);
    assert_eq!(h.run(&["ArmaReforgerServer", "-config", "x.json"]), 127);
    assert_eq!(NO_BRIDGE_RC, 127);

    let msg = h.refusal(&["ArmaReforgerServer", "-config", "x.json"]);
    assert!(msg.starts_with("hostrun: running inside a container with no host bridge available."));
    // bash `$1` — the program, single-quoted.
    assert!(msg.contains("so 'ArmaReforgerServer' would fail"));
    // bash `$*` — the whole command, space-joined, for the operator to paste.
    assert!(msg.ends_with("      ArmaReforgerServer -config x.json"));
    // The point of the message: name the misleading error so nobody "fixes" the toolchain again.
    assert!(msg.contains("misleading linker/GLIBC error"));
    assert!(msg.contains("distrobox-host-exec (or host-spawn)"));
}

#[test]
fn the_refusal_is_the_bash_heredoc() {
    // Byte-for-byte, with the glibc probe injected so the test does not depend on the machine.
    // The ONE deviation from the bash is the "and no C toolchain" clause — see `refusal_text`.
    let got = refusal_text(&["cargo", "build", "-p", "xtask"], "2.36");
    assert_eq!(
        got,
        "hostrun: running inside a container with no host bridge available.\n\
         \n\
         \x20 Needed: distrobox-host-exec (or host-spawn) to reach the real machine.\n\
         \x20 This container has glibc 2.36, and Steam, the Workbench and ArmaReforgerServer are\n\
         \x20 installed on the host against a newer one,\n\
         \x20 so 'cargo' would fail with a misleading linker/GLIBC error rather than a useful one.\n\
         \n\
         \x20 Run this command on the host instead:\n\
         \x20     cargo build -p xtask"
    );
}

#[test]
fn a_broken_bridge_answers_nothing_even_when_one_exists() {
    // S1's mechanism: the bridge is present and would work, and we still get no answer. This is
    // the ONLY way to reproduce T-608's trigger, because a real bridge cannot be made to flake.
    let metal = on_metal().broken();
    assert!(metal.capture(&["echo", "hi"]).is_none());
    // `require_host` short-circuits on the metal before any bridge question is asked — bash's
    // `if ! in_container; then return 0; fi` did the same, and S1 never calls it on the broken
    // host anyway (the override was scoped to the `kill_run` subshell).
    assert!(metal.require_host());
    // Inside a container, a broken bridge does fail the preflight.
    let boxed = Host::new(Some("distrobox-host-exec".into()), true).broken();
    assert!(
        !boxed.require_host(),
        "a broken bridge must not pass preflight"
    );
    assert!(boxed.capture(&["echo", "hi"]).is_none());
}

#[test]
fn capture_trimmed_deletes_all_whitespace_like_tr_d() {
    let h = on_metal();
    assert_eq!(
        h.capture_trimmed(&["printf", "  192.168.0.117 \n"]),
        "192.168.0.117"
    );
}

#[test]
fn instruction_name_is_the_paste_ready_bridge_not_the_resolved_one() {
    // PRESERVED ODDITY — see `instruction_name`.
    let h = Host::new(Some("host-spawn".into()), true);
    assert_eq!(h.instruction_name(), "distrobox-host-exec");
}

#[test]
fn trailing_version_is_grep_o_anchored_at_end() {
    // The real container line and the real host line — both measured 2026-08-12.
    assert_eq!(
        trailing_version("ldd (Debian GLIBC 2.36-9+deb12u14) 2.36").unwrap(),
        "2.36"
    );
    assert_eq!(trailing_version("ldd (GNU libc) 2.43").unwrap(), "2.43");
    // Anchored: a version that is not at end of line does not match, which is why bash took
    // `head -1` rather than grepping the whole output.
    assert_eq!(trailing_version("Copyright (C) 2024 FSF, Inc."), None);
    assert_eq!(trailing_version("ldd (Debian GLIBC 2.36-9+deb12u14)"), None);
    // Leftmost-longest ending at `$`, exactly like grep: `1.2.3` yields `2.3`, not `1.2.3`.
    assert_eq!(trailing_version("something 1.2.3").unwrap(), "2.3");
    // No dot, or no digits either side of it, is no match.
    assert_eq!(trailing_version("ldd 236"), None);
    assert_eq!(trailing_version("ldd .5"), None);
}

/// A host bridge executes in the host mount namespace, independently of libc versions.
#[test]
fn the_bridge_really_crosses_the_container_wall() {
    let h = Host::detect();
    if !h.is_in_container() || !h.has_bridge() {
        eprintln!("SKIP: not containerised with a bridge — nothing to cross");
        return;
    }
    let here =
        std::fs::read_link("/proc/self/ns/mnt").expect("container mount namespace is readable");
    let there = h
        .capture(&["readlink", "/proc/self/ns/mnt"])
        .expect("the bridge answered nothing at all");
    assert!(!there.trim().is_empty(), "host mount namespace is empty");
    assert_ne!(
        here.to_string_lossy(),
        there.trim(),
        "the bridge returned the container mount namespace — it did not cross"
    );
}

#[test]
fn detect_agrees_with_the_two_marker_files() {
    // `in_container()` is the free function bash exported; `Host::detect` must not drift from it.
    let h = Host::detect();
    assert_eq!(h.is_in_container(), in_container());
    assert_eq!(
        in_container(),
        std::path::Path::new("/run/.containerenv").exists()
            || std::path::Path::new("/.dockerenv").exists()
    );
    // Whatever side we are on, a detected host must be able to run something. In a container
    // that needs a real bridge; on the metal it needs nothing.
    if h.require_host() {
        assert_eq!(h.capture(&["echo", "hi"]).as_deref(), Some("hi\n"));
    }
}
