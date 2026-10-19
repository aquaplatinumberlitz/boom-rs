//! Nghiệm thu V1-030: manifest closure của một trận V1 (map Village00 KR).
//!
//! Dữ liệu game có bản quyền không nằm trong repo: đặt `BOOM_KR_PACK_DIR` trỏ thư mục pack
//! (ví dụ `/home/admin/boomkr-full`); thiếu biến thì test tự bỏ qua. Phần parse thuần (SHA-256,
//! bảng tên obj) nằm trong `src/closure.rs` nên luôn chạy trên CI.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use boom_assets::closure::{
    ClosureRole, UnknownKind, build_village00_manifest, resolve_theme, theme_names,
};
use boom_assets::legacy_map::{Build, parse as parse_map};
use boom_assets::pack::{Pack, load_index};

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn pack_root() -> Option<PathBuf> {
    let path = std::env::var_os("BOOM_KR_PACK_DIR")?;
    Some(PathBuf::from(path))
}

/// Đọc cặp `entry` → `sha256` từ `docs/evidence/inventory/kr_v1_content.json`.
///
/// File do tool Python sinh với `indent=2`: mỗi entry có khoá `"entry"` rồi tới khoá `"sha256"`.
fn fingerprint_index(path: &Path) -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut out = BTreeMap::new();
    let mut pending: Option<String> = None;
    for line in text.lines() {
        if let Some(entry) = json_field(line, "entry") {
            pending = Some(entry);
        } else if let Some(sha) = json_field(line, "sha256")
            && let Some(entry) = pending.take()
        {
            out.insert(entry, sha);
        }
    }
    out
}

/// Lấy giá trị chuỗi của một khoá JSON nằm trên một dòng.
fn json_field(line: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\":");
    let start = line.find(&marker)? + marker.len();
    let rest = line[start..].trim_start().strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

/// ID tile/obj đang dùng trong map, tách theo loại.
fn map_cell_ids(root: &Path) -> (BTreeSet<u32>, BTreeSet<u32>) {
    let pack = Pack::open(root, "FxMap", Build::Kr).expect("mở FxMap");
    let raw = pack.read_named("Map/Village00").expect("đọc Village00");
    let map = parse_map(&raw, Build::Kr).expect("parse Village00 KR");
    let mut tiles = BTreeSet::new();
    let mut objects = BTreeSet::new();
    for cell in &map.grid.cells {
        if let Some(tile) = cell.tile {
            tiles.insert(tile);
        }
        if let Some(object) = cell.obj {
            objects.insert(object);
        }
    }
    (tiles, objects)
}

#[test]
fn village00_closure_resolves_every_map_cell() {
    let Some(root) = pack_root() else {
        eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
        return;
    };
    let manifest = build_village00_manifest(&root).expect("dựng manifest closure");
    assert_eq!(manifest.schema, 1);
    assert_eq!(manifest.build, "kr");
    assert_eq!(manifest.map, "Village00");
    // Mọi ô của Village00 dùng tile set Tile00.
    assert_eq!(manifest.tileset, "Tile00");

    let (map_tiles, map_objects) = map_cell_ids(&root);
    let mut resolved_tiles = BTreeSet::new();
    let mut resolved_objects = BTreeSet::new();
    for entry in &manifest.entries {
        match entry.role {
            ClosureRole::Tile => {
                resolved_tiles.insert(entry.map_id.expect("ô nền phải có ID"));
            }
            ClosureRole::Object => {
                resolved_objects.insert(entry.map_id.expect("ô vật cản phải có ID"));
            }
            _ => {
                assert!(entry.map_id.is_none(), "entry không phải ô lại có ID");
                assert!(
                    entry.object_name.is_none(),
                    "entry không phải ô lại có tên obj"
                );
            }
        }
        // Mọi entry phải có SHA-256 hợp lệ.
        assert_eq!(entry.sha256.len(), 64, "{}", entry.entry);
    }

    // Ô nào chưa resolve được phải nằm trong unknown kèm lý do, không bỏ qua âm thầm.
    let mut unknown_tiles = BTreeSet::new();
    let mut unknown_objects = BTreeSet::new();
    for unknown in &manifest.unknown {
        assert!(
            !unknown.reason.is_empty(),
            "unknown {} thiếu lý do",
            unknown.id
        );
        if unknown.kind != UnknownKind::MapCell {
            continue;
        }
        let (kind, id) = unknown.id.split_once(':').expect("id dạng <loại>:<ID>");
        let id: u32 = id.parse().expect("ID là số");
        match kind {
            "tile" => {
                unknown_tiles.insert(id);
            }
            "obj" => {
                unknown_objects.insert(id);
            }
            other => panic!("loại ô lạ: {other}"),
        }
    }

    for id in &map_tiles {
        assert!(
            resolved_tiles.contains(id) || unknown_tiles.contains(id),
            "tile {id} không có entry và cũng không nằm trong unknown"
        );
    }
    for id in &map_objects {
        assert!(
            resolved_objects.contains(id) || unknown_objects.contains(id),
            "obj {id} không có entry và cũng không nằm trong unknown"
        );
    }
    // Không ô nào vừa có entry vừa nằm trong unknown.
    assert!(
        resolved_tiles.is_disjoint(&unknown_tiles)
            && resolved_objects.is_disjoint(&unknown_objects)
    );
}

