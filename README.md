# nssm-rs 🦀

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)

A modern, memory-safe, reliable service manager and process supervisor inspired by **NSSM (Non-Sucking Service Manager)**, written in pure Rust.

Since original NSSM was in the public domain, `nssm-rs` provides a **clean-room implementation** written from scratch in Rust under the permissive **MIT License**. It brings modern reliability, cross-platform process supervision, and seamless service lifecycle management to both **Windows** and **Linux/Unix**.

---

## ✨ Features

- **NSSM Command Line Compatibility**: Drop-in compatible syntax for `install`, `remove`, `start`, `stop`, `restart`, `status`, `set`, `get`, and `reset`.
- **Intelligent Restart Throttling**: Protects against CPU churn when applications crash repeatedly in quick succession with exponential backoff delay (`AppThrottle`).
- **Flexible Exit Action Policies**: Configure service behavior on process exit: `Restart`, `Ignore`, `Exit`, or `Suicide` (`AppExit`).
- **Standard I/O Redirection & Rotation**:
  - Redirect stdout and stderr to dedicated log files (`AppStdout`, `AppStderr`).
  - Automatic log rotation by file size (`AppRotateBytes`) or elapsed time (`AppRotateSeconds`).
  - Append (`OPEN_ALWAYS`) or Overwrite (`CREATE_ALWAYS`) modes.
- **Graceful Process Shutdown**:
  - Sends graceful stop signals (`SIGTERM`/Ctrl+C), waits for application cleanup, and safely escalates to kill if the process hangs.
- **Cross-Platform Architecture**:
  - **Windows**: Integrates with Windows Service Control Manager (SCM).
  - **Linux / Unix**: Generates and manages `systemd` unit files or runs standalone supervisor daemons.
  - **Direct Supervisor Mode (`exec`)**: Supervise any arbitrary command or script on-the-fly without registering a system service.

---

## 🚀 Quick Start

### Build

```bash
git clone https://github.com/mahir/nssm-rs.git
cd nssm-rs
cargo build --release
```

Binary output: `target/release/nssm-rs`

---

## 📖 Command Reference

### 1. Install a Service

```bash
nssm-rs install MyService /usr/bin/python3 app.py --port 8080
```

### 2. Configure Service Parameters

```bash
# Set working directory
nssm-rs set MyService AppDirectory /var/www/my-app

# Configure stdout and stderr log files
nssm-rs set MyService AppStdout /var/log/my-app.stdout.log
nssm-rs set MyService AppStderr /var/log/my-app.stderr.log

# Enable automatic log rotation at 10 MB (10,485,760 bytes)
nssm-rs set MyService AppRotateFiles 1
nssm-rs set MyService AppRotateBytes 10485760

# Set restart throttle delay (milliseconds)
nssm-rs set MyService AppThrottle 2000

# Set exit policy
nssm-rs set MyService AppExit Restart

# Add extra environment variable
nssm-rs set MyService AppEnvironmentExtra NODE_ENV=production
```

### 3. Retrieve and Reset Parameters

```bash
# Get parameter
nssm-rs get MyService AppDirectory

# Reset parameter to default
nssm-rs reset MyService AppStdout
```

### 4. Service Control

```bash
# Start service
nssm-rs start MyService

# Check status
nssm-rs status MyService

# Restart service
nssm-rs restart MyService

# Stop service
nssm-rs stop MyService

# Remove service
nssm-rs remove MyService confirm
```

### 5. Ad-Hoc Process Supervisor (`exec`)

Run and supervise any command directly with auto-restart, throttle protection, and log rotation:

```bash
nssm-rs exec python3 worker.py --stdout worker.log --rotate --rotate-bytes 5242880
```

---

## ⚙️ Supported Parameters

| Parameter | Type / Format | Description |
|---|---|---|
| `Application` | Path | Executable path |
| `AppParameters` | String | Command line arguments |
| `AppDirectory` | Path | Working directory |
| `AppStdout` | Path | File path for stdout redirection |
| `AppStderr` | Path | File path for stderr redirection |
| `AppStdoutCreationDisposition` | `2` or `4` | `2` = Overwrite, `4` = Append |
| `AppStderrCreationDisposition` | `2` or `4` | `2` = Overwrite, `4` = Append |
| `AppRotateFiles` | `0` or `1` | Enable log rotation |
| `AppRotateBytes` | Bytes (int) | Rotate file when size exceeds bytes |
| `AppRotateSeconds` | Seconds (int) | Rotate file after elapsed time |
| `AppThrottle` | Milliseconds | Minimum run time to reset backoff |
| `AppRestartDelay` | Milliseconds | Delay before initiating restart |
| `AppExit` | `Restart` / `Ignore` / `Exit` / `Suicide` | Action on process termination |
| `AppEnvironmentExtra` | `KEY=VALUE` | Inject extra environment variable |
| `AppPriority` | String | Process priority class |
| `DisplayName` | String | Service display name |
| `Description` | String | Human readable description |

---

## 🏗️ Architecture

```
nssm-rs
├── src/
│   ├── main.rs         # CLI entry point and command router
│   ├── cli.rs          # Clap CLI definition conforming to NSSM syntax
│   ├── config.rs       # Service configuration, serialization, get/set/reset
│   ├── supervisor.rs   # Process execution, crash detection, throttling & shutdown
│   ├── rotate.rs       # Size- and time-based log file rotation engine
│   └── service/
│       ├── mod.rs      # Cross-platform service dispatcher
│       ├── windows.rs  # Windows SCM integration (CreateService, sc.exe)
│       └── unix.rs     # Linux systemd unit generator & daemon supervisor
└── tests/
    └── supervisor_tests.rs # Configuration, rotation, and supervisor tests
```

---

## 📄 License

Distributed under the **MIT License**. See [`LICENSE`](LICENSE) for details.
