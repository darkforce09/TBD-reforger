//! The judges of the credential rotations W11 and W12: which credential was staged beside which
//! live one, whether the revocation named the live one, and whether the promoted credential
//! authenticates or opens a newer runtime session while the revoked one authenticates nothing.
//!
//! **Role:** [`Rotation`], the measurement keys its steps share, and the pure judges of its
//! credential reads.
//!
//! **Position:** used by `credential_waves.rs`, whose probes read the rows these judges parse
//! (`single_server_reads.rs`' credentials of one server and executor).
//!
//! **Signals & state:** none; pure functions over the rows and the step context.
//!
//! **Invariants:** a rotation stages exactly one credential beside exactly one live one and
//! measures both ids; a later step without those measurements is contradicted, never guessed;
//! a revoked credential that authenticated after its revocation contradicts every later judge.

use serde_json::Value;

use super::single_server_probes::step_measurement;
use super::wave_table::SingleServerStep;
use crate::fleet_procedure::fleet_reads::since_ms;
use crate::fleet_procedure::single_server_reads::CredentialRow;
use crate::procedure_runner::step::{ProbeVerdict, StepContext};
use crate::remote_actions::host_fixture_commands::CredentialExecutor;

/// One rotation: its executor, its three wave-table rows and the case they decide.
pub(super) struct Rotation {
    pub executor: CredentialExecutor,
    pub stage: &'static SingleServerStep,
    pub revoke: &'static SingleServerStep,
    pub promote: &'static SingleServerStep,
    pub case: &'static str,
}

impl Rotation {
    pub(super) fn staged_key(&self) -> String {
        step_measurement(self.stage.step_id, "staged_credential")
    }

    pub(super) fn previous_key(&self) -> String {
        step_measurement(self.stage.step_id, "previous_credential")
    }

    pub(super) fn revoked_generation_key(&self) -> String {
        step_measurement(self.revoke.step_id, "revoked_generation")
    }
}

/// The credential `id` among `rows`.
fn credential<'a>(rows: &'a [CredentialRow], id: &str) -> Option<&'a CredentialRow> {
    rows.iter().find(|row| row.credential_id == id)
}

/// The staged and the previous credential ids the stage step measured.
fn rotation_ids(context: &StepContext<'_>, rotation: &Rotation) -> Option<(String, String)> {
    let text = |key: String| {
        context
            .measurements
            .get(&key)
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    Some((text(rotation.staged_key())?, text(rotation.previous_key())?))
}

/// Holds when exactly one unrevoked credential was issued since the step began beside exactly
/// one credential live before it; measures both ids.
pub(super) fn staged(
    rows: &[CredentialRow],
    context: &StepContext<'_>,
    rotation: &Rotation,
) -> ProbeVerdict {
    let since = since_ms(context);
    let issued: Vec<&CredentialRow> = rows.iter().filter(|row| row.created_ms >= since).collect();
    let live: Vec<&CredentialRow> = rows
        .iter()
        .filter(|row| row.created_ms < since && row.revoked_ms.is_none())
        .collect();
    let (new, old) = match (issued.as_slice(), live.as_slice()) {
        ([], _) => {
            return ProbeVerdict::Pending("no credential issued since the step began".into());
        }
        ([new], [old]) if new.revoked_ms.is_none() => (new, old),
        ([new], [_]) => {
            return ProbeVerdict::Contradicted(format!(
                "the staged credential {} is already revoked",
                new.credential_id
            ));
        }
        (issued, live) => {
            return ProbeVerdict::Contradicted(format!(
                "a rotation stages one credential beside one live one, but {} were issued since \
                 the step began and {} were live before it",
                issued.len(),
                live.len()
            ));
        }
    };
    ProbeVerdict::Satisfied(
        ProbeVerdict::satisfied(format!(
            "credential {} staged at {} beside the live credential {}",
            new.credential_id, new.created_ms, old.credential_id
        ))
        .at(new.created_ms)
        .measure(rotation.staged_key(), new.credential_id.clone())
        .measure(rotation.previous_key(), old.credential_id.clone()),
    )
}

/// Holds when the previous credential was revoked since the step began; the staged one revoked
/// instead contradicts it.
pub(super) fn previous_revoked(
    rows: &[CredentialRow],
    context: &StepContext<'_>,
    rotation: &Rotation,
) -> ProbeVerdict {
    let Some((staged, previous)) = rotation_ids(context, rotation) else {
        return ProbeVerdict::Contradicted(
            "the stage step measured no credentials, so the revocation cannot be told apart".into(),
        );
    };
    if credential(rows, &staged).is_some_and(|row| row.revoked_ms.is_some()) {
        return ProbeVerdict::Contradicted(format!(
            "the staged credential {staged} was revoked instead of {previous}"
        ));
    }
    match credential(rows, &previous).map(|row| row.revoked_ms) {
        None => ProbeVerdict::Contradicted(format!("the previous credential {previous} is gone")),
        Some(None) => ProbeVerdict::Pending(format!("credential {previous} is not revoked yet")),
        Some(Some(revoked)) if revoked >= since_ms(context) => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!("credential {previous} revoked at {revoked}"))
                .at(revoked),
        ),
        Some(Some(revoked)) => ProbeVerdict::Contradicted(format!(
            "credential {previous} was revoked at {revoked}, before the step began"
        )),
    }
}

