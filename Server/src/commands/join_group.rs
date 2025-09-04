/*
Gestisce la logica di ingresso in un gruppo tramite codice invito. Verifica la validità e aggiorna lo stato.
*/

use std::sync::Arc;
use tokio::sync::RwLock;
use ruggine_common::ServerToClient;
use crate::state::{State, Tx};
use super::{ClientId, CommandResult};

pub async fn handle(group: String, invite_code: String, client_id: ClientId, tx: &Tx, state: &Arc<RwLock<State>>) -> CommandResult {
    let mut st = state.write().await;

    // Non consumare il codice subito: verifica prima che il join sia valido
    let (g, allowed) = match st.invites.get(&invite_code).cloned() {
        Some(v) => v,
        None => {
            let _ = tx.send(ServerToClient::Error { reason: "Invito non valido".into() });
            return CommandResult::continue_with(client_id);
        }
    };

    if g != group {
        let _ = tx.send(ServerToClient::Error { reason: "Invito non per questo gruppo".into() });
        return CommandResult::continue_with(client_id);
    }

    let id = match client_id { Some(id) => id, None => { let _ = tx.send(ServerToClient::Error { reason: "Non registrato".into() }); return CommandResult::continue_with(client_id); } };

    let my_nick = st.nicks_by_id.get(&id).cloned().unwrap_or_default();
    if !my_nick.eq_ignore_ascii_case(&allowed) {
        let _ = tx.send(ServerToClient::Error { reason: format!("Invito destinato a {allowed}") });
        return CommandResult::continue_with(client_id);
    }

    // Se già membro del gruppo, evita duplicati e segnala l'errore all'utente
    if st
        .groups
        .get(&group)
        .map_or(false, |g| g.members.contains(&id))
    {
        let _ = tx.send(ServerToClient::Error { reason: format!("Sei già membro del gruppo {group}") });
        return CommandResult::continue_with(client_id);
    }

    // L'utente può entrare: rimuovi il codice usato e qualsiasi altro invito pendente per lo stesso (gruppo, utente)
    // in modo che eventuali vecchi codici non diventino riutilizzabili in seguito
    // Consuma il codice usato
    st.invites.remove(&invite_code);
    let to_delete: Vec<String> = st
        .invites
        .iter()
        .filter_map(|(code, (gname, nick))| {
            if gname == &group && nick.eq_ignore_ascii_case(&my_nick) { Some(code.clone()) } else { None }
        })
        .collect();
    for c in to_delete { st.invites.remove(&c); }

    st.groups.entry(group.clone()).or_default().members.insert(id);

    let _ = tx.send(ServerToClient::Joined { group });

    CommandResult::continue_with(client_id)
}
