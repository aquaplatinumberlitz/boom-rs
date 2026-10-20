//! CLI của harness QA mạng V1-027.
//!
//! ```text
//! cargo run -p boom-qa -- --profile scope-v1 --clients 4 --seed 42
//! ```
//!
//! In ra **bảng markdown** rồi **JSON** của cùng một báo cáo. Mã thoát khác 0 khi lần chạy lỗi,
//! hết ngân sách thời gian, hoặc có trận không hội tụ state hash.

use std::process::ExitCode;

use boom_qa::harness::RunConfig;
use boom_qa::profile::QaProfile;
use boom_qa::run;

const USAGE: &str = "\
dùng: boom-qa [--profile <tên>] [--clients <2..4>] [--seed <n>] [--rounds <n>] [--json-out <file>]

  --profile <tên>   hồ sơ QA (mặc định: scope-v1; có: scope-v1, fast)
  --clients <n>     số bot headless, 2..4 (mặc định: 2)
  --seed <n>        seed lỗi mạng + input ngẫu nhiên của bot (mặc định: 0)
  --rounds <n>      số trận (mặc định: lấy theo hồ sơ)
  --json-out <file> ghi thêm báo cáo JSON ra file
  --help            in ra màn hình trợ giúp
";

fn main() -> ExitCode {
    match run_cli() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("boom-qa: {message}");
            ExitCode::from(2)
        }
    }
}

/// Tham số dòng lệnh đã parse.
struct Args {
    profile: String,
    clients: usize,
    seed: u64,
    rounds: Option<u32>,
    json_out: Option<String>,
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut args = Args {
        profile: "scope-v1".to_owned(),
        clients: 2,
        seed: 0,
        rounds: None,
        json_out: None,
    };
    let mut raw = std::env::args().skip(1);
    while let Some(arg) = raw.next() {
        let mut value = || {
            raw.next()
                .ok_or_else(|| format!("thiếu giá trị cho tham số `{arg}`"))
        };
        match arg.as_str() {
            "--profile" => args.profile = value()?,
            "--clients" => {
                let raw_value = value()?;
                args.clients = raw_value
                    .parse()
                    .map_err(|_| format!("--clients không phải số nguyên: {raw_value}"))?;
            }
            "--seed" => {
                let raw_value = value()?;
                args.seed = raw_value
                    .parse()
                    .map_err(|_| format!("--seed không phải số nguyên: {raw_value}"))?;
            }
            "--rounds" => {
                let raw_value = value()?;
                args.rounds = Some(
                    raw_value
                        .parse()
                        .map_err(|_| format!("--rounds không phải số nguyên: {raw_value}"))?,
                );
            }
            "--json-out" => args.json_out = Some(value()?),
            "--help" | "-h" => return Ok(None),
            other => return Err(format!("tham số lạ: {other}")),
        }
    }
    if !(2..=4).contains(&args.clients) {
        return Err(format!(
            "--clients phải trong 2..=4 (SCOPE_V1 §1), nhận {}",
            args.clients
        ));
    }
    Ok(Some(args))
}

fn run_cli() -> Result<ExitCode, String> {
    let Some(args) = parse_args()? else {
        print!("{USAGE}");
        return Ok(ExitCode::SUCCESS);
    };
    let profile = QaProfile::builtin(&args.profile).map_err(|err| err.to_string())?;
    let config = RunConfig {
        profile,
        clients: args.clients,
        seed: args.seed,
        rounds: args.rounds,
    };
    let report = run(&config).map_err(|err| err.to_string())?;
    print!("{}", report.to_markdown());
    println!("{}", report.to_json());
    if let Some(path) = &args.json_out
        && let Err(err) = std::fs::write(path, format!("{}\n", report.to_json()))
    {
        return Err(format!("không ghi được JSON ra `{path}`: {err}"));
    }
    if report.timed_out || !report.converged {
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}
