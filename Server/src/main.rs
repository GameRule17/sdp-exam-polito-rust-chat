use clap::Parser;
use ruggine_common::{ClientToServer, ServerToClient};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::{mpsc, RwLock},
};
use tracing::{error, info, warn};
use ctrlc;
use uuid::Uuid;

mod validation;
use validation::validate_nick_syntax;

type Tx = mpsc::UnboundedSender<ServerToClient>;
type Rx = mpsc::UnboundedReceiver<ServerToClient>;

#[derive(Default)]
struct Group {
    members: HashSet<Uuid>, // client_ids
}

#[derive(Default)]
struct State {
    
    users_by_nick: HashMap<String, Uuid>,
    
    nicks_by_id: HashMap<Uuid, String>,
    
    groups: HashMap<String, Group>,
    
    invites: HashMap<String, (String, String)>,
    
    clients: HashMap<Uuid, Tx>,
}

#[derive(Parser, Debug)]
#[command(name="ruggine-server")]
struct Args {
    /// Indirizzo di bind es. 0.0.0.0:7000
    #[arg(long, default_value = "127.0.0.1:7000")]
    bind: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Impostiamo il filtro a 'warn' per evitare log informativi all'avvio
    tracing_subscriber::fmt().with_env_filter("info").init();

    let args = Args::parse();
    let state = Arc::new(RwLock::new(State::default()));

    // Proviamo a bindare l'indirizzo; se fallisce mostriamo un messaggio più amichevole in italiano
    let listener = match TcpListener::bind(&args.bind).await {
        Ok(l) => {
            info!("Server in ascolto su {}", args.bind);
            l
        }
        Err(e) => {
            use std::io::ErrorKind;
            if e.kind() == ErrorKind::AddrInUse {
                // messaggio semplice e non fatale: ritorniamo Ok(()) per evitare l'errore rosso di cargo
                println!("Il server è già attivo su {}", args.bind);
                return Ok(());
            } else {
                println!("Impossibile avviare il server su {}: {}", args.bind, e);
                return Ok(());
            }
        }
    };

    // Gestore CTRL+C per shutdown pulito
    ctrlc::set_handler(move || {
        println!("Server non più in ascolto");
        std::process::exit(0);
    }).expect("Errore nel registrare il gestore CTRL+C");

    loop {
        let (socket, _addr) = listener.accept().await?;
        // address intentionally ignored to avoid noisy logs
        let st = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(socket, st).await {
                warn!("Connessione terminata con errore: {:?}", e);
            }
        });
    }
}

