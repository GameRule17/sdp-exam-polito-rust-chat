use clap::Parser;

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
