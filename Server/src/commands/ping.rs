use std::sync::Arc;
use tokio::sync::RwLock;
use ruggine_common::ServerToClient;
use crate::state::{State, Tx};
use super::{ClientId, CommandResult};

pub async fn handle(client_id: ClientId, tx: &Tx, _state: &Arc<RwLock<State>>) -> CommandResult {
    let _ = tx.send(ServerToClient::Pong);
    CommandResult::continue_with(client_id)
}