#[test]
fn village00_closure_uses_the_expected_tile_and_object_entries() {
    let Some(root) = pack_root() else {
        eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
        return;
    };
    let manifest = build_village00_manifest(&root).expect("dựng manifest closure");
    let entry_of = |role: ClosureRole, id: u32| {
        manifest
            .entries
            .iter()
            .find(|e| e.role == role && e.map_id == Some(id))
            .unwrap_or_else(|| panic!("không có entry {role:?} cho ID {id}"))
            .entry
            .clone()
    };
    // Nền của Village00: Grass00/01, Road00/01/02 (R-007).
    assert_eq!(entry_of(ClosureRole::Tile, 1942), "FxTile/Tile00/Grass00");
    assert_eq!(entry_of(ClosureRole::Tile, 1943), "FxTile/Tile00/Grass01");
    assert_eq!(entry_of(ClosureRole::Tile, 1944), "FxTile/Tile00/Road00");
    assert_eq!(entry_of(ClosureRole::Tile, 1945), "FxTile/Tile00/Road01");
    assert_eq!(entry_of(ClosureRole::Tile, 1946), "FxTile/Tile00/Road02");
    // Vật cản: gạch, cây, nhà, đài phun (R-006).
    assert_eq!(
        entry_of(ClosureRole::Object, 1933),
        "FxTile/Tile00/BoxOrange"
    );
    assert_eq!(entry_of(ClosureRole::Object, 1934), "FxTile/Tile00/BoxRed");
    assert_eq!(
        entry_of(ClosureRole::Object, 1954),
        "FxTile/Tile00/Fountain00"
    );
    assert_eq!(
        entry_of(ClosureRole::Object, 1955),
        "FxTile/Tile00/HouseBlue"
    );
    assert_eq!(
        entry_of(ClosureRole::Object, 1956),
        "FxTile/Tile00/HouseRed"
    );
    assert_eq!(
        entry_of(ClosureRole::Object, 1957),
        "FxTile/Tile00/HouseYellow"
    );
    assert_eq!(entry_of(ClosureRole::Object, 1958), "FxTile/Tile00/Tree00");
}

#[test]
fn closure_covers_map_preview_music_characters_and_items() {
    let Some(root) = pack_root() else {
        eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
        return;
    };
    let manifest = build_village00_manifest(&root).expect("dựng manifest closure");
    let entries: BTreeSet<&str> = manifest.entries.iter().map(|e| e.entry.as_str()).collect();
    for expected in [
        "FxMap/Map/Village00",
        "FxMap/Theme/Village",
        "FxMap/Etc/Village00_mm",
        "FxSound/MP3/Village",
        "FxContents/Attr/BomberAttr",
        "FxContents/Attr/ItemAttr",
        "fx/Item/Bomb",
        "fx/Item/Fire",
        "fx/Item/Roller",
        "FxDesktop/Bomb/Bomb00",
        "FxDesktop/Bomb/BombExplosion001",
    ] {
        assert!(entries.contains(expected), "thiếu entry {expected}");
    }
    // Bốn hướng đi và state chết của cả C00 lẫn C01.
    for character in ["C00", "C01"] {
        for direction in ["Up", "Down", "Left", "Right"] {
            for layer in ["Static", "Adjust"] {
                let entry =
                    format!("FxCharacter/Character/{character}-Walk{direction}_ALayer{layer}");
                assert!(entries.contains(entry.as_str()), "thiếu entry {entry}");
            }
        }
        for layer in ["Static", "Adjust"] {
            let entry = format!("FxCharacter/Character/{character}-DieLayer{layer}");
            assert!(entries.contains(entry.as_str()), "thiếu entry {entry}");
        }
    }
}

