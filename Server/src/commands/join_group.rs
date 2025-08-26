use std::sync::Arc;
use tokio::sync::RwLock;
use ruggine_common::ServerToClient;
use crate::state::{State, Tx};
use super::{ClientId, CommandResult};

pub async fn handle(group: String, invite_code: String, client_id: ClientId, tx: &Tx, state: &Arc<RwLock<State>>) -> CommandResult {
    let mut st = state.write().await;

    let (g, allowed) = match st.invites.remove(&invite_code) {
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
    if my_nick != allowed {
        let _ = tx.send(ServerToClient::Error { reason: format!("Invito destinato a {allowed}") });
        return CommandResult::continue_with(client_id);
    }

    st.groups.entry(group.clone()).or_default().members.insert(id);

    let _ = tx.send(ServerToClient::Joined { group });

    CommandResult::continue_with(client_id)
}
