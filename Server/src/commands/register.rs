/*
Gestisce la registrazione di un nuovo utente. Verifica la sintassi del nickname e l'unicità, aggiorna lo stato e invia la conferma.
*/

use super::{ClientId, CommandResult};
use crate::state::{State, Tx};
use crate::validation::validate_nick_syntax;
use ruggine_common::ServerToClient;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub async fn handle(
    nick: String,
    req_id: Uuid,
    client_id: ClientId,
    tx: &Tx,
    state: &Arc<RwLock<State>>,
) -> CommandResult {
    // Validazione sintassi lato server
    if let Err(reason) = validate_nick_syntax(&nick) {
        let _ = tx.send(ServerToClient::Registered {
            ok: false,
            reason: Some(reason),
        });
        return CommandResult::continue_with(client_id);
    }

    let mut st = state.write().await;

    // Controllo unicità case-insensitive senza hashmap aggiuntive
    let maybe_existing = st
        .users_by_nick
        .iter()
        .find(|(existing_nick, _)| existing_nick.eq_ignore_ascii_case(&nick))
        .map(|(n, id)| (n.clone(), *id));

    let (id, canonical_nick) = if let Some((existing_nick, existing_id)) = maybe_existing {
        if existing_id != req_id {
            let _ = tx.send(ServerToClient::Registered {
                ok: false,
                reason: Some(format!(
                    "Esiste già un utente con il nome '{}' (già registrato come '{}')",
                    nick, existing_nick
                )),
            });
            return CommandResult::continue_with(client_id);
        }
        // stesso client che riprova con case diverso: riusa l'ID e il nick canonico
        (existing_id, existing_nick)
    } else {
        // nuovo nick
        let id = st
            .users_by_nick
            .entry(nick.clone())
            .or_insert(req_id)
            .to_owned();
        (id, nick.clone())
    };

    st.nicks_by_id.insert(id, canonical_nick.clone());
    st.clients.insert(id, tx.clone());

    println!("{} si è connesso al server", canonical_nick);

    let _ = tx.send(ServerToClient::Registered {
        ok: true,
        reason: None,
    });

    CommandResult::continue_with(Some(id))
}
