use std::sync::Arc;
use tokio::sync::RwLock;
use ruggine_common::ServerToClient;
use crate::validation::validate_group_name_syntax;
use crate::state::{State, Tx};
use super::{ClientId, CommandResult};

pub async fn handle(group: String, client_id: ClientId, tx: &Tx, state: &Arc<RwLock<State>>) -> CommandResult {
    let mut st = state.write().await;
    let id = match client_id { Some(id) => id, None => { let _ = tx.send(ServerToClient::Error { reason: "Non registrato".into() }); return CommandResult::continue_with(client_id); } };

    if let Err(reason) = validate_group_name_syntax(&group) {
        let _ = tx.send(ServerToClient::Error { reason });
        return CommandResult::continue_with(client_id);
    }

    // Controllo case-insensitive per i gruppi
    let maybe_existing_group = st
        .groups
        .keys()
        .find(|existing_group| existing_group.eq_ignore_ascii_case(&group))
        .cloned();

    if let Some(existing_group) = maybe_existing_group {
        let _ = tx.send(ServerToClient::Error { reason: format!(
            "Esiste già un gruppo con il nome '{}' (già registrato come '{}')",
            group, existing_group
        )});
        return CommandResult::continue_with(client_id);
    }

    if st.users_by_nick.get(&group).is_some() {
        let _ = tx.send(ServerToClient::Error { reason: format!("Il nome '{group}' è già usato da un utente") });
        return CommandResult::continue_with(client_id);
    }
    let g = st.groups.entry(group.clone()).or_default();
    g.members.insert(id);
    // Conferma creazione gruppo
    let _ = tx.send(ServerToClient::GroupCreated { group });

    CommandResult::continue_with(client_id)
}
