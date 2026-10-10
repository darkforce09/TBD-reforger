//! One read-only call against a real `ttm` binary, run only when `TBD_TTM_BIN` names one: the
//! version document parses and names contract version 1. Without the variable the test prints a
//! skip notice and passes, so the suite never depends on a binary outside the workspace.

use crate::{BINARY_VARIABLE, TicketManager};

#[test]
fn a_real_ticket_manager_answers_its_version() {
    if std::env::var(BINARY_VARIABLE).map_or(true, |value| value.trim().is_empty()) {
        println!("skipped: {BINARY_VARIABLE} is unset, so no ttm binary is exercised");
        return;
    }
    let version = TicketManager::from_env()
        .version()
        .expect("the ticket manager answers `version`");
    assert_eq!(version.json_version, 1);
}
