# MANUALE DEL PROGETTISTA

## Introduzione

Questo progetto implementa una chat client-server in Rust, con architettura modulare e separazione tra client, server e common. Il sistema supporta gruppi, inviti, messaggi globali e logging delle risorse.

## Compatibilità

- Sistemi operativi supportati: Windows 11, MacOS Sequoia 15.6
- Versione minima Rust: 1.70 (consigliata sempre l'ultima stable)
- Architettura: x86_64 (testato anche su ARM64 con Mac M1/M2)
- Terminale: compatibile con cmd, PowerShell, Terminale Mac/Linux

## Flusso di esecuzione

- Il server avvia il logger, si mette in ascolto su una porta e accetta connessioni.
- Il client si connette, effettua handshake (nickname, id), riceve messaggi e invia comandi.
  - Per esperienza utente, si consiglia di avviare il client in due terminali separati per testare l'interazione.
- La comunicazione avviene tramite messaggi JSON serializzati (vedi common/lib.rs).
- I comandi sono gestiti in modo modulare sia lato client che server.

## Descrizione dei moduli principali

### Struttura delle cartelle

```
G26/
├── Client/         # Codice sorgente del client
├── Common/         # Tipi condivisi tra client e server
|── Documentation/  # Documentazione del progetto
├── Server/         # Codice sorgente del server
└── target/         # Output di compilazione
```

### Client

| Modulo       | Descrizione                                                              |
| ------------ | ------------------------------------------------------------------------ |
| args.rs      | Definisce la struct Args per i parametri da CLI (server, nick)           |
| commands.rs  | Funzione handle_command che interpreta la stringa utente e invia comandi |
| handshake.rs | Gestisce la registrazione utente, con retry se il nick non è accettato   |
| main.rs      | Avvia la connessione, effettua handshake, lancia la UI                   |
| messages.rs  | Converte i messaggi ServerToClient in stringhe leggibili per l'utente    |
| net.rs       | Funzione send per inviare messaggi serializzati al server                |
| terminal.rs  | Funzioni per ripristino terminale e richiesta nickname                   |
| ui.rs        | Gestisce il ciclo REPL, input da tastiera, output, scroll, colori        |

### Server

| Modulo        | Descrizione                                                                   |
| ------------- | ----------------------------------------------------------------------------- |
| commands/     | Ogni file implementa la logica di un comando (es. create_group, invite, ecc.) |
| args.rs       | Parametri di avvio server (porta, ecc.)                                       |
| connection.rs | Gestione handshake, lettura/scrittura TCP                                     |
| logger.rs     | Log periodico di CPU e runtime su file                                        |
| main.rs       | Avvio server, setup logger, shutdown pulito                                   |
| server.rs     | Loop principale, accettazione client, dispatch comandi                        |
| state.rs      | Stato condiviso (utenti, gruppi, messaggi)                                    |
| util.rs       | Utility generiche                                                             |
| validation.rs | Regole di validazione nickname/gruppi                                         |

### Common

| Modulo | Descrizione                                                                         |
| ------ | ----------------------------------------------------------------------------------- |
| lib.rs | Definisce i tipi di messaggio, errori, e le strutture condivise tra client e server |

## Scelte tecnologiche e librerie esterne

| Libreria                         | Scopo principale                                          | Componente           |
| -------------------------------- | --------------------------------------------------------- | -------------------- |
| **tokio**                        | gestisce la concorrenza e le operazioni di rete asincrone | Client/Server        |
| **serde / serde_json**           | Serializzazione/deserializzazione JSON                    | Client/Server/common |
| **uuid**                         | Identificatori unici per utenti/client                    | Client/Server/common |
| **clap**                         | Parsing degli argomenti da linea di comando               | Client/Server        |
| **sysinfo**                      | Monitoraggio risorse di sistema (CPU, memoria)            | Client/Server        |
| **tracing / tracing-subscriber** | Logging strutturato e diagnostica                         | Client/Server        |
| **anyhow / thiserror**           | Gestione degli errori                                     | Client/Server/common |
| **crossterm**                    | Interfaccia terminale avanzata (colori, input, ecc.)      | Client               |
| **chrono**                       | Gestione date e orari                                     | Client/Server        |
| **futures**                      | Primitive asincrone                                       | Client               |
| **ctrlc**                        | Gestione segnale di interruzione (CTRL+C)                 | Client/Server        |
| **directories**                  | Utility per directory di sistema                          | Client/Server        |
| **rand**                         | Generazione codici invito casuali                         | Server               |

## Strutture dati principali

| Componente | Struttura        | Descrizione                                                               |
| ---------- | ---------------- | ------------------------------------------------------------------------- |
| Client     | Args             | Parametri da linea di comando (server, nick)                              |
| Server     | State            | Stato globale del server: utenti, gruppi, inviti, canali                  |
| Server     | users_by_nick    | HashMap<String, Uuid> — Nickname → ID utente                              |
| Server     | nicks_by_id      | HashMap<Uuid, String> — ID utente → Nickname                              |
| Server     | groups           | HashMap<String, Group> — Nome gruppo → struttura gruppo                   |
| Server     | invites          | HashMap<String, (String, String)> — Codici invito → (gruppo, nickname)    |
| Server     | clients          | HashMap<Uuid, Tx> — ID utente → canale di comunicazione                   |
| Server     | Group            | Struttura gruppo con members: HashSet<Uuid>                               |
| Server     | Tx / Rx          | Canali Tokio per la comunicazione tra task e client                       |
| Common     | ClientToServer   | Enum dei messaggi dal client al server                                    |
| Common     | ServerToClient   | Enum dei messaggi dal server al client                                    |
| Common     | Struct condivisi | Strutture per serializzazione/deserializzazione (nickname, gruppo, testo) |

## Logs e monitoraggio

- Il server logga % di uso della CPU e runtime ogni 2 minuti in un file chiamato `Server/server_cpu.log`.
- Il logging è gestito in modo asincrono per non bloccare il server.
  ![Esempio logger](/Documentation/imgs/esempio_logs.png)

## Sicurezza e validazione

- Tutti i nickname e nomi gruppo sono validati lato server (lunghezza, caratteri, unicità, parole riservate).
- I messaggi sono serializzati in JSON e controllati.
- Gli errori sono gestiti in modo centralizzato e loggati.

## Dimensione applicativo

| Sistema operativo                     | ruggine-client | ruggine-server |
| ------------------------------------- | -------------- | -------------- |
| Windows 11 (versione debug)           | 6,9 MB         | 7,7 MB         |
| Windows 11 (versione release)         | 3,3 MB         | 3,5 MB         |
| MacOS Sequoia 15.6 (versione debug)   | 13,7 MB        | 15,2 MB        |
| MacOS Sequoia 15.6 (versione release) | 4,2  MB        | 4,3 MB         |

Le dimensioni possono variare leggermente in base alle opzioni di compilazione e alle dipendenze installate. Su MacOS i binari risultano più grandi a causa delle librerie statiche.
