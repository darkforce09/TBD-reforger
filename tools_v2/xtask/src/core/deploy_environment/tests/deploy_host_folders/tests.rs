use super::*;
use std::path::Path;

const PATH: &str = "/home/deploy/checkout/tools_v2/xtask/deploy/deploy.env";

fn environment(file: &str) -> DeployEnvironment {
    DeployEnvironment::from_text(Path::new(PATH), Some(file), []).expect("parses")
}

const ALL: [DeployHostFolder; 4] = [
    DeployHostFolder::Checkout,
    DeployHostFolder::Profile,
    DeployHostFolder::AddonsStaging,
    DeployHostFolder::ServerInstall,
];

#[test]
fn an_explicit_folder_wins_over_the_default() {
    let settings = environment("TBD_PROFILE_DIR=/srv/reforger/profile\n");
    let host = DeployHost::parse("deploy@192.0.2.10").expect("parses");
    assert_eq!(
        DeployHostFolder::Profile.resolve(&settings, &host),
        Ok("/srv/reforger/profile".to_string())
    );
}

#[test]
fn unset_folders_default_under_the_deploy_users_home() {
    let settings = environment("TBD_REMOTE_DIR=\n");
    let host = DeployHost::parse("deploy@192.0.2.10").expect("parses");
    let resolved: Vec<String> = ALL
        .iter()
        .map(|folder| folder.resolve(&settings, &host).expect("defaults"))
        .collect();
    assert_eq!(
        resolved,
        [
            "/home/deploy/tbd/repo",
            "/home/deploy/tbd/profile",
            "/home/deploy/tbd/addons-staging",
            "/home/deploy/steam/arma-reforger-server",
        ]
    );
}

#[test]
fn with_no_user_every_unset_folder_is_required() {
    let settings = environment("TBD_SERVER_DIR=/opt/reforger\n");
    let host = DeployHost::parse("192.0.2.10").expect("parses");
    for folder in ALL {
        let resolved = folder.resolve(&settings, &host);
        if folder == DeployHostFolder::ServerInstall {
            assert_eq!(resolved, Ok("/opt/reforger".to_string()));
        } else {
            assert_eq!(resolved, Err(settings.missing(folder.key())), "{folder:?}");
        }
    }
}
