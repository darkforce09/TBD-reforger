//! One server's machine credentials: the secrets its host agent and game runtime authenticate with.
//!
//! **Role:** declares the credential sheet the server card opens, and holds its state — the list
//! read for the selected server, the in-flight flag, and the one secret an issue answered with —
//! with the reads and the two changes: issue and revoke.
//! **Position:** a side sheet over the server control screen, opened from the selected server's
//! header.
//! **Signals & state:** [`CredentialPanel`] is one copyable handle, created by the server card for
//! the server it shows, reading that server's name from the card so a renamed server is named at
//! once. The issued secret lives in one signal, and nowhere else: it is cleared when
//! the operator says it is stored and whenever the sheet closes, and it is never written to storage
//! or a log.
//! **Invariants:** the secret is shown exactly once, because the backend never shows it again; the
//! list shows every credential's provenance, use and revocation but never a secret. A revocation
//! needs a reason, which the audit trail keeps, and revoking one credential leaves the server's
//! others untouched. Every request is browser-only; a native build keeps the list idle.

mod credential_sheet;
mod credential_text;

#[cfg(target_arch = "wasm32")]
pub(super) use credential_sheet::credential_sheet;
#[cfg(any(target_arch = "wasm32", test))]
pub(super) use credential_text::executor_label;

#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::MachineCredential;
#[cfg(target_arch = "wasm32")]
use frontend_session::AuthStore;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The credential list as read: not yet, in flight, refused with a reason, or answered.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, PartialEq)]
pub(super) enum CredentialList {
    Idle,
    Loading,
    Failed(String),
    Loaded(Vec<MachineCredential>),
}

/// A secret an issue answered with, and the credential it belongs to.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, PartialEq)]
pub(super) struct IssuedSecret {
    pub(super) label: String,
    pub(super) executor_kind: String,
    pub(super) secret: String,
}

/// Every signal the credential sheet runs on.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(super) struct CredentialPanel {
    pub(super) store: AuthStore,
    pub(super) server_id: StoredValue<String>,
    /// The server's name as the card reads it from the registry.
    pub(super) server_name: Signal<String>,
    pub(super) open: RwSignal<bool>,
    pub(super) list: RwSignal<CredentialList>,
    pub(super) busy: RwSignal<bool>,
    /// The one secret on screen, until the operator says it is stored or the sheet closes.
    pub(super) issued: RwSignal<Option<IssuedSecret>>,
}

#[cfg(target_arch = "wasm32")]
impl CredentialPanel {
    /// A shut sheet for one server, owned by the server card that creates it.
    pub(super) fn new(store: AuthStore, server_id: String, server_name: Signal<String>) -> Self {
        let panel = Self {
            store,
            server_id: StoredValue::new(server_id),
            server_name,
            open: RwSignal::new(false),
            list: RwSignal::new(CredentialList::Idle),
            busy: RwSignal::new(false),
            issued: RwSignal::new(None),
        };
        // A closed sheet holds no secret.
        Effect::new(move |_| {
            if !panel.open.get() {
                panel.issued.set(None);
            }
        });
        panel
    }

    /// Open the sheet and read the list.
    pub(super) fn open_sheet(self) {
        self.open.set(true);
        self.reload();
    }

    /// Read the list again.
    pub(super) fn reload(self) {
        self.list.set(CredentialList::Loading);
        leptos::task::spawn_local(async move {
            use frontend_transport::endpoints::machine_credentials::load_machine_credentials;
            let read =
                load_machine_credentials(self.store, &self.server_id.get_value().into()).await;
            self.list.set(match read {
                Ok(list) => CredentialList::Loaded(list.items),
                Err(e) => CredentialList::Failed(e.message_or("Could not load the credentials")),
            });
        });
    }

    /// Issue a credential; its secret is shown once, and the list is read again.
    pub(super) fn issue(self, issue: frontend_api_dtos::MachineCredentialIssue) {
        use frontend_transport::endpoints::machine_credentials::issue_machine_credential;
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        let toasts = frontend_ui::toast::use_toasts();
        leptos::task::spawn_local(async move {
            match issue_machine_credential(self.store, &self.server_id.get_value().into(), &issue)
                .await
            {
                Ok(issued) => {
                    self.issued.set(Some(IssuedSecret {
                        label: issued.credential.label.clone(),
                        executor_kind: issued.credential.executor_kind.clone(),
                        secret: issued.secret,
                    }));
                    toasts.success(format!("Issued {}", issued.credential.label));
                    self.reload();
                }
                Err(refusal) => toasts.error(refusal.message_or("Could not issue the credential")),
            }
            self.busy.set(false);
        });
    }

    /// Revoke one credential with the reason given; `on_revoked` runs once it is revoked.
    pub(super) fn revoke(
        self,
        credential: String,
        reason: String,
        on_revoked: impl FnOnce() + 'static,
    ) {
        use frontend_transport::endpoints::machine_credentials::revoke_machine_credential;
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        let toasts = frontend_ui::toast::use_toasts();
        leptos::task::spawn_local(async move {
            let server = self.server_id.get_value();
            match revoke_machine_credential(
                self.store,
                &server.as_str().into(),
                &credential.as_str().into(),
                &reason,
            )
            .await
            {
                Ok(revoked) => {
                    toasts.success(format!("Revoked {}", revoked.label));
                    on_revoked();
                    self.reload();
                }
                Err(refusal) => toasts.error(refusal.message_or("Could not revoke the credential")),
            }
            self.busy.set(false);
        });
    }
}

#[cfg(test)]
#[path = "tests/machine_credentials.rs"]
mod tests;
