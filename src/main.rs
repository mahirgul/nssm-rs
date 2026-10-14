use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use anyhow::{Context, Result};
use clap::Parser;

mod cli;
mod config;
mod rotate;
mod service;
mod supervisor;

use cli::{Cli, Commands};
use config::ServiceConfig;
use service::ServiceManager;
use supervisor::Supervisor;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Install {
            service_name,
            application,
            app_parameters,
        } => {
            let app_path = application.unwrap_or_else(|| {
                eprintln!("Error: Application executable path is required for installation.");
                std::process::exit(1);
            });

            let mut config = ServiceConfig::new(&service_name, &app_path);
            if !app_parameters.is_empty() {
                config.app_parameters = Some(app_parameters.join(" "));
            }

            ServiceManager::install(&config)?;
        }

        Commands::Remove {
            service_name,
            confirm,
        } => {
            if let Some(c) = confirm {
                if c.to_lowercase() != "confirm" {
                    println!("To remove service \"{service_name}\", append 'confirm'");
                    return Ok(());
                }
            }
            ServiceManager::remove(&service_name)?;
        }

        Commands::Start { service_name } => {
            ServiceManager::start(&service_name)?;
        }

        Commands::Stop { service_name } => {
            ServiceManager::stop(&service_name)?;
        }

        Commands::Restart { service_name } => {
            ServiceManager::restart(&service_name)?;
        }

        Commands::Status { service_name } => {
            let status = ServiceManager::status(&service_name)?;
            println!("{status}");
        }

        Commands::Set {
            service_name,
            parameter,
            value,
        } => {
            let cfg_path = ServiceConfig::default_config_path(&service_name);
            let mut config = ServiceConfig::load_from_file(&cfg_path).with_context(|| {
                format!("Service \"{service_name}\" not found. Did you run 'install'?")
            })?;

            config.set_parameter(&parameter, &value)?;
            config.save_to_file(&cfg_path)?;
            println!("Set parameter \"{parameter}\" for service \"{service_name}\".");
        }

        Commands::Get {
            service_name,
            parameter,
        } => {
            let cfg_path = ServiceConfig::default_config_path(&service_name);
            let config = ServiceConfig::load_from_file(&cfg_path).with_context(|| {
                format!("Service \"{service_name}\" not found.")
            })?;

            if let Some(val) = config.get_parameter(&parameter) {
                println!("{val}");
            } else {
                eprintln!("Parameter \"{parameter}\" is not set or unknown.");
            }
        }

        Commands::Reset {
            service_name,
            parameter,
        } => {
            let cfg_path = ServiceConfig::default_config_path(&service_name);
            let mut config = ServiceConfig::load_from_file(&cfg_path).with_context(|| {
                format!("Service \"{service_name}\" not found.")
            })?;

            config.reset_parameter(&parameter)?;
            config.save_to_file(&cfg_path)?;
            println!("Reset parameter \"{parameter}\" for service \"{service_name}\".");
        }

        Commands::Run { service_name } => {
            ServiceManager::run(&service_name)?;
        }

        Commands::Exec {
            application,
            args,
            dir,
            stdout,
            stderr,
            rotate,
            rotate_bytes,
            throttle,
        } => {
            let mut config = ServiceConfig::new("exec-process", &application);
            if !args.is_empty() {
                config.app_parameters = Some(args.join(" "));
            }
            config.app_directory = dir;
            config.app_stdout = stdout;
            config.app_stderr = stderr;
            config.app_rotate_files = rotate;
            config.app_rotate_bytes = rotate_bytes;
            config.app_throttle = throttle;

            let running = Arc::new(AtomicBool::new(true));
            let r_clone = Arc::clone(&running);

            ctrlc::set_handler(move || {
                r_clone.store(false, Ordering::Relaxed);
            })?;

            let mut supervisor = Supervisor::new(config, running);
            supervisor.run()?;
        }
    }

    Ok(())
}
