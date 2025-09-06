/*
Modulo Connection: gestisce la connessione TCP con ciascun client.
Si occupa di ricevere, interpretare e inoltrare i messaggi tra client e server, e di gestire la disconnessione.
*/

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
                break; // client disconnesso
            }
            if writer.write_all(b"\n").await.is_err() {
                break; // client disconnesso
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

                let res = dispatch(msg, client_id, &tx, &state).await;
                client_id = res.new_client_id;
                if res.close {
                    break;
                }
            }
            Ok(None) => {
                // EOF: client ha chiuso la connessione in modo pulito -> pulizia dello stato
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
            }
            Err(e) => {
                // Se è un reset/abort/broken pipe, trattalo come disconnessione normale
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
