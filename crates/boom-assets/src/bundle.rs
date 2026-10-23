//! Dựng bundle nội dung cho client: atlas ảnh + metadata ô của map + clip animation.
//!
//! Bundle là **output cục bộ** (không commit vào repo). Nó được dựng từ ba nguồn đã có
//! trong crate này:
//!
//! - [`crate::closure`] — manifest closure của một trận V1: entry pack nào cần có, vai trò,
//!   SHA-256, và danh sách phần chưa xác định kèm lý do (task #34).
//! - [`crate::sprite`] — bộ giải mã `CDataI16Alpha` v2: ảnh RGBA + frame 48 byte (task #33).
//! - [`crate::animation`] — bộ trích clip từ pack (task #52, RULES_V1 R-077…R-079).
//!
//! **Bảng mapping** (clip animation, asset UI, event→SFX, closure) là **data** ở
//! `data/assets/`, nạp qua [`boom_content::assets`]: mỗi dòng mang nhãn bằng chứng và
//! claim RULES_V1, nên đổi mapping là sửa file data chứ không sửa code.
//!
//! Kiểu đọc bundle nằm ở [`boom_content::bundle`] để client dùng được mà **không cần bộ giải
//! mã pack** (ARCHITECTURE §7). Định dạng bundle là quyết định `design` của dự án, mô tả ở
//! `docs/specification/assets/BUNDLES.md`.
//!
//! ## Tái lập
//!
//! Cùng pack ⇒ cùng `content_hash`:
//!
//! - Thứ tự sprite theo thứ tự manifest closure, rồi entry chỉ có trong bảng clip; thứ tự
//!   ảnh theo thứ tự blob gốc.
//! - Thứ tự clip theo thứ tự bảng clip ở `data/assets/animation-v1.toml`.
//! - Đóng gói atlas dùng thuật toán kệ (shelf) với thứ tự ảnh cố định
//!   (chiều cao giảm dần, rồi chiều rộng, rồi thứ tự ban đầu).
//! - `content_hash` băm **toàn bộ bundle**: metadata, SHA-256 của từng file atlas **và** payload audio,
//!   nên thay ảnh trong atlas thì hash đổi (đổi bộ mã PNG cũng đổi hash).

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use boom_content::animation::{AnimationClip, PresentationState};
use boom_content::assets::{AssetError, AssetTables, AudioMapping, UiMapping};
use boom_content::bundle::{
    AtlasPage, AudioAsset, Bundle, BundleError, Cell, Clip, EvidenceStatus, Frame, NonImageAsset,
    Sprite as BundleSprite, SpriteImage, UiElement, UiState, Unknown, sha256_hex,
};
use boom_content::rules::Status;

use crate::animation::extract_clips;
use crate::audio;
use crate::closure::{self, ClosureRole, UnknownKind, V1_MAP};
use crate::legacy_map::{Build, MapError, parse as parse_map};
use crate::pack::{Pack, PackError};
use crate::sprite::{self, SpriteError};

/// Chiều rộng tối đa của một trang atlas (px).
///
/// **Giới hạn kỹ thuật tự đặt** (`design` / `design`), không phải giới hạn của client gốc.
/// Số liệu KR đi kèm (`kr_static` / `confirmed`): cạnh ảnh lớn nhất đã quét là rộng 1198 px
/// và cao 1024 px (PACKS §Bộ giải mã Rust). Chọn 2048 để một trang chứa được mọi ảnh
/// Village00 mà vẫn vừa giới hạn texture của client.
pub const ATLAS_WIDTH: u32 = 2048;

/// Chiều cao tối đa của một trang atlas (px) — xem [`ATLAS_WIDTH`].
pub const ATLAS_HEIGHT: u32 = 2048;

/// Tên file metadata trong thư mục bundle.
pub const METADATA_FILE: &str = boom_content::bundle::METADATA_FILE;

/// Thư mục con chứa các trang atlas.
pub const ATLAS_DIR: &str = boom_content::bundle::ATLAS_DIR;

/// Vai trò closure nào là ảnh (đưa vào atlas); vai trò còn lại là dữ liệu/âm thanh.
fn is_image_role(role: ClosureRole) -> bool {
    matches!(
        role,
        ClosureRole::Tile
            | ClosureRole::Object
            | ClosureRole::Theme
            | ClosureRole::Preview
            | ClosureRole::CharacterWalk
            | ClosureRole::CharacterIdle
            | ClosureRole::CharacterTrapped
            | ClosureRole::CharacterDead
            | ClosureRole::BombArt
            | ClosureRole::FlameArt
            | ClosureRole::ExplosionArt
            | ClosureRole::TrappedArt
            | ClosureRole::ItemIcon
    )
}

/// Lý do ghi cho entry không phải ảnh.
fn non_image_reason(role: ClosureRole) -> &'static str {
    match role {
        ClosureRole::Map => "file map: lưới ô đã nằm trong bundle, byte map giữ để đối chiếu",
        ClosureRole::Music => "âm thanh nền: không phải ảnh, chưa đưa vào bundle",
        ClosureRole::CharacterAttr | ClosureRole::ItemAttr => {
            "bảng chỉ số: dữ liệu, không phải ảnh"
        }
        _ => "không phải ảnh",
    }
}

/// Ảnh đã giải mã, chưa đặt vào atlas.
struct RawImage {
    width: u32,
    height: u32,
    /// RGBA thô, 4 byte/pixel, thứ tự R, G, B, A.
    rgba: Vec<u8>,
}

/// Vị trí một ảnh trong atlas.
struct Placement {
    /// Chỉ số ảnh trong danh sách ảnh của bundle.
    image: usize,
    /// Toạ độ góc trên-trái trong trang (px).
    x: u32,
    /// Toạ độ góc trên-trái trong trang (px).
    y: u32,
}

/// Một trang atlas đã đóng gói.
struct PackedPage {
    placements: Vec<Placement>,
    width: u32,
    height: u32,
}

/// Bundle đã dựng kèm byte PNG của các trang atlas.
///
/// `content_hash` đã được chốt trong [`BuiltBundle::bundle`]; dựng lại từ cùng pack là ra
/// cùng hash.
pub struct BuiltBundle {
    /// Metadata bundle.
    pub bundle: Bundle,
    /// Byte PNG của từng trang atlas, theo thứ tình trang.
    pages: Vec<Vec<u8>>,
    /// Payload âm thanh thô, cùng thứ tự `bundle.audio`.
    audio: Vec<Vec<u8>>,
}

impl BuiltBundle {
    /// Ghi bundle ra thư mục: `bundle.toml` + các trang atlas trong `atlas/`.
    ///
    /// File `atlas/page-*.png` cũ không thuộc bundle này bị xoá để thư mục đầu ra không
    /// mang trang thừa.
    pub fn write(&self, out_dir: &Path) -> Result<(), BundleBuildError> {
        let atlas_dir = out_dir.join(ATLAS_DIR);
        fs::create_dir_all(&atlas_dir).map_err(|e| BundleBuildError::Io {
            path: atlas_dir.clone(),
            message: e.to_string(),
        })?;

        let mut written: Vec<String> = Vec::new();
        for (index, bytes) in self.pages.iter().enumerate() {
            let name = page_file_name(index);
            let path = atlas_dir.join(&name);
            fs::write(&path, bytes).map_err(|e| BundleBuildError::Io {
                path: path.clone(),
                message: e.to_string(),
            })?;
            written.push(name);
        }
        // Dọn trang atlas cũ không còn dùng (chỉ trong thư mục atlas của bundle).
        if let Ok(entries) = fs::read_dir(&atlas_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("page-") && name.ends_with(".png") && !written.contains(&name) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }

        let audio_dir = out_dir.join(boom_content::bundle::AUDIO_DIR);
        fs::create_dir_all(&audio_dir).map_err(|e| BundleBuildError::Io {
            path: audio_dir.clone(),
            message: e.to_string(),
        })?;
        let mut written_audio: Vec<String> = Vec::new();
        for (asset, bytes) in self.bundle.audio.iter().zip(&self.audio) {
            let name = asset
                .file
                .strip_prefix("audio/")
                .unwrap_or(&asset.file)
                .to_owned();
            let path = audio_dir.join(&name);
            fs::write(&path, bytes).map_err(|e| BundleBuildError::Io {
                path: path.clone(),
                message: e.to_string(),
            })?;
            written_audio.push(name);
        }
        if let Ok(entries) = fs::read_dir(&audio_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !written_audio.contains(&name) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }

        let meta_path = out_dir.join(METADATA_FILE);
        let text = self.bundle.to_toml().map_err(BundleBuildError::Bundle)?;
        fs::write(&meta_path, text).map_err(|e| BundleBuildError::Io {
            path: meta_path.clone(),
            message: e.to_string(),
        })?;
        Ok(())
    }
}

