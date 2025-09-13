/*
Modulo UI: implementa l'interfaccia utente testuale del client.
Gestisce input, output, rendering dei messaggi e interazione con l'utente.
*/

use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

use crossterm::{cursor, event, terminal, ExecutableCommand, QueueableCommand};
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use crossterm::event::{KeyCode, KeyModifiers, KeyEventKind};
use tokio::io::{BufReader, Lines};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;

use ruggine_common::{ClientToServer, ServerToClient};

use crate::commands::handle_command;
use crate::messages::render;
use crate::net::send;
use crate::terminal::restore_terminal;

pub async fn run_ui(
    mut reader_lines: Lines<BufReader<OwnedReadHalf>>,
    writer_half: Arc<Mutex<OwnedWriteHalf>>,
    my_nick: String,
    handshake_msgs: Vec<String>,
) -> anyhow::Result<()> {
    // Manteniamo un canale per inoltrare i messaggi del server all'interfaccia utente
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    /*
        Invia tutti i messaggi ricevuti durante l’handshake (ad esempio messaggi di benvenuto o conferma login)
        nel canale, così saranno visualizzati subito nell’interfaccia utente appena parte il ciclo principale
    */
    for m in handshake_msgs {
        let _ = msg_tx.send(m);
    }

    // Task che legge dal server e invia testo formattato sul canale
    let read_task = {
        let msg_tx = msg_tx.clone();

        // Spawno un task asincrono per visualizzare i messaggi ricevuti
        tokio::spawn(async move {
            while let Ok(Some(line)) = reader_lines.next_line().await {
                if let Ok(msg) = serde_json::from_str::<ServerToClient>(&line) {
                    let rendered = render(msg); // applico funzione render da messages.rs
                    let _ = msg_tx.send(rendered);
                }
            }
        })
    };

    // Gestione CTRL+C
    let writer_half_ctrlc = Arc::clone(&writer_half);
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let mut wh = writer_half_ctrlc.lock().await;
        let _ = send(
            &mut *wh,
            &ClientToServer::Logout {
                reason: Some("CTRL+C".to_string()),
            },
        ).await;
        
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        restore_terminal();
        std::process::exit(0);
    });

    /*
        Inizio gestione ciclo REPL (Read Eval Print Loop) e interfaccia utente:
        visualizzare i messaggi, gestire lo scrolling, prompt
    */
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(terminal::EnterAlternateScreen)?;
    stdout.execute(event::EnableMouseCapture)?;
    stdout.execute(cursor::Show)?;
    write!(stdout, "\x1b[?7l")?;
    stdout.flush()?;

    let prompt = "> ";
    let mut input = String::new();
    let mut messages: Vec<String> = Vec::new();
    let mut scroll_offset: usize = 0;


    // Funzione lambda di ridisegno della ui a seguito di modifiche di dimensione, scroll, ...
    let redraw = 
    |stdout: &mut io::Stdout, messages: &Vec<String>, scroll_offset: usize, input: &str| -> anyhow::Result<()> {

        let (cols, rows) = terminal::size()?; // Ottenimento dimensioni attuali del terminale
        let usable_rows = rows.saturating_sub(1); // Lascia una riga libera per il prompt di input
        let total = messages.len(); // Conta quanti messaggi totali ci sono da visualizzare

        // Calcola quanto si può scrollare al massimo: se ci sono più messaggi di quelli che entrano
        // nello schermo, questa variabile sarà > 0, altrimenti sarà 0.
        let max_scroll = total.saturating_sub(usable_rows as usize); 

        // Se l'utente ha scrollato più del massimo consentito, viene limitato al massimo scroll possibile
        let eff_scroll = scroll_offset.min(max_scroll);

        let end_index = total.saturating_sub(eff_scroll);
        let start_index = end_index.saturating_sub(usable_rows as usize);

        // Pulire tutto lo schermo del terminale prima di ridisegnare i messaggi
        stdout.queue(terminal::Clear(terminal::ClearType::All))?;

        // Seleziona la “finestra” di messaggi che devono essere effettivamente mostrati a schermo,
        // in base allo scroll e alle dimensioni del terminale
        let visible_messages = &messages[start_index..end_index];

        // PER OGNI messaggio da visualizzare
        for (i, line) in visible_messages.iter().enumerate() {
            
            // Sposta il cursore all’inizio della riga i
            stdout.queue(cursor::MoveTo(0, i as u16))?;
            // Pulisce tutta la riga corrente
            stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;

            let mut display = line.clone();
            // Se il messaggio è più lungo della larghezza del terminale, lo tronca per evitare che sbordi 
            // o causi problemi di visualizzazione
            if display.len() > cols as usize {
                display.truncate(cols as usize);
            }
            
            // Scelta del colore con cui visualizzare il messaggio in base alla tipologia
            let color = if display.starts_with("[error]") {
                Some(Color::Red)
            } else if display.starts_with("[server]") {
                Some(Color::Green)
            } else {
                None
            };
            if let Some(c) = color {
                stdout.queue(SetForegroundColor(c))?;
            }

            // Scrittura della riga con il colore desiderato
            write!(stdout, "{}", display)?;

            // Se era stato impostato un colore, si resetta
            if color.is_some() {
                stdout.queue(ResetColor)?;
            }
        }


        stdout.queue(cursor::MoveTo(0, rows.saturating_sub(1)))?;
        stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
        let mut inp = format!("{}{}", prompt, input);
        if inp.len() > cols as usize {
            inp.truncate(cols as usize);
        }
        write!(stdout, "{}", inp)?;
        stdout.queue(cursor::Show)?;
        stdout.flush()?;
        Ok(())
    };
    redraw(&mut stdout, &messages, scroll_offset, &input)?;

    loop {
        // select!: Attesa contemporanea di più eventi asincroni e consecutiva esecuzione non appena uno
        // di essi si verifica
        tokio::select! {
            // Ricezione di un messaggio dal canale
            maybe_msg = msg_rx.recv() => {
                if let Some(txt) = maybe_msg {
                    messages.push(txt);
                    // Se siamo ancorati in fondo (scroll_offset == 0) ridisegniamo subito.
                    // Se l'utente ha scrollato verso l'alto manteniamo la sua posizione relativa
                    if scroll_offset == 0 {
                        redraw(&mut stdout, &messages, scroll_offset, &input)?;
                    } else {
                        // Clamp dello scroll se il numero di messaggi non giustifica più l'offset corrente
                        let (_, rows) = terminal::size()?;
                        let usable_rows = rows.saturating_sub(1) as usize;
                        let total = messages.len();
                        let max_scroll = total.saturating_sub(usable_rows);
                        if scroll_offset > max_scroll { scroll_offset = max_scroll; }
                        redraw(&mut stdout, &messages, scroll_offset, &input)?;
                    }
                }
            }
            // yield_now permette al task corrente di cedere volontariamente il controllo, 
            // lasciando che altri task pronti vengano eseguiti prima di riprendere
            _ = tokio::task::yield_now() => {
                // controllo nuovo input ogni 30 secondi
                if event::poll(Duration::from_millis(30))? {

                    // controlla se c'è un evento
                    match event::read()? {

                        // se il tipo di evento avvenuto è di tasto premuto, si controlla quale
                        event::Event::Key(k) => {
                            if k.kind == KeyEventKind::Press {

                                match k.code {
                                    
                                    // Gestione CTRL+C
                                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                                        messages.push("Uscita dal client...".into());
                                        redraw(&mut stdout, &messages, scroll_offset, &input)?;
                                        {
                                            let mut wh = writer_half.lock().await;
                                            let _ = send(&mut *wh, &ClientToServer::Logout { reason: Some("CTRL+C".into()) }).await;
                                        }
                                        break;
                                    }

                                    // Gestione invio
                                    KeyCode::Enter => {
                                        let line = input.trim().to_string();
                                        if !line.is_empty() {
                                            messages.push(format!("> {}", line));
                                            let produced = handle_command(&line, &writer_half, &my_nick).await?;
                                            if !produced.is_empty() { messages.extend(produced); }
                                        }
                                        input.clear();
                                        if scroll_offset == 0 { redraw(&mut stdout, &messages, scroll_offset, &input)?; }
                                    }

                                    // Gestione scrittura di un carattere
                                    KeyCode::Char(ch) => {
                                        input.push(ch);
                                        redraw(&mut stdout, &messages, scroll_offset, &input)?;
                                    }
                                    KeyCode::Backspace => { input.pop(); redraw(&mut stdout, &messages, scroll_offset, &input)?; }
                                    KeyCode::Esc => { input.clear(); redraw(&mut stdout, &messages, scroll_offset, &input)?; }
                                    _ => {}
                                }
                            }
                        }
                        event::Event::Paste(p) => { input.push_str(&p); redraw(&mut stdout, &messages, scroll_offset, &input)?; }

                        // Gestione ScrollUp & ScrollDown del Mouse per riadattare la lista dei messaggi da visualizzare
                        // Di fatto, uno ScrollUp determina una salita in alto per vedere la chat passata
                        event::Event::Mouse(m) => {
                            use crossterm::event::MouseEventKind;
                            match m.kind {
                                MouseEventKind::ScrollUp => {
                                    if !messages.is_empty() {
                                        let (_, rows) = terminal::size()?;
                                        let usable_rows = rows.saturating_sub(1) as usize;
                                        let total = messages.len();
                                        if total > usable_rows { 
                                            let max_scroll = total - usable_rows; 
                                            if scroll_offset < max_scroll {
                                                scroll_offset += 1;
                                                redraw(&mut stdout, &messages, scroll_offset, &input)?;
                                            }
                                        }
                                    }
                                }
                                MouseEventKind::ScrollDown => {
                                    if scroll_offset > 0 {
                                        scroll_offset -= 1;
                                        redraw(&mut stdout, &messages, scroll_offset, &input)?;
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    stdout.execute(event::DisableMouseCapture)?;
    stdout.execute(cursor::Show)?;
    write!(stdout, "\x1b[?7h")?; // re-enable wrap
    stdout.execute(terminal::LeaveAlternateScreen)?;
    stdout.flush()?;
    println!("{} ti sei disconnesso correttamente", my_nick);
    read_task.abort();
    Ok(())
}
