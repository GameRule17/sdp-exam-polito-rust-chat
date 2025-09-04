# MANUALE DEL PROGETTISTA

## Introduzione
Questo progetto implementa una chat client-server in Rust, con architettura modulare e separazione tra client, server e common. Il sistema supporta gruppi, inviti, messaggi globali e logging delle risorse.

## Struttura delle cartelle

```
G26/
├── Client/         # Codice sorgente del client
├── Server/         # Codice sorgente del server
├── common/         # Tipi condivisi tra client e server
├── target/         # Output di compilazione
```

### Client/src
- **args.rs**: parsing degli argomenti da riga di comando
- **commands.rs**: gestione dei comandi utente
- **handshake.rs**: logica di registrazione e handshake
- **main.rs**: entrypoint, orchestrazione moduli
- **messages.rs**: formattazione messaggi dal server
- **net.rs**: invio messaggi al server
- **terminal.rs**: gestione terminale e input nickname
- **ui.rs**: interfaccia utente, REPL, input/output

### Server/src
- **args.rs**: parsing argomenti server
- **connection.rs**: gestione connessioni TCP
- **logger.rs**: logging CPU e runtime
- **main.rs**: entrypoint server
- **server.rs**: ciclo principale, accettazione client
- **state.rs**: stato condiviso server
- **util.rs**: utility generiche
- **validation.rs**: validazione nickname/gruppi
- **commands/**: moduli per ogni comando server

### common/src
- **lib.rs**: definizione tipi condivisi (ClientToServer, ServerToClient, errori)

## Scelte tecnologiche
- **Rust**: sicurezza, concorrenza, performance
- **Tokio**: runtime asincrono per networking
- **Clap**: parsing argomenti CLI
- **Crossterm**: gestione terminale cross-platform
- **Serde**: serializzazione JSON
- **Sysinfo**: monitoraggio risorse server

## Flusso di esecuzione
- Il server avvia il logger, si mette in ascolto su una porta e accetta connessioni.
- Il client si connette, effettua handshake (nickname, id), riceve messaggi e invia comandi.
- La comunicazione avviene tramite messaggi JSON serializzati (vedi common/lib.rs).
- I comandi sono gestiti in modo modulare sia lato client che server.

## Descrizione dei moduli principali

### Client
- **args.rs**: Definisce la struct Args per i parametri da CLI (server, nick).
- **commands.rs**: Funzione handle_command che interpreta la stringa utente e invia il comando appropriato al server.
- **handshake.rs**: Gestisce la registrazione utente, con retry se il nick non è accettato.
- **main.rs**: Avvia la connessione, effettua handshake, lancia la UI.
- **messages.rs**: Converte i messaggi ServerToClient in stringhe leggibili per l'utente.
- **net.rs**: Funzione send per inviare messaggi serializzati al server.
- **terminal.rs**: Funzioni per ripristino terminale e richiesta nickname.
- **ui.rs**: Gestisce il ciclo REPL, input da tastiera, output, scroll, colori.

### Server
- **args.rs**: Parametri di avvio server (porta, ecc.).
- **connection.rs**: Gestione handshake, lettura/scrittura TCP.
- **logger.rs**: Log periodico di CPU e runtime su file.
- **main.rs**: Avvio server, setup logger, shutdown pulito.
- **server.rs**: Loop principale, accettazione client, dispatch comandi.
- **state.rs**: Stato condiviso (utenti, gruppi, messaggi).
- **util.rs**: Utility generiche.
- **validation.rs**: Regole di validazione nickname/gruppi.
- **commands/**: Ogni file implementa la logica di un comando (es. create_group, invite, join_group, ecc.).

### common
- **lib.rs**: Definisce i tipi di messaggio, errori, e le strutture condivise tra client e server.

## Convenzioni di codice
- Ogni modulo ha un commento iniziale che ne descrive lo scopo.
- I commenti riga spiegano la logica delle funzioni e dei blocchi principali.
- I nomi delle funzioni e variabili sono descrittivi e in inglese.
- I comandi e le interfacce utente sono documentati nel codice e nel manuale utente.

## Estendibilità
- Per aggiungere un nuovo comando:
  1. Definire il tipo in common/lib.rs.
  2. Implementare la logica lato server in commands/.
  3. Gestire il comando lato client in commands.rs.
  4. Aggiornare la UI e la documentazione.
- Per aggiungere nuove regole di validazione, modificare validation.rs.
- Per nuove funzionalità, creare moduli separati e aggiornare main.rs per orchestrazione.

## Test
- I test possono essere aggiunti nella cartella `Client/tests` e `Server/tests`.
- Usare crate come `assert_cmd` e `predicates` per test end-to-end.
- Testare casi di errore, input non valido, e flussi di comando.

## Dipendenze
- Vedi i file Cargo.toml in Client, Server e common per la lista completa.

## Note finali
- Mantenere la documentazione aggiornata.
- Seguire le convenzioni di commento e modularità.
- Per domande o estensioni, consultare i commenti nei file e la sezione Estendibilità.
