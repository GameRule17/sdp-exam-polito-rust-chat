/*
Modulo Connection: gestisce la connessione TCP con ciascun client.
Si occupa di ricevere, interpretare e inoltrare i messaggi tra client e server, e di gestire la disconnessione.
*/
/* NDJSON -> Newline delimited JSON */

use std::sync::Arc;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::RwLock,
};
use tracing::error;
use uuid::Uuid;

use crate::commands::dispatch;
use ruggine_common::{ClientToServer, ServerToClient};

use crate::state::{Rx, State, Tx};
// Validazioni e utility ora sono usate nei singoli moduli comando

pub async fn handle_conn(stream: TcpStream, state: Arc<RwLock<State>>) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader).lines();

    // canale out verso questo client
    let (tx, mut rx): (Tx, Rx) = tokio::sync::mpsc::unbounded_channel();

    // task di scrittura: prende ServerToClient dal canale e li scrive in NDJSON
    /*
    NDJSON -> è un formato in cui ogni riga di un file o di uno stream contiene un oggetto JSON separato,
    terminato da un carattere di nuova linea (\n). */
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let line = match serde_json::to_string(&msg) {
                Ok(s) => s,
                Err(e) => {
                    error!("Errore di serializzazione: {e}");
                    continue;
                }
            };
            if writer.write_all(line.as_bytes()).await.is_err() {
                break; // se il client si disconnette, effettuo break, altrimenti invio il messaggio vero e proprio
            }
            if writer.write_all(b"\n").await.is_err() {
                break; // se il client si disconnette, effettuo break, altrimenti aggiungo il carattere di nuova linea
            }
        }
    });

    // id di questa connessione dopo Register
    let mut client_id: Option<Uuid> = None;

    // loop di lettura NDJSON — gestiamo anche EOF/errori come disconnessioni normali
    loop {
        match reader.next_line().await {
            Ok(Some(line)) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // parse sicuro del JSON -> enum
                //prova di conversione in un oggetto di tipo ClientToServer
                //campo 'kind' per il controllo che sia corretto con il rispettivo match nelle varie funzioni
                let msg: ClientToServer = match serde_json::from_str(line) {
                    Ok(m) => m,
                    Err(e) => {
                        error!("Errore parsing messaggio: {}", e);
                        let _ = tx.send(ServerToClient::Error {
                            reason: "JSON errato".into(),
                        });
                        continue;
                    }
                };

                // Questa struttura permette al server di sapere se deve aggiornare l’ID del client e 
                //se deve terminare la connessione dopo aver gestito un comando.
                let res = dispatch(msg, client_id, &tx, &state).await;
                client_id = res.new_client_id;
                if res.close {
                    break;
                }
            }
            Ok(None) => {
                /*Serve a pulire lo stato del server quando un client si disconnette in modo ordinato,
                 evitando utenti “fantasma” o risorse non liberate. */
                if let Some(id) = client_id.take() {
                    let mut st = state.write().await;
                    let nick_opt = st.nicks_by_id.get(&id).cloned();
                    if let Some(nick) = nick_opt {
                        println!("{} si è disconnesso dal server", nick);
                    }
                    //rimozione dagli utenti attivi
                    if let Some(nick) = st.nicks_by_id.get(&id).cloned() {
                        st.users_by_nick.remove(&nick);
                    }
                    st.nicks_by_id.remove(&id);
                    st.clients.remove(&id);
                }
                break;
            }
            Err(e) => {
                // Se è un reset/abort/broken pipe, trattalo come disconnessione normale
                    //errori che indicano che il client non è più connesso/raggiungibile
                //se errore di altro tipo, viene propagato (es CTRL+C)
                use std::io::ErrorKind;
                if matches!(
                    e.kind(),
                    ErrorKind::ConnectionReset
                        | ErrorKind::ConnectionAborted
                        | ErrorKind::BrokenPipe
                ) {
                    if let Some(id) = client_id.take() {
                        let mut st = state.write().await;
                        let nick_opt = st.nicks_by_id.get(&id).cloned();
                        if let Some(nick) = nick_opt {
                            println!("{} si è disconnesso dal server", nick);
                        }
                        if let Some(nick) = st.nicks_by_id.get(&id).cloned() {
                            st.users_by_nick.remove(&nick);
                        }
                        st.nicks_by_id.remove(&id);
                        st.clients.remove(&id);
                    }
                    break;
                } else {
                    return Err(e.into());
                }
            }
        }
    }

    Ok(())
}
