/*
Gestisce l'invio di messaggi a un gruppo. Verifica i permessi e inoltra il messaggio ai membri del gruppo.
*/

use super::{ClientId, CommandResult};
use crate::state::{State, Tx};
use ruggine_common::ServerToClient;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn handle(
    group: String,
    text: String,
    nick: String,
    client_id: ClientId,
    tx: &Tx,
    state: &Arc<RwLock<State>>,
) -> CommandResult {
    let st = state.read().await;

    let sender_id = st.users_by_nick.get(&nick).cloned();
    if let Some(sender_id) = sender_id {
        if !st
            .groups
            .get(&group)
            .map_or(false, |g| g.members.contains(&sender_id))
        {
            let _ = tx.send(ServerToClient::Error {
                reason: format!("Non sei membro di questo gruppo {group}"),
            });
            return CommandResult::continue_with(client_id);
        }
    } else {
        let _ = tx.send(ServerToClient::Error {
            reason: "ID del mittente invalido".into(),
        });
        return CommandResult::continue_with(client_id);
    }

    let id = match client_id {
        Some(id) => id,
        None => {
            let _ = tx.send(ServerToClient::Error {
                reason: "Non registrato".into(),
            });
            return CommandResult::continue_with(client_id);
        }
    };

    let my_nick = st
        .nicks_by_id
        .get(&id)
        .cloned()
        .unwrap_or_else(|| "???".into());

    if let Some(g) = st.groups.get(&group) {
        for member in &g.members {
            if member == &id {
                continue;
            } // non inviare a se stessi
            if let Some(txm) = st.clients.get(member) {
                let _ = txm.send(ServerToClient::Message {
                    group: group.clone(),
                    from: my_nick.clone(),
                    text: text.clone(),
                });
            }
        }
    } else {
        let _ = tx.send(ServerToClient::Error {
            reason: format!("Gruppo {group} inesistente"),
        });
    }

    CommandResult::continue_with(client_id)
}