/// Tên file trang atlas theo chỉ số (trong thư mục `atlas/`).
fn page_file_name(index: usize) -> String {
    format!("page-{index:03}.png")
}

/// Đường dẫn trang atlas tương đối so với thư mục bundle.
fn page_relative_path(index: usize) -> String {
    format!("{ATLAS_DIR}/{}", page_file_name(index))
}

/// Dựng bundle của một map V1 từ thư mục chứa pack KR.
///
/// Thư mục phải có `.idx`/`.idd` của các pack KR (`FxMap`, `FxTile`, `fx`, `FxCharacter`,
/// `FxContents`, `FxDesktop`, `FxSound`).
///
/// Bảng mapping lấy từ `data/assets/` (nguồn chuẩn duy nhất, nhúng trong crate).
///
/// # Errors
///
/// Trả [`BundleBuildError`] khi đọc pack lỗi, bảng data sai quy tắc, hoặc bundle dựng ra
/// không nhất quán.
pub fn build_bundle(pack_dir: &Path, map_name: &str) -> Result<BuiltBundle, BundleBuildError> {
    let tables = AssetTables::v1()?;
    build_bundle_with(pack_dir, map_name, &tables)
}

/// Dựng bundle với bảng mapping cho trước.
///
/// Dùng để kiểm "đổi một mapping ⇒ `content_hash` đổi" mà không sửa file trong repo.
///
/// # Errors
///
/// Trả [`BundleBuildError`] khi đọc pack lỗi hoặc bundle dựng ra không nhất quán.
pub fn build_bundle_with(
    pack_dir: &Path,
    map_name: &str,
    tables: &AssetTables,
) -> Result<BuiltBundle, BundleBuildError> {
    if map_name != V1_MAP {
        return Err(BundleBuildError::UnsupportedMap(map_name.to_owned()));
    }
    let manifest = closure::build_village00_manifest_with(pack_dir, &tables.closure)?;

    let map_pack = Pack::open(pack_dir, "FxMap", Build::Kr)?;
    let map_raw = map_pack.read_named(&format!("Map/{V1_MAP}"))?;
    drop(map_pack);
    let map = parse_map(&map_raw, Build::Kr)?;

    let mut unknown: Vec<Unknown> = Vec::new();
    for item in &manifest.unknown {
        push_unknown(
            &mut unknown,
            item.kind.as_str(),
            &item.id,
            &item.reason,
            &item.candidates,
        );
    }

    let mut sprites: Vec<BundleSprite> = Vec::new();
    let mut assets: Vec<NonImageAsset> = Vec::new();
    let mut raw_images: Vec<RawImage> = Vec::new();
    // Ảnh nào thuộc sprite nào: (sprite index, chỉ số ảnh trong sprite).
    let mut image_owner: Vec<(usize, u32)> = Vec::new();
    // Clip animation của V1, theo thứ tự bảng `data/assets/animation-v1.toml`.
    let mut clips: Vec<Clip> = Vec::new();

    for entry in &manifest.entries {
        if !is_image_role(entry.role) {
            assets.push(NonImageAsset {
                entry: entry.entry.clone(),
                role: entry.role.as_str().to_owned(),
                status: EvidenceStatus::from(entry.status),
                source_sha256: entry.sha256.clone(),
                bytes: entry.bytes,
                reason: non_image_reason(entry.role).to_owned(),
            });
            continue;
        }
        let (pack, path) = match entry.entry.split_once('/') {
            Some((pack, path)) => (pack, path),
            None => {
                push_unknown(
                    &mut unknown,
                    "sprite",
                    &entry.entry,
                    "entry thiếu phần pack",
                    &[],
                );
                continue;
            }
        };
        match decode_sprite(pack_dir, pack, path) {
            Ok(decoded) => {
                push_sprite(
                    &mut sprites,
                    &mut raw_images,
                    &mut image_owner,
                    BundleSprite {
                        id: entry.entry.clone(),
                        entry: entry.entry.clone(),
                        role: entry.role.as_str().to_owned(),
                        status: EvidenceStatus::from(entry.status),
                        source_sha256: entry.sha256.clone(),
                        map_id: entry.map_id,
                        object_name: entry.object_name.clone(),
                        images: Vec::new(),
                        frames: decoded.frames,
                    },
                    &decoded.images,
                );
            }
            Err(reason) => {
                // Ảnh không giải được thì ghi rõ lý do, không bỏ qua âm thầm.
                push_unknown(&mut unknown, "sprite", &entry.entry, &reason, &[]);
            }
        }
    }

    // Clip animation V1: bảng entry ở `boom_assets::animation` (RULES_V1 R-077…R-079).
    //
    // Ảnh của clip nằm trong sprite tương ứng: nếu entry đã có trong closure thì dùng
    // lại (một bản pixel trong atlas), nếu chỉ có trong bảng clip thì thêm vào bundle
    // kèm vai trò riêng.
    let clip_set = extract_clips(pack_dir, Build::Kr, &tables.animation.clips)?;
    for clip in &clip_set.clips {
        let sprite_id = format!("{}/{}", clip.pack, clip.entry);
        if !sprites.iter().any(|sprite| sprite.id == sprite_id) {
            let Some(role) = clip_only_role(clip.state) else {
                push_unknown(
                    &mut unknown,
                    "animation_clip",
                    &clip.id,
                    &format!(
                        "entry {sprite_id} chỉ có trong bảng clip animation mà chưa có vai trò \
                         trong bundle (cần claim state → animation, xem RULES_V1 §8)"
                    ),
                    &[],
                );
                continue;
            };
            match decode_sprite(pack_dir, &clip.pack, &clip.entry) {
                Ok(decoded) => {
                    push_sprite(
                        &mut sprites,
                        &mut raw_images,
                        &mut image_owner,
                        BundleSprite {
                            id: sprite_id.clone(),
                            entry: sprite_id.clone(),
                            role: role.to_owned(),
                            status: EvidenceStatus::from(clip.status),
                            source_sha256: decoded.sha256.clone(),
                            map_id: None,
                            object_name: None,
                            images: Vec::new(),
                            frames: decoded.frames,
                        },
                        &decoded.images,
                    );
                }
                Err(reason) => {
                    push_unknown(&mut unknown, "animation_clip", &clip.id, &reason, &[]);
                    continue;
                }
            }
        }
        clips.push(clip_to_bundle(clip, &sprite_id));
    }

    // Entry có ảnh trong closure nhưng chưa có clip nào dùng tới: ghi rõ lý do thay vì
    // bỏ qua âm thầm. Hiệu ứng nổ đã có clip (R-084) nên nhánh này chỉ là lưới an toàn
    // cho entry ảnh mới chưa có clip.
    for entry in &manifest.entries {
        if entry.role != ClosureRole::ExplosionArt {
            continue;
        }
        if clips
            .iter()
            .any(|clip| clip.sprite == entry.entry || clip.entry == entry.entry)
        {
            continue;
        }
        push_unknown(
            &mut unknown,
            "animation_clip",
            &entry.entry,
            "entry có ảnh trong atlas nhưng bảng clip animation của V1 chưa có clip nào dùng \
             tới; chưa có claim state → animation cho entry này (RULES_V1 R-077/R-078/R-084)",
            &[],
        );
    }

    // Asset UI của luồng V1 (V1-032): ảnh của phần tử nằm trong atlas như sprite thường,
    // còn trạng thái dùng (có chữ chờ duyệt / không dùng asset) nằm ở mục `[[ui]]`.
    let mut ui: Vec<UiElement> = Vec::with_capacity(tables.ui.elements.len());
    for asset in &tables.ui.elements {
        let mut sprite = None;
        let mut decode_error = None;
        // Phần tử bị loại thì không đọc ảnh: asset đó không thuộc closure V1.
        if asset.state != UiState::Excluded
            && let Some(entry) = asset.entry.as_deref()
        {
            match decode_ui_sprite(pack_dir, entry) {
                Ok((sprite_id, decoded)) => {
                    push_sprite(
                        &mut sprites,
                        &mut raw_images,
                        &mut image_owner,
                        BundleSprite {
                            id: sprite_id.clone(),
                            entry: sprite_id.clone(),
                            role: boom_content::bundle::ROLE_UI.to_owned(),
                            status: EvidenceStatus::from(Status::Confirmed),
                            source_sha256: decoded.sha256.clone(),
                            map_id: None,
                            object_name: None,
                            images: Vec::new(),
                            frames: decoded.frames,
                        },
                        &decoded.images,
                    );
                    sprite = Some(sprite_id);
                }
                Err(why) => {
                    // Ảnh không giải được thì ghi rõ lý do, không bỏ qua âm thầm.
                    push_unknown(
                        &mut unknown,
                        "ui",
                        &asset.id,
                        &format!("{entry}: {why}"),
                        &[],
                    );
                    decode_error = Some(why);
                }
            }
        }
        let (state, reason) = ui_state_and_reason(asset, decode_error.as_deref());
        ui.push(UiElement {
            id: asset.id.clone(),
            screen: asset.screen,
            role: asset.role,
            entry: asset.entry.clone(),
            state,
            reason,
            sprite,
            source: asset.source,
            // Ảnh đọc được thì payload đã xác nhận; ảnh **không** đọc được thì chỉ mới
            // biết entry trỏ tới pack KR — chưa xác nhận được payload, nên hạ xuống
            // `candidate` (không suy diễn quá bằng chứng, AGENTS §4).
            status: match (EvidenceStatus::from(asset.status), decode_error.is_some()) {
                (EvidenceStatus::Confirmed, true) => EvidenceStatus::Candidate,
                (status, _) => status,
            },
        });
    }

    // Đóng gói atlas: thứ tự ảnh cố định nên cùng pack cho cùng bố cục.
    let pages = pack_atlas(&raw_images)?;

    // Ghi vị trí ảnh vào metadata và dựng trang atlas.
    let mut atlas: Vec<AtlasPage> = Vec::with_capacity(pages.len());
    let mut page_bytes: Vec<Vec<u8>> = Vec::with_capacity(pages.len());
    for page in &pages {
        let mut buffer = vec![0u8; (page.width as usize) * (page.height as usize) * 4];
        for placement in &page.placements {
            let image = &raw_images[placement.image];
            blit(
                &mut buffer,
                page.width,
                placement.x,
                placement.y,
                image.width,
                image.height,
                &image.rgba,
            );
            let (sprite_index, image_index) = image_owner[placement.image];
            sprites[sprite_index].images[image_index as usize].page =
                Some(u32::try_from(atlas.len()).expect("số trang atlas nhỏ"));
            sprites[sprite_index].images[image_index as usize].x = placement.x;
            sprites[sprite_index].images[image_index as usize].y = placement.y;
        }
        let png = encode_png(&buffer, page.width, page.height)?;
        atlas.push(AtlasPage {
            file: page_relative_path(atlas.len()),
            width: page.width,
            height: page.height,
            sha256: sha256_hex(&png),
        });
        page_bytes.push(png);
    }

    // Ô của map: ID tile/obj theo file nguồn.
    let width = map.grid.width;
    let height = map.grid.height;
    let mut cells: Vec<Cell> = Vec::with_capacity(map.grid.cells.len());
    for (index, cell) in map.grid.cells.iter().enumerate() {
        let x = u16::try_from(index % usize::from(width)).expect("chỉ số ô vừa lưới");
        let y = u16::try_from(index / usize::from(width)).expect("chỉ số ô vừa lưới");
        cells.push(Cell {
            x,
            y,
            tile: cell.tile,
            object: cell.obj,
        });
    }

    // Mọi ô phải có sprite, hoặc nằm trong unknown kèm lý do.
    for cell in &cells {
        if let Some(tile) = cell.tile
            && !sprites
                .iter()
                .any(|s| s.role == boom_content::bundle::ROLE_TILE && s.map_id == Some(tile))
        {
            push_unknown(
                &mut unknown,
                UnknownKind::MapCell.as_str(),
                &format!("tile:{tile}"),
                &format!(
                    "ô ({}, {}) dùng tile {tile} nhưng không có sprite nền trong closure",
                    cell.x, cell.y
                ),
                &[],
            );
        }
        if let Some(object) = cell.object
            && !sprites
                .iter()
                .any(|s| s.role == boom_content::bundle::ROLE_OBJECT && s.map_id == Some(object))
        {
            push_unknown(
                &mut unknown,
                UnknownKind::MapCell.as_str(),
                &format!("obj:{object}"),
                &format!(
                    "ô ({}, {}) dùng obj {object} nhưng không có sprite vật cản trong closure",
                    cell.x, cell.y
                ),
                &[],
            );
        }
    }

    // Audio V1: payload thô + hash, cùng quy tắc entry/mapping AUDIO_V1. Chỉ nhận định dạng
    // macroquad đọc được; mp3/unknown bị bỏ qua có log thay vì đưa file không phát được.
    //
    // Metadata entry đọc từ **chính bảng cue được truyền vào** (nguồn chuẩn), không từ
    // bảng audio mặc định trong crate: entry ngoài bảng mặc định vẫn phải được đọc thật,
    // không được coi là thiếu (review Codex #139). Entry không có trong pack thì dựng
    // bundle lỗi — bỏ qua âm thanh âm thầm làm `content_hash` không còn ràng buộc được
    // nội dung hiển thị.
    let sound_pack = Pack::open(pack_dir, "FxSound", Build::Kr)?;
    let mut info_by_entry: std::collections::BTreeMap<String, audio::AudioInfo> =
        std::collections::BTreeMap::new();
    for entry in audio_entries_of(&tables.audio.cues) {
        let info = audio::read_entry(&sound_pack, entry)?;
        info_by_entry.insert(entry.to_owned(), info);
    }
    let mut audio_assets = Vec::new();
    let mut audio_files = Vec::new();
    for usage in &tables.audio.cues {
        let Some(info) = info_by_entry.get(usage.entry.as_str()) else {
            // Không xảy ra: metadata đã nạp cho mọi entry mà cue trỏ tới.
            eprintln!(
                "audio: thiếu entry {} — bỏ qua cue {}",
                usage.entry, usage.event
            );
            continue;
        };
        let format = info.format.label();
        if !matches!(
            info.format,
            audio::AudioFormat::Ogg | audio::AudioFormat::Wav
        ) {
            eprintln!(
                "audio: bỏ qua {} ({format}) — macroquad không phát được định dạng này",
                usage.entry
            );
            continue;
        }
        let bytes = sound_pack.read_named(&usage.entry)?;
        let file = format!(
            "{}/{}.{format}",
            boom_content::bundle::AUDIO_DIR,
            usage.event
        );
        audio_assets.push(AudioAsset {
            id: usage.event.clone(),
            entry: usage.entry.clone(),
            kind: info.kind.label().to_owned(),
            format: format.to_owned(),
            file,
            bytes: u32::try_from(bytes.len()).expect("audio vừa u32"),
            sha256: info.sha256.clone(),
            status: EvidenceStatus::from(usage.status),
        });
        audio_files.push(bytes);
    }

    let mut bundle = Bundle {
        schema: boom_content::bundle::SCHEMA,
        build: manifest.build.clone(),
        map: manifest.map.clone(),
        tileset: manifest.tileset.clone(),
        width,
        height,
        spawns: map.grid.spawns.clone(),
        content_hash: String::new(),
        atlas,
        sprites,
        assets,
        cells,
        unknown,
        clips,
        ui,
        audio: audio_assets,
    };
    bundle.content_hash = bundle.compute_content_hash();
    bundle.validate().map_err(BundleBuildError::Bundle)?;

    Ok(BuiltBundle {
        bundle,
        pages: page_bytes,
        audio: audio_files,
    })
}

