/*
Modulo Net: gestisce l'invio di messaggi dal client al server tramite la connessione TCP.
Serializza i dati e li trasmette in formato NDJSON.
*/

use tokio::net::tcp::OwnedWriteHalf;
use tokio::io::AsyncWriteExt;
use ruggine_common::ClientToServer;

pub async fn send(writer: &mut OwnedWriteHalf, msg: &ClientToServer) -> anyhow::Result<()> {
    let data = serde_json::to_string(msg)? + "\n"; // NDJSON
    writer.write_all(data.as_bytes()).await?;
    Ok(())
}
