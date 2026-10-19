//! `boom-assets map-info <file.map> --build kr|vn`: in kích thước, spawn, bảng rơi và lưới obj ID.

use std::collections::BTreeMap;
use std::process::ExitCode;

use boom_assets::legacy_map::{Build, parse};

fn usage() -> ExitCode {
    eprintln!("dùng: boom-assets map-info <file.map> --build kr|vn");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(cmd), Some(path)) = (args.first(), args.get(1)) else {
        return usage();
    };
    let build = match args
        .iter()
        .skip_while(|a| *a != "--build")
        .nth(1)
        .map(String::as_str)
    {
        Some("kr") => Build::Kr,
        Some("vn") => Build::Vn,
        _ => return usage(),
    };
    if cmd != "map-info" {
        return usage();
    }
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let map = match parse(&data, build) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let g = &map.grid;
    println!(
        "version {} build {build:?} size {}x{}",
        map.version, g.width, g.height
    );
    println!("header {:?}", map.header);
    println!("spawns {:?}", g.spawns);
    for d in &map.drops {
        println!(
            "drop {} weight {}",
            String::from_utf8_lossy(&d.name),
            d.weight
        );
    }
    println!(
        "drop_base {} placements {} tail {} B",
        map.drop_base,
        map.placements.len(),
        map.tail.len()
    );
    let mut objs: BTreeMap<Option<u32>, usize> = BTreeMap::new();
    for c in &g.cells {
        *objs.entry(c.obj).or_default() += 1;
    }
    println!("obj histogram {objs:?}");
    for row in g.cells.chunks(usize::from(g.width)) {
        let line: Vec<String> = row
            .iter()
            .map(|c| c.obj.map_or_else(|| "    .".into(), |o| format!("{o:5}")))
            .collect();
        println!("{}", line.join(""));
    }
    ExitCode::SUCCESS
}
