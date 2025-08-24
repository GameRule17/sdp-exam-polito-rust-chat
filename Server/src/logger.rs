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
            // Formattiamo cpu_usage in larghezza fissa con 2 decimali e virgola come separatore
            // es: " 300,00" o "   0,05" in modo che tutte le righe siano allineate
            let cpu_str = format!("{:7.2}", cpu_usage).replace('.', ",");
            // run_time (minuti) in larghezza fissa per tenere la colonna allineata
            let run_time_str = format!("{:3}", run_time);

            let log_line = format!(
                "[{}] CPU: {}% | Run Time: {} min\n",
                timestamp, cpu_str, run_time_str
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
