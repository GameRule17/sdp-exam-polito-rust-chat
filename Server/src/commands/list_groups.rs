/*
Restituisce la lista dei gruppi di cui l'utente è membro. Segnala errore se non appartiene a nessun gruppo.
*/

use super::{ClientId, CommandResult};
use crate::state::{State, Tx};
use ruggine_common::ServerToClient;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn handle(client_id: ClientId, tx: &Tx, state: &Arc<RwLock<State>>) -> CommandResult {
    let st = state.read().await;

    let id = match client_id {
        Some(id) => id,
        None => {
            let _ = tx.send(ServerToClient::Error {
                reason: "Non registrato".into(),
            });
            return CommandResult::continue_with(client_id);
        }
    };

    let groups: Vec<String> = st
        .groups
        .iter()
        .filter(|(_, gr)| gr.members.contains(&id))
        .map(|(name, _)| name.clone())
        .collect();

    if groups.is_empty() {
        let _ = tx.send(ServerToClient::Error {
            reason: "Nessun gruppo di appartenenza".into(),
        });
        return CommandResult::continue_with(client_id);
    }

    let _ = tx.send(ServerToClient::Groups { groups });

    CommandResult::continue_with(client_id)
}
