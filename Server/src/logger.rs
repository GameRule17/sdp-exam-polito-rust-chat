use sysinfo::{System, Pid};
use chrono::Local;
use tokio::time::{sleep, Duration};
use std::fs::OpenOptions;
use std::io::Write;
use anyhow::Result;

pub async fn start_cpu_logger(log_path: &str) -> Result<()> {
    let pid = Pid::from(std::process::id() as usize);
    let mut sys = System::new_all();

    loop {
        sys.refresh_processes();

        if let Some(proc) = sys.process(pid) {
            let cpu_usage = proc.cpu_usage();
            let run_time = proc.run_time() / 60;

            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

            let log_line = format!(
                "[{}] CPU: {:.2}% | Run Time: {} min\n",
                timestamp, cpu_usage, run_time
            );

            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)?;
            file.write_all(log_line.as_bytes())?;
        }

        sleep(Duration::from_secs(5)).await;
    }
}
