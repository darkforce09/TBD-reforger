//! The callers a route acceptance world authenticates as.
//!
//! **Role:** creates, per world, one account of every role, a second (peer) account of every
//! role, an administrator banned after its token was issued, two servers with live, revoked and
//! cross-server machine credentials, and resolves an [`Actor`] to its bearer.
//!
//! **Position:** test support; built by [`super::world`] through `tests/common` (`access_token`)
//! and the production credential service; read by the dimension runner and the part worlds.
//!
//! **Signals & state:** the accounts, servers and credentials are rows of the binary's
//! database; this module keeps their ids and secrets.
//!
//! **Invariants:** every account id is an 18-digit string unique to the world's namespace, so
//! worlds of one binary never share a row; the banned account's token is minted before the ban,
//! so its refusal proves the ban revokes live sessions; every machine secret comes from
//! `issue_machine_credential`, stored hashed exactly as the issue route stores it.

use api_identifiers::DiscordUserId;
use std::sync::atomic::{AtomicU32, Ordering};

use api_server_infrastructure::services::machine_credentials::{
    issue_machine_credential, revoke_machine_credential,
};
use api_state::AppState;
use fleet_wire_contract::ExecutorKind;
use uuid::Uuid;

use super::spec::{Actor, Executor, Role};
use crate::common;

/// An account with a live session token.
#[derive(Debug, Clone)]
pub struct UserActor {
    pub discord_id: String,
    pub role: Role,
    pub token: String,
}

/// The machine side of a world: its server, another server, and their credentials.
#[derive(Debug, Clone)]
pub struct MachineActors {
    /// The server every live and revoked credential of [`Actor::Machine`] belongs to.
    pub server: Uuid,
    /// The server of [`Actor::MachineOtherServer`]'s credentials.
    pub other_server: Uuid,
    secrets: Vec<(Actor, String)>,
}

/// Every caller of one world.
#[derive(Debug, Clone)]
pub struct Actors {
    /// The world's namespace (0–99), the middle digits of every account id.
    pub namespace: u8,
    suite: &'static str,
    users: Vec<UserActor>,
    peers: Vec<UserActor>,
    /// The administrator banned after its token was issued.
    pub banned: UserActor,
    pub machines: MachineActors,
    observability_token: String,
    fresh: std::sync::Arc<AtomicU32>,
}

fn account_id(namespace: u8, slot: u32) -> String {
    format!("97{namespace:02}{slot:014}")
}

fn executor_kind(executor: Executor) -> ExecutorKind {
    match executor {
        Executor::ModRuntime => ExecutorKind::ModRuntime,
        Executor::HostAgent => ExecutorKind::HostAgent,
    }
}

async fn user(state: &AppState, suite: &'static str, id: String, role: Role) -> UserActor {
    let token = common::access_token(state, suite, &id, role.as_str(), true).await;
    UserActor {
        discord_id: id,
        role,
        token,
    }
}

async fn register_server(state: &AppState, name: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active) VALUES ($1, '127.0.0.1'::inet, 2302, true) \
         RETURNING id",
    )
    .bind(name)
    .fetch_one(&state.pool)
    .await
    .unwrap_or_else(|error| panic!("register route acceptance server {name}: {error}"))
}

async fn credential(
    state: &AppState,
    server: Uuid,
    executor: Executor,
    author: &str,
    revoked: bool,
) -> String {
    let mut transaction = state.pool.begin().await.expect("begin credential fixture");
    let issued = issue_machine_credential(
        &mut transaction,
        server.into(),
        executor_kind(executor),
        "Route acceptance",
        &DiscordUserId::new(author),
    )
    .await
    .unwrap_or_else(|error| panic!("issue route acceptance credential: {error:?}"));
    if revoked {
        revoke_machine_credential(
            &mut transaction,
            server.into(),
            issued.credential.id,
            &DiscordUserId::new(author),
            "route acceptance revoked credential",
        )
        .await
        .unwrap_or_else(|error| panic!("revoke route acceptance credential: {error:?}"));
    }
    transaction
        .commit()
        .await
        .expect("commit credential fixture");
    issued.secret
}

