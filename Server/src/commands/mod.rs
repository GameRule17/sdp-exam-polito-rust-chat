/*
Modulo centrale dei comandi: definisce il tipo CommandResult, gestisce il dispatch dei comandi e importa tutti gli handler specifici.
*/

use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use ruggine_common::ClientToServer;

use crate::state::{State, Tx};

pub mod create_group;
pub mod global_message;
pub mod invite;
pub mod join_group;
pub mod leave_group;
pub mod list_groups;
pub mod list_users;
pub mod logout;
pub mod ping;
pub mod register;
pub mod send_message;

pub type ClientId = Option<Uuid>;

/// Risultato di un handler: opzionalmente aggiornare l'id client (per Register) e indicare se terminare la connessione
pub struct CommandResult {
    pub new_client_id: ClientId,
    pub close: bool,
}

impl CommandResult {
    pub fn continue_with(id: ClientId) -> Self {
        Self {
            new_client_id: id,
            close: false,
        }
    }
    pub fn close(id: ClientId) -> Self {
        Self {
            new_client_id: id,
            close: true,
        }
    }
}

pub async fn dispatch(
    msg: ClientToServer,
    client_id: ClientId,
    tx: &Tx,
    state: &Arc<RwLock<State>>,
) -> CommandResult {
    use ClientToServer::*;
    match msg {
        Register {
            nick,
            client_id: req_id,
        } => register::handle(nick, req_id, client_id, tx, state).await,
        CreateGroup { group } => create_group::handle(group, client_id, tx, state).await,
        Invite { group, nick } => invite::handle(group, nick, client_id, tx, state).await,
        LeaveGroup { group } => leave_group::handle(group, client_id, tx, state).await,
        JoinGroup { group, invite_code } => {
            join_group::handle(group, invite_code, client_id, tx, state).await
        }
        SendMessage { group, text, nick } => {
            send_message::handle(group, text, nick, client_id, tx, state).await
        }
        ListGroups => list_groups::handle(client_id, tx, state).await,
        ListUsers => list_users::handle(client_id, tx, state).await,
        GlobalMessage { text } => global_message::handle(text, client_id, tx, state).await,
        Logout { reason } => logout::handle(reason, client_id, tx, state).await,
        Ping => ping::handle(client_id, tx, state).await,
    }
}
