//! The shared ssh transport: the precedence of the settings, the argv of each form, and the
//! password kept out of every argv and every `Debug` rendering.
use super::*;

#[test]
fn secure_shell_transport_password_wins_over_identity_file() {
    assert_eq!(
        SshBase::from_settings(Some("pw"), Some("/k/id")),
        SshBase::Pass("pw".into())
    );
    assert_eq!(
        SshBase::from_settings(None, Some("/k/id")),
        SshBase::Identity("/k/id".into())
    );
    assert_eq!(SshBase::from_settings(None, None), SshBase::Plain);
}

#[test]
fn secure_shell_transport_argv_of_each_form() {
    let remote = vec!["true".to_string()];
    assert_eq!(
        ssh_argv(&SshBase::Plain, "deploy@h", &remote),
        ["ssh", "-o", "StrictHostKeyChecking=no", "deploy@h", "true"]
    );
    assert_eq!(
        ssh_argv(&SshBase::Identity("/k/id".into()), "deploy@h", &remote),
        [
            "ssh",
            "-i",
            "/k/id",
            "-o",
            "StrictHostKeyChecking=no",
            "deploy@h",
            "true"
        ]
    );
    assert_eq!(
        ssh_argv(&SshBase::Pass("pw".into()), "deploy@h", &remote),
        [
            "sshpass",
            "-e",
            "ssh",
            "-o",
            "StrictHostKeyChecking=no",
            "deploy@h",
            "true"
        ]
    );
}

#[test]
fn secure_shell_transport_keeps_the_password_out_of_argv_and_debug() {
    let base = SshBase::Pass("ssh-password-canary".into());
    let argv = ssh_argv(&base, "deploy@h", &["true".to_string()]);
    assert!(argv.iter().all(|a| !a.contains("ssh-password-canary")));
    assert!(!base.rsync_e().contains("ssh-password-canary"));
    assert!(!format!("{base:?}").contains("ssh-password-canary"));
    assert_eq!(base.password(), Some("ssh-password-canary"));
    assert_eq!(SSH_PASSWORD_VARIABLE, "SSHPASS");
}
