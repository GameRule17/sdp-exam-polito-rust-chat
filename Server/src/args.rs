use clap::Parser;

/*
Modulo Args: gestisce il parsing degli argomenti da linea di comando per il server.
Permette di specificare l'indirizzo di bind su cui il server ascolta.
*/

#[derive(Parser, Debug)]
#[command(name = "ruggine-server")]
pub struct Args {
    /// Indirizzo di bind es. 0.0.0.0:7000
    #[arg(long, default_value = "127.0.0.1:7000")]
    pub bind: String,
}
//commento di prova