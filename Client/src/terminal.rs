/*
Modulo Terminal: gestisce le operazioni sul terminale locale, come la modalità raw, il prompt del nickname e il ripristino dello stato.
*/

use std::io::{self, Write};

pub fn restore_terminal() {
    let _ = crossterm::terminal::disable_raw_mode();
    let mut stdout = std::io::stdout();
    let _ = crossterm::execute!(
        stdout,
        crossterm::event::DisableMouseCapture,
        crossterm::cursor::Show,
        crossterm::terminal::LeaveAlternateScreen
    );
    let _ = write!(stdout, "\x1b[?7h"); // riabilita wrapping delle righe
    let _ = stdout.flush();
}

pub fn prompt_nick() -> anyhow::Result<String> {
    loop {
        print!("Scegli un nickname: ");
        io::stdout().flush()?;
        let mut s = String::new();
        std::io::stdin().read_line(&mut s)?;
        let s = s.trim();
        if s.is_empty() {
            eprintln!("Il nickname non può essere vuoto.");
            continue;
        }
        return Ok(s.to_string());
    }
}
