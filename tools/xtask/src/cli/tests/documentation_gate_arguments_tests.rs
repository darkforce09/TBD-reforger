//! The documentation gate arguments as the CLI parses them: the repeatable `--path`, the
//! `--with-untracked` switch and link-check's `--report`.

use crate::cli::{Cli, TopCmd};
use crate::commands::verify::cli::{DocumentationGateArgs, VerifyCmd};
use clap::Parser;

/// The documentation gate arguments `verb` parses from `extra`.
fn gate_arguments(verb: &str, extra: &[&str]) -> DocumentationGateArgs {
    let mut line = vec!["xtask", "verify", verb];
    line.extend_from_slice(extra);
    match Cli::try_parse_from(line).expect("the verb parses").cmd {
        TopCmd::Verify {
            cmd:
                VerifyCmd::ReadmeCoverage { arguments }
                | VerifyCmd::MarkdownPlacement { arguments }
                | VerifyCmd::LinkCheck { arguments, .. },
        } => arguments,
        other => panic!("{verb} parsed as {other:?}"),
    }
}

#[test]
fn every_documentation_verb_takes_a_repeatable_path_and_the_untracked_flag() {
    for verb in ["readme-coverage", "markdown-placement", "link-check"] {
        let arguments = gate_arguments(
            verb,
            &["--path", "mod", "--with-untracked", "--path", "tools"],
        );
        assert_eq!(arguments.paths, ["mod", "tools"], "{verb}");
        assert!(arguments.with_untracked, "{verb}");
        let bare = gate_arguments(verb, &[]);
        assert!(bare.paths.is_empty(), "{verb}");
        assert!(
            !bare.with_untracked,
            "{verb}: CI's committed view is the default"
        );
    }
}

#[test]
fn the_verb_takes_the_report_flag_beside_the_documentation_gate_arguments() {
    let parsed = Cli::try_parse_from([
        "xtask",
        "verify",
        "link-check",
        "--report",
        "--path",
        "mod",
        "--with-untracked",
        "--path",
        "tools",
    ])
    .expect("the verb parses");
    match parsed.cmd {
        TopCmd::Verify {
            cmd: VerifyCmd::LinkCheck { report, arguments },
        } => {
            assert!(report);
            assert_eq!(arguments.paths, ["mod", "tools"]);
            assert!(arguments.with_untracked);
        }
        other => panic!("link-check parsed as {other:?}"),
    }
    let bare = Cli::try_parse_from(["xtask", "verify", "link-check"]).expect("no flags");
    assert!(matches!(
        bare.cmd,
        TopCmd::Verify {
            cmd: VerifyCmd::LinkCheck {
                report: false,
                arguments: DocumentationGateArgs { paths, with_untracked: false },
            },
        } if paths.is_empty()
    ));
}