/// Các entry âm thanh mà các cue của bảng trỏ tới, dedup và giữ thứ tự bảng.
///
/// Bảng truyền vào là nguồn chuẩn: entry nào **không** có trong bảng audio mặc định của
/// crate vẫn phải được đọc từ pack, không được coi là thiếu (review Codex #139).
fn audio_entries_of(cues: &[AudioMapping]) -> Vec<&str> {
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    let mut entries: Vec<&str> = Vec::new();
    for cue in cues {
        if seen.insert(cue.entry.as_str()) {
            entries.push(cue.entry.as_str());
        }
    }
    entries
}

/// Sprite đã giải mã từ pack, sẵn sàng đưa ảnh vào atlas.
struct DecodedSprite {
    /// Frame của blob, theo thứ tự gốc.
    frames: Vec<Frame>,
    /// Ảnh của blob, theo thứ tự gốc (ảnh 0×0 là ảnh giữ chỗ, không có pixel).
    images: Vec<RawImage>,
    /// SHA-256 của payload gốc trong pack.
    sha256: String,
}

/// Đọc entry pack và giải thành sprite + ảnh RGBA.
///
/// Trả lỗi (dạng lý do) nếu blob không phải sprite, có ảnh rỗng, hoặc frame trỏ tới ảnh
/// không có.
fn decode_sprite(pack_dir: &Path, pack: &str, path: &str) -> Result<DecodedSprite, String> {
    let handle = Pack::open(pack_dir, pack, Build::Kr).map_err(|e| e.to_string())?;
    let blob = handle
        .read_named(path)
        .map_err(|e| format!("đọc entry thất bại: {e}"))?;
    drop(handle);

    let sha256 = sha256_hex(&blob);
    let parsed = sprite::parse(&blob).map_err(|e| format!("không parse được sprite: {e}"))?;
    let mut images: Vec<RawImage> = Vec::with_capacity(parsed.images.len());
    for (index, image) in parsed.images.iter().enumerate() {
        if image.width == 0 || image.height == 0 {
            // Ảnh giữ chỗ 0×0 của client gốc: không có pixel, không đặt trong atlas.
            images.push(RawImage {
                width: image.width,
                height: image.height,
                rgba: Vec::new(),
            });
            continue;
        }
        let rgba = parsed
            .decode_image(index)
            .map_err(|e| format!("giải ảnh {index} thất bại: {e}"))?;
        images.push(RawImage {
            width: image.width,
            height: image.height,
            rgba,
        });
    }

    let mut frames: Vec<Frame> = Vec::with_capacity(parsed.frames.len());
    for frame in &parsed.frames {
        let image_index = u32::try_from(frame.image_index).ok();
        let Some(image_index) = image_index.filter(|index| (*index as usize) < images.len()) else {
            return Err(format!(
                "frame trỏ tới ảnh {} không có (blob có {} ảnh)",
                frame.image_index,
                images.len()
            ));
        };
        frames.push(Frame {
            image: image_index,
            offset_x: frame.offset_x,
            offset_y: frame.offset_y,
            flag: frame.flag,
            x0: frame.source.x0,
            y0: frame.source.y0,
            x1: frame.source.x1,
            y1: frame.source.y1,
            canvas_width: frame.canvas.width,
            canvas_height: frame.canvas.height,
            pivot_x: frame.pivot.x,
            pivot_y: frame.pivot.y,
        });
    }
    Ok(DecodedSprite {
        frames,
        images,
        sha256,
    })
}

