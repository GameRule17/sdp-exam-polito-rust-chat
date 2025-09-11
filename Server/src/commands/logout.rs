/*
Gestisce la disconnessione di un utente dal server. Aggiorna lo stato e rimuove l'utente da gruppi e strutture dati.
*/

use super::{ClientId, CommandResult};
use crate::state::{State, Tx};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn handle(
    reason: Option<String>,
    client_id: ClientId,
    _tx: &Tx,
    state: &Arc<RwLock<State>>,
) -> CommandResult {
    let mut st = state.write().await;
    let mut new_id = client_id;
    if let Some(id) = new_id.take() {
        let nick_opt = st.nicks_by_id.get(&id).cloned();
        if let Some(nick) = &nick_opt {
            if let Some(r) = &reason {
                if r.to_lowercase().contains("ctrl") || r.to_lowercase().contains("c") {
                    println!("{} si è disconnesso dal server (ctrl+c)", nick);
                } else {
                    println!("{} si è disconnesso dal server ({})", nick, r);
                }
            } else {
                println!("{} si è disconnesso dal server", nick);
            }
            st.users_by_nick.remove(nick);
        }
        // Rimuovi l'utente da tutti i gruppi e cancella i gruppi vuoti
        for (_name, g) in st.groups.iter_mut() {
            g.members.remove(&id);
        }
        st.groups.retain(|_, g| !g.members.is_empty());

        // Rimuovi tutti gli inviti associati al nickname dell'utente
        if let Some(nick) = &nick_opt {
            let to_remove: Vec<String> = st.invites.iter()
                .filter_map(|(code, (_group, invite_nick))| {
                    if invite_nick.eq_ignore_ascii_case(nick) { Some(code.clone()) } else { None }
                })
                .collect();
            for code in to_remove {
                st.invites.remove(&code);
            }
        }

        st.nicks_by_id.remove(&id);
        st.clients.remove(&id);
    }

    CommandResult {
        new_client_id: new_id,
        close: true,
    }
}
