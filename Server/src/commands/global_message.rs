use std::sync::Arc;
use tokio::sync::RwLock;
use ruggine_common::ServerToClient;
use crate::state::{State, Tx};
use super::{ClientId, CommandResult};

pub async fn handle(text: String, client_id: ClientId, tx: &Tx, state: &Arc<RwLock<State>>) -> CommandResult {
    let st = state.read().await;
    let id = match client_id { Some(id) => id, None => { let _ = tx.send(ServerToClient::Error { reason: "Non registrato".into() }); return CommandResult::continue_with(client_id); } };
    let my_nick = st.nicks_by_id.get(&id).cloned().unwrap_or_else(|| "???".into());

    for (client_id, txm) in &st.clients {
        if *client_id != id {
            let _ = txm.send(ServerToClient::GlobalMessage { from: my_nick.clone(), text: text.clone() });
        }
    }

    CommandResult::continue_with(client_id)
}