/// Đọc entry pack của một phần tử UI và giải thành sprite + ảnh RGBA.
///
/// ID của sprite trong bundle là chính entry pack (`<pack>/<entry>`), nên `[[ui]]` trỏ
/// tới `[[sprites]]` bằng đúng đường dẫn đó.
fn decode_ui_sprite(pack_dir: &Path, entry: &str) -> Result<(String, DecodedSprite), String> {
    let (pack, path) = entry
        .split_once('/')
        .ok_or_else(|| format!("entry {entry} thiếu phần pack"))?;
    let decoded = decode_sprite(pack_dir, pack, path)?;
    Ok((entry.to_owned(), decoded))
}

/// Trạng thái dùng và lý do của phần tử UI sau khi thử đọc ảnh từ pack.
///
/// Ảnh đọc được → giữ nguyên trạng thái của bảng closure. Ảnh **không giải được** →
/// hạ xuống `excluded` kèm lý do kỹ thuật: phần tử đó không có ảnh trong atlas nên
/// client vẽ text thay, còn bundle vẫn dựng được — cùng cách xử lý sprite và clip
/// không giải được.
fn ui_state_and_reason(asset: &UiMapping, decode_error: Option<&str>) -> (UiState, String) {
    match decode_error {
        None => (asset.state, asset.reason.to_owned()),
        Some(why) => (
            UiState::Excluded,
            format!("{}; ảnh không giải được: {why}", asset.reason),
        ),
    }
}

/// Đưa một sprite đã giải mã vào bundle: ảnh có pixel vào atlas, metadata vào `[[sprites]]`.
///
/// `sprite.images` được điền ở đây; vị trí trong trang atlas ghi sau khi đóng gói.
fn push_sprite(
    sprites: &mut Vec<BundleSprite>,
    raw_images: &mut Vec<RawImage>,
    image_owner: &mut Vec<(usize, u32)>,
    mut sprite: BundleSprite,
    images: &[RawImage],
) {
    let sprite_index = sprites.len();
    let mut sprite_images: Vec<SpriteImage> = Vec::with_capacity(images.len());
    for (image_index, image) in images.iter().enumerate() {
        let empty = image.width == 0 || image.height == 0;
        if !empty {
            image_owner.push((sprite_index, image_index as u32));
            raw_images.push(RawImage {
                width: image.width,
                height: image.height,
                rgba: image.rgba.clone(),
            });
        }
        sprite_images.push(SpriteImage {
            index: image_index as u32,
            // Ảnh rỗng không đặt trong atlas; ảnh có pixel sẽ được ghi vị trí
            // sau khi đóng gói.
            page: if empty { None } else { Some(0) },
            x: 0,
            y: 0,
            width: image.width,
            height: image.height,
            pixel_sha256: sha256_hex(&image.rgba),
        });
    }
    sprite.images = sprite_images;
    sprites.push(sprite);
}

/// Vai trò của sprite **bổ sung**: entry chỉ có trong bảng clip animation, chưa có trong
/// closure manifest (closure chỉ liệt kê entry đã fingerprint cho một trận V1).
///
/// `closure::ClosureRole` không có vai trò cho hai trường hợp này và `closure.rs` ngoài
/// phạm vi sửa của task này, nên bundle ghi vai trò riêng (xem BUNDLES.md §3).
pub const ROLE_CLIP_START: &str = "character_start";

/// Vai trò của sprite **bổ sung**: lửa nhạt (motion blur) theo hướng.
pub const ROLE_CLIP_MOTION_BLUR: &str = "flame_motion_blur";

/// Vai trò cho entry chỉ có trong bảng clip animation; `None` nếu chưa có vai trò nào.
fn clip_only_role(state: PresentationState) -> Option<&'static str> {
    match state {
        PresentationState::Start => Some(ROLE_CLIP_START),
        PresentationState::Flame(boom_content::animation::FlameKind::MotionBlur(_)) => {
            Some(ROLE_CLIP_MOTION_BLUR)
        }
        _ => None,
    }
}

/// Đổi clip của bộ trích sang clip trong bundle.
///
/// Frame của clip dùng **cùng** kiểu với frame của sprite: chỉ số ảnh trỏ vào ảnh của
/// sprite `sprite_id` trong bảng `[[sprites]]`.
fn clip_to_bundle(clip: &AnimationClip, sprite_id: &str) -> Clip {
    Clip {
        id: clip.id.clone(),
        state: clip.state,
        pack: clip.pack.clone(),
        entry: clip.entry.clone(),
        layer: clip.layer,
        sprite_unk: clip.sprite_unk,
        sprite: sprite_id.to_owned(),
        source: clip.source,
        status: EvidenceStatus::from(clip.status),
        claims: clip.claims.clone(),
        note: clip.note.clone(),
        timing: clip.timing.clone(),
        frames: clip
            .frames
            .iter()
            .map(|frame| Frame {
                image: frame.image_index,
                offset_x: frame.offset_x,
                offset_y: frame.offset_y,
                flag: frame.flag,
                x0: frame.source.x0,
                y0: frame.source.y0,
                x1: frame.source.x1,
                y1: frame.source.y1,
                canvas_width: frame.canvas.width,
                canvas_height: frame.canvas.height,
                pivot_x: frame.pivot.x,
                pivot_y: frame.pivot.y,
            })
            .collect(),
    }
}

/// Đóng gói ảnh vào các trang atlas (thuật toán kệ).
///
/// Thứ tự ảnh: chiều cao giảm dần, rồi chiều rộng giảm dần, rồi thứ tự ban đầu — cố định
/// nên cùng đầu vào cho cùng bố cục.
fn pack_atlas(images: &[RawImage]) -> Result<Vec<PackedPage>, BundleBuildError> {
    let mut order: Vec<usize> = (0..images.len()).collect();
    order.sort_by(|&a, &b| {
        let left = &images[a];
        let right = &images[b];
        right
            .height
            .cmp(&left.height)
            .then(right.width.cmp(&left.width))
            .then(a.cmp(&b))
    });

    let mut pages: Vec<PackedPage> = Vec::new();
    let mut current = PackedPage {
        placements: Vec::new(),
        width: ATLAS_WIDTH,
        height: 0,
    };
    let mut x = 0u32;
    let mut y = 0u32;
    let mut shelf = 0u32;
    for index in order {
        let image = &images[index];
        if image.width > ATLAS_WIDTH || image.height > ATLAS_HEIGHT {
            return Err(BundleBuildError::ImageTooLarge {
                width: image.width,
                height: image.height,
            });
        }
        if x + image.width > ATLAS_WIDTH {
            y += shelf;
            x = 0;
            shelf = 0;
        }
        if y + image.height > ATLAS_HEIGHT {
            current.height = y + shelf;
            pages.push(current);
            current = PackedPage {
                placements: Vec::new(),
                width: ATLAS_WIDTH,
                height: 0,
            };
            x = 0;
            y = 0;
            shelf = 0;
        }
        current.placements.push(Placement { image: index, x, y });
        x += image.width;
        shelf = shelf.max(image.height);
    }
    current.height = y + shelf;
    if !current.placements.is_empty() {
        pages.push(current);
    }
    Ok(pages)
}