impl Actors {
    /// Create every caller of the world `namespace` in the binary's database.
    pub async fn create(state: &AppState, suite: &'static str, namespace: u8) -> Actors {
        let mut users = Vec::new();
        let mut peers = Vec::new();
        for (slot, role) in (10u32..).zip(Role::ALL) {
            users.push(user(state, suite, account_id(namespace, slot), role).await);
            peers.push(user(state, suite, account_id(namespace, slot + 10), role).await);
        }
        let banned = user(state, suite, account_id(namespace, 30), Role::Admin).await;
        sqlx::query("UPDATE users SET is_banned = true, ban_reason = 'route acceptance' WHERE discord_id = $1")
            .bind(&banned.discord_id)
            .execute(&state.pool)
            .await
            .expect("ban the route acceptance account");
        let author = users.last().expect("an administrator").discord_id.clone();
        let server = register_server(state, &format!("Route acceptance {namespace}")).await;
        let other_server =
            register_server(state, &format!("Route acceptance other {namespace}")).await;
        let mut secrets = Vec::new();
        for executor in [Executor::ModRuntime, Executor::HostAgent] {
            let live = credential(state, server, executor, &author, false).await;
            secrets.push((Actor::Machine(executor), live));
            let other = credential(state, other_server, executor, &author, false).await;
            secrets.push((Actor::MachineOtherServer(executor), other));
            let revoked = credential(state, server, executor, &author, true).await;
            secrets.push((Actor::MachineRevoked(executor), revoked));
        }
        Actors {
            namespace,
            suite,
            users,
            peers,
            banned,
            machines: MachineActors {
                server,
                other_server,
                secrets,
            },
            observability_token: state.cfg.observability_token.clone(),
            fresh: std::sync::Arc::new(AtomicU32::new(1000)),
        }
    }

    /// The world's account of `role`.
    pub fn user(&self, role: Role) -> &UserActor {
        &self.users[Role::ALL
            .iter()
            .position(|r| *r == role)
            .expect("every role")]
    }

    /// The world's second account of `role`.
    pub fn peer(&self, role: Role) -> &UserActor {
        &self.peers[Role::ALL
            .iter()
            .position(|r| *r == role)
            .expect("every role")]
    }

    /// The account behind `actor`, when it is one.
    pub fn account(&self, actor: Actor) -> Option<&UserActor> {
        match actor {
            Actor::User(role) => Some(self.user(role)),
            Actor::Peer(role) => Some(self.peer(role)),
            Actor::Banned => Some(&self.banned),
            _ => None,
        }
    }

    /// The `Authorization` bearer `actor` presents, or `None` for no header.
    pub fn bearer(&self, actor: Actor) -> Option<String> {
        match actor {
            Actor::Anonymous => None,
            Actor::InvalidBearer => Some("not-a-token".into()),
            Actor::User(_) | Actor::Peer(_) | Actor::Banned => {
                self.account(actor).map(|account| account.token.clone())
            }
            Actor::MachineMalformed => Some("tbdm_not-a-credential".into()),
            Actor::Observability => Some(self.observability_token.clone()),
            Actor::WrongObservability => Some(format!("{}-wrong", self.observability_token)),
            Actor::Machine(_) | Actor::MachineOtherServer(_) | Actor::MachineRevoked(_) => self
                .machines
                .secrets
                .iter()
                .find(|(owner, _)| *owner == actor)
                .map(|(_, secret)| secret.clone()),
        }
    }

    /// A new account of `role` in this world's namespace with a live session; `arma_linked`
    /// gives it an initial Arma identity.
    pub async fn fresh_user(&self, state: &AppState, role: Role, arma_linked: bool) -> UserActor {
        let slot = self.fresh.fetch_add(1, Ordering::Relaxed);
        let id = account_id(self.namespace, slot);
        let token = common::access_token(state, self.suite, &id, role.as_str(), arma_linked).await;
        UserActor {
            discord_id: id,
            role,
            token,
        }
    }
}
