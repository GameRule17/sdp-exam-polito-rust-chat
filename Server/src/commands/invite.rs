use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;
use ruggine_common::ServerToClient;
use crate::{util::short_code, state::{State, Tx}};
use super::{ClientId, CommandResult};

pub async fn handle(group: String, nick: String, _client_id: ClientId, tx: &Tx, state: &Arc<RwLock<State>>) -> CommandResult {
    let mut st = state.write().await;
    if !st.groups.contains_key(&group) {
        let _ = tx.send(ServerToClient::Error { reason: format!("Gruppo {group} inesistente") });
        return CommandResult::continue_with(_client_id);
    }
    // lookup utente destinatario case-insensitive
    let id_user = st
        .users_by_nick
        .iter()
        .find(|(existing_nick, _)| existing_nick.eq_ignore_ascii_case(&nick))
        .map(|(_, id)| *id);

    if (st.users_by_nick.get(&nick).is_none()) || (id_user.is_none()) {
        let _ = tx.send(ServerToClient::Error { reason: format!("Utente {nick} inesistente") });
        return CommandResult::continue_with(_client_id);
    }

    if st
        .groups
        .get(&group)
        .map_or(false, |g| g.members.contains(&id_user.unwrap()))
    {
        let _ = tx.send(ServerToClient::Error { reason: format!("Utente {nick} già membro del gruppo {group}") });
        return CommandResult::continue_with(_client_id);
    }

    let code = short_code();
    st.invites.insert(code.clone(), (group.clone(), nick.clone()));

    // invia il codice di invito al client destinatario
    if let Some(id) = id_user {
        if let Some(txm) = st.clients.get(&id) {
            let _ = txm.send(ServerToClient::InviteCode {
                group: group.clone(),
                code: code.clone(),
                client_id: _client_id
                    .and_then(|id| st.nicks_by_id.get(&id).cloned())
                    .unwrap_or_default(),
            });
        } else {
            warn!("Client {} non trovato per invio codice invito", id);
        }
    }

    let _ = tx.send(ServerToClient::MessageServer {
        text: format!("Utente {} invitato correttamente al gruppo {}", nick, group),
    });

    CommandResult::continue_with(_client_id)
}
