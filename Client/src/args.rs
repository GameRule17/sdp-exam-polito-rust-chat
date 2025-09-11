use clap::Parser; // notare la lib

/*
Modulo Args: gestisce il parsing degli argomenti da linea di comando per il client.
Permette di specificare l'indirizzo del server e il nickname dell'utente.
*/

#[derive(Parser, Debug, Clone)]
#[command(name = "ruggine-client")]
pub struct Args {
    /// Indirizzo del server es. 127.0.0.1:7000
    #[arg(long, default_value = "127.0.0.1:7000")]
    pub server: String,

    /// Nickname (se omesso, verrà richiesto all'avvio e ritentato se rifiutato)
    #[arg(long)]
    pub nick: Option<String>,
}
