use clap::Parser;
use ruggine_common::{ClientToServer, ServerToClient};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    net::TcpStream,
};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "ruggine-client")]
struct Args {
    /// Indirizzo del server es. 127.0.0.1:7000
    #[arg(long, default_value = "127.0.0.1:7000")]
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

    // Stretta di mano con retry: prendi il lock async temporaneamente
    let mut wh = writer_half.lock().await;
    let (_client_id, my_nick, handshake_msgs): (Uuid, String, Vec<String>) =
        register_handshake(&args, &mut *wh, &mut reader_lines).await?;
    drop(wh);

    // --- Set up asynchronous input with crossterm for proper line redraw ---
    use crossterm::{cursor, event, terminal, QueueableCommand};
    use crossterm::ExecutableCommand;
    use std::time::Duration;

    // We keep a channel to forward server messages to the UI printer so we can redraw properly.
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // Inoltra qui i messaggi generati durante la handshake (non stampati direttamente prima)
    for m in handshake_msgs {
        let _ = msg_tx.send(m);
    }

    // Task che legge dal server e invia testo formattato sul canale
    let mut reader_for_task = reader_lines;
    let read_task = tokio::spawn(async move {
        while let Ok(Some(line)) = reader_for_task.next_line().await {
            if let Ok(msg) = serde_json::from_str::<ServerToClient>(&line) {
                let rendered = match msg {
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
                        format!("Gruppi di appartenenza: {:?}", groups)
                    }
                    ServerToClient::ListUsers { users } => format!("Users: {:?}", users),
                    ServerToClient::Error { reason } => format!("[server] {}", reason),
                    ServerToClient::Pong => "[server] pong".to_string(),
                    ServerToClient::GlobalMessage { from, text } => {
                        format!("[globale] <{}> {}", from, text)
                    }
                    ServerToClient::GroupCreated { group } => {
                        format!("[server] gruppo '{}' creato correttamente!", group)
                    }
                };
                let _ = msg_tx.send(rendered);
            }
        }
    });

    // Gestione SIGINT (CTRL+C): invia sempre il logout al server (async)
    let writer_half_ctrlc = Arc::clone(&writer_half);
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let mut wh = writer_half_ctrlc.lock().await;
        // invia logout e poi chiudi la metà di scrittura per assicurare il flush
        let _ = send(
            &mut *wh,
            &ClientToServer::Logout {
                reason: Some("CTRL+C".to_string()),
            },
        )
        .await;
        // prova a chiudere/flushare la scrittura e attendi un breve intervallo
        let _ = wh.shutdown().await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        std::process::exit(0);
    });

    // REPL con gestione manuale della riga di input
    // Entriamo in raw mode per gestire tasti singolarmente
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(terminal::EnterAlternateScreen)?;
    // semplice prompt persistente
    let prompt = "> ";
    let mut input = String::new();

    // Funzione locale per ridisegnare la riga corrente
    let redraw = |stdout: &mut io::Stdout, input: &str| -> anyhow::Result<()> {
        stdout
            .queue(cursor::MoveToColumn(0))?
            .queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
        write!(stdout, "{}{}", prompt, input)?;
        stdout.flush()?;
        Ok(())
    };
    redraw(&mut stdout, &input)?;

    // Event loop: multiplex tra input tasti e messaggi dal server
    loop {
        tokio::select! {
            maybe_msg = msg_rx.recv() => {
                if let Some(txt) = maybe_msg {
                    // Stampa messaggio su nuova linea, poi ridisegna input
                    stdout.queue(cursor::MoveToColumn(0))?;
                    stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
                    writeln!(stdout, "{}", txt)?;
                    redraw(&mut stdout, &input)?;
                } else {
                    // canale chiuso => server reader task terminato
                }
            }
            // Gestione input da tastiera non bloccante
            _ = tokio::task::yield_now() => {
                // Poll crossterm events con timeout breve
                if event::poll(Duration::from_millis(30))? {
                    match event::read()? {
                        event::Event::Key(k) => {
                            use crossterm::event::{KeyCode, KeyModifiers, KeyEventKind};
                            // Consider only key presses (ignore repeats & releases)
                            if k.kind == KeyEventKind::Press {
                                match k.code {
                                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                                        // Simula /quit
                                        stdout.queue(cursor::MoveToColumn(0))?;
                                        stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
                                        writeln!(stdout, "Uscita dal client...")?;
                                        {
                                            let mut wh = writer_half.lock().await;
                                            let _ = send(&mut *wh, &ClientToServer::Logout { reason: Some("CTRL+C".into()) }).await;
                                        }
                                        break;
                                    }
                                    KeyCode::Enter => {
                                        let line = input.trim().to_string();
                                        stdout.queue(cursor::MoveToColumn(0))?;
                                        stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
                                        if !line.is_empty() {
                                            // echo la riga inviata sopra i messaggi
                                            writeln!(stdout, "> {}", line)?;
                                            stdout.flush()?;
                                            handle_command(&line, &writer_half, &my_nick).await?;
                                        }
                                        input.clear();
                                        redraw(&mut stdout, &input)?;
                                    }
                                    KeyCode::Char(ch) => {
                                        input.push(ch);
                                        redraw(&mut stdout, &input)?;
                                    }
                                    KeyCode::Backspace => { input.pop(); redraw(&mut stdout, &input)?; }
                                    KeyCode::Esc => { input.clear(); redraw(&mut stdout, &input)?; }
                                    _ => {}
                                }
                            }
                        }
                        event::Event::Paste(p) => { input.push_str(&p); redraw(&mut stdout, &input)?; }
                        _ => {}
                    }
                }
            }
        }
    }

    // Ripristina terminale
    terminal::disable_raw_mode()?;
    stdout.execute(terminal::LeaveAlternateScreen)?;
    println!("{} ti sei disconnesso correttamente", my_nick);

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
async fn register_handshake(
    args: &Args,
    writer: &mut OwnedWriteHalf,
    reader: &mut Lines<BufReader<OwnedReadHalf>>,
) -> anyhow::Result<(Uuid, String, Vec<String>)> {
    loop {
        let nick: String = match &args.nick {
            Some(n) => n.trim().to_string(),
            None => prompt_nick()?,
        };

        let client_id = Uuid::new_v4();
        send(
            writer,
            &ClientToServer::Register {
                nick: nick.clone(),
                client_id,
            },
        )
        .await?;

        // Aspetta una risposta
        let line = match reader.next_line().await? {
            Some(l) => l,
            None => anyhow::bail!("Connessione chiusa durante la registrazione"),
        };

        match serde_json::from_str::<ServerToClient>(&line) {
            Ok(ServerToClient::Registered { ok, reason }) => {
                if ok {
                    // Non stampiamo qui: ritorniamo i messaggi a main che li inoltra all'UI
                    let mut msgs = Vec::new();
                    msgs.push(format!("[server] utente {} loggato correttamente", nick));
                    msgs.push("[server] Per visualizzare il menu invia '/' ".to_string());
                    return Ok((client_id, nick, msgs));
                } else {
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
// Funzione che gestisce i comandi e messaggi (estratta per riuso nel REPL raw-mode)
async fn handle_command(line: &str, writer_half: &Arc<Mutex<OwnedWriteHalf>>, my_nick: &str) -> anyhow::Result<()> {
    if line == "/help" || line == "/" {
        println!("");
        println!("============================= MENU COMANDI ================================");
        println!("/help (o /)                  visualizza questo menu dettagliato");
        println!("/create <name>               crea un nuovo gruppo con nome <name>");
        println!("/invite <group> <nick>       invita l'utente <nick> nel gruppo <group>");
        println!("/users                       mostra tutti gli utenti connessi");
        println!("/groups                      mostra i gruppi di appartenenza");
        println!("/msg <group> <text>          invia il messaggio <text> al gruppo <group>");
        println!("/quit                        esci dal client");
        println!("==========================================================================");
        println!("");
    } else if line == "/quit" { // comportati come CTRL+C: logout, reset terminale e exit
        let mut wh = writer_half.lock().await;
        let _ = send(&mut *wh, &ClientToServer::Logout { reason: None }).await;
        let _ = wh.shutdown().await;
        // ripristina terminale prima di uscire
        let _ = crossterm::terminal::disable_raw_mode();
        let mut stdout = io::stdout();
    let _ = crossterm::execute!(stdout, crossterm::terminal::LeaveAlternateScreen);
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
        } else { eprintln!("uso: /invite <group> <nick>"); }
    } else if let Some(rest) = line.strip_prefix("/join ") {
        let mut it = rest.splitn(2, ' ');
        if let (Some(group), Some(code)) = (it.next(), it.next()) {
            let mut wh = writer_half.lock().await;
            let _ = send(&mut *wh, &ClientToServer::JoinGroup { group: group.into(), invite_code: code.into() }).await;
        } else { eprintln!("uso: /join <group> <code>"); }
    } else if let Some(group) = line.strip_prefix("/leave ") {
        let group = group.trim();
        if group.is_empty() { eprintln!("uso: /leave <group>"); }
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
        } else { eprintln!("uso: /msg <group> <text>"); }
    } else if line.starts_with('/') {
        println!("[server] comando errato");
    } else {
        let mut wh = writer_half.lock().await;
        let _ = send(&mut *wh, &ClientToServer::GlobalMessage { text: line.to_string() }).await;
    }
    Ok(())
}
