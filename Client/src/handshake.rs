/*
Modulo Handshake: gestisce la fase di registrazione e handshake tra client e server.
Effettua il login e gestisce la negoziazione del nickname.
*/

use ruggine_common::{ClientToServer, ServerToClient};
use tokio::io::{BufReader, Lines};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use uuid::Uuid;

use crate::args::Args;
use crate::net::send;
use crate::terminal::prompt_nick;

// Registrazione con retry finché il nick è accettato
pub async fn register_handshake(
    args: &Args,
    // due metà di una connessione TCP asincrona gestita da Tokio
    writer: &mut OwnedWriteHalf, // invio di messaggi al server
    reader: &mut Lines<BufReader<OwnedReadHalf>>,
    // metà di lettura incapsulata in in un buffer ed in un iteratore di linee
    // di modo da gestire la lettura linea per linea
) -> anyhow::Result<(Uuid, String, Vec<String>)> { // client_id, nick, msgs
    loop {

        // se passo negli Args il nick
        let nick: String = match &args.nick {
            Some(n) => n.trim().to_string(),
            None => {
                // Disabilita la raw mode prima di chiedere il nick
                // RAW MODE: ripristina il comportamento normale del terminale
                // visualizzazione caratteri, caratteri speciali
                let _ = crossterm::terminal::disable_raw_mode();
                prompt_nick()? // si veda terminal.rs
            }
        };

        let client_id = Uuid::new_v4(); // creazione id randomico
        send(
            writer,
            &ClientToServer::Register {
                nick: nick.clone(),
                client_id,
            },
        ).await?;

        // Aspetta una risposta
        let line = match reader.next_line().await? {
            Some(l) => l,
            None => anyhow::bail!("Connessione chiusa durante la registrazione"),
            // bail! interrompe l'esecuzione della funzione ma non causa l'arresto
            // del programma come la macro panic!
        };

        match serde_json::from_str::<ServerToClient>(&line) {
            Ok(ServerToClient::Registered { ok, reason }) => {
                if ok {
                    let mut msgs = Vec::new();
                    msgs.push(format!("[server] utente {} loggato correttamente", nick));
                    msgs.push("[server] Per visualizzare il menu invia '/' ".to_string());
                    return Ok((client_id, nick, msgs));
                } else {
                    // Se il campo reason (Option<String>) contiene un valore (Some),
                    // viene usato quel valore. Se invece è None (cioè il server non ha fornito
                    // una motivazione), viene restituita la stringa di default "motivo sconosciuto"
                    eprintln!(
                        "[server] Registrazione rifiutata: {}",
                        reason.unwrap_or_else(|| "motivo sconosciuto".into())
                    );
                }
            }
            Ok(other) => {
                eprintln!(
                    "[server] risposta inattesa durante la registrazione: {:?}",
                    other
                );
            }
            Err(e) => {
                eprintln!("Parse della risposta di registrazione fallito: {e}");
            }
        }

        if args.nick.is_some() {
            // se --nick era passato ma rifiutato, la prossima iterazione chiederà interattivamente
        }
    }
}
