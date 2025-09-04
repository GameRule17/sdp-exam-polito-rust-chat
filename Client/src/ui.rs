/*
Modulo UI: implementa l'interfaccia utente testuale del client.
Gestisce input, output, rendering dei messaggi e interazione con l'utente.
*/

use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

use crossterm::{cursor, event, terminal, ExecutableCommand, QueueableCommand};
use tokio::io::{BufReader, Lines};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;

use ruggine_common::{ServerToClient, ClientToServer};

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
    for m in handshake_msgs { let _ = msg_tx.send(m); }

    // Task che legge dal server e invia testo formattato sul canale
    let read_task = {
        let msg_tx = msg_tx.clone();
        tokio::spawn(async move {
            while let Ok(Some(line)) = reader_lines.next_line().await {
                if let Ok(msg) = serde_json::from_str::<ServerToClient>(&line) {
                    let rendered = render(msg);
                    let _ = msg_tx.send(rendered);
                }
            }
        })
    };

    // Gestione SIGINT (CTRL+C)
    let writer_half_ctrlc = Arc::clone(&writer_half);
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let mut wh = writer_half_ctrlc.lock().await;
        let _ = send(&mut *wh, &ClientToServer::Logout { reason: Some("CTRL+C".to_string()) }).await;
    // OwnedWriteHalf non espone shutdown diretto; drop del writer dopo il logout
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        restore_terminal();
        std::process::exit(0);
    });

    // REPL + interfaccia
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(terminal::EnterAlternateScreen)?;
    stdout.execute(event::EnableMouseCapture)?;
    stdout.execute(cursor::Show)?;
    write!(stdout, "\x1b[?7l")?; // disable line wrap
    stdout.flush()?;

    let prompt = "> ";
    let mut input = String::new();
    let mut messages: Vec<String> = Vec::new();
    let mut scroll_offset: usize = 0;

    let redraw = |stdout: &mut io::Stdout, messages: &Vec<String>, scroll_offset: usize, input: &str| -> anyhow::Result<()> {
        let (cols, rows) = terminal::size()?;
        let usable_rows = rows.saturating_sub(1);
        let total = messages.len();
        let end_index = total.saturating_sub(scroll_offset);
        let start_index = end_index.saturating_sub(usable_rows as usize);
        stdout.queue(terminal::Clear(terminal::ClearType::All))?;
        let visible_messages = &messages[start_index..end_index];
        for (i, line) in visible_messages.iter().enumerate() {
            stdout.queue(cursor::MoveTo(0, i as u16))?;
            stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
            let mut display = line.clone();
            if display.len() > cols as usize { display.truncate(cols as usize); }
            use crossterm::style::{Color, SetForegroundColor, ResetColor};
            let color = if display.starts_with("[error]") { Some(Color::Red) }
                else if display.starts_with("[server]") { Some(Color::Green) }
                else { None };
            if let Some(c) = color { stdout.queue(SetForegroundColor(c))?; }
            write!(stdout, "{}", display)?;
            if color.is_some() { stdout.queue(ResetColor)?; }
        }
        stdout.queue(cursor::MoveTo(0, rows.saturating_sub(1)))?;
        stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
        let mut inp = format!("{}{}", prompt, input);
        if inp.len() > cols as usize { inp.truncate(cols as usize); }
        write!(stdout, "{}", inp)?;
        stdout.queue(cursor::Show)?;
        stdout.flush()?;
        Ok(())
    };
    redraw(&mut stdout, &messages, scroll_offset, &input)?;

    loop {
        tokio::select! {
            maybe_msg = msg_rx.recv() => {
                if let Some(txt) = maybe_msg {
                    messages.push(txt);
                    if scroll_offset == 0 { redraw(&mut stdout, &messages, scroll_offset, &input)?; }
                }
            }
            _ = tokio::task::yield_now() => {
                if event::poll(Duration::from_millis(30))? {
                    match event::read()? {
                        event::Event::Key(k) => {
                            use crossterm::event::{KeyCode, KeyModifiers, KeyEventKind};
                            if k.kind == KeyEventKind::Press {
                                match k.code {
                                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                                        messages.push("Uscita dal client...".into());
                                        redraw(&mut stdout, &messages, scroll_offset, &input)?;
                                        {
                                            let mut wh = writer_half.lock().await;
                                            let _ = send(&mut *wh, &ClientToServer::Logout { reason: Some("CTRL+C".into()) }).await;
                                        }
                                        break;
                                    }
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
                        event::Event::Mouse(m) => {
                            use crossterm::event::MouseEventKind;
                            match m.kind {
                                MouseEventKind::ScrollUp => {
                                    if messages.len() > 0 {
                                        let max_off = messages.len().saturating_sub(1);
                                        if scroll_offset < max_off { scroll_offset += 1; redraw(&mut stdout, &messages, scroll_offset, &input)?; }
                                    }
                                }
                                MouseEventKind::ScrollDown => {
                                    if scroll_offset > 0 { scroll_offset -= 1; redraw(&mut stdout, &messages, scroll_offset, &input)?; }
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
