use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use once_cell::sync::Lazy;

/// Global log file handle
static LOG_FILE: Lazy<Mutex<Option<std::fs::File>>> = Lazy::new(|| Mutex::new(None));

/// Global debug mode flag
static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

/// Get the log directory path (~/.cache/tmx/)
fn log_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|p| p.join(".cache").join("tmx"))
}

/// Get the log file path (~/.cache/tmx/tmx.log)
fn log_path() -> Option<PathBuf> {
    log_dir().map(|p| p.join("tmx.log"))
}

/// Initialize the logger, creating the log directory if needed.
/// Should be called once at startup.
///
/// # Arguments
/// * `verbose` - If true, enables debug level logging
pub fn init(verbose: bool) {
    // Set debug mode
    DEBUG_MODE.store(verbose, Ordering::SeqCst);

    let Some(dir) = log_dir() else {
        return;
    };

    // Create log directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("Warning: Could not create log directory: {}", e);
        return;
    }

    let Some(path) = log_path() else {
        return;
    };

    // Open log file in append mode
    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => {
            let mut guard = LOG_FILE.lock().unwrap();
            *guard = Some(file);
            drop(guard);
            let mode = if verbose { "debug" } else { "info" };
            log(&format!("--- tmx session started (log level: {}) ---", mode));
        }
        Err(e) => {
            eprintln!("Warning: Could not open log file: {}", e);
        }
    }
}

/// Check if debug mode is enabled
pub fn is_debug() -> bool {
    DEBUG_MODE.load(Ordering::SeqCst)
}

/// Log a message to the log file with timestamp
pub fn log(message: &str) {
    let Ok(mut guard) = LOG_FILE.lock() else {
        return;
    };

    let Some(ref mut file) = *guard else {
        return;
    };

    // Format timestamp as ISO-like: YYYY-MM-DD HH:MM:SS
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            // Simple UTC timestamp formatting
            let days = secs / 86400;
            let time_secs = secs % 86400;
            let hours = time_secs / 3600;
            let minutes = (time_secs % 3600) / 60;
            let seconds = time_secs % 60;

            // Calculate date from days since epoch (1970-01-01)
            let (year, month, day) = days_to_ymd(days);
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                year, month, day, hours, minutes, seconds
            )
        })
        .unwrap_or_else(|_| "unknown".to_string());

    // Format: [timestamp] message
    let _ = writeln!(file, "[{}] {}", timestamp, message);
    let _ = file.flush();
}

/// Convert days since Unix epoch to year, month, day
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Simplified calculation - good enough for logging
    let mut remaining = days as i64;
    let mut year = 1970i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }

    let leap = is_leap_year(year);
    let days_in_months: [i64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1i64;
    for days_in_month in days_in_months {
        if remaining < days_in_month {
            break;
        }
        remaining -= days_in_month;
        month += 1;
    }

    (year as u64, month as u64, (remaining + 1) as u64)
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Log a debug message (only logged when -v flag is used)
pub fn debug(message: &str) {
    if is_debug() {
        log(&format!("[DEBUG] {}", message));
    }
}

/// Log an info message
pub fn info(message: &str) {
    log(&format!("[INFO] {}", message));
}

/// Log an error message
pub fn error(message: &str) {
    log(&format!("[ERROR] {}", message));
}

