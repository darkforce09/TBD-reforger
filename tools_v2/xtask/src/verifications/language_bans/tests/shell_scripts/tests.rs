use super::{python3_in_command_position, shebang_names_python, shebang_names_shell};

#[test]
fn counts_real_shell_shebangs() {
    // An extensionless tool an inventory would have missed.
    assert!(shebang_names_shell("#!/usr/bin/env bash"));
    assert!(shebang_names_shell("#!/bin/sh"));
    assert!(shebang_names_shell("#!/bin/bash -e"));
    assert!(shebang_names_shell("#!/bin/dash"));
    assert!(shebang_names_shell("#!/usr/bin/zsh"));
    assert!(shebang_names_shell("#!/usr/bin/env -S bash -euo pipefail"));
    assert!(shebang_names_shell("#!  /bin/bash"));
    assert!(shebang_names_shell("#!/usr/bin/env ksh"));
    assert!(shebang_names_shell("#!/usr/bin/env fish"));
}

#[test]
fn does_not_sweep_in_rust_inner_attributes() {
    // THE TRAP. tools_v2/xtask/src/main.rs opens with this, at byte 0, `#!` and all. A
    // `starts_with("#!")` test would file every Rust file carrying an inner attribute into
    // the language ban, and the gate would then demand they be ported to Rust.
    assert!(!shebang_names_shell("#![allow(clippy::collapsible_if)]"));
    assert!(!shebang_names_shell("#![no_std]"));
    assert!(!shebang_names_shell("#![doc = \"run with bash\"]"));
    assert!(!shebang_names_python("#![allow(clippy::collapsible_if)]"));
}

#[test]
fn does_not_sweep_in_other_interpreters_or_prose() {
    assert!(!shebang_names_shell("#!/usr/bin/env python3"));
    assert!(!shebang_names_shell("#!/usr/bin/env node"));
    assert!(!shebang_names_shell("#!/usr/bin/perl"));
    // `#!` must be at the very front, and a mention of a shell is not a shebang.
    assert!(!shebang_names_shell("  #!/bin/bash"));
    assert!(!shebang_names_shell(
        "// see scripts/foo.sh, run under bash"
    ));
    assert!(!shebang_names_shell("#!"));
    assert!(!shebang_names_shell(""));
    // `sh` must be the interpreter, not merely a substring of one.
    assert!(!shebang_names_shell("#!/usr/bin/shellcheck"));
    assert!(!shebang_names_shell("#!/usr/bin/env bashful"));
}

#[test]
fn python_shebang_is_python_not_shell() {
    assert!(shebang_names_python("#!/usr/bin/env python3"));
    assert!(shebang_names_python("#!/usr/bin/python3"));
    assert!(shebang_names_python("#!/usr/bin/env python"));
    assert!(!shebang_names_python("#!/usr/bin/env bash"));
}

#[test]
fn python3_command_position_ignores_comments() {
    assert!(!python3_in_command_position(
        "# deliberately no python3 here"
    ));
    assert!(!python3_in_command_position("// python3 -c 'print(1)'"));
    assert!(!python3_in_command_position("    // python3 -c 'print(1)'"));
    assert!(!python3_in_command_position("echo python3"));
    assert!(!python3_in_command_position(r#"Command::new("python3")"#));
}

#[test]
fn python3_command_position_catches_invocations() {
    assert!(python3_in_command_position("python3 -c 'print(1)'"));
    assert!(python3_in_command_position("  /usr/bin/python3 foo"));
    assert!(python3_in_command_position("env python3 -c 'x'"));
    assert!(python3_in_command_position("#!/usr/bin/env python3"));
    assert!(python3_in_command_position("echo hi; python3 -c 'x'"));
}
