/*
Restituisce la lista degli utenti connessi, evidenziando il richiedente come "(tu)".
*/

use std::sync::Arc;
use tokio::sync::RwLock;
use ruggine_common::ServerToClient;
use crate::state::{State, Tx};
use super::{ClientId, CommandResult};

pub async fn handle(client_id: ClientId, tx: &Tx, state: &Arc<RwLock<State>>) -> CommandResult {
    let st = state.read().await;
    let id = match client_id { Some(id) => id, None => { let _ = tx.send(ServerToClient::Error { reason: "Non registrato".into() }); return CommandResult::continue_with(client_id); } };

    // Metti il richiedente come primo elemento marcato " (tu)" e ordina alfabeticamente gli altri
    let mut others: Vec<String> = st
        .nicks_by_id
        .iter()
        .filter_map(|(uid, nick)| if uid == &id { None } else { Some(nick.clone()) })
        .collect();
    others.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

    let me = st.nicks_by_id.get(&id).cloned().unwrap_or_default();
    let mut users: Vec<String> = Vec::with_capacity(1 + others.len());
    users.push(format!("{} (tu)", me));
    users.extend(others);

    let _ = tx.send(ServerToClient::ListUsers { users });

    CommandResult::continue_with(client_id)
}
