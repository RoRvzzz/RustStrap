/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

pub struct Logger {
    file: Mutex<Option<File>>,
    history: Mutex<Vec<String>>,
    pub file_location: Mutex<Option<String>>,
    initialized: Mutex<bool>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            file: Mutex::new(None),
            history: Mutex::new(Vec::new()),
            file_location: Mutex::new(None),
            initialized: Mutex::new(false),
        }
    }

    pub fn initialize(&self, base_dir: &Path) {
        let dir = base_dir.join("Logs");
        let timestamp = chrono_utc_filename();
        let filename = format!("Ruststrap_{timestamp}.log");
        let location = dir.join(&filename);

        if *self.initialized.lock().unwrap() {
            return;
        }

        let _ = fs::create_dir_all(&dir);

        if location.exists() {
            return;
        }

        match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&location)
        {
            Ok(f) => {
                *self.file.lock().unwrap() = Some(f);
                *self.initialized.lock().unwrap() = true;
                *self.file_location.lock().unwrap() = Some(location.to_string_lossy().to_string());
                let history = self.history.lock().unwrap();
                if !history.is_empty() {
                    let joined = history.join("\r\n");
                    self.write_to_log_inner(&joined);
                }
                self.write_line("Logger::Initialize", "Finished initializing!");
                self.cleanup_old_logs(&dir);
            }
            Err(_) => {}
        }
    }

    pub fn write_line(&self, ident: &str, message: &str) {
        let timestamp = chrono_utc_timestamp();
        let line = format!("{timestamp} [{ident}] {message}");
        let user_profile = std::env::var("USERPROFILE").unwrap_or_default();
        let sanitized = if !user_profile.is_empty() {
            line.replace(&user_profile, "%UserProfile%")
        } else {
            line.clone()
        };
        self.history.lock().unwrap().push(sanitized.clone());
        if *self.initialized.lock().unwrap() {
            self.write_to_log_inner(&sanitized);
        }
    }

    pub fn write_exception(&self, ident: &str, err: &dyn std::fmt::Display) {
        self.write_line(ident, &format!("{err}"));
    }
    pub fn as_document(&self) -> String {
        self.history.lock().unwrap().join("\n")
    }
    fn write_to_log_inner(&self, message: &str) {
        if let Some(f) = self.file.lock().unwrap().as_mut() {
            let _ = write!(f, "{message}\r\n");
            let _ = f.flush();
        }
    }
    fn cleanup_old_logs(&self, log_dir: &Path) {
        let threshold =
            std::time::SystemTime::now() - std::time::Duration::from_secs(7 * 24 * 3600);

        if let Ok(entries) = fs::read_dir(log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                if let Ok(meta) = path.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if modified < threshold {
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }
}

fn chrono_utc_timestamp() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}

fn chrono_utc_filename() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn logger_buffered_write() {
        let logger = Logger::new();
        logger.write_line("Test", "Hello");
        logger.write_line("Test", "World");
        let doc = logger.as_document();
        assert!(doc.contains("Hello"));
        assert!(doc.contains("World"));
    }
}
