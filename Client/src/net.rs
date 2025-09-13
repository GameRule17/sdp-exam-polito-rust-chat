/*
Modulo Net: gestisce l'invio di messaggi dal client al server tramite la connessione TCP.
Serializza i dati e li trasmette in formato NDJSON.
NDJSON (Newline Delimited JSON): Ogni riga è un JSON valido e le righe sono separate da 
un carattere di newline (\n).
*/

use ruggine_common::ClientToServer;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;

pub async fn send(writer: &mut OwnedWriteHalf, msg: &ClientToServer) -> anyhow::Result<()> {
    let data = serde_json::to_string(msg)? + "\n"; // Serializza in formato NDJSON
    writer.write_all(data.as_bytes()).await?;
    Ok(())
}
