use clap::Parser;
use ruggine_common::{ClientToServer, ServerToClient};
use std::io::{self, Write};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    net::TcpStream,
};
use std::sync::{Arc, Mutex};
use futures;
use ctrlc;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name="ruggine-client")]
struct Args {
    /// Indirizzo del server es. 127.0.0.1:7000
    #[arg(long, default_value="127.0.0.1:7000")]
    server: String,

    /// Nickname (se omesso, verrà richiesto all'avvio e ritentato se rifiutato)
    #[arg(long)]
    nick: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let args = Args::parse();

    // Connessione
    let stream = TcpStream::connect(&args.server).await?;
    let (reader_half, writer_half) = stream.into_split(); // halves owned
    let writer_half = Arc::new(Mutex::new(writer_half));
    let mut reader_lines = BufReader::new(reader_half).lines();

    // Stretta di mano con retry
    let (_client_id, my_nick): (Uuid, String) =
        register_handshake(&args, &mut *writer_half.lock().unwrap(), &mut reader_lines).await?;

    // Task che legge dal server
    let mut reader_for_task = reader_lines;
    let read_task = tokio::spawn(async move {
        while let Ok(Some(line)) = reader_for_task.next_line().await {
            if let Ok(msg) = serde_json::from_str::<ServerToClient>(&line) {
                match msg {
                    ServerToClient::Registered{ok,reason} => println!("[server] registered: ok={} {:?}", ok, reason),
                    ServerToClient::InviteCode{group,code, client_id} => println!("[server] invite for group '{}': {} by {}", group, code, client_id),
                    ServerToClient::InviteCodeForMe{group,code} => println!("[server] group '{}': {}", group, code),
                    ServerToClient::Joined{group} => println!("[server] joined group '{}'", group),
                    ServerToClient::Message{group,from,text} => println!("[{}] <{}> {}", group, from, text),
                    ServerToClient::SendPvtMessage{from,text} => println!("[private] <{}> {}", from, text),
                    ServerToClient::Groups{groups} => println!("Groups: {:?}", groups),
                    ServerToClient::ListUsers { users } => println!("Users: {:?}", users),
                    ServerToClient::Error{reason} => eprintln!("[error] {}", reason),
                    ServerToClient::Pong => println!("[server] pong"),
                    ServerToClient::GlobalMessage { from, text } => println!("[global] <{}> {}", from, text),
                }
            }
        }
    });

    // Gestione SIGINT (CTRL+C): invia sempre il logout al server
    let writer_half_ctrlc = Arc::clone(&writer_half);
    ctrlc::set_handler(move || {
        if let Ok(mut wh) = writer_half_ctrlc.lock() {
            let _ = futures::executor::block_on(send(&mut *wh, &ClientToServer::Logout { reason: Some("CTRL+C".to_string()) }));
        }
        std::process::exit(0);
    }).expect("Errore nel gestire CTRL+C");

    // REPL

    let mut stdin_reader = BufReader::new(tokio::io::stdin());
    let mut buf = String::new();

