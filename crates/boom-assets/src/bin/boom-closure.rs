//! CLI dựng manifest closure của một trận V1: map → entry pack cần có.
//!
//! Dùng: `boom-closure <thư mục pack> [--out <file>]`. Không có `--out` thì manifest TOML
//! in ra stdout. Thư mục pack phải chứa `.idx`/`.idd` của các pack KR (ví dụ
//! `/home/admin/boomkr-full`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use boom_assets::closure::build_village00_manifest;

struct Options {
    pack_dir: PathBuf,
    out: Option<PathBuf>,
}

fn usage() -> ExitCode {
    eprintln!("dùng: boom-closure <thư mục pack> [--out <file>]");
    ExitCode::from(2)
}

fn options(args: &[String]) -> Option<Options> {
    let mut pack_dir = None;
    let mut out = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" => {
                out = Some(PathBuf::from(args.get(index + 1)?));
                index += 2;
            }
            other => {
                if pack_dir.is_some() {
                    return None;
                }
                pack_dir = Some(PathBuf::from(other));
                index += 1;
            }
        }
    }
    Some(Options {
        pack_dir: pack_dir?,
        out,
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(opts) = options(&args) else {
        return usage();
    };
    let manifest = match build_village00_manifest(&opts.pack_dir) {
        Ok(manifest) => manifest,
        Err(e) => {
            eprintln!("{}: {e}", opts.pack_dir.display());
            return ExitCode::FAILURE;
        }
    };
    let text = manifest.to_toml();
    match &opts.out {
        Some(path) => {
            if let Err(e) = write_file(path, &text) {
                eprintln!("{}: {e}", path.display());
                return ExitCode::FAILURE;
            }
            eprintln!(
                "{}: {} entry, {} unknown → {}",
                opts.pack_dir.display(),
                manifest.entries.len(),
                manifest.unknown.len(),
                path.display()
            );
        }
        None => print!("{text}"),
    }
    ExitCode::SUCCESS
}

fn write_file(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)
}
