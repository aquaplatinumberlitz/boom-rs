//! CLI server V1: `health` runtime/offline (ADR 0011 §8.2) và tiến trình admin/heartbeat T1.
//!
//! Game listener/content startup thuộc lớp triển khai P7; `serve` hiện mở admin listener + heartbeat
//! để health runtime có nguồn sự thật là tiến trình, không chỉ đọc file.

use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use boom_server::{
    AdminServer, HealthState, Heartbeat, Logger, OperationalConfig, new_instance_id, now_ms,
    run_health_cli,
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("health") => health_command(&args),
        None | Some("serve") => serve_command(),
        Some(other) => {
            eprintln!("lệnh không hỗ trợ: {other}; dùng boom-server health [--offline] | serve");
            ExitCode::from(2)
        }
    }
}

fn health_command(args: &[String]) -> ExitCode {
    let offline = args.iter().any(|arg| arg == "--offline");
    let config = match OperationalConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("lỗi cấu hình: {err}");
            return ExitCode::from(3);
        }
    };
    let outcome = run_health_cli(&config, offline);
    if outcome.code == 3 {
        let message = outcome
            .body
            .get("error")
            .and_then(|value| value.as_str())
            .unwrap_or("lỗi CLI health");
        eprintln!("{message}");
        return ExitCode::from(3);
    }
    match serde_json::to_string(&outcome.body) {
        Ok(body) => println!("{body}"),
        Err(err) => {
            eprintln!("lỗi serialize health: {err}");
            return ExitCode::from(3);
        }
    }
    ExitCode::from(u8::try_from(outcome.code).unwrap_or(3))
}

fn serve_command() -> ExitCode {
    let config = match OperationalConfig::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("lỗi cấu hình: {err}");
            return ExitCode::from(3);
        }
    };
    let instance_id = new_instance_id();
    let state = Arc::new(HealthState::new(
        config.clone(),
        instance_id.clone(),
        now_ms(),
    ));
    let logger = Arc::new(Logger::stdout(
        config.fingerprint(),
        instance_id,
        config.content_epoch,
    ));
    let admin = match AdminServer::start(&config, state.clone(), logger.clone()) {
        Ok(admin) => admin,
        Err(err) => {
            eprintln!("không mở được admin listener: {err}");
            return ExitCode::from(2);
        }
    };
    let heartbeat = match Heartbeat::start(state, logger, config.heartbeat_interval_ms) {
        Ok(heartbeat) => heartbeat,
        Err(err) => {
            eprintln!("không ghi được health.json: {err}");
            return ExitCode::from(2);
        }
    };
    eprintln!("admin listener {}", admin.local_addr());
    let _admin = admin;
    let _heartbeat = heartbeat;
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
