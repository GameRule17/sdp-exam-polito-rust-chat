use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio::sync::broadcast;
use futures::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};

mod logger;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChatMessage {
    from: String,
    body: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    // Avvio del logger in background
    tokio::spawn(async {
        if let Err(e) = logger::start_cpu_logger("server_cpu.log").await {
            eprintln!("Errore logger CPU: {:?}", e);
        }
    });

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Server in ascolto su 127.0.0.1:8080");

    // Canale broadcast per inviare messaggi a tutti i client
    let (tx, _rx) = broadcast::channel::<ChatMessage>(100);

    loop {
        let (stream, _) = listener.accept().await?;
        let tx = tx.clone();
        let rx = tx.subscribe();

        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, tx, rx).await {
                eprintln!("Connessione terminata: {}", e);
            }
        });
    }
}

async fn handle_client(
    stream: TcpStream,
    tx: broadcast::Sender<ChatMessage>,
    mut rx: broadcast::Receiver<ChatMessage>,
) -> anyhow::Result<()> {
    let framed = Framed::new(stream, LengthDelimitedCodec::new());
    let (mut writer, mut reader) = framed.split();

    // 1️⃣ chiedi username
    writer.send(serde_json::to_vec(&ChatMessage {
        from: "SERVER".into(),
        body: "Inserisci il tuo username:".into(),
    })?.into()).await?;

    let username_msg = reader.next().await.ok_or_else(|| anyhow::anyhow!("Connessione chiusa"))??;
    let username: ChatMessage = serde_json::from_slice(&username_msg)?;
    let username = username.body.clone();

    println!("{} si è connesso", username);

    // 2️⃣ task per ricevere dal client e fare broadcast
    let tx_clone = tx.clone();
    let uname_clone = username.clone();
    tokio::spawn(async move {
        while let Some(Ok(bytes)) = reader.next().await {
            if let Ok(msg) = serde_json::from_slice::<ChatMessage>(&bytes) {
                let _ = tx_clone.send(ChatMessage {
                    from: uname_clone.clone(),
                    body: msg.body,
                });
            }
        }
    });

    // 3️⃣ invio messaggi dal broadcast
    while let Ok(msg) = rx.recv().await {
        let json = serde_json::to_vec(&msg)?;
        if let Err(e) = writer.send(json.into()).await {
            eprintln!("Errore invio a {}: {}", username, e);
            break;
        }
    }

    Ok(())
}