/// Chép ảnh RGBA vào buffer trang atlas tại `(x, y)`.
fn blit(buffer: &mut [u8], page_width: u32, x: u32, y: u32, width: u32, height: u32, rgba: &[u8]) {
    for row in 0..height {
        let target = ((y + row) as usize) * (page_width as usize) * 4 + (x as usize) * 4;
        let source = (row as usize) * (width as usize) * 4;
        let len = (width as usize) * 4;
        buffer[target..target + len].copy_from_slice(&rgba[source..source + len]);
    }
}

/// Mã hoá buffer RGBA thành PNG (8 bit, kênh RGBA).
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, BundleBuildError> {
    let mut out: Vec<u8> = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| BundleBuildError::Png(e.to_string()))?;
        writer
            .write_image_data(rgba)
            .map_err(|e| BundleBuildError::Png(e.to_string()))?;
    }
    Ok(out)
}

/// Thêm một mục unknown; không lặp lại cùng một ID.
fn push_unknown(
    unknown: &mut Vec<Unknown>,
    kind: &str,
    id: &str,
    reason: &str,
    candidates: &[String],
) {
    if unknown.iter().any(|item| item.id == id) {
        return;
    }
    unknown.push(Unknown {
        kind: kind.to_owned(),
        id: id.to_owned(),
        reason: reason.to_owned(),
        candidates: candidates.to_vec(),
    });
}

/// Lỗi khi dựng hoặc ghi bundle.
#[derive(Debug)]
pub enum BundleBuildError {
    /// Lỗi dựng manifest closure.
    Closure(crate::closure::ClosureError),
    /// Lỗi đọc pack.
    Pack(PackError),
    /// Lỗi đọc file map.
    Map(MapError),
    /// Lỗi giải sprite.
    Sprite(SpriteError),
    /// Lỗi trích clip animation.
    Animation(crate::animation::ExtractError),
    /// Lỗi đọc âm thanh.
    Audio(crate::audio::AudioError),
    /// Bundle không nhất quán (tham chiếu hỏng, hash lệch).
    Bundle(BundleError),
    /// Map chưa hỗ trợ.
    UnsupportedMap(String),
    /// Ảnh vượt trần của trang atlas.
    ImageTooLarge {
        /// Chiều rộng ảnh (px).
        width: u32,
        /// Chiều cao ảnh (px).
        height: u32,
    },
    /// Lỗi đọc/ghi file.
    Io {
        /// Đường dẫn file.
        path: PathBuf,
        /// Thông điệp lỗi gốc.
        message: String,
    },
    /// Lỗi mã hoá PNG.
    Png(String),
    /// Bảng mapping trong `data/assets/` sai quy tắc.
    Assets(AssetError),
}

impl fmt::Display for BundleBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closure(e) => write!(f, "lỗi closure: {e}"),
            Self::Pack(e) => write!(f, "lỗi pack: {e}"),
            Self::Map(e) => write!(f, "lỗi map: {e}"),
            Self::Sprite(e) => write!(f, "lỗi sprite: {e}"),
            Self::Animation(e) => write!(f, "lỗi trích clip animation: {e}"),
            Self::Audio(e) => write!(f, "lỗi âm thanh: {e}"),
            Self::Bundle(e) => write!(f, "lỗi bundle: {e}"),
            Self::UnsupportedMap(name) => {
                write!(f, "chưa hỗ trợ dựng bundle cho map {name:?}")
            }
            Self::ImageTooLarge { width, height } => write!(
                f,
                "ảnh {width}x{height} vượt trần atlas {ATLAS_WIDTH}x{ATLAS_HEIGHT}"
            ),
            Self::Io { path, message } => write!(f, "{}: {message}", path.display()),
            Self::Png(e) => write!(f, "lỗi mã hoá PNG: {e}"),
            Self::Assets(e) => write!(f, "bảng mapping content sai quy tắc: {e}"),
        }
    }
}

impl std::error::Error for BundleBuildError {}

impl From<crate::closure::ClosureError> for BundleBuildError {
    fn from(value: crate::closure::ClosureError) -> Self {
        Self::Closure(value)
    }
}

impl From<PackError> for BundleBuildError {
    fn from(value: PackError) -> Self {
        Self::Pack(value)
    }
}

impl From<MapError> for BundleBuildError {
    fn from(value: MapError) -> Self {
        Self::Map(value)
    }
}

impl From<SpriteError> for BundleBuildError {
    fn from(value: SpriteError) -> Self {
        Self::Sprite(value)
    }
}

impl From<crate::audio::AudioError> for BundleBuildError {
    fn from(value: crate::audio::AudioError) -> Self {
        Self::Audio(value)
    }
}

impl From<crate::animation::ExtractError> for BundleBuildError {
    fn from(value: crate::animation::ExtractError) -> Self {
        Self::Animation(value)
    }
}

impl From<BundleError> for BundleBuildError {
    fn from(value: BundleError) -> Self {
        Self::Bundle(value)
    }
}

impl From<AssetError> for BundleBuildError {
    fn from(value: AssetError) -> Self {
        Self::Assets(value)
    }
}

