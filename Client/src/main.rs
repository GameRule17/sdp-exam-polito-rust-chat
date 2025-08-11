use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use futures::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use std::io::{self, Write};

#[derive(Serialize, Deserialize, Debug)]
struct ChatMessage {
    from: String,
    body: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let framed = Framed::new(stream, LengthDelimitedCodec::new());
    let (mut writer, mut reader) = framed.split();

    // 🔹 Task ricezione messaggi dal server
    tokio::spawn(async move {
        while let Some(Ok(bytes)) = reader.next().await {
            if let Ok(msg) = serde_json::from_slice::<ChatMessage>(&bytes) {
                println!("[{}] {}", msg.from, msg.body);
            }
        }
    });

    // 🔹 Invio messaggi da stdin
    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush()?; // forza la stampa del prompt
        let mut buf = String::new();
        stdin.read_line(&mut buf)?;
        let buf = buf.trim().to_string();

        if buf.is_empty() {
            continue;
        }

        let msg = ChatMessage {
            from: "".into(), // il server imposta il nome
            body: buf,
        };
        let json = serde_json::to_vec(&msg)?;
        writer.send(json.into()).await?;
    }
}
