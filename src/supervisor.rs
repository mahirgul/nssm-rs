//! Process supervision engine: process execution, stdout/stderr redirection,
//! restart throttling, and graceful shutdown.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use anyhow::{Context, Result};

use crate::config::{AppExitAction, ServiceConfig};
use crate::rotate::{LogRotationConfig, RotatedLogWriter};

pub struct Supervisor {
    config: ServiceConfig,
    running: Arc<AtomicBool>,
}

impl Supervisor {
    pub fn new(config: ServiceConfig, running: Arc<AtomicBool>) -> Self {
        Self { config, running }
    }

    /// Run the supervisor loop. Blocks until shutdown or application exit according to policy.
    pub fn run(&mut self) -> Result<()> {
        let mut crash_count: u32 = 0;

        log::info!(
            "Starting supervisor for service '{}' [Application: {}]",
            self.config.name,
            self.config.application
        );

        while self.running.load(Ordering::Relaxed) {
            let start_time = Instant::now();

            let mut child = match self.spawn_process() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to spawn child process: {e}");
                    crash_count += 1;
                    let throttle_ms = self.calculate_throttle_ms(crash_count);
                    log::warn!("Throttling restart for {throttle_ms} ms due to spawn error");
                    thread::sleep(Duration::from_millis(throttle_ms));
                    continue;
                }
            };

            let pid = child.id();
            log::info!("Child process spawned successfully with PID {pid}");

            // Monitor loop: check if child exited or if stop was signaled
            let status = loop {
                match child.try_wait() {
                    Ok(Some(exit_status)) => break exit_status,
                    Ok(None) => {
                        if !self.running.load(Ordering::Relaxed) {
                            log::info!("Stop requested. Terminating child PID {pid}");
                            self.kill_process_gracefully(&mut child);
                            return Ok(());
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(e) => {
                        log::error!("Error waiting on child PID {pid}: {e}");
                        break child.wait()?;
                    }
                }
            };

            let elapsed = start_time.elapsed();
            log::info!(
                "Child process PID {pid} exited with status {:?} after {:.2}s",
                status,
                elapsed.as_secs_f32()
            );

            // If shutdown was requested while child exited, stop now
            if !self.running.load(Ordering::Relaxed) {
                break;
            }

            // Check if process ran longer than throttle threshold
            let throttle_threshold = Duration::from_millis(self.config.app_throttle);
            if elapsed >= throttle_threshold {
                crash_count = 0; // Reset crash counter if it ran steadily
            } else {
                crash_count += 1;
                let backoff_ms = self.calculate_throttle_ms(crash_count);
                log::warn!(
                    "Process died rapidly ({:?} < {:?}). Crash count: {}. Throttling restart for {}ms",
                    elapsed,
                    throttle_threshold,
                    crash_count,
                    backoff_ms
                );
                thread::sleep(Duration::from_millis(backoff_ms));
            }

            // Apply AppRestartDelay if configured
            if self.config.app_restart_delay > 0 {
                thread::sleep(Duration::from_millis(self.config.app_restart_delay));
            }

            // Check exit action policy
            match self.config.app_exit {
                AppExitAction::Restart => {
                    log::info!("Restarting service '{}'...", self.config.name);
                    continue;
                }
                AppExitAction::Ignore | AppExitAction::Exit => {
                    log::info!("Service exit action configured as {:?}. Exiting.", self.config.app_exit);
                    break;
                }
                AppExitAction::Suicide => {
                    log::info!("Service exit action configured as Suicide. Terminating immediately.");
                    std::process::exit(status.code().unwrap_or(1));
                }
            }
        }

        Ok(())
    }

    fn spawn_process(&self) -> Result<Child> {
        let mut cmd = Command::new(&self.config.application);

        // Parameters
        if let Some(params_str) = &self.config.app_parameters {
            let args = parse_cmdline(params_str);
            cmd.args(args);
        }

        // Working directory
        if let Some(dir) = &self.config.app_directory {
            cmd.current_dir(dir);
        }

        // Environment variables
        for (k, v) in &self.config.app_environment_extra {
            cmd.env(k, v);
        }

        // Stdout redirection
        if let Some(out_path) = &self.config.app_stdout {
            let rot_cfg = LogRotationConfig {
                file_path: PathBuf::from(out_path),
                append: self.config.app_stdout_append,
                rotate_enabled: self.config.app_rotate_files,
                max_bytes: self.config.app_rotate_bytes,
                max_seconds: self.config.app_rotate_seconds,
            };
            let writer = RotatedLogWriter::new(rot_cfg)
                .context("Failed to open stdout log destination")?;
            let file = writer.get_file_handle()?;
            cmd.stdout(Stdio::from(file));
        }

        // Stderr redirection
        if let Some(err_path) = &self.config.app_stderr {
            let rot_cfg = LogRotationConfig {
                file_path: PathBuf::from(err_path),
                append: self.config.app_stderr_append,
                rotate_enabled: self.config.app_rotate_files,
                max_bytes: self.config.app_rotate_bytes,
                max_seconds: self.config.app_rotate_seconds,
            };
            let writer = RotatedLogWriter::new(rot_cfg)
                .context("Failed to open stderr log destination")?;
            let file = writer.get_file_handle()?;
            cmd.stderr(Stdio::from(file));
        }

        let child = cmd.spawn().with_context(|| {
            format!("Failed to execute process '{}'", self.config.application)
        })?;

        Ok(child)
    }

    fn kill_process_gracefully(&self, child: &mut Child) {
        let pid = child.id();

        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            let nix_pid = Pid::from_raw(pid as i32);
            log::info!("Sending SIGTERM to child PID {pid}");
            let _ = kill(nix_pid, Signal::SIGTERM);

            // Wait up to 3 seconds for graceful exit
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                if let Ok(Some(_)) = child.try_wait() {
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }

            // Force kill with SIGKILL if still alive
            log::warn!("Process PID {pid} didn't exit in time, sending SIGKILL");
            let _ = kill(nix_pid, Signal::SIGKILL);
            let _ = child.wait();
        }

        #[cfg(windows)]
        {
            // On Windows, use process kill
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn calculate_throttle_ms(&self, crashes: u32) -> u64 {
        let base = self.config.app_throttle.max(1000);
        let multiplier = 2u64.pow(crashes.min(6).saturating_sub(1));
        (base * multiplier).min(60_000) // Cap at 60s
    }
}

/// Helper to parse command line string into argument list respecting quotes
pub fn parse_cmdline(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    for ch in s.chars() {
        if (ch == '"' || ch == '\'') && (!in_quotes || ch == quote_char) {
            in_quotes = !in_quotes;
            quote_char = ch;
        } else if ch.is_whitespace() && !in_quotes {
            if !current.is_empty() {
                args.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}
