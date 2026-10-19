//! CLI đọc dữ liệu client gốc:
//!
//! - `map-info <file.map> [--build kr|vn]`: kích thước, spawn, bảng rơi và lưới obj ID.
//! - `pack-list <pack-dir> [stem] [--build kr|vn]`: liệt kê pack, hoặc entry của một pack.
//! - `pack-extract <pack-dir> <stem> <entry> <out> [--build kr|vn]`: ghi payload đã giải mã.
//! - `sprite-info <pack-dir> <stem> <entry> [--build kr|vn]`: header, kích thước ảnh và
//!   frame của sprite `CDataI16Alpha` v2.
//! - `sprite-png <pack-dir> <stem> <entry> <out-dir> [--build kr|vn]`: ghi mỗi ảnh của
//!   sprite ra PNG để soi bằng mắt.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;
use std::process::ExitCode;

use boom_assets::legacy_map::{Build, parse};
use boom_assets::pack::{Pack, PackIndex, load_index};
use boom_assets::sprite::{self, Sprite};

struct Options {
    positional: Vec<String>,
    build: Build,
}

fn usage() -> ExitCode {
    eprintln!(
        "dùng:\n  \
         boom-assets map-info <file.map> [--build kr|vn]\n  \
         boom-assets pack-list <pack-dir> [stem] [--build kr|vn]\n  \
         boom-assets pack-extract <pack-dir> <stem> <entry> <out> [--build kr|vn]\n  \
         boom-assets sprite-info <pack-dir> <stem> <entry> [--build kr|vn]\n  \
         boom-assets sprite-png <pack-dir> <stem> <entry> <out-dir> [--build kr|vn]"
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
        Some("sprite-info") => sprite_info(&opts),
        Some("sprite-png") => sprite_png(&opts),
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

/// Nạp entry từ pack và parse thành sprite; báo lỗi rõ ràng rồi trả `None`.
fn load_sprite(dir: &Path, stem: &str, entry: &str, build: Build) -> Option<Sprite> {
    let pack = match Pack::open(dir, stem, build) {
        Ok(pack) => pack,
        Err(e) => {
            eprintln!("{stem}: {e}");
            return None;
        }
    };
    let blob = match pack.read_named(entry) {
        Ok(blob) => blob,
        Err(e) => {
            eprintln!("{stem}: {e}");
            return None;
        }
    };
    match sprite::parse(&blob) {
        Ok(sprite) => Some(sprite),
        Err(e) => {
            eprintln!("{entry}: {e}");
            None
        }
    }
}

/// `sprite-info`: header, kích thước ảnh và field frame (giữ nguyên số theo MAPS §3).
fn sprite_info(opts: &Options) -> ExitCode {
    let [dir, stem, entry] = opts.positional.as_slice() else {
        return usage();
    };
    let Some(sprite) = load_sprite(Path::new(dir), stem, entry, opts.build) else {
        return ExitCode::FAILURE;
    };
    println!(
        "{entry}: {} frame, {} ảnh, unk {}",
        sprite.frames.len(),
        sprite.images.len(),
        sprite.unk
    );
    for (index, image) in sprite.images.iter().enumerate() {
        println!(
            "  ảnh {index}: {}x{}, {} byte RLE",
            image.width,
            image.height,
            image.data().len()
        );
    }
    for (index, frame) in sprite.frames.iter().enumerate() {
        println!(
            "  frame {index}: offset=({},{}) flag={} ảnh={} src=({},{},{},{}) canvas={}x{} pivot=({},{})",
            frame.offset_x,
            frame.offset_y,
            frame.flag,
            frame.image_index,
            frame.source.x0,
            frame.source.y0,
            frame.source.x1,
            frame.source.y1,
            frame.canvas.width,
            frame.canvas.height,
            frame.pivot.x,
            frame.pivot.y
        );
    }
    ExitCode::SUCCESS
}

/// `sprite-png`: ghi mỗi ảnh của sprite ra PNG để soi bằng mắt.
fn sprite_png(opts: &Options) -> ExitCode {
    let [dir, stem, entry, out_dir] = opts.positional.as_slice() else {
        return usage();
    };
    let Some(sprite) = load_sprite(Path::new(dir), stem, entry, opts.build) else {
        return ExitCode::FAILURE;
    };
    let out_dir = Path::new(out_dir);
    if let Err(e) = fs::create_dir_all(out_dir) {
        eprintln!("{}: {e}", out_dir.display());
        return ExitCode::FAILURE;
    }
    let mut ok = true;
    for index in 0..sprite.images.len() {
        let image = &sprite.images[index];
        if image.data().is_empty() || image.width == 0 || image.height == 0 {
            continue;
        }
        let rgba = match sprite.decode_image(index) {
            Ok(rgba) => rgba,
            Err(e) => {
                eprintln!("{entry}: ảnh {index}: {e}");
                ok = false;
                continue;
            }
        };
        let path = out_dir.join(format!("{stem}_{}_{index}.png", file_stem(entry)));
        match write_png(&path, &rgba, image.width, image.height) {
            Ok(()) => println!("{} ({}x{})", path.display(), image.width, image.height),
            Err(e) => {
                eprintln!("{}: {e}", path.display());
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

/// Ghi RGBA ra PNG (8 bit, kênh RGBA).
fn write_png(path: &Path, rgba: &[u8], width: u32, height: u32) -> Result<(), Box<dyn Error>> {
    let file = File::create(path)?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    Ok(())
}

/// Tên entry thành tên file an toàn (giữ chữ, số và gạch dưới).
fn file_stem(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
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
