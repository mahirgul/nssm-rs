#[allow(unused_imports, dead_code)]
use anyhow::{Context, Result};
use crate::config::ServiceConfig;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowsServiceState {
    Running,
    Stopped,
    Paused,
    StartPending,
    StopPending,
    Unknown,
}

pub struct WindowsServiceManager;

#[cfg(windows)]
impl WindowsServiceManager {
    pub fn install(config: &ServiceConfig) -> Result<()> {
        let current_exe = std::env::current_exe()?;
        let exe_str = current_exe.to_string_lossy();

        // Save service configuration to disk
        let cfg_path = ServiceConfig::default_config_path(&config.name);
        config.save_to_file(&cfg_path)?;

        // Command to run nssm-rs as service host
        let bin_path = format!("\"{}\" run \"{}\"", exe_str, config.name);

        let status = std::process::Command::new("sc.exe")
            .args([
                "create",
                &config.name,
                &format!("binPath= {bin_path}"),
                &format!("DisplayName= {}", config.display_name.as_deref().unwrap_or(&config.name)),
                "start= auto",
            ])
            .status()
            .context("Failed to register service with Windows SCM via sc.exe")?;

        if !status.success() {
            anyhow::bail!("sc.exe create returned error code {:?}", status.code());
        }

        println!("Service \"{}\" installed successfully on Windows!", config.name);
        Ok(())
    }

    pub fn remove(name: &str) -> Result<()> {
        let _ = std::process::Command::new("sc.exe").args(["stop", name]).status();
        let status = std::process::Command::new("sc.exe")
            .args(["delete", name])
            .status()
            .context("Failed to delete service via sc.exe")?;

        if !status.success() {
            anyhow::bail!("sc.exe delete failed");
        }

        let cfg_path = ServiceConfig::default_config_path(name);
        if cfg_path.exists() {
            let _ = std::fs::remove_file(cfg_path);
        }

        println!("Service \"{name}\" removed successfully from Windows!");
        Ok(())
    }

    pub fn start(name: &str) -> Result<()> {
        let status = std::process::Command::new("sc.exe")
            .args(["start", name])
            .status()
            .context("Failed to start service via sc.exe")?;

        if !status.success() {
            anyhow::bail!("sc.exe start failed");
        }
        println!("Service \"{name}\" started successfully!");
        Ok(())
    }

    pub fn stop(name: &str) -> Result<()> {
        let status = std::process::Command::new("sc.exe")
            .args(["stop", name])
            .status()
            .context("Failed to stop service via sc.exe")?;

        if !status.success() {
            anyhow::bail!("sc.exe stop failed");
        }
        println!("Service \"{name}\" stopped successfully!");
        Ok(())
    }

    pub fn restart(name: &str) -> Result<()> {
        let _ = Self::stop(name);
        std::thread::sleep(std::time::Duration::from_millis(500));
        Self::start(name)
    }

    pub fn status(name: &str) -> Result<String> {
        let output = std::process::Command::new("sc.exe")
            .args(["query", name])
            .output()
            .context("Failed to query service status via sc.exe")?;

        let text = String::from_utf8_lossy(&output.stdout);
        if text.contains("RUNNING") {
            Ok("SERVICE_RUNNING".to_string())
        } else if text.contains("STOPPED") {
            Ok("SERVICE_STOPPED".to_string())
        } else if text.contains("PAUSED") {
            Ok("SERVICE_PAUSED".to_string())
        } else {
            Ok("SERVICE_UNKNOWN".to_string())
        }
    }

    pub fn run(name: &str) -> Result<()> {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use crate::supervisor::Supervisor;

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

#[cfg(not(windows))]
impl WindowsServiceManager {
    pub fn install(_config: &ServiceConfig) -> Result<()> {
        anyhow::bail!("Windows SCM is only supported on Windows targets");
    }

    pub fn remove(_name: &str) -> Result<()> {
        anyhow::bail!("Windows SCM is only supported on Windows targets");
    }

    pub fn start(_name: &str) -> Result<()> {
        anyhow::bail!("Windows SCM is only supported on Windows targets");
    }

    pub fn stop(_name: &str) -> Result<()> {
        anyhow::bail!("Windows SCM is only supported on Windows targets");
    }

    pub fn restart(_name: &str) -> Result<()> {
        anyhow::bail!("Windows SCM is only supported on Windows targets");
    }

    pub fn status(_name: &str) -> Result<String> {
        anyhow::bail!("Windows SCM is only supported on Windows targets");
    }

    pub fn run(_name: &str) -> Result<()> {
        anyhow::bail!("Windows SCM is only supported on Windows targets");
    }
}