async fn handle_conn(stream: TcpStream, state: Arc<RwLock<State>>) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader).lines();

    // canale out verso questo client
    let (tx, mut rx): (Tx, Rx) = mpsc::unbounded_channel();

    // task di scrittura: prende ServerToClient dal canale e li scrive in NDJSON
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let line = match serde_json::to_string(&msg) {
                Ok(s) => s,
                Err(e) => {
                    error!("Serialize error: {e}");
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
                            reason: "Malformed JSON".into(),
                        });
                        continue;
                    }
                };



                match msg {
            ClientToServer::Register { nick, client_id: req_id } => {
                // Validazione sintassi lato server
                if let Err(reason) = validate_nick_syntax(&nick) {
                    let _ = tx.send(ServerToClient::Registered { ok: false, reason: Some(reason) });
                    continue;
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
                                nick,
                                existing_nick
                            )),
                        });
                        continue;
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
                client_id = Some(id);

                println!("{} si è connesso al server", canonical_nick);

                let _ = tx.send(ServerToClient::Registered {
                    ok: true,
                    reason: None,
                });
            }

            

            ClientToServer::CreateGroup { group } => {
                let mut st = state.write().await;
                let id = match client_id {
                    Some(id) => id,
                    None => {
                        let _ = tx.send(ServerToClient::Error {
                            reason: "Non registrato".into(),
                        });
                        continue;
                    }
                };

                if (st.groups.contains_key(&group)) || (st.users_by_nick.get(&group).is_some()) {
                    let _ = tx.send(ServerToClient::Error {
                        reason: format!("Gruppo {group} già esistente"),
                    });
                    continue;
                }
                let g = st.groups.entry(group.clone()).or_default();
                g.members.insert(id);
                // Conferma creazione gruppo
                let _ = tx.send(ServerToClient::GroupCreated { group });
            }



            ClientToServer::Invite { group, nick } => {
                let mut st = state.write().await;
                if !st.groups.contains_key(&group) {
                    let _ = tx.send(ServerToClient::Error {
                        reason: format!("Gruppo {group} inesistente"),
                    });
                    continue;
                }
                // lookup utente destinatario case-insensitive
                let id_user = st
                    .users_by_nick
                    .iter()
                    .find(|(existing_nick, _)| existing_nick.eq_ignore_ascii_case(&nick))
                    .map(|(_, id)| *id);
                
                if (st.users_by_nick.get(&nick).is_none()) || (id_user.is_none()) {
                    let _ = tx.send(ServerToClient::Error {
                        reason: format!("Utente {nick} inesistente"),
                    });
                    continue;
                }

                if st.groups.get(&group).map_or(false, |g|g.members.contains(&id_user.unwrap())){
                    let _ = tx.send(ServerToClient::Error {
                        reason: format!("Utente {nick} già membro del gruppo {group}"),
                    });
                    continue;
                }

                let code = short_code();
                st.invites.insert(code.clone(), (group.clone(), nick.clone()));

                // invia il codice di invito al client a cui faccio riferimento con il comanda \invite 
                if let Some(id) = id_user {
                    if let Some(txm) = st.clients.get(&id) {
                        let _ = txm.send(ServerToClient::InviteCode {
                            group: group.clone(),
                            code: code.clone(),
                            client_id: client_id.and_then(|id| st.nicks_by_id.get(&id).cloned()).unwrap_or_default(),
                        });
                    } else {
                        warn!("Client {} non trovato per invio codice invito", id);
                    }
            }

            //mi faccio restituire il codice di invito a me stesso una volta che ho creato il gruppo
                let _ = tx.send(ServerToClient::InviteCodeForMe {
                    group,
                    code,
                });
        }


            ClientToServer::LeaveGroup { group } => {
                let mut st = state.write().await;
                let id = match client_id {
                    Some(id) => id,
                    None => {
                        let _ = tx.send(ServerToClient::Error {
                            reason: "Non registrato".into(),
                        });
                        continue;
                    }
                };

                match st.groups.get_mut(&group) {
                    Some(g) => {
                        if !g.members.remove(&id) {
                            let _ = tx.send(ServerToClient::Error {
                                reason: format!("Non sei membro del gruppo {group}"),
                            });
                            continue;
                        }
                        if g.members.is_empty() {
                            st.groups.remove(&group);
                        }
                        let _ = tx.send(ServerToClient::Left { group });
                    }
                    None => {
                        let _ = tx.send(ServerToClient::Error {
                            reason: format!("Gruppo {group} inesistente"),
                        });
                        continue;
                    }
                }
            }


            ClientToServer::JoinGroup { group, invite_code } => {
                let mut st = state.write().await;

                let (g, allowed) = match st.invites.remove(&invite_code) {
                    Some(v) => v,
                    None => {
                        let _ = tx.send(ServerToClient::Error {
                            reason: "Invito non valido".into(),
                        });
                        continue;
                    }
                };

                if g != group {
                    let _ = tx.send(ServerToClient::Error {
                        reason: "Invito non per questo gruppo".into(),
                    });
                    continue;
                }

                let id = match client_id {
                    Some(id) => id,
                    None => {
                        let _ = tx.send(ServerToClient::Error {
                            reason: "Non registrato".into(),
                        });
                        continue;
                    }
                };

                let my_nick = st.nicks_by_id.get(&id).cloned().unwrap_or_default();
                if my_nick != allowed {
                    let _ = tx.send(ServerToClient::Error {
                        reason: format!("Invito destinato a {allowed}"),
                    });
                    continue;
                }

                st.groups
                    .entry(group.clone())
                    .or_default()
                    .members
                    .insert(id);

                let _ = tx.send(ServerToClient::Joined { group });
            }

            // SendPvtMessage handling removed (DM feature deprecated)

            ClientToServer::SendMessage { group, text, nick  } => {
                let st = state.read().await;

                let sender_id = st.users_by_nick.get(&nick).cloned();
                if let Some(sender_id) = sender_id {
                    if !st.groups.get(&group).map_or(false, |g| g.members.contains(&sender_id)) {
                        let _ = tx.send(ServerToClient::Error {
                            reason: format!("You are not a member of group {group}"),
                        });
                        continue;
                    }
                } else {
                    let _ = tx.send(ServerToClient::Error {
                        reason: "Sender ID is invalid".into(),
                    });
                    continue;
                }

                let id = match client_id {
                    Some(id) => id,
                    None => {
                        let _ = tx.send(ServerToClient::Error {
                            reason: "Client not registered".into(),
                        });
                        continue;
                    }
                };

                let my_nick = st
                    .nicks_by_id
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| "???".into());

                if let Some(g) = st.groups.get(&group) {
                    for member in &g.members {
                        if member == &id {
                            continue; // non inviare a se stessi
                        }
                        if let Some(txm) = st.clients.get(member) {
                            let _ = txm.send(ServerToClient::Message {
                                group: group.clone(),
                                from: my_nick.clone(),
                                text: text.clone(),
                            });
                        }
                    }
                } else {
                    let _ = tx.send(ServerToClient::Error {
                        reason: format!("Group {group} does not exist"),
                    });
                }
            }

            ClientToServer::ListGroups => {
                let st = state.read().await;

                let id = match client_id {
                    Some(id) => id,
                    None => {
                        let _ = tx.send(ServerToClient::Error {
                            reason: "Non registrato".into(),
                        });
                        continue;
                    }
                };
                
                if st.groups.is_empty() {
                    let _ = tx.send(ServerToClient::Error {
                        reason: "Nessun gruppo disponibile".into(),
                    });
                    continue;
                }
                let groups: Vec<String> = st
                    .groups
                    .iter()
                    .filter(|(_, gr)| gr.members.contains(&id))
                    .map(|(name, _)| name.clone())
                    .collect();

                let _ = tx.send(ServerToClient::Groups { groups });
            }

            ClientToServer::ListUsers => {
                let st = state.read().await;

                let id = match client_id {
                    Some(id) => id,
                    None => {
                        let _ = tx.send(ServerToClient::Error {
                            reason: "Non registrato".into(),
                        });
                        continue;
                    }
                };

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
            }

            ClientToServer::GlobalMessage { text } => {
                let st = state.read().await;

                let id = match client_id {
                    Some(id) => id,
                    None => {
                        let _ = tx.send(ServerToClient::Error {
                            reason: "Non registrato".into(),
                        });
                        continue;
                    }
                };

                let my_nick = st
                    .nicks_by_id
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| "???".into());

                for (client_id, txm) in &st.clients {
                    if *client_id != id { // non inviare a se stessi
                        let _ = txm.send(ServerToClient::GlobalMessage {
                            from: my_nick.clone(),
                            text: text.clone(),
                        });
                    }
                }
            }

            ClientToServer::Logout { reason } => {
                let mut st = state.write().await;
                if let Some(id) = client_id.take() {
                    let nick_opt = st.nicks_by_id.get(&id).cloned();
                    if let Some(nick) = &nick_opt {
                            // Se il client ha inviato un reason che indica CTRL+C, mostriamolo
                            if let Some(r) = reason {
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
                    st.nicks_by_id.remove(&id);
                    st.clients.remove(&id);
                }
                break;
            }

            ClientToServer::Ping => {
                let _ = tx.send(ServerToClient::Pong);
            }
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
                    // rimuovi anche la mappatura users_by_nick per permettere nuovo login con lo stesso nick
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
                if matches!(e.kind(), ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::BrokenPipe) {
                    if let Some(id) = client_id.take() {
                        let mut st = state.write().await;
                        let nick_opt = st.nicks_by_id.get(&id).cloned();
                        if let Some(nick) = nick_opt {
                            println!("{} si è disconnesso dal server", nick);
                        }
                        // rimuovi anche la mappatura users_by_nick
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

// codice invito breve (6 char alfanumerici)
fn short_code() -> String {
    use rand::{distributions::Alphanumeric, Rng};
    let mut rng = rand::thread_rng();
    (0..6).map(|_| rng.sample(Alphanumeric) as char).collect()
}
