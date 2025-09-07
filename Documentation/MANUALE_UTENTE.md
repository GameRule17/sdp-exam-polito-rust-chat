<!-- cargo run sia su client che su server -->

# MANUALE UTENTE

## Introduzione

Questa guida spiega come installare, avviare e utilizzare la chat client-server sviluppata in Rust. Il sistema permette di comunicare in gruppi, inviare messaggi globali e gestire utenti tramite comandi da terminale.

## Requisiti per compatibilità

- Sistema operativo: Windows o MacOS (compatibilità certificata con Windows 11 e MacOS Sequoia 15.6)
<!-- - Rust installato -->

## Installazione

1. Clona il repository:
   ```
   git clone https://github.com/PdS2425-C2/G26.git
   cd G26
   ```
2. Compila il server:
   ```
   cd Server
   cargo build --release
   ```
3. Compila il client:
   ```
   cd ../Client
   cargo build --release
   ```

per compilare in modalità debug, digitare solo

```
cargo build
```

sia lato server che client.

## Avvio del Server

1. Vai nella cartella `Server`:
   ```
   cd Server
   ```
2. Avvia il server:
   ```
   cargo run --release
   ```
   o in modalità debug
   ```
   cargo run
   ```
   Oppure esegui direttamente il file binario:
   - Su Windows: doppio click su `target\release\ruggine-server.exe`
   - Su Linux/MacOS: `./target/release/ruggine-server`

## Avvio del Client

1. Vai nella cartella `Client`:
   ```
   cd ../Client
   ```
2. Avvia il client:
   ```
   cargo run --release
   ```
   o in modalità debug
   ```
   cargo run
   ```
   Oppure esegui direttamente il file binario:
   - Su Windows: doppio click su `target\release\ruggine-client.exe`
   - Su Linux/MacOS: `./target/release/ruggine-client`

## Utilizzo del Client

Dopo l'avvio, puoi interagire tramite i comandi elencati sotto. Puoi anche inviare messaggi globali semplicemente scrivendo il testo e premendo invio.

## Tabella comandi principali

| Comando                   | Descrizione                            |
| ------------------------- | -------------------------------------- |
| `/help` o `/`             | Visualizza il menu dei comandi         |
| `/create <nome>`          | Crea un nuovo gruppo                   |
| `/invite <gruppo> <nick>` | Invita un utente in un gruppo          |
| `/join <gruppo> <codice>` | Unisciti a un gruppo con codice invito |
| `/leave <gruppo>`         | Esci da un gruppo                      |
| `/users`                  | Mostra tutti gli utenti connessi       |
| `/groups`                 | Mostra i gruppi di appartenenza        |
| `/msg <gruppo> <testo>`   | Invia un messaggio a un gruppo         |
| `/quit`                   | Esci dal client                        |

## Esempio di sessione

![Esempio di sessione1](/Documentation/imgs/esempio_chat.png)
![Esempio di sessione1](/Documentation/imgs/esempio_chat_server.png)

## Limitazioni

- Max 32 caratteri per nickname e nomi gruppo
- Nickname **non** può essere "server" o "client"
- Solo caratteri alfanumerici ASCII
- Nomi gruppo o nickname gestiti mediante trim (quindi spazi aggiuntivi all'inizio o fine verranno rimossi)

## Supporto

Per problemi o domande, contatta il progettista o consulta il MANUALE_PROGETTISTA.md.
