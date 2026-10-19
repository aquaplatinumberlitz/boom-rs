//! Nghiệm thu bộ đọc pack trên dữ liệu KR thật.
//!
//! Dữ liệu game có bản quyền không nằm trong repo: đặt `BOOM_KR_PACK_DIR` trỏ thư mục pack
//! (ví dụ `/home/admin/boomkr-full`); thiếu biến thì test tự bỏ qua. Test vector và test lỗi
//! nằm trong `src/pack.rs` nên luôn chạy trên CI.

use std::path::PathBuf;

use boom_assets::legacy_map::Build;
use boom_assets::pack::{Pack, load_index};

/// Thống kê index KR trong `docs/evidence/inventory/kr_feature_inventory.json`.
const KR_PACKS: &[(&str, usize)] = &[
    ("CharTE", 20_744),
    ("EmoTE", 10_040),
    ("FxAsset", 11),
    ("FxCharacter", 3_571),
    ("FxContents", 873),
    ("FxDesktop", 2_077),
    ("FxHiddenCatch", 303),
    ("FxMap", 3_033),
    ("FxMonster", 1_403),
    ("FxSound", 797),
    ("FxStage", 1_274),
    ("FxTile", 3_248),
    ("fx", 4_093),
];

fn pack_root() -> Option<PathBuf> {
    let path = std::env::var_os("BOOM_KR_PACK_DIR")?;
    Some(PathBuf::from(path))
}

#[test]
fn kr_pack_indexes_match_inventory_counts() {
    let Some(root) = pack_root() else {
        eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
        return;
    };
    let mut total = 0usize;
    for &(stem, entries) in KR_PACKS {
        let index = load_index(&root, stem, Build::Kr).unwrap_or_else(|e| panic!("{stem}: {e}"));
        assert_eq!(index.version, Some(15), "{stem}");
        assert_eq!(index.entries.len(), entries, "{stem}");
        total += entries;
    }
    // 13 pack / 51.467 entry: cùng con số trong kr_feature_inventory.json.
    assert_eq!(total, 51_467);
}

#[test]
fn kr_village00_and_preview_payloads_match_inventory() {
    let Some(root) = pack_root() else {
        eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
        return;
    };
    let pack = Pack::open(&root, "FxMap", Build::Kr).expect("mở FxMap");

    // Map thô: SHA-256 trong docs/evidence/inventory/kr_v1_content.json (role "map").
    let map = pack.read_named("Map/Village00").expect("đọc Village00");
    assert_eq!(map.len(), 1_834);
    assert_eq!(
        sha256_hex(&map),
        "bd34c418ecc33f126f2fee4c21549e78cdc5c31d7805a2af82bff0e635b67a07"
    );

    // Preview `_mm`: nhánh zlib; SHA-256 trong cùng inventory (role "preview").
    let preview = pack
        .read_named("Etc/Village00_mm")
        .expect("đọc preview Village00");
    assert_eq!(preview.len(), 48_089);
    assert_eq!(
        sha256_hex(&preview),
        "950b4576d50274092af68a7e20402489d61038f791288d5097562203b91600be"
    );
}

#[test]
fn missing_entry_is_an_error_naming_the_entry() {
    let Some(root) = pack_root() else {
        eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
        return;
    };
    let pack = Pack::open(&root, "FxMap", Build::Kr).expect("mở FxMap");
    let error = pack.read_named("Map/KhongTonTai").unwrap_err();
    assert!(error.to_string().contains("Map/KhongTonTai"));
}

/// SHA-256 thuần Rust cho test nghiệm thu; tránh thêm dependency chỉ để kiểm một hash.
fn sha256_hex(data: &[u8]) -> String {
    sha256(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[allow(clippy::needless_range_loop)] // lịch SHA-256 cần nhiều từ trước đó, không chỉ một slice
fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];

    let mut state: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity(data.len() + 72);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.as_chunks::<64>().0 {
        let mut schedule = [0u32; 64];
        for (index, word) in chunk.as_chunks::<4>().0.iter().enumerate() {
            schedule[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let s0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let s1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);
        let (mut e, mut f, mut g, mut h) = (state[4], state[5], state[6], state[7]);
        for (&k, &w) in K.iter().zip(schedule.iter()) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k)
                .wrapping_add(w);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut out = [0u8; 32];
    for (index, word) in state.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[test]
fn sha256_helper_matches_known_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
