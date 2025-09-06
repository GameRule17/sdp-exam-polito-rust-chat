/*
Entry point del client chat.
Inizializza la connessione, effettua l'handshake e avvia l'interfaccia utente.
*/

mod args;
mod commands;
mod handshake;
mod messages;
mod net;
mod terminal;
mod ui;

use args::Args;
use clap::Parser; // per Args::parse
use handshake::register_handshake;
use std::sync::Arc;
use terminal::restore_terminal;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::panic::set_hook(Box::new(|info| {
        restore_terminal();
        eprintln!("PANICO: {}", info);
    }));
    tracing_subscriber::fmt().with_env_filter("info").init();

    let args = Args::parse();

    let stream = TcpStream::connect(&args.server).await?;
    let (reader_half, writer_half) = stream.into_split();
    let writer_half = Arc::new(Mutex::new(writer_half));
    let mut reader_lines = BufReader::new(reader_half).lines();

    let mut wh = writer_half.lock().await;
    let (_client_id, my_nick, handshake_msgs): (Uuid, String, Vec<String>) =
        register_handshake(&args, &mut *wh, &mut reader_lines).await?;
    drop(wh);

    ui::run_ui(reader_lines, writer_half, my_nick, handshake_msgs).await
}
