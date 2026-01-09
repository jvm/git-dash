use std::fs;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct Logger {
    file: Mutex<fs::File>,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn init_logger(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::create(path)?;
    LOGGER
        .set(Logger {
            file: Mutex::new(file),
        })
        .map_err(|_| "logger already initialized")?;
    Ok(())
}

pub fn log_debug(message: &str) {
    let Some(logger) = LOGGER.get() else { return };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    let line = format!("[{}.{:03}] {message}\n", now.as_secs(), now.subsec_millis());
    if let Ok(mut file) = logger.file.lock() {
        let _ = file.write_all(line.as_bytes());
    }
}
