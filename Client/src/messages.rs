/*
Modulo Messages: si occupa di formattare e visualizzare i messaggi ricevuti dal server.
Traduce le strutture ServerToClient in stringhe leggibili per l'utente.
*/

use ruggine_common::ServerToClient;

pub fn render(msg: ServerToClient) -> String {
    match msg {
        ServerToClient::Registered { ok, reason } => {
            format!("[server] registrazione: ok={} {:?}", ok, reason)
        }
        ServerToClient::InviteCode { group, code, client_id } => format!(
            "[server] codice invito per il gruppo '{}': {} da {}",
            group, code, client_id
        ),
        ServerToClient::InviteCodeForMe { group, code } => format!(
            "[server] codice invito per il gruppo '{}': {}",
            group, code
        ),
        ServerToClient::Joined { group } => {
            format!("[server] sei entrato nel gruppo '{}'", group)
        }
        ServerToClient::Left { group } => {
            format!("[server] sei uscito dal gruppo '{}'", group)
        }
        ServerToClient::Message { group, from, text } => {
            format!("[{}] <{}> {}", group, from, text)
        }
        ServerToClient::MessageServer { text } => format!("[server] {}", text),
        ServerToClient::Groups { groups } => {
            format!("[server] Gruppi di appartenenza: {:?}", groups)
        }
        ServerToClient::ListUsers { users } => format!("[server] Users: {:?}", users),
        ServerToClient::Error { reason } => format!("[error] {}", reason),
        ServerToClient::Pong => "[server] pong".to_string(),
        ServerToClient::GlobalMessage { from, text } => {
            format!("[globale] <{}> {}", from, text)
        }
        ServerToClient::GroupCreated { group } => {
            format!("[server] gruppo '{}' creato correttamente!", group)
        }
    }
}
