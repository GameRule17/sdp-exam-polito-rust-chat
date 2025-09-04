/*
Modulo Commands: contiene la logica per interpretare e gestire i comandi inseriti dall'utente nel client.
Invia le richieste appropriate al server e gestisce la risposta locale.
*/

use std::sync::Arc;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex;
use ruggine_common::ClientToServer;

use crate::net::send;
use crate::terminal::restore_terminal;

// Client: nessuna validazione rigida; il server applica le regole definitive
// Funzione che gestisce i comandi e messaggi (estratta per riuso nel REPL raw-mode)
pub async fn handle_command(line: &str, writer_half: &Arc<Mutex<OwnedWriteHalf>>, my_nick: &str) -> anyhow::Result<Vec<String>> {
    let mut out = Vec::new();
    if line == "/help" || line == "/" {
        out.push(String::new());
        out.push("============================= MENU COMANDI ================================".into());
        out.push("/help (o /)                  visualizza questo menu dettagliato".into());
        out.push("/create <name>               crea un nuovo gruppo con nome <name>".into());
        out.push("/invite <group> <nick>       invita l'utente <nick> nel gruppo <group>".into());
        out.push("/join <group> <code>         unisciti al gruppo <group> con il codice <code>".into());
        out.push("/leave <group>               esci dal gruppo <group>".into());
        out.push("/users                       mostra tutti gli utenti connessi".into());
        out.push("/groups                      mostra i gruppi di appartenenza".into());
        out.push("/msg <group> <text>          invia il messaggio <text> al gruppo <group>".into());
        out.push("/quit                        esci dal client".into());
        out.push("==========================================================================".into());
        out.push(String::new());
    } else if line == "/quit" {
        // comportati come CTRL+C: invia logout, ripristina il terminale e esci al shell
        let mut wh = writer_half.lock().await;
        let _ = send(&mut *wh, &ClientToServer::Logout { reason: None }).await;
    // OwnedWriteHalf non espone shutdown su Windows direttamente; chiudiamo drop del lock

        // ripristina stato terminale prima di uscire
        restore_terminal();

        println!("{} ti sei disconnesso correttamente", my_nick);
        std::process::exit(0);
    } else if let Some(rest) = line.strip_prefix("/create ") {
        let mut wh = writer_half.lock().await;
        let _ = send(&mut *wh, &ClientToServer::CreateGroup { group: rest.to_string() }).await;
    } else if let Some(rest) = line.strip_prefix("/invite ") {
        let mut it = rest.splitn(2, ' ');
        if let (Some(group), Some(nick)) = (it.next(), it.next()) {
            let mut wh = writer_half.lock().await;
            let _ = send(&mut *wh, &ClientToServer::Invite { group: group.into(), nick: nick.into() }).await;
        } else { out.push("[error] uso: /invite <group> <nick>".into()); }
    } else if let Some(rest) = line.strip_prefix("/join ") {
        let mut it = rest.splitn(2, ' ');
        if let (Some(group), Some(code)) = (it.next(), it.next()) {
            let mut wh = writer_half.lock().await;
            let _ = send(&mut *wh, &ClientToServer::JoinGroup { group: group.into(), invite_code: code.into() }).await;
        } else { out.push("[error] uso: /join <group> <code>".into()); }
    } else if let Some(group) = line.strip_prefix("/leave ") {
        let group = group.trim();
        if group.is_empty() { out.push("[error] uso: /leave <group>".into()); }
        else {
            let mut wh = writer_half.lock().await;
            let _ = send(&mut *wh, &ClientToServer::LeaveGroup { group: group.into() }).await;
        }
    } else if line == "/users" {
        let mut wh = writer_half.lock().await;
        let _ = send(&mut *wh, &ClientToServer::ListUsers).await;
    } else if line == "/groups" {
        let mut wh = writer_half.lock().await;
        let _ = send(&mut *wh, &ClientToServer::ListGroups).await;
    } else if let Some(rest) = line.strip_prefix("/msg ") {
        let mut it = rest.splitn(2, ' ');
        if let (Some(group), Some(text)) = (it.next(), it.next()) {
            let mut wh = writer_half.lock().await;
            let _ = send(&mut *wh, &ClientToServer::SendMessage { group: group.into(), text: text.into(), nick: my_nick.to_string() }).await;
        } else { out.push("[error] uso: /msg <group> <text>".into()); }
    } else if line.starts_with('/') {
        out.push("[error] comando errato".into());
    } else {
        let mut wh = writer_half.lock().await;
        let _ = send(&mut *wh, &ClientToServer::GlobalMessage { text: line.to_string() }).await;
    }
    Ok(out)
}
