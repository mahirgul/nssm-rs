//! Service configuration and parameter definitions compatible with NSSM.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppExitAction {
    Restart,
    Ignore,
    Exit,
    Suicide,
}

impl AppExitAction {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "restart" => Some(AppExitAction::Restart),
            "ignore" => Some(AppExitAction::Ignore),
            "exit" => Some(AppExitAction::Exit),
            "suicide" => Some(AppExitAction::Suicide),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AppExitAction::Restart => "Restart",
            AppExitAction::Ignore => "Ignore",
            AppExitAction::Exit => "Exit",
            AppExitAction::Suicide => "Suicide",
        }
    }
}

impl fmt::Display for AppExitAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// NSSM-compatible service parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub application: String,
    pub app_parameters: Option<String>,
    pub app_directory: Option<String>,
    pub app_environment_extra: HashMap<String, String>,
    pub app_stdout: Option<String>,
    pub app_stderr: Option<String>,
    pub app_stdout_append: bool,
    pub app_stderr_append: bool,
    pub app_rotate_files: bool,
    pub app_rotate_bytes: Option<u64>,
    pub app_rotate_seconds: Option<u64>,
    pub app_throttle: u64, // ms before restart without throttle
    pub app_restart_delay: u64, // ms delay before restarting
    pub app_exit: AppExitAction,
    pub app_priority: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub startup_type: String,
}

impl ServiceConfig {
    pub fn new(name: &str, application: &str) -> Self {
        Self {
            name: name.to_string(),
            application: application.to_string(),
            app_parameters: None,
            app_directory: None,
            app_environment_extra: HashMap::new(),
            app_stdout: None,
            app_stderr: None,
            app_stdout_append: true,
            app_stderr_append: true,
            app_rotate_files: false,
            app_rotate_bytes: None,
            app_rotate_seconds: None,
            app_throttle: 1500,
            app_restart_delay: 0,
            app_exit: AppExitAction::Restart,
            app_priority: "NORMAL".to_string(),
            display_name: Some(name.to_string()),
            description: None,
            startup_type: "SERVICE_AUTO_START".to_string(),
        }
    }

    /// Sets parameter by NSSM parameter name (case-insensitive)
    pub fn set_parameter(&mut self, param: &str, value: &str) -> anyhow::Result<()> {
        match param.to_lowercase().as_str() {
            "application" => self.application = value.to_string(),
            "appparameters" => self.app_parameters = Some(value.to_string()),
            "appdirectory" => self.app_directory = Some(value.to_string()),
            "appstdout" => self.app_stdout = Some(value.to_string()),
            "appstderr" => self.app_stderr = Some(value.to_string()),
            "appstdoutcreationdisposition" => {
                // 2 = CREATE_ALWAYS (overwrite), 4 = OPEN_ALWAYS (append)
                self.app_stdout_append = value.trim() != "2";
            }
            "appstderrcreationdisposition" => {
                self.app_stderr_append = value.trim() != "2";
            }
            "approtatefiles" => {
                self.app_rotate_files = value.trim() == "1" || value.trim().eq_ignore_ascii_case("true");
            }
            "approtatebytes" => {
                self.app_rotate_bytes = Some(value.trim().parse()?);
            }
            "approtateseconds" => {
                self.app_rotate_seconds = Some(value.trim().parse()?);
            }
            "appthrottle" => {
                self.app_throttle = value.trim().parse()?;
            }
            "apprestartdelay" => {
                self.app_restart_delay = value.trim().parse()?;
            }
            "appexit" => {
                if let Some(action) = AppExitAction::from_str(value) {
                    self.app_exit = action;
                } else {
                    anyhow::bail!("Invalid AppExit action. Expected Restart, Ignore, Exit, or Suicide.");
                }
            }
            "apppriority" => {
                self.app_priority = value.to_uppercase();
            }
            "displayname" => {
                self.display_name = Some(value.to_string());
            }
            "description" => {
                self.description = Some(value.to_string());
            }
            "start" | "type" => {
                self.startup_type = value.to_string();
            }
            "appenvironmentextra" => {
                // Format KEY=VALUE
                if let Some((k, v)) = value.split_once('=') {
                    self.app_environment_extra.insert(k.trim().to_string(), v.trim().to_string());
                } else {
                    anyhow::bail!("Invalid AppEnvironmentExtra format. Expected KEY=VALUE.");
                }
            }
            other => anyhow::bail!("Unknown parameter '{other}'"),
        }
        Ok(())
    }

    /// Gets parameter value by NSSM parameter name
    pub fn get_parameter(&self, param: &str) -> Option<String> {
        match param.to_lowercase().as_str() {
            "application" => Some(self.application.clone()),
            "appparameters" => self.app_parameters.clone(),
            "appdirectory" => self.app_directory.clone(),
            "appstdout" => self.app_stdout.clone(),
            "appstderr" => self.app_stderr.clone(),
            "appstdoutcreationdisposition" => Some(if self.app_stdout_append { "4" } else { "2" }.to_string()),
            "appstderrcreationdisposition" => Some(if self.app_stderr_append { "4" } else { "2" }.to_string()),
            "approtatefiles" => Some(if self.app_rotate_files { "1" } else { "0" }.to_string()),
            "approtatebytes" => self.app_rotate_bytes.map(|v| v.to_string()),
            "approtateseconds" => self.app_rotate_seconds.map(|v| v.to_string()),
            "appthrottle" => Some(self.app_throttle.to_string()),
            "apprestartdelay" => Some(self.app_restart_delay.to_string()),
            "appexit" => Some(self.app_exit.to_string()),
            "apppriority" => Some(self.app_priority.clone()),
            "displayname" => self.display_name.clone(),
            "description" => self.description.clone(),
            "start" | "type" => Some(self.startup_type.clone()),
            _ => None,
        }
    }

    /// Resets parameter to default value
    pub fn reset_parameter(&mut self, param: &str) -> anyhow::Result<()> {
        match param.to_lowercase().as_str() {
            "appparameters" => self.app_parameters = None,
            "appdirectory" => self.app_directory = None,
            "appstdout" => self.app_stdout = None,
            "appstderr" => self.app_stderr = None,
            "appstdoutcreationdisposition" => self.app_stdout_append = true,
            "appstderrcreationdisposition" => self.app_stderr_append = true,
            "approtatefiles" => self.app_rotate_files = false,
            "approtatebytes" => self.app_rotate_bytes = None,
            "approtateseconds" => self.app_rotate_seconds = None,
            "appthrottle" => self.app_throttle = 1500,
            "apprestartdelay" => self.app_restart_delay = 0,
            "appexit" => self.app_exit = AppExitAction::Restart,
            "apppriority" => self.app_priority = "NORMAL".to_string(),
            "description" => self.description = None,
            "appenvironmentextra" => self.app_environment_extra.clear(),
            other => anyhow::bail!("Cannot reset or unknown parameter '{other}'"),
        }
        Ok(())
    }

    /// Save configuration to JSON file on disk
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load configuration from JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn default_config_path(service_name: &str) -> PathBuf {
        #[cfg(windows)]
        {
            let program_data = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".into());
            PathBuf::from(program_data).join("nssm-rs").join("services").join(format!("{service_name}.json"))
        }
        #[cfg(not(windows))]
        {
            let base_dir = std::env::var("HOME").unwrap_or_else(|_| "/etc".into());
            PathBuf::from(base_dir).join(".nssm-rs").join("services").join(format!("{service_name}.json"))
        }
    }
}
