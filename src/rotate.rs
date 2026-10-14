//! Log file handling and rotation engine supporting size-based and time-based rotation.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use chrono::Local;

#[derive(Debug, Clone)]
pub struct LogRotationConfig {
    pub file_path: PathBuf,
    pub append: bool,
    pub rotate_enabled: bool,
    pub max_bytes: Option<u64>,
    pub max_seconds: Option<u64>,
}

pub struct RotatedLogWriter {
    config: LogRotationConfig,
    current_file: Option<File>,
    current_size: u64,
    created_at: SystemTime,
}

impl RotatedLogWriter {
    pub fn new(config: LogRotationConfig) -> io::Result<Self> {
        let mut writer = Self {
            config,
            current_file: None,
            current_size: 0,
            created_at: SystemTime::now(),
        };

        writer.rotate_if_needed()?;
        writer.open_current_file()?;

        Ok(writer)
    }

    /// Checks if rotation conditions are met and rotates file if necessary
    pub fn rotate_if_needed(&mut self) -> io::Result<()> {
        if !self.config.rotate_enabled || !self.config.file_path.exists() {
            return Ok(());
        }

        let metadata = match fs::metadata(&self.config.file_path) {
            Ok(m) => m,
            Err(_) => return Ok(()),
        };

        let mut needs_rotation = false;

        // Check byte size
        if let Some(max_b) = self.config.max_bytes {
            if metadata.len() >= max_b {
                needs_rotation = true;
            }
        }

        // Check time elapsed
        if let Some(max_s) = self.config.max_seconds {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                    if elapsed.as_secs() >= max_s {
                        needs_rotation = true;
                    }
                }
            }
        }

        if needs_rotation {
            self.perform_rotation()?;
        }

        Ok(())
    }

    fn perform_rotation(&mut self) -> io::Result<()> {
        // Close current file handle
        self.current_file = None;

        let path = &self.config.file_path;
        if !path.exists() {
            return Ok(());
        }

        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("log");
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("log");

        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        let rotated_name = format!("{file_stem}-{timestamp}.{ext}");
        let rotated_path = parent.join(rotated_name);

        log::info!("Rotating log file {:?} to {:?}", path, rotated_path);
        fs::rename(path, rotated_path)?;

        self.current_size = 0;
        self.created_at = SystemTime::now();

        Ok(())
    }

    fn open_current_file(&mut self) -> io::Result<()> {
        if let Some(parent) = self.config.file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut options = OpenOptions::new();
        options.create(true).write(true);

        if self.config.append {
            options.append(true);
        } else {
            options.truncate(true);
        }

        let file = options.open(&self.config.file_path)?;
        let metadata = file.metadata()?;
        self.current_size = metadata.len();
        self.current_file = Some(file);

        Ok(())
    }

    pub fn get_file_handle(&self) -> io::Result<File> {
        self.config.file_path.parent().map(fs::create_dir_all);

        let mut options = OpenOptions::new();
        options.create(true).write(true);
        if self.config.append {
            options.append(true);
        } else {
            options.truncate(true);
        }

        options.open(&self.config.file_path)
    }
}