#[test]
fn closure_sha256_matches_the_v1_content_inventory() {
    let Some(root) = pack_root() else {
        eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
        return;
    };
    let manifest = build_village00_manifest(&root).expect("dựng manifest closure");
    let inventory = fingerprint_index(&repo_path("docs/evidence/inventory/kr_v1_content.json"));
    assert!(
        inventory.len() >= 103,
        "inventory V1 content ít hơn 103 entry: {}",
        inventory.len()
    );

    // Phần giao với 103 entry đã fingerprint phải khớp SHA-256 y hệt.
    let mut checked = 0usize;
    for entry in &manifest.entries {
        let Some(expected) = inventory.get(&entry.entry) else {
            continue;
        };
        assert_eq!(
            &entry.sha256, expected,
            "SHA-256 khác inventory: {}",
            entry.entry
        );
        checked += 1;
    }
    // 11 entry cố định + 32 sprite nhân vật của C00/C01 nằm trong inventory.
    assert_eq!(checked, 43, "số entry trùng inventory: {checked}");

    // Entry ngoài inventory thì phải là phần mới của closure (tile/obj, lửa, bong bóng).
    for entry in &manifest.entries {
        if !inventory.contains_key(&entry.entry) {
            assert!(
                entry.entry.starts_with("FxTile/")
                    || entry.entry.ends_with("Fire00")
                    || entry.entry.ends_with("Bubble00"),
                "entry lạ không có trong inventory: {}",
                entry.entry
            );
        }
    }
}

#[test]
fn missing_pack_directory_is_reported_not_panicked() {
    let error = build_village00_manifest(Path::new("/tmp/boom-khong-ton-tai")).unwrap_err();
    let message = error.to_string();
    assert!(!message.is_empty());
}

/// Số liệu đo trên pack KR 00967 làm locator cho claim R-071 (quy tắc tra theme tổng quát,
/// trạng thái `candidate`): rule khớp 914/1.151 map, còn BGM `FxSound/MP3/<theme>` chỉ tồn
/// tại cho 32/80 theme dùng được.
///
/// Test bỏ qua khi thiếu `BOOM_KR_PACK_DIR`. Đổi pack thì số trong RULES_V1 R-071 phải sửa
/// theo, hoặc claim phải được xác nhận lại.
#[test]
fn kr_pack_theme_rule_coverage_matches_recorded_evidence() {
    let Some(root) = pack_root() else {
        eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
        return;
    };
    let index = load_index(&root, "FxMap", Build::Kr).expect("FxMap");
    let themes = theme_names(&index);
    assert_eq!(themes.len(), 142, "số thư mục Theme/* của FxMap KR đã đổi");

    let names: Vec<&str> = index.entries.iter().map(|e| e.name.as_str()).collect();
    let maps: Vec<&str> = names
        .iter()
        .filter_map(|name| name.strip_prefix("Map/"))
        .collect();
    let previews: BTreeSet<&str> = names
        .iter()
        .filter_map(|name| name.strip_prefix("Etc/"))
        .collect();
    let sound = load_index(&root, "FxSound", Build::Kr).expect("FxSound");
    let music: BTreeSet<&str> = sound
        .entries
        .iter()
        .map(|e| e.name.as_str())
        .filter(|name| name.starts_with("MP3/"))
        .collect();

    // Preview theo tên map: mọi map trong pack đều có entry `Etc/<Map>_mm`.
    assert_eq!(previews.len(), maps.len(), "số entry Etc/* đã đổi");
    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut matched = 0usize;
    for map in &maps {
        let preview = format!("{map}_mm");
        assert!(
            previews.contains(preview.as_str()),
            "thiếu preview {preview}"
        );
        if let Some(theme) = resolve_theme(map, &themes) {
            matched += 1;
            used.insert(theme);
        }
    }
    assert_eq!(maps.len(), 1151, "số map trong FxMap KR đã đổi");
    assert_eq!(matched, 914, "số map khớp quy tắc theme đã đổi");
    assert_eq!(used.len(), 80, "số theme dùng được đã đổi");
    let with_music = used
        .iter()
        .filter(|theme| music.contains(format!("MP3/{theme}").as_str()))
        .count();
    assert_eq!(with_music, 32, "số theme có MP3/<theme> đã đổi");
}
