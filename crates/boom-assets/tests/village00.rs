//! Kiểm trên Village00 thật. Dữ liệu game không nằm trong repo: đặt đường dẫn file đã giải nén vào
//! `BOOM_KR_VILLAGE00_MAP` / `BOOM_VN_VILLAGE00_MAP`; thiếu biến thì test tự bỏ qua.

use boom_assets::legacy_map::{Build, LegacyMap, parse};

fn load(var: &str, build: Build) -> Option<LegacyMap> {
    let path = std::env::var_os(var)?;
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("{var}: {e}"));
    Some(parse(&data, build).unwrap())
}

#[test]
fn kr_village00_matches_probe() {
    let Some(m) = load("BOOM_KR_VILLAGE00_MAP", Build::Kr) else {
        eprintln!("bỏ qua: chưa đặt BOOM_KR_VILLAGE00_MAP");
        return;
    };
    // RULES_V1 R-002, R-005, R-055.
    assert_eq!((m.version, m.grid.width, m.grid.height), (1000, 15, 13));
    assert_eq!(m.grid.spawns, vec![(13, 11), (1, 11), (1, 1), (13, 1)]);
    for &(x, y) in &m.grid.spawns {
        let cell = m.grid.cells[usize::from(y) * 15 + usize::from(x)];
        assert_eq!(cell.obj, None, "spawn ({x},{y}) phải là ô không có obj");
    }
    let names: Vec<_> = m
        .drops
        .iter()
        .map(|d| String::from_utf8_lossy(&d.name).into_owned())
        .collect();
    assert!(names.iter().all(|n| n.starts_with("Lucci")), "{names:?}");
    assert_eq!(m.drops.len(), 6);
}

#[test]
fn kr_and_vn_village00_share_layout() {
    let (Some(kr), Some(vn)) = (
        load("BOOM_KR_VILLAGE00_MAP", Build::Kr),
        load("BOOM_VN_VILLAGE00_MAP", Build::Vn),
    ) else {
        eprintln!("bỏ qua: cần cả BOOM_KR_VILLAGE00_MAP và BOOM_VN_VILLAGE00_MAP");
        return;
    };
    // RULES_V1 R-003, R-004: cùng bố cục, ID tile KR = VN + 937.
    assert_eq!(kr.grid.spawns, vn.grid.spawns);
    for (k, v) in kr.grid.cells.iter().zip(&vn.grid.cells) {
        assert_eq!(k.obj.is_some(), v.obj.is_some());
        assert_eq!(k.tile.zip(v.tile).map(|(a, b)| a - b), Some(937));
    }
}
