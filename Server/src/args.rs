use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ruggine-server")]
pub struct Args {
    /// Indirizzo di bind es. 0.0.0.0:7000
    #[arg(long, default_value = "127.0.0.1:7000")]
    pub bind: String,
}