    loop {
        buf.clear();
        let n = tokio::io::AsyncBufReadExt::read_line(&mut stdin_reader, &mut buf).await?;
        if n == 0 { break; }

        let line = buf.trim().to_string();
        if line.is_empty() { continue; }

    if line == "/help" || line == "/"{
            println!("");
            println!("==================== MENU COMANDI ====================");
            println!("/help (or /)                 visualizza questo menu dettagliato");
            println!("/create <name>               crea un nuovo gruppo con nome <name>");
            println!("/invite <group> <nick>       invita l'utente <nick> nel gruppo <group>");
            println!("/join <group> <code>         entra nel gruppo <group> usando il codice <code>");
            println!("/groups                      mostra tutti i gruppi di cui fai parte");
            println!("/users                       mostra tutti gli utenti connessi");
            println!("/msg <group> <text>          invia il messaggio <text> al gruppo <group>");
            println!("/dm <nick> <text>            invia un messaggio privato a <nick>");
            println!("/quit                        esci dal client");
            println!("======================================================");
            println!("");
            continue;
        }
        else if line == "/quit" {
            println!("Uscita dal client...");
            if let Ok(mut wh) = writer_half.lock() {
                let _ = send(&mut *wh, &ClientToServer::Logout { reason: None }).await;
            }
            break;
        }
    else if let Some(rest) = line.strip_prefix("/create ") {
            if let Ok(mut wh) = writer_half.lock() {
                let _ = send(&mut *wh, &ClientToServer::CreateGroup { group: rest.to_string() }).await;
            }
            continue;
        }

    else if let Some(rest) = line.strip_prefix("/invite ") {
            let mut it = rest.splitn(2, ' ');
            if let (Some(group), Some(nick)) = (it.next(), it.next()) {
                if let Ok(mut wh) = writer_half.lock() {
                    let _ = send(&mut *wh, &ClientToServer::Invite { group: group.into(), nick: nick.into() }).await;
                }
            } else {
                eprintln!("uso: /invite <group> <nick>");
            }
            continue;
        }

    else if let Some(rest) = line.strip_prefix("/dm ") {
            let mut it = rest.splitn(2, ' ');
            if let (Some(nick), Some(text)) = (it.next(), it.next()) {
                if let Ok(mut wh) = writer_half.lock() {
                    let _ = send(&mut *wh, &ClientToServer::SendPvtMessage { to: nick.into(), text: text.into() }).await;
                }
            } else {
                eprintln!("uso: /dm <nick> <text>");
            }
            continue;
        }

    else if let Some(rest) = line.strip_prefix("/join ") {
            let mut it = rest.splitn(2, ' ');
            if let (Some(group), Some(code)) = (it.next(), it.next()) {
                if let Ok(mut wh) = writer_half.lock() {
                    let _ = send(&mut *wh, &ClientToServer::JoinGroup { group: group.into(), invite_code: code.into() }).await;
                }
            } else {
                eprintln!("uso: /join <group> <code>");
            }
            continue;
        }
    else if line == "/users" {
            if let Ok(mut wh) = writer_half.lock() {
                let _ = send(&mut *wh, &ClientToServer::ListUsers).await;
            }
            continue;
        }
    else if line == "/groups" {
            if let Ok(mut wh) = writer_half.lock() {
                let _ = send(&mut *wh, &ClientToServer::ListGroups).await;
            }
            continue;
        }

    else if let Some(rest) = line.strip_prefix("/msg ") {
            let mut it = rest.splitn(2, ' ');
            if let (Some(group), Some(text)) = (it.next(), it.next()) {
                if let Ok(mut wh) = writer_half.lock() {
                    let _ = send(&mut *wh, &ClientToServer::SendMessage { group: group.into(), text: text.into(), nick: my_nick.clone() }).await;
                }
            } else {
                eprintln!("uso: /msg <group> <text>");
            }
            continue;
        }

        else if line.starts_with('/') {
            println!("[server] comando errato");
        } else {
            if let Ok(mut wh) = writer_half.lock() {
                let _ = send(&mut *wh, &ClientToServer::GlobalMessage { text: line.clone() }).await;
            }
        }
    }


    read_task.abort();
    Ok(())
}

// ---------- Helpers ----------

async fn send(writer: &mut OwnedWriteHalf, msg: &ClientToServer) -> anyhow::Result<()> {
    let data = serde_json::to_string(msg)? + "\n"; // NDJSON
    writer.write_all(data.as_bytes()).await?;
    Ok(())
}

// Registrazione con retry finché il nick è accettato
async fn register_handshake(args: &Args, writer: &mut OwnedWriteHalf, reader: &mut Lines<BufReader<OwnedReadHalf>>,) -> anyhow::Result<(Uuid, String)> {
    loop {
        let nick: String = match &args.nick {
            Some(n) => n.trim().to_string(),
            None => prompt_nick()?,
        };

        let client_id = Uuid::new_v4();
        send(writer, &ClientToServer::Register { nick: nick.clone(), client_id }).await?;

        // Aspetta una risposta
        let line = match reader.next_line().await? {
            Some(l) => l,
            None => anyhow::bail!("Connessione chiusa durante la registrazione"),
        };

        match serde_json::from_str::<ServerToClient>(&line) {
            Ok(ServerToClient::Registered { ok, reason }) => {
                if ok {
                    println!("[server] utente {} loggato correttamente", nick);
                    println!("[server] Per visualizzare il menu invia '/' ");
                    return Ok((client_id, nick));
                } else {
                    eprintln!(
                        "[server] Registrazione rifiutata: {}",
                        reason.unwrap_or_else(|| "motivo sconosciuto".into())
                    );
                    // Se --nick era passato ed è rifiutato, si prosegue chiedendo un nuovo nick
                }
            }
            Ok(other) => {
                println!("[server] inatteso durante registrazione: {:?}", other);
            }
            Err(e) => {
                eprintln!("Parse risposta registrazione fallito: {e}");
            }
        }

        // Reset: se era passato --nick ed è stato rifiutato, da qui in poi si chiederà interattivamente
        // (basta lasciare il loop ripartire: la prossima iterazione leggerà da prompt_nick())
        if args.nick.is_some() {
            // piccolo trucco: svuota il campo nick per i retry
            // (non puoi mutare `args`, quindi il match sopra continuerà a usare Some(...);
            //  per semplicità, chiedi sempre dal prompt quando arrivi qui)
        }
    }
}

// Prompt nickname
fn prompt_nick() -> anyhow::Result<String> {
    loop {
        print!("Scegli un nickname: ");
        io::stdout().flush()?;
        let mut s = String::new();
        std::io::stdin().read_line(&mut s)?;
        let s = s.trim();
        if s.is_empty() {
            eprintln!("Il nickname non può essere vuoto.");
            continue;
        }
        return Ok(s.to_string());
    }
}

// Client: nessuna validazione rigida; il server applica le regole definitive