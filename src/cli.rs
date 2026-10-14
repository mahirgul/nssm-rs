//! Command-line interface definitions conforming to NSSM syntax.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "nssm-rs",
    author = "nssm-rs Contributors",
    version = "0.1.0",
    about = "A modern, reliable service manager and process supervisor inspired by NSSM, written in Rust",
    long_about = "Clean-room MIT-licensed service manager and supervisor. Supports Windows Service Control Manager, Linux systemd unit generation, and cross-platform process throttling and log rotation."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Install a service: nssm-rs install <servicename> <application> [<app_parameters>...]
    Install {
        /// Unique name of the service
        service_name: String,

        /// Path to application executable
        application: Option<String>,

        /// Arguments passed to application
        app_parameters: Vec<String>,
    },

    /// Remove an installed service: nssm-rs remove <servicename> [confirm]
    Remove {
        /// Name of the service to remove
        service_name: String,

        /// Optional confirmation ("confirm")
        confirm: Option<String>,
    },

    /// Start a service: nssm-rs start <servicename>
    Start {
        /// Name of the service to start
        service_name: String,
    },

    /// Stop a service: nssm-rs stop <servicename>
    Stop {
        /// Name of the service to stop
        service_name: String,
    },

    /// Restart a service: nssm-rs restart <servicename>
    Restart {
        /// Name of the service to restart
        service_name: String,
    },

    /// Query the status of a service: nssm-rs status <servicename>
    Status {
        /// Name of the service to query
        service_name: String,
    },

    /// Set a service parameter: nssm-rs set <servicename> <parameter> <value>
    Set {
        /// Name of the service
        service_name: String,

        /// Parameter name (Application, AppDirectory, AppParameters, AppStdout, AppStderr, AppThrottle, AppExit, etc.)
        parameter: String,

        /// Value to set
        value: String,
    },

    /// Get a service parameter value: nssm-rs get <servicename> <parameter>
    Get {
        /// Name of the service
        service_name: String,

        /// Parameter name to retrieve
        parameter: String,
    },

    /// Reset a service parameter to default: nssm-rs reset <servicename> <parameter>
    Reset {
        /// Name of the service
        service_name: String,

        /// Parameter name to reset
        parameter: String,
    },

    /// Internal service runner loop: nssm-rs run <servicename>
    Run {
        /// Name of the service to execute
        service_name: String,
    },

    /// Supervise an arbitrary command directly without installing: nssm-rs exec <app> [<args>...]
    Exec {
        /// Application executable
        application: String,

        /// Application arguments
        args: Vec<String>,

        /// Working directory
        #[arg(long = "dir")]
        dir: Option<String>,

        /// Redirect stdout to file
        #[arg(long = "stdout")]
        stdout: Option<String>,

        /// Redirect stderr to file
        #[arg(long = "stderr")]
        stderr: Option<String>,

        /// Enable log file rotation
        #[arg(long = "rotate")]
        rotate: bool,

        /// Max log file size in bytes before rotating
        #[arg(long = "rotate-bytes")]
        rotate_bytes: Option<u64>,

        /// Throttle restart delay in milliseconds
        #[arg(long = "throttle", default_value_t = 1500)]
        throttle: u64,
    },
}