/// The contradiction when the revoked credential authenticated after its revocation.
fn revoked_credential_used(rows: &[CredentialRow], previous: &str) -> Option<ProbeVerdict> {
    let row = credential(rows, previous)?;
    match (row.last_used_ms, row.revoked_ms) {
        (Some(used), Some(revoked)) if used > revoked => Some(ProbeVerdict::Contradicted(format!(
            "the revoked credential {previous} authenticated at {used}, after its revocation at \
             {revoked}"
        ))),
        _ => None,
    }
}

/// Holds when the promoted credential authenticated since the step began and the revoked one
/// nothing after its revocation.
pub(super) fn new_credential_used(
    rows: &[CredentialRow],
    context: &StepContext<'_>,
    rotation: &Rotation,
) -> ProbeVerdict {
    let Some((staged, previous)) = rotation_ids(context, rotation) else {
        return ProbeVerdict::Contradicted("the stage step measured no credentials".into());
    };
    if let Some(verdict) = revoked_credential_used(rows, &previous) {
        return verdict;
    }
    let Some(row) = credential(rows, &staged) else {
        return ProbeVerdict::Contradicted(format!("the staged credential {staged} is gone"));
    };
    match (row.revoked_ms, row.last_used_ms) {
        (Some(revoked), _) => ProbeVerdict::Contradicted(format!(
            "the promoted credential {staged} was revoked at {revoked}"
        )),
        (None, None) => ProbeVerdict::Pending(format!("credential {staged} has not authenticated")),
        (None, Some(used)) if used >= since_ms(context) => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!(
                "the promoted credential {staged} authenticated at {used}; the revoked {previous} \
                 authenticated nothing after its revocation"
            ))
            .at(used),
        ),
        (None, Some(used)) => ProbeVerdict::Contradicted(format!(
            "credential {staged} authenticated at {used}, before its promotion"
        )),
    }
}

/// Holds when the previous credential's newest runtime session ended `credential_revoked`;
/// measures that generation.
pub(super) fn session_revoked(
    rows: &[CredentialRow],
    context: &StepContext<'_>,
    rotation: &Rotation,
) -> ProbeVerdict {
    let Some((_, previous)) = rotation_ids(context, rotation) else {
        return ProbeVerdict::Contradicted("the stage step measured no credentials".into());
    };
    let Some(row) = credential(rows, &previous) else {
        return ProbeVerdict::Contradicted(format!("the previous credential {previous} is gone"));
    };
    let Some(session) = row.sessions.iter().max_by_key(|session| session.generation) else {
        return ProbeVerdict::Contradicted(format!(
            "credential {previous} opened no runtime session, so its revocation ended none"
        ));
    };
    match session.end_reason.as_deref() {
        None => ProbeVerdict::Pending(format!("generation {} is still open", session.generation)),
        Some("credential_revoked") => {
            let satisfaction = ProbeVerdict::satisfied(format!(
                "generation {} (started {:?}, last heartbeat {:?}) ended credential_revoked",
                session.generation, session.started_ms, session.heartbeat_ms
            ))
            .measure(rotation.revoked_generation_key(), session.generation);
            ProbeVerdict::Satisfied(match row.revoked_ms {
                Some(revoked) => satisfaction.at(revoked),
                None => satisfaction,
            })
        }
        Some(reason) => ProbeVerdict::Contradicted(format!(
            "the revoked credential's newest session, generation {}, ended {reason}, not \
             credential_revoked",
            session.generation
        )),
    }
}

/// Holds when a session the promoted credential opened since the step began, newer than the
/// revoked generation, heartbeats and is open.
pub(super) fn new_generation(
    rows: &[CredentialRow],
    context: &StepContext<'_>,
    rotation: &Rotation,
) -> ProbeVerdict {
    let Some((staged, previous)) = rotation_ids(context, rotation) else {
        return ProbeVerdict::Contradicted("the stage step measured no credentials".into());
    };
    if let Some(verdict) = revoked_credential_used(rows, &previous) {
        return verdict;
    }
    let Some(revoked_generation) = context
        .measurements
        .get(&rotation.revoked_generation_key())
        .and_then(Value::as_u64)
    else {
        return ProbeVerdict::Contradicted(
            "the revocation measured no ended generation, so a newer one cannot be told apart"
                .into(),
        );
    };
    let since = since_ms(context);
    let newest = credential(rows, &staged).and_then(|row| {
        row.sessions
            .iter()
            .filter(|session| session.started_ms.is_some_and(|started| started >= since))
            .max_by_key(|session| session.generation)
    });
    let Some(session) = newest else {
        return ProbeVerdict::Pending(format!(
            "credential {staged} opened no runtime session since the step began"
        ));
    };
    if session.generation <= revoked_generation {
        return ProbeVerdict::Contradicted(format!(
            "generation {} is not newer than the revoked generation {revoked_generation}",
            session.generation
        ));
    }
    match (session.heartbeat_ms, session.end_reason.as_deref()) {
        (Some(heartbeat), None) => ProbeVerdict::Satisfied(
            ProbeVerdict::satisfied(format!(
                "generation {} opened with credential {staged} heartbeats (revoked generation \
                 {revoked_generation})",
                session.generation
            ))
            .at(heartbeat),
        ),
        (_, Some(reason)) => {
            ProbeVerdict::Pending(format!("generation {} ended {reason}", session.generation))
        }
        (None, None) => ProbeVerdict::Pending(format!(
            "generation {} has not heartbeated yet",
            session.generation
        )),
    }
}
