use sysinfo::{System, Pid};
use chrono::Local;
use tokio::time::{sleep, Duration};
use std::fs::OpenOptions;
use std::io::Write;
use anyhow::Result;

pub async fn start_cpu_logger(log_path: &str) -> Result<()> {
    // Ottengo il pid del processo server
    let pid = Pid::from(std::process::id() as usize);
    // Crea un'istanza mutabile della struttura System della crate sysinfo,
    // inizializzando la raccolta di tutte le informazioni di sistema disponibili
    let mut sys = System::new_all();

    loop {
        sleep(Duration::from_secs(120)).await;

        // Ottiene tutti i processi del sistema e ne aggiorna le informazioni
        sys.refresh_processes();

        // Seleziona il processo server dalla lista dei processi del sistema
        if let Some(proc) = sys.process(pid) {

            // Acquisisce i dati relativi al processo
            let cpu_usage = proc.cpu_usage();
            let run_time = proc.run_time() / 60;

            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            
            // Formatta la stringa da inserire come append nel file
            let log_line = format!(
                "[{}] CPU: {:.2}% | Run Time: {} min\n",
                timestamp, cpu_usage, run_time
            );
            
            // Apertura file con le seguenti opzioni
            let mut file = OpenOptions::new()
                .create(true) // Se esiste già lo apre solamente
                .append(true) // Modalità append
                .open(log_path)?; // Apre il file in log_path passato come parametro
            file.write_all(log_line.as_bytes())?; // Scrittura dell'intero buffer
        }
    }
}
