/*
Entry point del server chat.
Inizializza il logger, la struttura di stato e avvia il ciclo di accettazione delle connessioni.
*/

use clap::Parser;
use ctrlc;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

mod args;
pub mod commands;
mod connection;
mod logger;
mod server;
mod state;
mod util;
mod validation;

use args::Args;
use state::State;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Avvio del logger in background - task asincrono
    //serve per loggare l'uso della CPU periodicamente (ogni due minuti)
    tokio::spawn(async {
        if let Err(e) = logger::start_cpu_logger("server_cpu.log").await {
            eprintln!("Errore logger CPU: {:?}", e);
        }
    });

    //configura il logging affinché vengano visualizzati solo 
    //i messaggi informativi, di warning ed errore, con un formato leggibile in console.
    tracing_subscriber::fmt().with_env_filter("info").init();

    let args = Args::parse();
    let state = Arc::new(RwLock::new(State::default()));

    // Log dell'indirizzo di bind (il bind vero avviene nel modulo server)
    info!("Server in ascolto su {}", args.bind);

    // Gestore CTRL+C per shutdown pulito
    ctrlc::set_handler(move || {
        println!("Server non più in ascolto");
        std::process::exit(0);
    })
    .expect("Errore nel registrare il gestore CTRL+C");

    // Avvia il loop del server (bind + accept + spawn connessioni)
    server::run(&args.bind, state).await
}
