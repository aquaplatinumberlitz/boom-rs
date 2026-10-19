//! CLI đọc dữ liệu client gốc:
//!
//! - `map-info <file.map> [--build kr|vn]`: kích thước, spawn, bảng rơi và lưới obj ID.
//! - `pack-list <pack-dir> [stem] [--build kr|vn]`: liệt kê pack, hoặc entry của một pack.
//! - `pack-extract <pack-dir> <stem> <entry> <out> [--build kr|vn]`: ghi payload đã giải mã.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use boom_assets::legacy_map::{Build, parse};
use boom_assets::pack::{Pack, PackIndex, load_index};

struct Options {
    positional: Vec<String>,
    build: Build,
}

fn usage() -> ExitCode {
    eprintln!(
        "dùng:\n  \
         boom-assets map-info <file.map> [--build kr|vn]\n  \
         boom-assets pack-list <pack-dir> [stem] [--build kr|vn]\n  \
         boom-assets pack-extract <pack-dir> <stem> <entry> <out> [--build kr|vn]"
    );
    ExitCode::from(2)
}

/// Tách `--build` khỏi tham số vị trí; mặc định KR, giá trị lạ là lỗi dùng lệnh.
fn options(args: &[String]) -> Option<Options> {
    let mut positional = Vec::new();
    let mut build = Build::Kr;
    let mut index = 1; // bỏ tên lệnh
    while index < args.len() {
        if args[index] == "--build" {
            build = match args.get(index + 1).map(String::as_str) {
                Some("kr") => Build::Kr,
                Some("vn") => Build::Vn,
                _ => return None,
            };
            index += 2;
        } else {
            positional.push(args[index].clone());
            index += 1;
        }
    }
    Some(Options { positional, build })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(opts) = options(&args) else {
        return usage();
    };
    match args.first().map(String::as_str) {
        Some("map-info") => map_info(&opts),
        Some("pack-list") => pack_list(&opts),
        Some("pack-extract") => pack_extract(&opts),
        _ => usage(),
    }
}

fn map_info(opts: &Options) -> ExitCode {
    let [path] = opts.positional.as_slice() else {
        return usage();
    };
    let build = opts.build;
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

fn pack_list(opts: &Options) -> ExitCode {
    let (dir, only) = match opts.positional.as_slice() {
        [dir] => (dir.as_str(), None),
        [dir, stem] => (dir.as_str(), Some(stem.as_str())),
        _ => return usage(),
    };
    let dir = Path::new(dir);
    if let Some(stem) = only {
        return list_entries(dir, stem, opts.build);
    }
    let stems = match pack_stems(dir) {
        Ok(stems) => stems,
        Err(e) => {
            eprintln!("{}: {e}", dir.display());
            return ExitCode::FAILURE;
        }
    };
    let mut ok = true;
    for stem in &stems {
        match load_index(dir, stem, opts.build) {
            Ok(index) => println!("{stem}: {}", describe(&index)),
            Err(e) => {
                eprintln!("{stem}: {e}");
                ok = false;
            }
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn list_entries(dir: &Path, stem: &str, build: Build) -> ExitCode {
    match load_index(dir, stem, build) {
        Ok(index) => {
            println!("{stem}: {}", describe(&index));
            println!("name\toffset\tsize\tkey\tcrc");
            for entry in &index.entries {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    entry.name, entry.offset, entry.size, entry.key, entry.crc
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{stem}: {e}");
            ExitCode::FAILURE
        }
    }
}

fn pack_extract(opts: &Options) -> ExitCode {
    let [dir, stem, name, out] = opts.positional.as_slice() else {
        return usage();
    };
    let pack = match Pack::open(Path::new(dir), stem, opts.build) {
        Ok(pack) => pack,
        Err(e) => {
            eprintln!("{stem}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let data = match pack.read_named(name) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("{stem}: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::write(out, &data) {
        eprintln!("{out}: {e}");
        return ExitCode::FAILURE;
    }
    println!("{name} -> {out} ({} B)", data.len());
    ExitCode::SUCCESS
}

fn describe(index: &PackIndex) -> String {
    let version = index
        .version
        .map_or_else(|| "index cũ".to_owned(), |v| format!("version {v}"));
    format!("{version}, {} entry", index.entries.len())
}

fn pack_stems(dir: &Path) -> std::io::Result<Vec<String>> {
    let mut stems = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if !path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("idx"))
        {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        stems.push(stem.to_owned());
    }
    stems.sort();
    Ok(stems)
}
