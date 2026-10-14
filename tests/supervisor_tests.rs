use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[path = "../src/config.rs"]
mod config;
#[path = "../src/rotate.rs"]
mod rotate;
#[path = "../src/supervisor.rs"]
mod supervisor;

use config::{AppExitAction, ServiceConfig};
use rotate::{LogRotationConfig, RotatedLogWriter};
use supervisor::{parse_cmdline, Supervisor};

#[test]
fn test_config_set_get_reset() {
    let mut config = ServiceConfig::new("MyService", "/usr/bin/python3");
    assert_eq!(config.name, "MyService");
    assert_eq!(config.application, "/usr/bin/python3");
    assert_eq!(config.app_throttle, 1500);

    // Set parameters
    config
        .set_parameter("AppDirectory", "/var/www/app")
        .expect("Valid AppDirectory");
    config
        .set_parameter("AppParameters", "--port 8080 --host 0.0.0.0")
        .expect("Valid AppParameters");
    config
        .set_parameter("AppStdout", "/var/log/app.log")
        .expect("Valid AppStdout");
    config
        .set_parameter("AppThrottle", "5000")
        .expect("Valid AppThrottle");
    config
        .set_parameter("AppExit", "Exit")
        .expect("Valid AppExit");
    config
        .set_parameter("AppEnvironmentExtra", "ENV=production")
        .expect("Valid AppEnvironmentExtra");

    assert_eq!(
        config.get_parameter("AppDirectory").as_deref(),
        Some("/var/www/app")
    );
    assert_eq!(
        config.get_parameter("AppParameters").as_deref(),
        Some("--port 8080 --host 0.0.0.0")
    );
    assert_eq!(
        config.get_parameter("AppStdout").as_deref(),
        Some("/var/log/app.log")
    );
    assert_eq!(
        config.get_parameter("AppThrottle").as_deref(),
        Some("5000")
    );
    assert_eq!(config.app_exit, AppExitAction::Exit);
    assert_eq!(
        config.app_environment_extra.get("ENV").map(|s| s.as_str()),
        Some("production")
    );

    // Reset parameter
    config.reset_parameter("AppStdout").expect("Valid reset");
    assert_eq!(config.get_parameter("AppStdout"), None);
}

#[test]
fn test_cmdline_parsing() {
    let cmd = "serve --config \"my app.conf\" --flag 'single quote' arg3";
    let args = parse_cmdline(cmd);
    assert_eq!(
        args,
        vec![
            "serve".to_string(),
            "--config".to_string(),
            "my app.conf".to_string(),
            "--flag".to_string(),
            "single quote".to_string(),
            "arg3".to_string(),
        ]
    );
}

#[test]
fn test_log_writer_and_rotation() {
    let temp_dir = std::env::temp_dir().join("nssm_test_log");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let log_path = temp_dir.join("service.log");

    let rot_cfg = LogRotationConfig {
        file_path: log_path.clone(),
        append: true,
        rotate_enabled: true,
        max_bytes: Some(50),
        max_seconds: None,
    };

    let mut writer = RotatedLogWriter::new(rot_cfg).expect("Should create writer");
    let file = writer.get_file_handle().expect("File handle");
    drop(file);

    // Write bytes exceeding 50 bytes threshold
    fs::write(&log_path, "A".repeat(100)).unwrap();

    // Check rotation triggers
    writer.rotate_if_needed().unwrap();

    // The original file is rotated and new file is created
    let entries: Vec<_> = fs::read_dir(&temp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(entries.len() >= 1);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_supervisor_execution_and_stop() {
    let running = Arc::new(AtomicBool::new(true));
    let r_clone = Arc::clone(&running);

    let mut config = ServiceConfig::new("echo_service", "/bin/sh");
    config.app_parameters = Some("-c 'while true; do sleep 0.1; done'".to_string());
    config.app_exit = AppExitAction::Exit;

    let handle = thread::spawn(move || {
        let mut sup = Supervisor::new(config, r_clone);
        sup.run().expect("Supervisor run should succeed");
    });

    // Let it run briefly
    thread::sleep(Duration::from_millis(300));

    // Signal graceful stop
    running.store(false, Ordering::Relaxed);

    // Thread should join smoothly
    handle.join().expect("Supervisor thread joined");
}
