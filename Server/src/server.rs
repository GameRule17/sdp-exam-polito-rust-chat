use std::sync::Arc;
use tokio::{net::TcpListener, sync::RwLock};
use tracing::{info, warn};

use crate::connection::handle_conn;
use crate::state::State;

pub async fn run(bind_addr: &str, state: Arc<RwLock<State>>) -> anyhow::Result<()> {
    // Proviamo a bindare l'indirizzo; se fallisce mostriamo un messaggio più amichevole in italiano
    let listener = match TcpListener::bind(bind_addr).await {
        Ok(l) => {
            info!("Bind riuscito su {}", bind_addr);
            l
        }
        Err(e) => {
            use std::io::ErrorKind;
            if e.kind() == ErrorKind::AddrInUse {
                println!("Il server è già attivo su {}", bind_addr);
                return Ok(());
            } else {
                println!("Impossibile avviare il server su {}: {}", bind_addr, e);
                return Ok(());
            }
        }
    };

    loop {
        let (socket, _addr) = listener.accept().await?;
        let st = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(socket, st).await {
                warn!("Connessione terminata con errore: {:?}", e);
            }
        });
    }
}
