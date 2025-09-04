# MANUALE UTENTE

## Introduzione
Questa guida spiega come installare, avviare e utilizzare la chat client-server sviluppata in Rust. Il sistema permette di comunicare in gruppi, inviare messaggi globali e gestire utenti tramite comandi da terminale.

---

## Requisiti
- Sistema operativo: Windows, Linux o MacOS
- Rust installato ([https://rustup.rs](https://rustup.rs))
- Connessione di rete locale

---

## Installazione
1. Clona il repository:
   ```
   git clone <URL_DEL_REPO>
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

---

## Avvio del Server
1. Vai nella cartella `Server`:
   ```
   cd Server
   ```
2. Avvia il server:
   ```
   cargo run --release
   ```
   Il server si mette in ascolto sulla porta di default (es. 127.0.0.1:7000).

---

## Avvio del Client
1. Vai nella cartella `Client`:
   ```
   cd ../Client
   ```
2. Avvia il client:
   ```
   cargo run --release -- --server 127.0.0.1:7000 --nick TuoNick
   ```
   Se non specifichi il nickname, ti verrà richiesto all'avvio.

---

## Utilizzo del Client

Dopo l'avvio, puoi interagire tramite i seguenti comandi:

- `/help` o `/` : Visualizza il menu dei comandi
- `/create <nome>` : Crea un nuovo gruppo
- `/invite <gruppo> <nick>` : Invita un utente in un gruppo
- `/join <gruppo> <codice>` : Unisciti a un gruppo con codice invito
- `/leave <gruppo>` : Esci da un gruppo
- `/users` : Mostra tutti gli utenti connessi
- `/groups` : Mostra i gruppi di appartenenza
- `/msg <gruppo> <testo>` : Invia un messaggio a un gruppo
- `/quit` : Esci dal client

Puoi anche inviare messaggi globali semplicemente scrivendo il testo e premendo invio.

---

## Esempio di sessione
```
> /create amici
> /invite amici luca
> /msg amici Ciao a tutti!
> /users
> /quit
```

---

## Risoluzione dei problemi
- **Errore di connessione:** Verifica che il server sia avviato e l'indirizzo sia corretto.
- **Nickname rifiutato:** Scegli un nickname valido (non "server" o "client", solo lettere e numeri, max 32 caratteri).
- **Comando non riconosciuto:** Usa `/help` per vedere la sintassi corretta.
- **Crash o chiusura improvvisa:** Consulta i log o riprova ad avviare il programma.

---

## FAQ
- **Posso usare più client contemporaneamente?** Sì, basta avviare più istanze del client.
- **Come cambio nickname?** Riavvia il client con un nuovo nickname.
- **Come creo un gruppo privato?** Usa `/create` e invita solo chi vuoi.

---

## Supporto
Per problemi o domande, contatta il progettista o consulta il MANUALE_PROGETTISTA.md.
