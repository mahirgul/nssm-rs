//! Linux / Unix Service implementation: systemd integration and supervisor daemon.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use anyhow::{Context, Result};

use crate::config::ServiceConfig;
use crate::supervisor::Supervisor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceState {
    Running(u32),
    Stopped,
    Failed,
    Unknown,
}

impl ServiceState {
    pub fn as_str(&self) -> String {
        match self {
            ServiceState::Running(pid) => format!("SERVICE_RUNNING (PID: {pid})"),
            ServiceState::Stopped => "SERVICE_STOPPED".to_string(),
            ServiceState::Failed => "SERVICE_FAILED".to_string(),
            ServiceState::Unknown => "SERVICE_UNKNOWN".to_string(),
        }
    }
}

pub struct UnixServiceManager;

impl UnixServiceManager {
    /// Installs service configuration and creates a systemd service unit file
    pub fn install(config: &ServiceConfig) -> Result<()> {
        // Save nssm-rs service configuration file
        let cfg_path = ServiceConfig::default_config_path(&config.name);
        config.save_to_file(&cfg_path)?;
        println!("Service configuration saved to {:?}", cfg_path);

        // Generate and install systemd unit file
        if is_systemd_available() {
            install_systemd_unit(config)?;
            println!("Installed systemd unit for service '{}'", config.name);
        } else {
            println!("Systemd not detected. Service will run in supervisor daemon mode.");
        }

        println!("Service \"{}\" installed successfully!", config.name);
        Ok(())
    }

    /// Removes service and uninstalls systemd unit file
    pub fn remove(name: &str) -> Result<()> {
        if is_systemd_available() {
            let _ = Command::new("systemctl").args(["stop", name]).status();
            let _ = Command::new("systemctl").args(["disable", name]).status();

            let unit_path = PathBuf::from(format!("/etc/systemd/system/{name}.service"));
            if unit_path.exists() {
                let _ = fs::remove_file(unit_path);
                let _ = Command::new("systemctl").arg("daemon-reload").status();
            }
        }

        let cfg_path = ServiceConfig::default_config_path(name);
        if cfg_path.exists() {
            fs::remove_file(&cfg_path)?;
        }

        println!("Service \"{name}\" removed successfully!");
        Ok(())
    }

    /// Starts service
    pub fn start(name: &str) -> Result<()> {
        if is_systemd_available() {
            let status = Command::new("systemctl")
                .args(["start", name])
                .status()
                .context("Failed to run systemctl start")?;
            if status.success() {
                println!("Service \"{name}\" started successfully!");
                return Ok(());
            }
        }

        // Fallback or standalone supervisor mode
        println!("Starting service \"{name}\" via supervisor runner...");
        Self::run(name)?;
        Ok(())
    }

    /// Stops service
    pub fn stop(name: &str) -> Result<()> {
        if is_systemd_available() {
            let status = Command::new("systemctl")
                .args(["stop", name])
                .status()
                .context("Failed to run systemctl stop")?;
            if status.success() {
                println!("Service \"{name}\" stopped successfully!");
                return Ok(());
            }
        }

        println!("Service \"{name}\" stopped.");
        Ok(())
    }

    /// Restarts service
    pub fn restart(name: &str) -> Result<()> {
        if is_systemd_available() {
            let status = Command::new("systemctl")
                .args(["restart", name])
                .status()
                .context("Failed to run systemctl restart")?;
            if status.success() {
                println!("Service \"{name}\" restarted successfully!");
                return Ok(());
            }
        }

        Self::stop(name)?;
        Self::start(name)?;
        Ok(())
    }

    /// Queries service status
    pub fn status(name: &str) -> Result<ServiceState> {
        if is_systemd_available() {
            let output = Command::new("systemctl")
                .args(["is-active", name])
                .output();
            if let Ok(out) = output {
                let status_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if status_str == "active" {
                    // Try to extract Main PID
                    let pid_out = Command::new("systemctl")
                        .args(["show", "--property", "MainPID", "--value", name])
                        .output();
                    let pid = pid_out
                        .ok()
                        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u32>().ok())
                        .unwrap_or(0);
                    return Ok(ServiceState::Running(pid));
                } else if status_str == "inactive" {
                    return Ok(ServiceState::Stopped);
                } else if status_str == "failed" {
                    return Ok(ServiceState::Failed);
                }
            }
        }

        let cfg_path = ServiceConfig::default_config_path(name);
        if cfg_path.exists() {
            Ok(ServiceState::Stopped)
        } else {
            Ok(ServiceState::Unknown)
        }
    }

    /// Runs service supervisor loop directly (called by service host / daemon)
    pub fn run(name: &str) -> Result<()> {
        let cfg_path = ServiceConfig::default_config_path(name);
        let config = ServiceConfig::load_from_file(&cfg_path)
            .with_context(|| format!("Service configuration not found at {:?}", cfg_path))?;

        let running = Arc::new(AtomicBool::new(true));
        let r_clone = Arc::clone(&running);

        ctrlc::set_handler(move || {
            r_clone.store(false, Ordering::Relaxed);
        })?;

        let mut supervisor = Supervisor::new(config, running);
        supervisor.run()
    }
}

fn is_systemd_available() -> bool {
    Path::new("/run/systemd/system").exists()
}

fn install_systemd_unit(config: &ServiceConfig) -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let current_exe_str = current_exe.to_string_lossy();

    let desc = config.description.as_deref().unwrap_or(&config.name);

    let unit_content = format!(
        "[Unit]\n\
Description={desc}\n\
After=network.target\n\
\n\
[Service]\n\
Type=simple\n\
ExecStart={current_exe_str} run {name}\n\
Restart=always\n\
RestartSec=2\n\
\n\
[Install]\n\
WantedBy=multi-user.target\n",
        desc = desc,
        current_exe_str = current_exe_str,
        name = config.name
    );

    let unit_path = PathBuf::from(format!("/etc/systemd/system/{}.service", config.name));
    if let Err(e) = fs::write(&unit_path, &unit_content) {
        log::warn!("Could not write system unit to {:?} (requires sudo): {e}", unit_path);
        // Fallback to user unit
        if let Ok(home) = std::env::var("HOME") {
            let user_unit_dir = PathBuf::from(home).join(".config/systemd/user");
            fs::create_dir_all(&user_unit_dir)?;
            let user_unit_path = user_unit_dir.join(format!("{}.service", config.name));
            let _ = fs::write(&user_unit_path, &unit_content);
            println!("Wrote user systemd service to {:?}", user_unit_path);
        }
    } else {
        let _ = Command::new("systemctl").arg("daemon-reload").status();
        let _ = Command::new("systemctl").args(["enable", &config.name]).status();
    }

    Ok(())
}