/// Chuyển trạng thái bằng chứng của closure sang nhãn của bundle.
#[must_use]
pub fn status_label(status: Status) -> &'static str {
    EvidenceStatus::from(status).as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use boom_content::bundle::UiScreen;

    /// Thư mục pack KR thật; test bỏ qua khi chưa đặt `BOOM_KR_PACK_DIR`.
    fn pack_root() -> Option<PathBuf> {
        let path = std::env::var_os("BOOM_KR_PACK_DIR")?;
        Some(PathBuf::from(path))
    }

    /// Thư mục tạm cho test; tự xoá khi test xong.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("boom-bundle-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("tạo thư mục tạm");
        dir
    }

    /// Ảnh tổng hợp để kiểm thuật toán đóng gói (không cần pack thật).
    fn raw(width: u32, height: u32) -> RawImage {
        RawImage {
            width,
            height,
            rgba: vec![0x7Fu8; (width as usize) * (height as usize) * 4],
        }
    }

    #[test]
    fn atlas_packing_is_deterministic_and_inside_the_page() {
        let images = vec![raw(40, 40), raw(30, 70), raw(10, 10), raw(60, 20)];
        let first = pack_atlas(&images).expect("đóng gói");
        let second = pack_atlas(&images).expect("đóng gói lại");
        assert_eq!(first.len(), second.len());
        for (left, right) in first.iter().zip(second.iter()) {
            assert_eq!(left.height, right.height);
            assert_eq!(left.placements.len(), right.placements.len());
            for (a, b) in left.placements.iter().zip(right.placements.iter()) {
                assert_eq!((a.image, a.x, a.y), (b.image, b.x, b.y));
            }
        }
        // Mọi ảnh nằm trong trang, không chồng nhau theo trục x trong cùng kệ.
        for page in &first {
            assert!(page.height <= ATLAS_HEIGHT);
            for placement in &page.placements {
                let image = &images[placement.image];
                assert!(placement.x + image.width <= ATLAS_WIDTH);
                assert!(placement.y + image.height <= page.height);
            }
        }
    }

    #[test]
    fn atlas_rejects_an_image_larger_than_a_page() {
        let images = vec![raw(ATLAS_WIDTH + 1, 10)];
        assert!(matches!(
            pack_atlas(&images),
            Err(BundleBuildError::ImageTooLarge { .. })
        ));
    }

    /// Nghiệm thu V1-030: dựng bundle hai lần từ pack thật cho cùng `content_hash`, và
    /// reader trong `boom-content` nạp lại được, kiểm mọi tham chiếu.
    #[test]
    fn village00_bundle_is_reproducible_and_reloadable() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let first = build_bundle(&root, V1_MAP).expect("dựng bundle");
        let second = build_bundle(&root, V1_MAP).expect("dựng bundle lần hai");
        assert_eq!(
            first.bundle.content_hash, second.bundle.content_hash,
            "dựng hai lần phải cho cùng content_hash"
        );
        assert_eq!(first.bundle, second.bundle, "metadata phải giống nhau");
        assert_eq!(first.pages, second.pages, "byte PNG phải giống nhau");

        // Ghi ra đĩa rồi nạp lại bằng reader của boom-content.
        let dir = temp_dir("repro");
        first.write(&dir).expect("ghi bundle");
        let loaded = boom_content::bundle::load(&dir).expect("nạp lại bundle");
        assert_eq!(loaded.content_hash, first.bundle.content_hash);
        assert_eq!(loaded.sprites.len(), first.bundle.sprites.len());
        assert_eq!(loaded.cells.len(), 15 * 13, "lưới Village00 là 15x13");
        assert_eq!(loaded.clips.len(), first.bundle.clips.len());
        assert!(!loaded.clips.is_empty(), "bundle phải có clip animation");
        assert_eq!(
            loaded.audio.len(),
            first.bundle.audio.len(),
            "bundle phải nạp lại bảng audio"
        );
        assert_eq!(loaded.audio.len(), 15, "AUDIO_V1 có 15 entry V1");
        assert!(
            loaded.audio.iter().any(|audio| audio.id == "bgm_loop"),
            "phải có BGM Village"
        );

        // Mọi ô có sprite nền, hoặc nằm trong unknown kèm lý do.
        for cell in &loaded.cells {
            let tile = cell.tile.expect("ô Village00 luôn có nền");
            let resolved = loaded
                .sprites
                .iter()
                .any(|s| s.role == "tile" && s.map_id == Some(tile));
            let unknown = loaded
                .unknown
                .iter()
                .any(|u| u.id == format!("tile:{tile}") && !u.reason.is_empty());
            assert!(
                resolved || unknown,
                "ô ({}, {}) chưa có sprite",
                cell.x,
                cell.y
            );
        }
        // Có cả sprite nền lẫn vật cản cho Village00.
        assert!(
            loaded
                .sprites
                .iter()
                .any(|s| s.role == "object" && s.map_id.is_some()),
            "bundle phải có sprite vật cản"
        );
        // SHA-256 file map trong bundle khớp giá trị đã ghi ở SCOPE_V1 §2.
        let map_asset = loaded
            .assets
            .iter()
            .find(|asset| asset.entry == "FxMap/Map/Village00")
            .expect("bundle phải giữ entry file map");
        assert_eq!(
            map_asset.source_sha256,
            "bd34c418ecc33f126f2fee4c21549e78cdc5c31d7805a2af82bff0e635b67a07"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// Nghiệm thu V1-032: bundle phải mang bảng UI của luồng V1 kèm trạng thái dùng, và
    /// ảnh của phần tử nằm trong atlas.
    ///
    /// Mọi phần tử có asset đều phải trỏ tới một sprite có thật; phần tử bị loại thì
    /// không có ảnh trong atlas. Không phần tử nào ở trạng thái "đã duyệt": ảnh có chữ
    /// chờ quyết định duyệt ảnh.
    #[test]
    fn bundle_carries_the_v1_ui_flow_with_its_review_state() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let built = build_bundle(&root, V1_MAP).expect("dựng bundle");
        let ui = &built.bundle.ui;
        assert!(!ui.is_empty(), "bundle phải có bảng UI");

        // Mọi màn hình của luồng V1 đều phải có mặt.
        for screen in [
            UiScreen::Login,
            UiScreen::Lobby,
            UiScreen::Room,
            UiScreen::Match,
            UiScreen::Result,
        ] {
            assert!(
                !built.bundle.ui_by_screen(screen).is_empty(),
                "màn {screen:?} chưa có phần tử UI"
            );
        }

        for element in ui {
            assert!(!element.id.is_empty());
            assert!(
                !element.reason.is_empty(),
                "phần tử {} thiếu lý do",
                element.id
            );
            match element.state {
                UiState::Selected | UiState::NeedsReviewText => {
                    assert!(
                        element.entry.is_some(),
                        "phần tử {} phải có entry",
                        element.id
                    );
                    let sprite = element
                        .sprite
                        .as_deref()
                        .unwrap_or_else(|| panic!("phần tử {} thiếu sprite", element.id));
                    assert!(
                        built.bundle.sprites.iter().any(|item| item.id == sprite),
                        "phần tử {} trỏ sprite không có trong bundle",
                        element.id
                    );
                }
                UiState::Excluded => {
                    assert!(
                        element.sprite.is_none(),
                        "phần tử {} bị loại thì không được có ảnh trong atlas",
                        element.id
                    );
                }
            }
        }

        // Ảnh có chữ không bao giờ ở trạng thái "đã duyệt": bảng chỉ có ba trạng thái.
        assert!(
            ui.iter()
                .any(|element| element.state == UiState::NeedsReviewText)
        );
        assert!(ui.iter().any(|element| element.state == UiState::Selected));

        // Reader của boom-content nạp lại được bundle có bảng UI.
        let dir = temp_dir("ui");
        built.write(&dir).expect("ghi bundle");
        let loaded = boom_content::bundle::load(&dir).expect("nạp lại bundle");
        assert_eq!(loaded.ui.len(), ui.len());
        assert_eq!(
            loaded.ui_element("lobby.create").map(|item| item.state),
            Some(UiState::NeedsReviewText)
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// Ảnh của phần tử UI **không giải được** thì phần tử hạ xuống `excluded` kèm lý do
    /// kỹ thuật: không có ảnh trong atlas thì client vẽ text thay, còn bundle vẫn dựng
    /// được (cùng cách xử lý sprite và clip không giải được).
    #[test]
    fn ui_element_with_an_undecodable_image_falls_back_to_excluded() {
        let asset = UiMapping {
            id: "lobby.create".to_owned(),
            screen: UiScreen::Lobby,
            role: boom_content::bundle::UiRole::Button,
            entry: Some("FxStage/Lobby/LB_Bt_T_Create-NM".to_owned()),
            state: UiState::NeedsReviewText,
            reason: "ảnh nút có chữ Hàn".to_owned(),
            source: boom_content::rules::Source::KrStatic,
            status: boom_content::rules::Status::Confirmed,
            claims: Vec::new(),
            note: None,
        };

        // Ảnh đọc được: giữ nguyên trạng thái của bảng closure.
        let (state, reason) = ui_state_and_reason(&asset, None);
        assert_eq!(state, UiState::NeedsReviewText);
        assert_eq!(reason, asset.reason);

        // Ảnh không giải được: hạ xuống `excluded` và ghi rõ lý do.
        let (state, reason) = ui_state_and_reason(&asset, Some("blob không đúng phiên bản"));
        assert_eq!(state, UiState::Excluded);
        assert!(
            reason.contains("blob không đúng phiên bản") && reason.contains("ảnh nút có chữ Hàn"),
            "lý do phải giữ cả ghi chú closure lẫn lỗi kỹ thuật: {reason}"
        );
    }

    /// Bundle của map khác chưa hỗ trợ thì báo lỗi rõ ràng.
    #[test]
    fn unsupported_map_is_reported() {
        match build_bundle(Path::new("/tmp/boom-khong-ton-tai"), "Village01") {
            Err(BundleBuildError::UnsupportedMap(name)) => assert_eq!(name, "Village01"),
            Err(other) => panic!("lỗi sai: {other}"),
            Ok(_) => panic!("map khác phải bị từ chối"),
        }
    }

    /// Nghiệm thu issue #123: bundle dựng từ bảng data (`data/assets/`) phải cho
    /// **cùng** `content_hash` với bundle dựng từ bảng Rust trước khi chuyển mapping
    /// sang data — nội dung không đổi thì hash không đổi.
    #[test]
    fn bundle_from_the_data_tables_matches_the_recorded_content_hash() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let built = build_bundle(&root, V1_MAP).expect("dựng bundle");
        assert_eq!(
            built.bundle.content_hash,
            "dea731aa0ca62cbac6b01f0627700f886905717258d0ceeeeaf90f715f1f5eae",
            "hash ghi ở BUNDLES §6 phải khớp bundle dựng từ data/assets/"
        );
    }

    /// Nghiệm thu review Codex #139: bảng audio **truyền vào** là nguồn chuẩn — entry mà
    /// cue trỏ tới nằm ngoài bảng audio mặc định của crate vẫn phải được đọc thật.
    ///
    /// Trước khi sửa, `build_bundle_with` nạp lại bảng audio mặc định để lấy metadata
    /// entry: entry mới bị coi là thiếu và cue bị bỏ khỏi bundle dù asset có thật.
    #[test]
    fn audio_cue_outside_the_default_table_is_still_built() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let base = build_bundle(&root, V1_MAP).expect("dựng bundle");
        let mut tables = AssetTables::v1().expect("bảng V1");
        let cue = tables
            .audio
            .cues
            .iter_mut()
            .find(|cue| cue.event == "bomb_placed")
            .expect("cue bomb_placed");
        // Entry có thật trong pack `FxSound` nhưng không có trong bảng audio mặc định.
        cue.entry = "Wave/BombExplosion002".to_owned();
        let changed = build_bundle_with(&root, V1_MAP, &tables).expect("dựng bundle");

        // Cue không bị bỏ, và mang entry mới.
        assert_eq!(
            changed.bundle.audio.len(),
            base.bundle.audio.len(),
            "không được bỏ cue nào"
        );
        let audio = changed
            .bundle
            .audio
            .iter()
            .find(|audio| audio.id == "bomb_placed")
            .expect("cue bomb_placed vẫn phải có trong bundle");
        assert_eq!(audio.entry, "Wave/BombExplosion002", "phải mang entry mới");
        assert_eq!(audio.format, "ogg", "entry mới là Ogg");
        assert_eq!(audio.bytes, 23168, "kích thước payload của entry mới");

        // Hash phải là hash của entry mới, không phải của entry cũ.
        let pack = Pack::open(&root, "FxSound", Build::Kr).expect("mở pack FxSound");
        let expected = audio::read_entry(&pack, "Wave/BombExplosion002").expect("đọc entry mới");
        assert_eq!(audio.sha256, expected.sha256, "hash phải của entry mới");
        let old = base
            .bundle
            .audio
            .iter()
            .find(|audio| audio.id == "bomb_placed")
            .expect("cue bomb_placed của bundle gốc");
        assert_ne!(old.sha256, audio.sha256, "hash phải đổi theo entry");

        // Đổi entry ⇒ `content_hash` đổi.
        assert_ne!(
            base.bundle.content_hash, changed.bundle.content_hash,
            "đổi entry của cue phải đổi content_hash"
        );
    }

    /// Bảng audio truyền vào là nguồn chuẩn: entry của cue ngoài bảng mặc định vẫn phải
    /// được đọc, và entry lặp chỉ đọc một lần (review Codex #139).
    ///
    /// Test này không cần pack thật nên chạy được ở CI.
    #[test]
    fn audio_entries_come_from_the_given_table() {
        let mut tables = AssetTables::v1().expect("bảng V1");
        let cue = tables
            .audio
            .cues
            .iter_mut()
            .find(|cue| cue.event == "bomb_placed")
            .expect("cue bomb_placed");
        cue.entry = "Wave/BombExplosion002".to_owned();
        let entries = audio_entries_of(&tables.audio.cues);
        assert!(
            entries.contains(&"Wave/BombExplosion002"),
            "entry ngoài bảng audio mặc định vẫn phải được đọc"
        );
        assert_eq!(
            entries.len(),
            tables.audio.cues.len(),
            "bảng V1 không có entry lặp"
        );

        // Entry lặp chỉ được đọc một lần.
        let mut cues = tables.audio.cues.clone();
        cues.push(cues[0].clone());
        let entries = audio_entries_of(&cues);
        let mut unique = entries.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(entries.len(), unique.len(), "entry lặp phải được dedup");
    }

    /// Nghiệm thu issue #123: đổi **một** mapping ⇒ `content_hash` đổi.
    ///
    /// Bảng data là nguồn chuẩn; nếu đổi mapping mà hash không đổi thì reader không còn
    /// ràng buộc được nội dung hiển thị.
    #[test]
    fn changing_one_mapping_changes_the_content_hash() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let base = build_bundle(&root, V1_MAP).expect("dựng bundle");

        // Đổi nhãn bằng chứng của một clip.
        let mut tables = AssetTables::v1().expect("bảng V1");
        let clip = tables
            .animation
            .clips
            .iter_mut()
            .find(|clip| clip.id == "c00.walk.down.static")
            .expect("clip đi của C00");
        clip.status = Status::Candidate;
        let changed = build_bundle_with(&root, V1_MAP, &tables).expect("dựng bundle");
        assert_ne!(
            base.bundle.content_hash, changed.bundle.content_hash,
            "đổi nhãn bằng chứng của clip phải đổi content_hash"
        );

        // Đổi lý do của một phần tử UI.
        let mut tables = AssetTables::v1().expect("bảng V1");
        let element = tables
            .ui
            .elements
            .iter_mut()
            .find(|element| element.id == "lobby.create")
            .expect("phần tử UI lobby.create");
        element.reason = "lý do đã đổi".to_owned();
        let changed = build_bundle_with(&root, V1_MAP, &tables).expect("dựng bundle");
        assert_ne!(
            base.bundle.content_hash, changed.bundle.content_hash,
            "đổi lý do của phần tử UI phải đổi content_hash"
        );

        // Bảng không đổi thì hash vẫn như cũ.
        let same = build_bundle(&root, V1_MAP).expect("dựng bundle lại");
        assert_eq!(base.bundle.content_hash, same.bundle.content_hash);
    }

    /// Nghiệm thu V1-031: bundle phải mang **mọi** clip của bảng animation V1, với nhãn
    /// bằng chứng đúng: clip R-077 là `confirmed`, clip R-078 là `candidate`.
    ///
    /// Thời lượng chưa có nguồn (R-079) thì mọi clip phải là `unknown` và **không** mang
    /// số — bundle không tự đặt nhịp.
    #[test]
    fn bundle_carries_every_v1_clip_with_its_evidence_label() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let built = build_bundle(&root, V1_MAP).expect("dựng bundle");
        let sources = crate::animation::v1_clip_sources().expect("bảng clip V1 hợp lệ");
        assert_eq!(built.bundle.clips.len(), sources.len());

        for source in &sources {
            let clip = built
                .bundle
                .clip(&source.id)
                .unwrap_or_else(|| panic!("thiếu clip {}", source.id));
            assert_eq!(clip.claims, source.claims, "clip {}", clip.id);
            let expected_status = match source.claims.first().map(String::as_str) {
                Some("R-077") | Some("R-084") => "confirmed",
                Some("R-078") => "candidate",
                other => panic!("claim không lạ trong bảng clip: {other:?}"),
            };
            assert_eq!(
                clip.status.as_str(),
                expected_status,
                "nhãn bằng chứng của clip {} sai",
                clip.id
            );
            assert!(!clip.frames.is_empty(), "clip {} không có frame", clip.id);
            assert_eq!(
                clip.timing.ms(),
                None,
                "clip {} không được mang thời lượng đoán mò",
                clip.id
            );
        }
    }

    /// Nghiệm thu V1-031: mọi clip phải tham chiếu ảnh có thật, rect nằm trong ảnh và
    /// pivot nằm trong canvas; reader phải nạp lại được bundle đó.
    #[test]
    fn every_clip_references_an_existing_image_and_pivot_inside_the_canvas() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let built = build_bundle(&root, V1_MAP).expect("dựng bundle");
        for clip in &built.bundle.clips {
            let sprite = built
                .bundle
                .sprites
                .iter()
                .find(|sprite| sprite.id == clip.sprite)
                .unwrap_or_else(|| panic!("clip {} trỏ sprite không có", clip.id));
            for frame in &clip.frames {
                let image = sprite
                    .images
                    .get(frame.image as usize)
                    .unwrap_or_else(|| panic!("clip {} trỏ ảnh không có", clip.id));
                assert!(
                    frame.x1 - frame.x0 <= image.width as i32
                        && frame.y1 - frame.y0 <= image.height as i32,
                    "clip {}: rect frame không nằm trong ảnh",
                    clip.id
                );
                assert!(
                    frame.canvas_width >= frame.x1 - frame.x0
                        && frame.canvas_height >= frame.y1 - frame.y0,
                    "clip {}: canvas nhỏ hơn hình nguồn",
                    clip.id
                );
                assert!(
                    frame.pivot_x >= 0
                        && frame.pivot_x <= frame.canvas_width
                        && frame.pivot_y >= 0
                        && frame.pivot_y <= frame.canvas_height,
                    "clip {}: pivot nằm ngoài canvas",
                    clip.id
                );
            }
        }

        let dir = temp_dir("clips");
        built.write(&dir).expect("ghi bundle");
        let loaded = boom_content::bundle::load(&dir).expect("nạp lại bundle");
        assert_eq!(loaded.clips.len(), built.bundle.clips.len());
        let _ = fs::remove_dir_all(&dir);
    }

    /// Entry ảnh **đã có** clip thì không được ghi trong `unknown`: `unknown` chỉ dành
    /// cho phần chưa xác định, có lý do (R-084: hiệu ứng nổ đã có clip nổ).
    #[test]
    fn art_with_a_clip_is_not_reported_as_unknown() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let built = build_bundle(&root, V1_MAP).expect("dựng bundle");
        // Hiệu ứng nổ đã có clip (R-084) nên không còn nằm trong `unknown`.
        let explosion = built
            .bundle
            .clip("explosion.main")
            .expect("bundle phải có clip nổ");
        assert_eq!(explosion.entry, "Bomb/BombExplosion001");
        assert!(!explosion.frames.is_empty(), "clip nổ phải có frame");
        assert!(
            !built
                .bundle
                .unknown
                .iter()
                .any(|unknown| unknown.kind == "animation_clip"),
            "entry ảnh đã có clip thì không được ghi trong unknown"
        );
    }

    /// Nghiệm thu V1-030 (issue #121): sau khi đối chiếu **bảng state của client** trong
    /// ODT, bundle thật không còn mục `unknown` nào, và nhãn bằng chứng của sprite nhân vật
    /// **tách hai tầng**:
    ///
    /// - `walk`/`dead`: `confirmed` — tên entry và dãi frame đọc được trong pack (R-077),
    ///   bảng ODT xác nhận thêm nhãn → entry.
    /// - `idle` (`Blink`) và `trapped` (`Faint`): `design` — bảng ODT chỉ xác nhận **nhãn →
    ///   entry**, còn mapping state V1 → clip là **quyết định của V1** (R-086); không
    ///   phải `confirmed` vì chưa có consumer nào xác nhận state core V1 dùng nhãn đó.
    /// - `BlinkSmall`: `candidate` — V1 **không chọn** họ này cho `idle` (bảng state của
    ///   C00/C01 không có state `BlinkSmall`). Không thấy trong bảng state **không** kết
    ///   luận là client không dùng: họ sprite này có trong ODT ở khối
    ///   `Character/<Id>_Display_A`.
    ///
    /// Bảng ODT không nằm trong closure (là dữ liệu lúc dựng, không phải nội dung lúc
    /// chơi) nhưng closure **đọc chính nó** để kiểm entry nhân vật mình khai.
    #[test]
    fn village00_bundle_has_no_unknown_and_carries_the_odt_evidence() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let built = build_bundle(&root, V1_MAP).expect("dựng bundle");
        assert!(
            built.bundle.unknown.is_empty(),
            "bundle còn mục unknown: {:?}",
            built.bundle.unknown
        );

        let status_of = |role: &str, entry: &str| {
            built
                .bundle
                .sprites
                .iter()
                .find(|sprite| sprite.role == role && sprite.entry == entry)
                .unwrap_or_else(|| panic!("bundle thiếu sprite {entry}"))
                .status
                .as_str()
                .to_owned()
        };
        for character in ["C00", "C01"] {
            assert_eq!(
                status_of(
                    "character_idle",
                    &format!("FxCharacter/Character/{character}-Blink_ALayerStatic")
                ),
                "design",
                "{character}: mapping `idle` → `Blink` là quyết định của V1 (R-086), \
                 bảng ODT chỉ xác nhận nhãn → entry"
            );
            assert_eq!(
                status_of(
                    "character_idle",
                    &format!("FxCharacter/Character/{character}-BlinkSmall_ALayerStatic")
                ),
                "candidate",
                "{character}: `BlinkSmall` là ứng viên, V1 không chọn"
            );
            assert_eq!(
                status_of(
                    "character_trapped",
                    &format!("FxCharacter/Character/{character}-FaintLayerStatic")
                ),
                "design",
                "{character}: mapping `trapped` → `Faint` là quyết định của V1 (R-086)"
            );
            assert_eq!(
                status_of(
                    "character_walk",
                    &format!("FxCharacter/Character/{character}-WalkDown_ALayerStatic")
                ),
                "confirmed",
                "{character}: `Walk` xác nhận bằng tên entry và dãi frame trong pack (R-077)"
            );
            assert_eq!(
                status_of(
                    "character_dead",
                    &format!("FxCharacter/Character/{character}-DieLayerStatic")
                ),
                "confirmed",
                "{character}: `Die` xác nhận bằng tên entry và dãi frame trong pack (R-077)"
            );
        }
    }
    /// Nghiệm thu V1-030 (issue #121): bốn spawn của Village00 phải khác nhau, không nằm
    /// trên ô chặn, và mỗi spawn phải có đường đi tới ít nhất một ô trống cách ≥ 2 ô
    /// (BFS trên lưới địa hình).
    #[test]
    fn village00_spawns_are_distinct_unblocked_and_have_an_escape_route() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let built = build_bundle(&root, V1_MAP).expect("dựng bundle");
        let profile = boom_content::map::MapProfile::from_toml(include_str!(
            "../../../data/maps/village00-kr.toml"
        ))
        .expect("nạp bảng phân loại ô");
        let terrain = profile
            .classify(&built.bundle.map_grid())
            .expect("phân loại ô của Village00");
        assert_eq!(terrain.spawns.len(), 4, "Village00 có bốn spawn");

        let report = terrain
            .check_spawns(2)
            .expect("bốn spawn của Village00 đều có đường thoát");
        assert_eq!(report.spawns.len(), 4);
        let mut seen: Vec<(u16, u16)> = Vec::new();
        for spawn in &report.spawns {
            assert!(
                !seen.contains(&spawn.position),
                "spawn bị lặp: {:?}",
                spawn.position
            );
            seen.push(spawn.position);
            assert_eq!(
                spawn.kind,
                boom_content::map::CellKind::Floor,
                "spawn {:?} nằm trên ô chặn",
                spawn.position
            );
            assert!(
                spawn.far_enough >= 1,
                "spawn {:?} không có ô trống cách ≥ 2 ô (đi được {} ô, xa nhất {})",
                spawn.position,
                spawn.reachable,
                spawn.max_distance
            );
        }
    }

    /// PNG trong suốt cùng kích thước — dùng để mô phỏng "thay toàn bộ atlas".
    fn transparent_png(width: u32, height: u32) -> Vec<u8> {
        let rgba = vec![0u8; (width as usize) * (height as usize) * 4];
        let mut out: Vec<u8> = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("ghi header PNG");
            writer.write_image_data(&rgba).expect("ghi pixel PNG");
        }
        out
    }

    /// Regression review PR #55 (P2 #1): trên bundle thật, thay toàn bộ atlas bằng PNG
    /// trong suốt cùng kích thước và cập nhật `sha256` của file — reader phải từ chối.
    ///
    /// Đúng probe của reviewer: trước khi sửa, `load()` trả `Ok` với **cùng**
    /// `content_hash` dù nội dung hiển thị đã khác.
    #[test]
    fn load_rejects_real_atlas_replaced_with_other_pixels() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let built = build_bundle(&root, V1_MAP).expect("dựng bundle");
        let dir = temp_dir("swap-real");
        built.write(&dir).expect("ghi bundle");

        // Thay file atlas bằng PNG trong suốt cùng kích thước trang.
        let page = &built.bundle.atlas[0];
        let transparent = transparent_png(page.width, page.height);
        std::fs::write(dir.join(ATLAS_DIR).join("page-000.png"), &transparent)
            .expect("ghi PNG thay thế");

        // Cập nhật `sha256` của file, giữ nguyên `content_hash` và mọi `pixel_sha256`.
        let mut swapped = built.bundle.clone();
        swapped.atlas[0].sha256 = sha256_hex(&transparent);
        std::fs::write(
            dir.join(METADATA_FILE),
            swapped.to_toml().expect("xuất TOML"),
        )
        .expect("ghi metadata");

        assert!(
            matches!(
                boom_content::bundle::load(&dir),
                Err(boom_content::bundle::BundleError::ContentHash { .. })
            ),
            "atlas bị thay mà content_hash cũ vẫn phải bị từ chối"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    /// Nghiệm thu V1-030: pixel trong trang atlas phải khớp SHA-256 của ảnh đã ghi.
    ///
    /// Đọc lại PNG đã ghi bằng bộ giải mã PNG, cắt đúng vùng của từng ảnh và so với
    /// `pixel_sha256` trong metadata — kiểm cả đường giải mã → đóng gói → ghi file.
    #[test]
    fn atlas_pixels_match_the_recorded_image_hashes() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let built = build_bundle(&root, V1_MAP).expect("dựng bundle");
        let mut checked = 0usize;
        for (page_index, bytes) in built.pages.iter().enumerate() {
            let page = &built.bundle.atlas[page_index];
            let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
            let mut reader = decoder.read_info().expect("đọc header PNG");
            let mut buffer = vec![
                0u8;
                reader
                    .output_buffer_size()
                    .expect("kích thước buffer PNG phải biết trước")
            ];
            let info = reader.next_frame(&mut buffer).expect("giải PNG");
            assert_eq!(info.color_type, png::ColorType::Rgba);
            assert_eq!(info.width, page.width);
            assert_eq!(info.height, page.height);
            let stride = page.width as usize * 4;
            for sprite in &built.bundle.sprites {
                for image in &sprite.images {
                    let Some(index) = image.page else {
                        continue;
                    };
                    assert_eq!(index as usize, page_index);
                    let width = image.width as usize;
                    let mut region = Vec::with_capacity(width * image.height as usize * 4);
                    for row in 0..image.height as usize {
                        let start = (image.y as usize + row) * stride + image.x as usize * 4;
                        region.extend_from_slice(&buffer[start..start + width * 4]);
                    }
                    assert_eq!(
                        sha256_hex(&region),
                        image.pixel_sha256,
                        "ảnh {} của sprite {} không khớp pixel trong atlas",
                        image.index,
                        sprite.id
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 100, "số ảnh kiểm tra quá ít: {checked}");
    }
}
