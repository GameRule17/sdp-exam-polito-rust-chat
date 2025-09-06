/*
Gestisce il comando di ping inviando una risposta pong al client.
*/

use super::{ClientId, CommandResult};
use crate::state::{State, Tx};
use ruggine_common::ServerToClient;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn handle(client_id: ClientId, tx: &Tx, _state: &Arc<RwLock<State>>) -> CommandResult {
    let _ = tx.send(ServerToClient::Pong);
    CommandResult::continue_with(client_id)
}
