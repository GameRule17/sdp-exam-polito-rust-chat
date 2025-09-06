/*
Gestisce la logica di uscita da un gruppo. Rimuove l'utente e cancella il gruppo se vuoto.
*/

use super::{ClientId, CommandResult};
use crate::state::{State, Tx};
use ruggine_common::ServerToClient;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn handle(
    group: String,
    client_id: ClientId,
    tx: &Tx,
    state: &Arc<RwLock<State>>,
) -> CommandResult {
    let mut st = state.write().await;
    let id = match client_id {
        Some(id) => id,
        None => {
            let _ = tx.send(ServerToClient::Error {
                reason: "Non registrato".into(),
            });
            return CommandResult::continue_with(client_id);
        }
    };

    match st.groups.get_mut(&group) {
        Some(g) => {
            if !g.members.remove(&id) {
                let _ = tx.send(ServerToClient::Error {
                    reason: format!("Non sei membro del gruppo {group}"),
                });
                return CommandResult::continue_with(client_id);
            }
            if g.members.is_empty() {
                st.groups.remove(&group);
            }
            let _ = tx.send(ServerToClient::Left { group });
        }
        None => {
            let _ = tx.send(ServerToClient::Error {
                reason: format!("Gruppo {group} inesistente"),
            });
            return CommandResult::continue_with(client_id);
        }
    }

    CommandResult::continue_with(client_id)
}
