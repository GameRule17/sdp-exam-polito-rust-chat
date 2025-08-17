use clap::Parser;
use ruggine_common::{ClientToServer, ServerToClient};
use std::io::{self, Write};
use tokio::{net::TcpStream, io::{AsyncWriteExt, AsyncBufReadExt, BufReader}};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name="ruggine-client")]
struct Args {
    /// Indirizzo del server es. 127.0.0.1:7000
    #[arg(long, default_value="127.0.0.1:7000")]
    server: String,

    /// Nickname (se omesso, verrà richiesto all'avvio)
    #[arg(long)]
    nick: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let args = Args::parse();

   
    let nick = match args.nick {
        Some(n) => normalize_nick(n)?,
        None => prompt_nick()?,
    };

    
    let client_id = Uuid::new_v4();

    
    let stream = TcpStream::connect(&args.server).await?;
    let (reader, mut writer) = stream.into_split(); // metà owned
    let mut reader = BufReader::new(reader).lines();

    // manda subito il Register
    let reg = ClientToServer::Register { nick: nick.clone(), client_id };
    send(&mut writer, &reg).await?;

    
    let read_task = tokio::spawn(async move {
        while let Ok(Some(line)) = reader.next_line().await {
            if let Ok(msg) = serde_json::from_str::<ServerToClient>(&line) {
                match msg {
                    ServerToClient::Registered { ok, reason } =>
                        println!("[server] registered: ok={ok} {:?}", reason),
                    ServerToClient::InviteCode { group, code } =>
                        println!("[server] invite for group '{group}': {code}"),
                    ServerToClient::Joined { group } =>
                        println!("[server] joined group '{group}'"),
                    ServerToClient::Message { group, from, text } =>
                        println!("[{group}] <{from}> {text}"),
                    ServerToClient::SendPvtMessage { from, text } =>
                        println!("[private] <{from}> {text}"),
                    ServerToClient::Groups { groups } =>
                        println!("Groups: {:?}", groups),
                    ServerToClient::Error { reason } =>
                        eprintln!("[error] {reason}"),
                    ServerToClient::Pong =>
                        println!("[server] pong"),
                }
            }
        }
    });

    
    println!(r#"Comandi:
\help
\group create <name>
\invite <group> <nick>
\join <group> <code>
\list
\msg <group> <text>
\dm <nick> <text>
\quit
"#);

    let mut stdin_reader = BufReader::new(tokio::io::stdin());
    let mut buf = String::new();

    loop {
        print!("> "); io::stdout().flush().unwrap();
        buf.clear();
        let n = tokio::io::AsyncBufReadExt::read_line(&mut stdin_reader, &mut buf).await?;
        if n == 0 { break; }

        let line = buf.trim().to_string();
        if line.is_empty() { continue; }

        if line == r"\help" {
            println!("\\group create <name>\n\\invite <group> <nick>\n\\join <group> <code>\n\\list\n\\msg <group> <text>\n\\quit");
            continue;
        }
        if line == r"\quit" {
            let _ = send(&mut writer, &ClientToServer::Logout).await;
            break;
        }
        if let Some(rest) = line.strip_prefix(r"\group create ") {
            let _ = send(&mut writer, &ClientToServer::CreateGroup { group: rest.to_string() }).await;
            continue;
        }
        if let Some(rest) = line.strip_prefix(r"\invite ") {
            let mut it = rest.splitn(2, ' ');
            if let (Some(group), Some(nick)) = (it.next(), it.next()) {
                let _ = send(&mut writer, &ClientToServer::Invite { group: group.into(), nick: nick.into() }).await;
            } else {
                eprintln!("uso: \\invite <group> <nick>");
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix(r"\dm ") {
            let mut it = rest.splitn(2, ' ');
            if let (Some(nick), Some(text)) = (it.next(), it.next()) {
                let _ = send(&mut writer, &ClientToServer::SendPvtMessage { to: nick.into(), text: text.into() }).await;
            } else {
                eprintln!("uso: \\dm <nick> <text>");
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix(r"\join ") {
            let mut it = rest.splitn(2, ' ');
            if let (Some(group), Some(code)) = (it.next(), it.next()) {
                let _ = send(&mut writer, &ClientToServer::JoinGroup { group: group.into(), invite_code: code.into() }).await;
            } else {
                eprintln!("uso: \\join <group> <code>");
            }
            continue;
        }
        if line == r"\list" {
            let _ = send(&mut writer, &ClientToServer::ListGroups).await;
            continue;
        }
        if let Some(rest) = line.strip_prefix(r"\msg ") {
            let mut it = rest.splitn(2, ' ');
            if let (Some(group), Some(text)) = (it.next(), it.next()) {
                let _ = send(&mut writer, &ClientToServer::SendMessage { group: group.into(), text: text.into() }).await;
            } else {
                eprintln!("uso: \\msg <group> <text>");
            }
            continue;
        }

        println!("Comando sconosciuto. \\help per aiuto.");
    }

    read_task.abort();
    Ok(())
}

// —— helpers ——

async fn send(writer: &mut tokio::net::tcp::OwnedWriteHalf, msg: &ClientToServer) -> anyhow::Result<()> {
    let data = serde_json::to_string(msg)? + "\n"; // NDJSON
    writer.write_all(data.as_bytes()).await?;
    Ok(())
}

// Chiede un nickname valido da stdin (bloccante, chiamato prima della connessione)
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
        // valida: niente spazi “interni” se non li vuoi, limite di lunghezza, ecc.
        if s.len() > 32 {
            eprintln!("Nickname troppo lungo (max 32).");
            continue;
        }
        return Ok(s.to_string());
    }
}

// Se passato via CLI, normalizza/valida il nick
fn normalize_nick(n: String) -> anyhow::Result<String> {
    let s = n.trim().to_string();
    if s.is_empty() {
        anyhow::bail!("--nick non può essere vuoto");
    }
    if s.len() > 32 {
        anyhow::bail!("--nick troppo lungo (max 32)");
    }
    Ok(s)
}
