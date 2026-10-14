pub mod unix;
pub mod windows;

use anyhow::Result;
use crate::config::ServiceConfig;

/// Unified cross-platform service management API
pub struct ServiceManager;

impl ServiceManager {
    pub fn install(config: &ServiceConfig) -> Result<()> {
        #[cfg(windows)]
        {
            windows::WindowsServiceManager::install(config)
        }
        #[cfg(not(windows))]
        {
            unix::UnixServiceManager::install(config)
        }
    }

    pub fn remove(name: &str) -> Result<()> {
        #[cfg(windows)]
        {
            windows::WindowsServiceManager::remove(name)
        }
        #[cfg(not(windows))]
        {
            unix::UnixServiceManager::remove(name)
        }
    }

    pub fn start(name: &str) -> Result<()> {
        #[cfg(windows)]
        {
            windows::WindowsServiceManager::start(name)
        }
        #[cfg(not(windows))]
        {
            unix::UnixServiceManager::start(name)
        }
    }

    pub fn stop(name: &str) -> Result<()> {
        #[cfg(windows)]
        {
            windows::WindowsServiceManager::stop(name)
        }
        #[cfg(not(windows))]
        {
            unix::UnixServiceManager::stop(name)
        }
    }

    pub fn restart(name: &str) -> Result<()> {
        #[cfg(windows)]
        {
            windows::WindowsServiceManager::restart(name)
        }
        #[cfg(not(windows))]
        {
            unix::UnixServiceManager::restart(name)
        }
    }

    pub fn status(name: &str) -> Result<String> {
        #[cfg(windows)]
        {
            windows::WindowsServiceManager::status(name)
        }
        #[cfg(not(windows))]
        {
            unix::UnixServiceManager::status(name).map(|s| s.as_str())
        }
    }

    pub fn run(name: &str) -> Result<()> {
        #[cfg(windows)]
        {
            windows::WindowsServiceManager::run(name)
        }
        #[cfg(not(windows))]
        {
            unix::UnixServiceManager::run(name)
        }
    }
}
