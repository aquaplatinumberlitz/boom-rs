//! Client V1 trên macroquad (ADR 0010), chế độ offline cho V1-033.
//!
//! Phạm vi task: input (mũi tên + Space, rebind, mất focus nhả phím), integer scale 1×/2× bằng
//! render target + nearest + copy shader, và chạy `boom-core` trực tiếp với tick cố định tách khỏi
//! render. Địa hình là placeholder vẽ theo `CellKind`; khi có `--bundle <dir>` thì nạp bundle
//! content đã validate, dựng địa hình Village00 từ bundle và vẽ tile/vật cản bằng atlas thật.
//! Nhân vật, bom và lửa vẽ bằng **clip animation** trong bundle: chọn clip theo state core và
//! frame theo tick mô phỏng (module [`anim`], quy ước ở
//! `docs/specification/assets/BUNDLES.md` §7).
//!
//! Tùy chọn người dùng đọc/ghi từ `options.toml` (module [`options`], ADR 0011 §7): volume,
//! mute và keybind. Keybind trong file **thay** hằng số phím mặc định; `--rebind` của dòng lệnh
//! vẫn hoạt động và được ghi đè lên trên (đổi sau, thắng trước).
//!
//! Cờ `--connect <addr>` mở chế độ mạng: module [`net`] (handshake, auth, gửi input, dự đoán và
//! reconciliation theo ADR 0005/0009). Server chặn mọi kind trước `AuthOk` (ADR 0009 Addendum A
//! §A.1) nên chế độ mạng cần `--user`/`--password` (hoặc biến môi trường `BOOM_USER`/
//! `BOOM_PASSWORD`). Không có cờ thì giữ chế độ offline.
//!
//! Nội dung chạy được nạp từ đĩa khi được trỏ: `--rules <file>` (rules profile) và
//! `--l10n <file>` (chuỗi UI V1); không trỏ thì dùng bản nhúng trong binary. Thiếu file
//! hoặc sai hash → thông điệp có **đường dẫn** và **cách sửa**, exit code khác 0, không
//! panic (V1-036).

mod anim;
mod audio;
mod input;
mod net;
mod options;
mod render;
mod scene;
mod text;
mod ui;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use boom_content::bundle::Bundle;
use boom_content::l10n::L10n;
use boom_content::map::{CellKind, MapProfile, Terrain};
use boom_content::rules::RulesProfile;
use boom_core::{
    Cell, CharacterStats, Dir, Life, Phase, Player, PlayerInput, PlayerSetup, Rules,
    SimulationEvent, SimulationState, Stat, World, mode_by_id,
};
use macroquad::material::{
    Material, MaterialParams, gl_use_default_material, gl_use_material, load_material,
};
use macroquad::miniquad::{PipelineParams, ShaderSource};
use macroquad::prelude::*;

use anim::FrameClock;
use input::{InputAction, InputAdapter, InputEvent, InputTracker};
use options::{ClientOptions, VOLUME_MAX, VOLUME_MIN};
use render::Atlas;
use scene::Scene;
use ui::ShotScreen;

const HUD_HEIGHT: f32 = 40.0;
const MAP_WIDTH: u16 = 15;
const MAP_HEIGHT: u16 = 13;
const DEFAULT_TICKS: u32 = 60;
const DEFAULT_SEED: u64 = 0xC0FFEE;

/// ID lượt của client offline.
///
/// Client V1 chạy **một** lượt mỗi phiên nên ID lượt cố định; sổ trình diễn dùng
/// `(round_id, EventId)` làm khoá chống event lặp (ADR 0005, R-089). Khi nối server
/// (P7) ID lượt lấy từ đầu bài, không hard-code.
const ROUND_ID: u32 = 0;

// Placeholder cho tới khi có bundle content (task bundle). Không phải giá trị cân bằng/gameplay:
// chỉ đủ để dựng 2 người và kiểm input/scale/focus. Thay bằng CharacterStats từ content ở P6.
const DEMO_STATS: CharacterStats = CharacterStats {
    speed: Stat::new(1, 1),
    bombs: Stat::new(1, 1),
    range: Stat::new(1, 1),
};

const SPAWNS: [(u16, u16); 4] = [(1, 1), (13, 1), (1, 11), (13, 11)];

// Copy shader tối thiểu cho bước đưa render target ra cửa sổ. Tắt blend vì target đã có RGB đúng;
// blend mặc định sẽ nhân đôi alpha và làm tối vùng bán trong suốt (xem spike ADR 0010).
const COPY_VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}"#;

const COPY_FRAGMENT: &str = r#"#version 100
varying lowp vec2 uv;
varying lowp vec4 color;

uniform sampler2D Texture;

void main() {
    gl_FragColor = color * texture2D(Texture, uv);
}"#;

const COPY_METAL: &str = r#"#include <metal_stdlib>
using namespace metal;

struct Uniforms
{
    float4x4 Model;
    float4x4 Projection;
};

struct Vertex
{
    float3 position    [[attribute(0)]];
    float2 texcoord    [[attribute(1)]];
    float4 color0      [[attribute(2)]];
};

struct RasterizerData
{
    float4 position [[position]];
    float4 color [[user(locn0)]];
    float2 uv [[user(locn1)]];
};

vertex RasterizerData vertexShader(Vertex v [[stage_in]], constant Uniforms& uniforms [[buffer(0)]])
{
    RasterizerData out;
    out.position = uniforms.Model * uniforms.Projection * float4(v.position, 1);
    out.color = v.color0 / 255.0;
    out.uv = v.texcoord;
    return out;
}

fragment float4 fragmentShader(RasterizerData in [[stage_in]], texture2d<float> tex [[texture(0)]], sampler texSmplr [[sampler(0)]])
{
    return in.color * tex.sample(texSmplr, in.uv);
}"#;

fn main() {
    let opts = match Options::parse() {
        Ok(opts) => opts,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    // Chế độ demo UI (V1-032): chạy trọn luồng màn hình V1 — đăng nhập → sảnh →
    // phòng chờ → trận → kết quả → phòng — với nguồn dữ liệu giả lập, không cần
    // server. Bản mạng nối vào sau khi #89/#99 merge (task riêng).
    if opts.ui_demo || opts.ui_shot.is_some() {
        let model = ui::UiModel::new(ui::DemoBackend::new(), ui::Texts::embedded());
        let conf = macroquad::miniquad::conf::Conf {
            window_title: "boom-rs client — UI demo".to_owned(),
            window_width: i32::try_from(ui::WIDTH).unwrap_or(800),
            window_height: i32::try_from(ui::HEIGHT).unwrap_or(600),
            window_resizable: false,
            high_dpi: false,
            sample_count: 1,
            ..Default::default()
        };
        macroquad::Window::from_config(
            conf,
            ui::run_demo(
                model,
                ui::UiOptions {
                    bundle: opts.bundle.clone(),
                    snapshot: opts.snapshot.clone(),
                    shot: opts.ui_shot,
                },
            ),
        );
        return;
    }
    // Tùy chọn người dùng: đọc `options.toml`; file hỏng/thiếu thì dùng mặc định kèm cảnh báo.
    let path = options::resolve_path(opts.options_path.as_deref());
    let mut client_options = match &path {
        Ok(path) => {
            let loaded = options::load(path);
            for warning in &loaded.warnings {
                eprintln!("cảnh báo options: {warning}");
            }
            match loaded.status {
                options::LoadStatus::Missing => {
                    eprintln!("cảnh báo options: chưa có file options, dùng mặc định");
                }
                options::LoadStatus::Defaults => {
                    eprintln!("cảnh báo options: file options không dùng được, dùng mặc định");
                }
                options::LoadStatus::Loaded => {}
            }
            loaded.options
        }
        Err(error) => {
            eprintln!(
                "cảnh báo options: không xác định được đường dẫn file ({error}); dùng mặc định"
            );
            ClientOptions::default()
        }
    };
    // Cờ dòng lệnh đè lên file: đổi sau, thắng trước.
    if let Some(volume) = opts.master_volume {
        client_options.master_volume = volume;
    }
    if let Some(volume) = opts.bgm_volume {
        client_options.bgm_volume = volume;
    }
    if let Some(volume) = opts.sfx_volume {
        client_options.sfx_volume = volume;
    }
    if let Some(mute) = opts.mute {
        client_options.mute = mute;
    }
    // Nội dung: rules profile và bộ chuỗi UI. Mặc định dùng bản nhúng trong binary;
    // `--rules`/`--l10n` trỏ sang file của gói phát hành (V1-036).
    let profile = match load_rules(&opts.rules) {
        Ok(profile) => profile,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let l10n = match load_l10n(&opts.l10n) {
        Ok(l10n) => l10n,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    eprintln!(
        "nội dung: rules {} · chuỗi {} ({} khóa)",
        profile.profile_id,
        l10n.locale(),
        l10n.len()
    );
    let mut game = match Game::new(opts.seed, client_options, opts.bundle.as_deref(), &profile) {
        Ok(game) => game,
        Err(error) => {
            eprintln!("không dựng được client: {error}");
            std::process::exit(2);
        }
    };
    // Chế độ mạng: nối server và dùng module `net`; không có `--connect` thì giữ offline.
    if let Some(addr) = opts.connect.clone() {
        let credentials = match net_credentials(
            &opts,
            std::env::var("BOOM_USER").ok(),
            std::env::var("BOOM_PASSWORD").ok(),
        ) {
            Ok(credentials) => credentials,
            Err(error) => {
                eprintln!("không lấy được thông tin đăng nhập: {error}");
                std::process::exit(2);
            }
        };
        match net::connect(&addr, &credentials) {
            Ok(client) => game.net = Some(client),
            Err(error) => {
                eprintln!("không vào được chế độ mạng: {error}");
                std::process::exit(2);
            }
        }
    }
    if opts.demo || opts.demo_death {
        game.enable_demo(opts.demo_death);
    }
    for &(action, key) in &opts.rebinds {
        game.input.rebind(action, key);
    }
    // Keybind hiệu lực = keybind trong file đã bị cờ dòng lệnh đè lên.
    game.options.keybinds = *game.input.binds();
    if opts.save_options {
        let path = match &path {
            Ok(path) => path,
            Err(error) => {
                eprintln!("không ghi được options: {error}");
                std::process::exit(2);
            }
        };
        if let Err(error) = options::save(&game.options, path) {
            eprintln!("không ghi được options: {error}");
            std::process::exit(2);
        }
        println!("đã ghi options: {}", path.display());
        return;
    }
    let conf = window_conf(&game, &opts);
    macroquad::Window::from_config(conf, run(game, opts));
}

/// Lỗi nạp file nội dung (rules/bundle/l10n).
///
/// Thông điệp **luôn** kèm đường dẫn và cách sửa: người dùng phải biết sửa ở đâu,
/// không chỉ biết "có lỗi" (V1-036: thiếu file hoặc sai hash → báo rõ đường dẫn +
/// cách sửa, exit code khác 0, không panic).
#[derive(Debug)]
struct ContentError(String);

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ContentError {}

/// Rules profile: file trên đĩa nếu có, không thì bản nhúng trong binary.
fn load_rules(path: &Option<PathBuf>) -> Result<RulesProfile, ContentError> {
    match path {
        Some(path) => load_rules_file(path),
        None => RulesProfile::from_toml(include_str!("../../../data/rules/v1-dev.toml")).map_err(
            |error| {
                ContentError(format!(
                    "bản rules nhúng trong binary không dùng được: {error}"
                ))
            },
        ),
    }
}

/// Nạp rules profile từ file; lỗi kèm đường dẫn và cách sửa.
fn load_rules_file(path: &Path) -> Result<RulesProfile, ContentError> {
    let text = std::fs::read_to_string(path).map_err(|error| {
        ContentError(format!(
            "không đọc được file rules: {} ({error})\n  \
             sửa: file phải tồn tại và đọc được; gói phát hành đặt rules ở data/rules/ \
             (ví dụ data/rules/v1-dev.toml)",
            path.display()
        ))
    })?;
    RulesProfile::from_toml(&text).map_err(|error| {
        ContentError(format!(
            "file rules không dùng được: {} ({error})\n  \
             sửa: file phải là rules profile TOML hợp lệ \
             (xem docs/specification/gameplay/RULES_V1.md)",
            path.display()
        ))
    })
}

/// Bộ chuỗi UI: file trên đĩa nếu có, không thì bản nhúng trong binary.
fn load_l10n(path: &Option<PathBuf>) -> Result<L10n, ContentError> {
    match path {
        Some(path) => load_l10n_file(path),
        None => L10n::from_toml(include_str!("../../../data/l10n/vi-v1.toml")).map_err(|error| {
            ContentError(format!(
                "bản chuỗi nhúng trong binary không dùng được: {error}"
            ))
        }),
    }
}

/// Nạp bộ chuỗi UI từ file; lỗi kèm đường dẫn và cách sửa.
fn load_l10n_file(path: &Path) -> Result<L10n, ContentError> {
    let bytes = std::fs::read(path).map_err(|error| {
        ContentError(format!(
            "không đọc được file chuỗi: {} ({error})\n  \
             sửa: file phải tồn tại; gói phát hành đặt chuỗi ở data/l10n/ \
             (ví dụ data/l10n/vi-v1.toml)",
            path.display()
        ))
    })?;
    L10n::from_bytes(&bytes).map_err(|error| {
        ContentError(format!(
            "file chuỗi không dùng được: {} ({error})\n  \
             sửa: file phải là file chuỗi UI V1 hợp lệ \
             (sinh bằng tools/l10n_export.py, \
             xem docs/specification/localization/UI_STRINGS_V1.md)",
            path.display()
        ))
    })
}

/// Lỗi nạp bundle: kèm thư mục bundle và cách dựng lại từ pack.
fn bundle_error(dir: &Path, error: &dyn fmt::Display) -> ContentError {
    ContentError(format!(
        "không nạp được bundle: {}\n  lỗi: {error}\n  \
         sửa: dựng lại bundle từ pack của bạn: \
         `boom-assets bundle --map Village00 --out <dir> --pack-dir <thư mục pack>`",
        dir.display()
    ))
}

fn window_conf(game: &Game, opts: &Options) -> macroquad::miniquad::conf::Conf {
    // Chế độ mạng: tiêu đề cửa sổ mang build id của server (fingerprint ADR 0011 §8.3).
    let title = game
        .net
        .as_ref()
        .and_then(net::NetClient::server_version)
        .map_or_else(
            || "boom-rs client — offline".to_owned(),
            |version| format!("boom-rs client — {version}"),
        );
    macroquad::miniquad::conf::Conf {
        window_title: title,
        window_width: (game.logic_size[0] * opts.scale) as i32,
        window_height: (game.logic_size[1] * opts.scale) as i32,
        window_resizable: false,
        high_dpi: false,
        sample_count: 1,
        ..Default::default()
    }
}

fn load_copy_material() -> Material {
    let shader = if cfg!(target_vendor = "apple") {
        ShaderSource::Msl {
            program: COPY_METAL,
        }
    } else {
        ShaderSource::Glsl {
            vertex: COPY_VERTEX,
            fragment: COPY_FRAGMENT,
        }
    };
    load_material(
        shader,
        MaterialParams {
            pipeline_params: PipelineParams {
                color_blend: None,
                alpha_blend: None,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("copy material")
}

struct Game {
    world: World,
    state: SimulationState,
    events: Vec<SimulationEvent>,
    input: InputTracker,
    options: ClientOptions,
    logic_size: [u32; 2],
    /// Scene dựng từ bundle; `None` = scene placeholder (không có `--bundle`).
    scene: Option<Scene>,
    /// Thư mục bundle, để nạp trang atlas khi đã có cửa sổ.
    bundle_dir: Option<PathBuf>,
    /// Bundle đã nạp: chứa clip animation để vẽ nhân vật, bom và lửa.
    bundle: Option<Bundle>,
    /// **Đồng hồ trình diễn**: nhịp phát frame (từ rules profile) + tick trình diễn.
    ///
    /// Tick trình diễn bám `state.tick` khi mô phỏng còn chạy và **tiến nốt** cùng nhịp
    /// tick khi lượt **kết thúc** (core dừng), để chuỗi chết phát trọn clip rồi mới giữ
    /// khung cuối có ảnh (R-089).
    clock: anim::PresentationClock,
    /// Sổ trình diễn: hiệu ứng nổ, chết, nhốt và mở đầu lượt, gắn với event server
    /// đã xác nhận (R-089).
    journal: anim::PresentationJournal,
    /// ID lượt hiện tại của client; tăng khi lượt mới bắt đầu.
    round_id: u32,
    /// Tick của lượt trước — phát hiện lượt mới (state được dựng lại, tick về 0).
    last_tick: u32,
    /// Vị trí nhân vật ở tick trước (để biết nhân vật đang đi hay đứng yên).
    previous_positions: BTreeMap<u8, (i32, i32)>,
    /// Kịch bản demo offline (`--demo`); `None` = người chơi điều khiển.
    demo: Option<Demo>,
    /// Quyết định cue âm thanh thuần (test được); phần phát nằm ở `audio::MacroquadAudio`.
    audio_state: audio::AudioState,
    /// Cue tích luỹ giữa các tick trong một frame; `take` sau khi phát.
    audio_plan: audio::AudioPlan,
    /// Chế độ mạng: server là authority, client chỉ dự đoán di chuyển của chính mình.
    net: Option<net::NetClient<boom_net::TcpTransport>>,
    /// Trạng thái chế độ mạng để hiện trên HUD.
    net_status: Option<NetStatus>,
    /// Thời điểm (đồng hồ render) nhận snapshot mới nhất: gốc của alpha nội suy.
    net_snapshot_at: f64,
    /// Chu kỳ gửi snapshot (ms) do server khai trong `Welcome`.
    net_snapshot_interval_ms: u64,
}

/// Thông tin chế độ mạng hiển thị trên HUD: server xác nhận tới tick nào, client đang dự đoán
/// tới tick nào, round hiện tại và số event server mới (đã dedup).
#[derive(Clone, Copy, Debug, Default)]
struct NetStatus {
    confirmed_tick: u32,
    predicted_tick: u32,
    round: u32,
    slot: u8,
    events: usize,
    /// Lượt đã kết thúc (kết quả do server quyết).
    ended: bool,
    /// Số lệnh input chưa được server ack.
    pending: u32,
}

impl Game {
    fn new(
        seed: u64,
        options: ClientOptions,
        bundle_dir: Option<&Path>,
        profile: &RulesProfile,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let rules = Rules::from_profile(profile)?;
        let mode = mode_by_id("battle_ffa").ok_or("mode battle_ffa chưa đăng ký")?;
        let tile_px = u32::try_from(rules.tile.0 / 1000)?;
        // Nhịp phát frame: thời lượng frame lấy từ profile (không hard-code trong code).
        let frame_clock = FrameClock {
            frame_ms: u32::try_from(profile.int_in(
                "presentation.frame_duration_ms",
                "ms",
                1,
                10_000,
            )?)?,
            ticks_per_second: rules.ticks_per_second,
        };
        // Đồng hồ trình diễn: nhịp frame từ profile, tick trình diễn bắt đầu từ 0.
        let clock = anim::PresentationClock::new(frame_clock, 0);

        // Scene: bundle content nếu có, không thì địa hình placeholder.
        let (terrain, scene, bundle) = match bundle_dir {
            Some(dir) => {
                // Phân loại ô nằm ở bảng chuẩn `data/maps/`, không nhân bản trong bundle.
                let map_profile =
                    MapProfile::from_toml(include_str!("../../../data/maps/village00-kr.toml"))?;
                let loaded = scene::load(dir, &map_profile, tile_px)
                    .map_err(|error| bundle_error(dir, &error))?;
                // Không bỏ qua phần chưa xác định một cách âm thầm (AGENTS §4).
                for unknown in &loaded.bundle.unknown {
                    eprintln!(
                        "cảnh báo bundle: chưa xác định {} {} — {}",
                        unknown.kind, unknown.id, unknown.reason
                    );
                }
                if loaded.scene.unknown_cells > 0 {
                    eprintln!(
                        "cảnh báo bundle: {} ô của map chưa có sprite (xem `unknown` trong bundle.toml)",
                        loaded.scene.unknown_cells
                    );
                }
                // Thiếu clip thì client vẽ ô màu thay thế — báo ngay từ đầu (AGENTS §4).
                for clip_id in scene::required_clip_ids() {
                    if !loaded.bundle.clips.iter().any(|clip| clip.id == clip_id) {
                        if clip_id == anim::EXPLOSION_CLIP {
                            // Không có clip nổ thì không có gì để phát hiệu ứng nổ.
                            eprintln!(
                                "cảnh báo bundle: không có clip {clip_id} — hiệu ứng nổ không phát"
                            );
                        } else {
                            eprintln!(
                                "cảnh báo bundle: không có clip {clip_id} — sẽ vẽ ô màu thay thế"
                            );
                        }
                    }
                }
                let terrain = scene::terrain(&loaded.bundle, &map_profile)?;
                (terrain, Some(loaded.scene), Some(loaded.bundle))
            }
            None => (placeholder_terrain(), None, None),
        };
        let world = World::new(terrain, rules, mode)?;

        let roster = [
            PlayerSetup {
                id: 0,
                stats: DEMO_STATS,
            },
            PlayerSetup {
                id: 1,
                stats: DEMO_STATS,
            },
        ];
        let state = SimulationState::new_round(&world, &roster, seed)?;
        // Sổ trình diễn của lượt: hiệu ứng mở đầu lượt là **client tự sinh** (core V1
        // chưa có event mở đầu lượt), còn nổ/chết/nhốt gắn với event server (R-089).
        let mut journal = anim::PresentationJournal::new(ROUND_ID);
        record_round_start(&mut journal, &state);
        // Kích thước logic: bản đồ theo bundle (hoặc theo địa hình placeholder) + HUD.
        let map_size = scene.as_ref().map_or_else(
            || {
                [
                    u32::from(world.terrain().width) * tile_px,
                    u32::from(world.terrain().height) * tile_px,
                ]
            },
            Scene::map_size,
        );
        let logic_size = [map_size[0], map_size[1] + HUD_HEIGHT as u32];
        let previous_positions = state
            .players
            .iter()
            .map(|player| (player.id, (player.x.0, player.y.0)))
            .collect();
        Ok(Self {
            world,
            state,
            events: Vec::new(),
            input: InputTracker::new(options.keybinds),
            options,
            logic_size,
            scene,
            bundle_dir: bundle_dir.map(Path::to_path_buf),
            bundle,
            clock,
            journal,
            round_id: ROUND_ID,
            last_tick: 0,
            previous_positions,
            demo: None,
            audio_state: audio::AudioState::new(0),
            audio_plan: audio::AudioPlan::default(),
            net: None,
            net_status: None,
            net_snapshot_at: 0.0,
            net_snapshot_interval_ms: 1,
        })
    }

    /// Bật kịch bản demo offline (chỉ đẩy input, không sửa state).
    ///
    /// `death` = kịch bản **chết**: người chơi đặt bom rồi đứng yên nên bị lửa bắt và
    /// chết — dùng để chụp ảnh chuỗi chết (R-089). Với roster mặc định **hai** người,
    /// lượt **kết thúc** ngay lúc chết; chuỗi chết vẫn phát trọn clip vì đồng hồ
    /// trình diễn tiến nốt sau khi mô phỏng dừng (xem [`Game::tick`]).
    fn enable_demo(&mut self, death: bool) {
        self.demo = Some(Demo::new(&self.world, &self.state, death));
    }

    /// Chuỗi **chết** đang phát ở frame `frame` (dùng để chụp ảnh local).
    ///
    /// Đồng hồ **trình diễn** (không phải `state.tick`): sau khi lượt kết thúc mô phỏng
    /// dừng nhưng chuỗi chết vẫn phát nốt (R-089).
    fn death_ready(&self, frame: usize) -> bool {
        let frames = self.frame_counts();
        self.state.players.iter().any(|player| {
            self.journal
                .active_for(anim::Subject::Player(player.id), &self.clock, &frames)
                .is_some_and(|active| {
                    active.kind == anim::PresentationKind::Death && active.frame == frame
                })
        })
    }

    /// Chuỗi chết đã **hết**: nhân vật chết giữ khung cuối **có ảnh** của clip chết.
    fn death_hold_ready(&self) -> bool {
        let frames = self.frame_counts();
        self.state.players.iter().any(|player| {
            player.life == Life::Dead
                && self
                    .journal
                    .active_for(anim::Subject::Player(player.id), &self.clock, &frames)
                    .is_none()
        })
    }

    /// Một tick của client: lỗi core (chế độ offline) hoặc lỗi mạng (chế độ mạng).
    fn tick(&mut self) -> Result<(), TickError> {
        if let Some(net) = self.net.as_mut() {
            // Chế độ mạng: đọc message đang chờ (không chặn vòng render), dự đoán một bước
            // của chính mình rồi gửi cửa sổ lệnh chưa ack. State hiển thị là state dự đoán.
            net.pump(Duration::ZERO)?;
            let input = PlayerInput {
                dir: self.input.direction(),
                place_bomb: self.input.take_bomb(),
            };
            // Lượt đã kết thúc thì `predict` không sinh lệnh và `resend` không gửi: kết quả do
            // server quyết, client giữ state confirmed và vẫn nhận message.
            net.predict(input)?;
            net.resend()?;
            let mut status = self.net_status.unwrap_or_default();
            let confirmed_tick = net.confirmed_state().map_or(0, |state| state.tick);
            if confirmed_tick != status.confirmed_tick {
                // Snapshot mới: đặt lại gốc thời gian của alpha nội suy.
                self.net_snapshot_at = get_time();
            }
            if let Some(state) = net.state() {
                // Vị trí ở tick trước để animation biết nhân vật đang đi hay đứng yên.
                self.previous_positions = state
                    .players
                    .iter()
                    .map(|player| (player.id, (player.x.0, player.y.0)))
                    .collect();
                self.state = state.clone();
                status.predicted_tick = state.tick;
                // Đồng hồ trình diễn bám tick của state server (đã xác nhận/dự đoán).
                self.clock.tick = state.tick;
            }
            // Người chơi khác được **nội suy** giữa hai snapshot gần nhất (ADR 0005): alpha chạy
            // từ snapshot cũ tới snapshot mới theo thời gian đã trôi, clamp ở snapshot cuối
            // (không ngoại suy). Chính mình thì dùng state dự đoán (movement-only).
            let own_slot = net.player_slot();
            let alpha = net_alpha(self.net_snapshot_at, self.net_snapshot_interval_ms);
            for player in &mut self.state.players {
                if Some(player.id) == own_slot {
                    continue;
                }
                if let Some((x, y)) = net.interpolated_position(player.id, alpha) {
                    player.x = boom_core::Mpx(x);
                    player.y = boom_core::Mpx(y);
                }
            }
            status.confirmed_tick = confirmed_tick;
            status.round = net.round_id().unwrap_or(0);
            status.slot = net.player_slot().unwrap_or(0);
            status.ended = matches!(net.phase(), Some(Phase::Ended(_)));
            status.pending = u32::try_from(net.pending_len()).unwrap_or(u32::MAX);
            // Event server mới (đã dedup theo `(round_id, EventId)`): presentation journal sẽ
            // dùng, chưa gắn âm thanh/hiệu ứng ở task này.
            status.events += net.take_events().len();
            self.net_status = Some(status);
            self.net_snapshot_interval_ms = net.snapshot_interval_ms().max(1);
            // Dọn hiệu ứng đã hết khoảng clip theo đồng hồ trình diễn.
            self.prune_presentation();
            return Ok(());
        }
        if self.state.phase != Phase::Playing {
            // Lượt đã **kết thúc**: mô phỏng dừng (core trả `RoundOver` nếu bước thêm)
            // nhưng **hiệu ứng trình diễn** phải chạy nốt — chuỗi chết phát trọn clip rồi
            // mới giữ khung cuối có ảnh (R-089). Đồng hồ trình diễn tiến **cùng nhịp
            // tick** nên chuỗi chết không bị kẹt ở frame đầu.
            self.advance_presentation();
            return Ok(());
        }
        // Lượt mới (state được dựng lại, tick về 0) ⇒ ID lượt tăng, sổ trình diễn dọn
        // mục cũ và quên event của lượt trước: khoá event là `(round_id, EventId)`
        // nên event lượt cũ không lặp lại được ở lượt mới (R-089).
        if self.state.tick < self.last_tick {
            self.round_id += 1;
            self.journal.set_round(self.round_id);
            record_round_start(&mut self.journal, &self.state);
        }
        // Lưu vị trí trước bước: state core không có cờ "đang đi" nên client tự so
        // vị trí giữa hai tick (module `anim`).
        self.previous_positions = self
            .state
            .players
            .iter()
            .map(|player| (player.id, (player.x.0, player.y.0)))
            .collect();
        let input = match &mut self.demo {
            Some(demo) => {
                let ids: Vec<u8> = self.state.players.iter().map(|player| player.id).collect();
                ids.iter()
                    .map(|id| demo.input_for(*id, &self.state, &self.world))
                    .collect::<Vec<_>>()
            }
            None => vec![
                PlayerInput {
                    dir: self.input.direction(),
                    place_bomb: self.input.take_bomb(),
                },
                PlayerInput::default(),
            ],
        };
        self.state.step(&self.world, &input, &mut self.events)?;
        if let Some(bundle) = &self.bundle {
            let plan = self.audio_state.plan(bundle, 0, &self.events);
            self.audio_plan.cues.extend(plan.cues);
            self.audio_plan.start_bgm |= plan.start_bgm;
        }
        // Presentation: đọc event để phát hiệu ứng nổ/chết/nhốt (R-088, R-089).
        self.absorb_events();
        // Đồng hồ trình diễn bám `state.tick` khi mô phỏng còn chạy.
        self.clock.tick = self.state.tick;
        // Bỏ hiệu ứng đã hết khoảng clip.
        self.prune_presentation();
        // Event đã đọc xong: xóa để không phình bộ nhớ trong chế độ offline.
        self.events.clear();
        self.last_tick = self.state.tick;
        Ok(())
    }

    /// Tiến **đồng hồ trình diễn** khi mô phỏng đã dừng (lượt kết thúc) và dọn hiệu ứng
    /// hết khoảng clip.
    ///
    /// Nhịp tiến **cùng** nhịp tick mô phỏng (`sim.ticks_per_second`), nên công thức chọn
    /// frame của R-085 vẫn đúng sau khi lượt kết thúc.
    fn advance_presentation(&mut self) {
        self.clock.tick = self.clock.tick.saturating_add(1);
        self.prune_presentation();
    }

    /// Dọn mục sổ đã hết khoảng clip (khoá event **vẫn giữ** — R-089).
    fn prune_presentation(&mut self) {
        let frames = self.frame_counts();
        self.journal.prune(&self.clock, &frames);
    }

    /// Đọc event của tick vừa chạy: nổ → hiệu ứng nổ, chết/nhốt → chuỗi chết/nhốt.
    ///
    /// Sổ trình diễn tự loại event **lặp** (cùng `(round_id, EventId)`) nên reconcile
    /// không phát lại hiệu ứng (ADR 0005, R-089).
    fn absorb_events(&mut self) {
        for event in &self.events {
            self.journal.record_event(event);
        }
    }

    /// Số frame của mỗi clip trong bundle (bảng tra của sổ trình diễn).
    fn frame_counts(&self) -> anim::FrameCounts {
        self.bundle
            .as_ref()
            .map_or_else(anim::FrameCounts::default, scene::frame_counts)
    }

    /// Scene đã đủ để chụp ảnh demo: nhân vật sống, bom đang chờ và lửa.
    fn demo_ready(&self) -> bool {
        self.demo
            .as_ref()
            .is_some_and(|demo| demo.ready(&self.state, &self.world))
    }

    fn tile_px(&self) -> f32 {
        self.world.rules().tile.0 as f32 / 1000.0
    }

    /// Lấy cue tích luỹ để phần phát xử lý; gọi sau mỗi frame.
    fn take_audio_plan(&mut self) -> audio::AudioPlan {
        std::mem::take(&mut self.audio_plan)
    }

    /// Toạ độ vẽ của người chơi: chính mình dùng state dự đoán, người khác dùng **nội suy**
    /// giữa hai snapshot gần nhất (ADR 0005).
    fn draw_position(&self, player: &Player) -> (f32, f32) {
        let own_slot = self.net.as_ref().and_then(net::NetClient::player_slot);
        if Some(player.id) == own_slot {
            return (player.x.0 as f32 / 1000.0, player.y.0 as f32 / 1000.0);
        }
        let alpha = net_alpha(self.net_snapshot_at, self.net_snapshot_interval_ms);
        match self
            .net
            .as_ref()
            .and_then(|net| net.interpolated_position(player.id, alpha))
        {
            Some((x, y)) => (x as f32 / 1000.0, y as f32 / 1000.0),
            None => (player.x.0 as f32 / 1000.0, player.y.0 as f32 / 1000.0),
        }
    }
}

/// Alpha nội suy hiện tại (đơn vị 1/65536): `0` = snapshot cũ, `ALPHA_ONE` = snapshot mới.
///
/// Thời gian đã trôi kể từ snapshot mới nhất (`snapshot_at`, đồng hồ render) chia cho chu kỳ
/// snapshot do server khai (`interval_ms`); kết quả clamp ở `ALPHA_ONE` để **không ngoại suy**
/// (ADR 0005).
fn net_alpha(snapshot_at: f64, interval_ms: u64) -> u32 {
    let elapsed_ms = ((get_time() - snapshot_at) * 1000.0).max(0.0) as u64;
    let interval = interval_ms.max(1);
    let scaled = elapsed_ms.saturating_mul(u64::from(net::ALPHA_ONE)) / interval;
    u32::try_from(scaled)
        .unwrap_or(net::ALPHA_ONE)
        .min(net::ALPHA_ONE)
}

/// Lỗi của một tick client: lỗi core (chế độ offline) hoặc lỗi mạng (chế độ mạng).
#[derive(Debug)]
enum TickError {
    /// Lỗi mô phỏng của `boom-core`.
    Core(boom_core::CoreError),
    /// Lỗi mạng của module `net` (handshake, codec, transport).
    Net(net::NetError),
}

impl fmt::Display for TickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => write!(f, "{error}"),
            Self::Net(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for TickError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Core(error) => Some(error),
            Self::Net(error) => Some(error),
        }
    }
}

impl From<boom_core::CoreError> for TickError {
    fn from(value: boom_core::CoreError) -> Self {
        Self::Core(value)
    }
}

impl From<net::NetError> for TickError {
    fn from(value: net::NetError) -> Self {
        Self::Net(value)
    }
}

/// Ghi hiệu ứng **mở đầu lượt** cho từng nhân vật vào sổ trình diễn.
///
/// Mở đầu lượt là **client tự sinh**: core V1 chưa có event mở đầu lượt, nên mục này
/// **không** có khoá event (ADR 0005, R-089). Khi event server xác nhận cùng đối
/// tượng tới thì mục dự đoán bị thay.
fn record_round_start(journal: &mut anim::PresentationJournal, state: &SimulationState) {
    for player in &state.players {
        journal.record_local(
            anim::PresentationKind::Start,
            anim::Subject::Player(player.id),
            state.tick,
        );
    }
}

/// Kịch bản demo offline: mỗi người chơi đặt một bom rồi đi ra xa.
///
/// Kịch bản **chỉ đẩy input** vào core như người chơi, không sửa state: mọi thứ trong
/// ảnh đều là kết quả mô phỏng thật. Dùng để chụp ảnh có nhân vật + bom + lửa.
///
/// V1 giới hạn mỗi người chơi có **một** bom cùng lúc, nên cần **hai** người và hai
/// bom đặt lệch nhau: bom của người thứ nhất nổ tạo lửa, bom của người thứ hai còn
/// chờ nên ảnh có cả bom lẫn lửa. Người chơi đi ra xa ít nhất hai ô nên không bị lửa
/// của bom mình chạm tới.
///
/// Kịch bản **chết** (`death`): người chơi đặt bom rồi **đứng yên** nên bị lửa bắt và
/// chết — dùng để chụp ảnh chuỗi chết (R-089).
struct Demo {
    /// Hướng đi của từng người chơi (chọn theo địa hình: có ô trống phía trước).
    dirs: BTreeMap<u8, Dir>,
    /// Ô đặt bom của từng người chơi.
    bomb_cells: BTreeMap<u8, Cell>,
    /// Ô của từng người chơi ở lần gọi trước (để biết vừa bước sang ô mới).
    last_cells: BTreeMap<u8, Cell>,
    /// Người chơi đặt bom ngay (bom của họ nổ sớm, tạo lửa).
    place_now: BTreeSet<u8>,
    /// Kịch bản **chết**: đặt bom rồi đứng yên để bị lửa bắt.
    death: bool,
    /// Người chơi **nạn nhân** của kịch bản chết (đặt bom rồi đứng yên).
    victim: Option<u8>,
}

impl Demo {
    /// Chọn hướng đi cho từng người chơi.
    ///
    /// Người chơi đầu tiên đặt bom ngay; người chơi sau đặt bom khi vừa bước sang ô
    /// mới, nên bom của họ nổ muộn hơn và còn chờ được khi ảnh được chụp.
    ///
    /// Kịch bản `death`: chỉ **một** người chơi (người đầu) đặt bom rồi đứng yên nên
    /// bị lửa bắt và chết; người khác đứng yên xa nên lượt còn tiếp và chuỗi chết phát
    /// trọn clip — dùng để chụp ảnh chuỗi chết (R-089).
    fn new(world: &World, state: &SimulationState, death: bool) -> Self {
        let mut dirs = BTreeMap::new();
        let mut place_now = BTreeSet::new();
        let mut victim = None;
        for (index, player) in state.players.iter().enumerate() {
            let cell = world.cell_at(player.x, player.y);
            dirs.insert(
                player.id,
                if death {
                    Dir::None
                } else {
                    choose_demo_direction(world, cell)
                },
            );
            if death {
                if index == 0 {
                    place_now.insert(player.id);
                    victim = Some(player.id);
                }
            } else if index == 0 {
                place_now.insert(player.id);
            }
        }
        Self {
            dirs,
            bomb_cells: BTreeMap::new(),
            last_cells: BTreeMap::new(),
            place_now,
            death,
            victim,
        }
    }

    /// Input của demo cho người chơi `id`.
    ///
    /// Bom đặt một lần: người đặt ngay thì đặt ở tick đầu, người còn lại đặt khi vừa
    /// bước sang ô mới. Sau đó người chơi chỉ đi theo hướng đã chọn.
    ///
    /// Kịch bản `death`: chỉ nạn nhân đặt bom một lần rồi **đứng yên** để bị lửa bắt
    /// và chết; người khác đứng yên.
    fn input_for(&mut self, id: u8, state: &SimulationState, world: &World) -> PlayerInput {
        let Some(player) = state.player(id) else {
            return PlayerInput::default();
        };
        let cell = world.cell_at(player.x, player.y);
        if self.death {
            let place_bomb = self.victim == Some(id) && !self.bomb_cells.contains_key(&id);
            if place_bomb {
                self.bomb_cells.insert(id, cell);
            }
            return PlayerInput {
                dir: Dir::None,
                place_bomb,
            };
        }
        let dir = self.dirs.get(&id).copied().unwrap_or(Dir::Right);
        let moved = self
            .last_cells
            .insert(id, cell)
            .is_some_and(|last| last != cell);
        let place_bomb = if self.bomb_cells.contains_key(&id) {
            false
        } else if self.place_now.contains(&id) {
            true
        } else {
            moved
        };
        if place_bomb {
            self.bomb_cells.insert(id, cell);
        }
        PlayerInput { dir, place_bomb }
    }

    /// Scene đã đủ để chụp: nhân vật sống, bom đang chờ và lửa; không nhân vật nào đứng
    /// chồng lên ô bom (để ảnh không bị che).
    fn ready(&self, state: &SimulationState, world: &World) -> bool {
        if state.flames.is_empty() || state.bombs.is_empty() {
            return false;
        }
        state.players.iter().all(|player| {
            player.life == Life::Alive
                && !state
                    .bombs
                    .iter()
                    .any(|bomb| bomb.cell == world.cell_at(player.x, player.y))
        })
    }
}

/// Hướng demo: thứ tự cố định để kịch bản tái lập được.
const DEMO_DIRECTIONS: [Dir; 4] = [Dir::Right, Dir::Down, Dir::Left, Dir::Up];

/// Chọn hướng đi cho demo: ưu tiên hướng có ít nhất hai ô nền phía trước.
fn choose_demo_direction(world: &World, cell: Cell) -> Dir {
    for step in [2, 1] {
        for dir in DEMO_DIRECTIONS {
            if (1..=step).all(|offset| demo_cell_free(world, cell, dir, offset)) {
                return dir;
            }
        }
    }
    Dir::Right
}

/// Ô `step` ô về hướng `dir` từ `cell` còn là nền đi được không.
fn demo_cell_free(world: &World, cell: Cell, dir: Dir, step: i32) -> bool {
    let (dx, dy) = dir.delta();
    let target = Cell::new(cell.x + dx * step, cell.y + dy * step);
    world.terrain().kind(target.x, target.y) == Some(CellKind::Floor)
}

async fn run(mut game: Game, opts: Options) {
    let target = render_target(game.logic_size[0], game.logic_size[1]);
    target.texture.set_filter(FilterMode::Nearest);
    let copy_material = load_copy_material();
    let mut scale = opts.scale;

    // Trang atlas chỉ nạp được khi đã có cửa sổ (texture cần GL context).
    let atlas = match (game.bundle_dir.as_deref(), game.scene.as_ref()) {
        (Some(dir), Some(scene)) => match Atlas::load(dir, &scene.pages) {
            Ok(atlas) => Some(atlas),
            Err(error) => {
                eprintln!("không nạp được trang atlas: {error}");
                std::process::exit(2);
            }
        },
        _ => None,
    };

    let mut audio = match (game.bundle_dir.as_deref(), game.bundle.as_ref()) {
        (Some(dir), Some(bundle)) if !bundle.audio.is_empty() => {
            Some(audio::MacroquadAudio::load(dir, bundle).await)
        }
        _ => None,
    };

    if let Some(path) = &opts.snapshot {
        if opts.demo_death {
            // Chế độ demo chết: chạy tới khi chuỗi chết đang phát ở frame cần chụp
            // (hoặc đã hết, nếu `--death-hold`), hoặc hết lượt.
            //
            // Vòng lặp chạy theo **đồng hồ trình diễn**: lượt hai người **kết thúc** ngay
            // lúc chết nên mô phỏng dừng, nhưng đồng hồ trình diễn vẫn tiến để chuỗi
            // chết phát trọn clip (R-089).
            let limit = game.world.rules().round_ticks;
            while !(if opts.death_hold {
                game.death_hold_ready()
            } else {
                game.death_ready(opts.death_frame)
            }) && game.clock.tick < limit
            {
                if let Err(error) = game.tick() {
                    eprintln!("demo chết: không chạy được tick: {error}");
                    std::process::exit(2);
                }
            }
            let ready = if opts.death_hold {
                game.death_hold_ready()
            } else {
                game.death_ready(opts.death_frame)
            };
            if !ready {
                eprintln!(
                    "demo chết: hết lượt mà chuỗi chết chưa tới trạng thái cần chụp (frame {})",
                    opts.death_frame
                );
            }
        } else if opts.demo {
            // Chế độ demo: chạy tới khi scene đủ nhân vật + bom + lửa (hoặc hết lượt).
            let limit = game.world.rules().round_ticks;
            while !game.demo_ready()
                && game.state.tick < limit
                && game.state.phase == Phase::Playing
            {
                if let Err(error) = game.tick() {
                    eprintln!("demo: không chạy được tick: {error}");
                    std::process::exit(2);
                }
            }
            if !game.demo_ready() {
                eprintln!("demo: hết lượt mà scene vẫn chưa đủ (nhân vật + bom + lửa)");
            }
        } else {
            for _ in 0..opts.ticks {
                if let Err(error) = game.tick() {
                    eprintln!("snapshot: không chạy được tick: {error}");
                    std::process::exit(2);
                }
            }
        }
        let _ = path;
    }

    let step = 1.0 / f64::from(game.world.rules().ticks_per_second);
    let mut accumulator = 0.0;
    let mut last = get_time();
    let mut frame = 0u32;
    let mut input_adapter = InputAdapter::default();
    let input_subscriber = macroquad::input::utils::register_input_subscriber();

    loop {
        // Lấy event theo đúng thứ tự macroquad đã nhận từ backend (key-down/up, WindowMinimized).
        macroquad::input::utils::repeat_all_miniquad_input(&mut input_adapter, input_subscriber);
        input_adapter.drain_into(&mut game.input);
        let now = get_time();
        let delta = if opts.snapshot.is_some() {
            0.0
        } else {
            (now - last).min(0.25)
        };
        last = now;

        if opts.snapshot.is_none() {
            if is_key_pressed(KeyCode::Escape) {
                macroquad::miniquad::window::quit();
            }
            if is_key_pressed(KeyCode::S) {
                scale = if scale == 1 { 2 } else { 1 };
                macroquad::miniquad::window::set_window_size(
                    game.logic_size[0] * scale,
                    game.logic_size[1] * scale,
                );
            }
            // Phím debug để kiểm nhánh focus-loss ngay cả khi cửa sổ không mất focus thật.
            if is_key_pressed(KeyCode::F2) {
                game.input.apply(InputEvent::FocusLost);
            }

            accumulator += delta;
            let mut steps = 0;
            while accumulator >= step && steps < 8 {
                if let Err(error) = game.tick() {
                    eprintln!("dừng mô phỏng: {error}");
                    std::process::exit(2);
                }
                accumulator -= step;
                steps += 1;
            }
        }

        if let Some(audio) = &mut audio {
            let plan = game.take_audio_plan();
            audio.apply(&plan, &game.options);
        }
        draw_scene(&game, atlas.as_ref(), &target, &copy_material, scale);
        frame += 1;

        if let Some(path) = &opts.snapshot
            && frame >= 3
        {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            get_screen_data().export_png(&path.to_string_lossy());
            println!(
                "snapshot {} ({}x{}, scale {scale}) tick {}",
                path.display(),
                game.logic_size[0] * scale,
                game.logic_size[1] * scale,
                game.clock.tick
            );
            std::process::exit(0);
        }
        next_frame().await;
    }
}

fn draw_scene(
    game: &Game,
    atlas: Option<&Atlas>,
    target: &RenderTarget,
    copy_material: &Material,
    scale: u32,
) {
    let mut camera = Camera2D::from_display_rect(Rect::new(
        0.0,
        0.0,
        game.logic_size[0] as f32,
        game.logic_size[1] as f32,
    ));
    camera.render_target = Some(target.clone());
    set_camera(&camera);
    clear_background(Color::from_rgba(8, 10, 14, 255));

    match (game.scene.as_ref(), atlas) {
        (Some(scene), Some(atlas)) => draw_map_from_bundle(game, scene, atlas),
        _ => draw_placeholder_map(game),
    }

    // Bóng bom chờ xác nhận: dữ liệu presentation, tách khỏi state dự đoán (ADR 0005). Bom thật
    // hiện theo event/snapshot của server.
    if let Some(net) = game.net.as_ref() {
        for shadow in net.bomb_shadows() {
            draw_rectangle(
                shadow.x as f32 / 1000.0 - 10.0,
                HUD_HEIGHT + shadow.y as f32 / 1000.0 - 10.0,
                20.0,
                20.0,
                Color::from_rgba(245, 245, 245, 140),
            );
        }
    }

    draw_rectangle(
        0.0,
        0.0,
        game.logic_size[0] as f32,
        HUD_HEIGHT,
        Color::new(0.03, 0.04, 0.06, 1.0),
    );
    // Chế độ mạng: tick server đã xác nhận, tick client đang dự đoán, round và số event mới.
    let net_text = match game.net_status {
        Some(status) => format!(
            "  net: round {} slot {}  server {} dự đoán {}  chờ ack {}  event {}{}{}",
            status.round,
            status.slot,
            status.confirmed_tick,
            status.predicted_tick,
            status.pending,
            status.events,
            if status.ended { "  kết quả" } else { "" },
            // Tên hiển thị của tài khoản đã `AuthOk` (ADR 0011 §4).
            game.net
                .as_ref()
                .and_then(net::NetClient::account)
                .map_or_else(String::new, |account| format!("  {}", account.display_name)),
        ),
        None => String::new(),
    };
    draw_text(
        format!(
            "boom-client {}  tick {}  {} Hz  {}x  vol {}/{}/{}{}  {}={:?}  {}={:?}  held {}  F2 focus{net_text}",
            game.scene
                .as_ref()
                .map_or_else(|| "placeholder".to_owned(), |scene| scene.map.clone()),
            game.clock.tick,
            game.world.rules().ticks_per_second,
            scale,
            game.options.effective_master(),
            game.options.effective_bgm(),
            game.options.effective_sfx(),
            if game.options.mute { " mute" } else { "" },
            InputAction::Up.label(),
            game.input.binds().up,
            InputAction::Bomb.label(),
            game.input.binds().bomb,
            game.input.held().len()
        ),
        8.0,
        25.0,
        16.0,
        Color::new(0.9, 0.95, 1.0, 1.0),
    );

    set_default_camera();
    clear_background(Color::new(0.01, 0.01, 0.02, 1.0));
    gl_use_material(copy_material);
    draw_texture_ex(
        &target.texture,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(
                (game.logic_size[0] * scale) as f32,
                (game.logic_size[1] * scale) as f32,
            )),
            flip_y: true,
            ..Default::default()
        },
    );
    gl_use_default_material();
}

/// Scene placeholder: địa hình vẽ theo `CellKind`, nhân vật là hình chữ nhật.
fn draw_placeholder_map(game: &Game) {
    let tile = game.tile_px();
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let kind = game.state.field.kind(Cell::new(i32::from(x), i32::from(y)));
            let color = match kind {
                Some(CellKind::Floor) => Color::from_rgba(24, 42, 38, 255),
                Some(CellKind::Solid) => Color::from_rgba(92, 100, 110, 255),
                Some(CellKind::Breakable) => Color::from_rgba(126, 78, 44, 255),
                Some(CellKind::Unknown) | None => Color::from_rgba(180, 30, 150, 255),
            };
            draw_rectangle(
                f32::from(x) * tile,
                HUD_HEIGHT + f32::from(y) * tile,
                tile,
                tile,
                color,
            );
        }
    }

    for player in &game.state.players {
        // Chính mình dùng state dự đoán; người khác dùng nội suy giữa hai snapshot (ADR 0005).
        let (x, y) = game.draw_position(player);
        draw_rectangle(
            x - 12.0,
            HUD_HEIGHT + y - 12.0,
            24.0,
            24.0,
            player_color(player),
        );
    }
}

/// Scene từ bundle: nền vẽ trước, vật cản/nhân vật/bom/lửa theo z-order hàng.
///
/// Nhân vật, bom và lửa vẽ bằng clip animation trong bundle; clip thiếu thì vẽ ô màu
/// thay thế (không panic, không bỏ qua âm thầm — AGENTS §4).
fn draw_map_from_bundle(game: &Game, scene: &Scene, atlas: &Atlas) {
    let origin = (0.0, HUD_HEIGHT);
    let Some(bundle) = game.bundle.as_ref() else {
        return;
    };
    let plan = scene::presentation_plan(
        bundle,
        scene,
        &game.state,
        &game.world,
        &game.clock,
        &game.previous_positions,
        &game.journal,
    );
    for item in &plan.ground {
        atlas.draw(item, origin);
    }
    for item in &plan.items {
        match item {
            scene::PlanItem::Object(item) | scene::PlanItem::Sprite { item, .. } => {
                atlas.draw(item, origin)
            }
            scene::PlanItem::Fallback { rect, .. } => draw_rectangle(
                origin.0 + rect.x as f32,
                origin.1 + rect.y as f32,
                rect.width as f32,
                rect.height as f32,
                Color::from_rgba(180, 30, 150, 255),
            ),
        }
    }
}

/// Màu placeholder của nhân vật (chưa có sprite nhân vật trong bundle).
fn player_color(player: &Player) -> Color {
    match player.life {
        Life::Alive if player.id == 0 => Color::from_rgba(245, 200, 80, 255),
        Life::Alive => Color::from_rgba(80, 190, 245, 255),
        Life::Trapped { .. } => Color::from_rgba(245, 130, 40, 255),
        Life::Dead => Color::from_rgba(80, 80, 80, 255),
    }
}

fn placeholder_terrain() -> Terrain {
    let mut kinds = Vec::with_capacity(usize::from(MAP_WIDTH) * usize::from(MAP_HEIGHT));
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let border = x == 0 || y == 0 || x + 1 == MAP_WIDTH || y + 1 == MAP_HEIGHT;
            let pillar = x % 2 == 0 && y % 2 == 0;
            let kind = if border || pillar {
                CellKind::Solid
            } else if (u32::from(x) * 7 + u32::from(y) * 3) % 11 == 0 {
                CellKind::Breakable
            } else {
                CellKind::Floor
            };
            kinds.push(kind);
        }
    }
    // Spawn phải là nền; đặt sau pattern để pattern không chặn spawn.
    for &(x, y) in &SPAWNS {
        let index = usize::from(y) * usize::from(MAP_WIDTH) + usize::from(x);
        kinds[index] = CellKind::Floor;
    }
    Terrain {
        width: MAP_WIDTH,
        height: MAP_HEIGHT,
        kinds,
        spawns: SPAWNS.to_vec(),
    }
}

/// Tham số dòng lệnh (khác với [`ClientOptions`] — tùy chọn người dùng trong `options.toml`).
#[derive(Clone, Debug)]
struct Options {
    scale: u32,
    snapshot: Option<PathBuf>,
    ticks: u32,
    seed: u64,
    rebinds: Vec<(InputAction, KeyCode)>,
    /// Đường dẫn file options (đè lên quy ước OS và biến môi trường).
    options_path: Option<PathBuf>,
    /// Ghi lại tùy chọn hiện tại vào file rồi thoát.
    save_options: bool,
    master_volume: Option<u8>,
    bgm_volume: Option<u8>,
    sfx_volume: Option<u8>,
    mute: Option<bool>,
    /// Thư mục bundle content (`boom-assets bundle`); không có thì scene placeholder.
    bundle: Option<PathBuf>,
    /// File rules profile (TOML) thay cho bản nhúng trong binary.
    rules: Option<PathBuf>,
    /// File chuỗi UI V1 (TOML) thay cho bản nhúng trong binary.
    l10n: Option<PathBuf>,
    /// Chạy kịch bản demo offline (đặt bom, đi ra xa, đặt bom thứ hai) để chụp ảnh.
    demo: bool,
    /// Kịch bản demo **chết** (đặt bom rồi đứng yên) để chụp ảnh chuỗi chết.
    demo_death: bool,
    /// Frame của chuỗi chết cần chụp (dùng cùng `--demo-death`).
    death_frame: usize,
    /// Chụp trạng thái **sau** chuỗi chết (nhân vật giữ khung cuối có ảnh).
    death_hold: bool,
    /// Địa chỉ server để nối (chế độ mạng). Không có thì client chạy offline.
    connect: Option<String>,
    /// Tên đăng nhập cho chế độ mạng (fallback: biến môi trường `BOOM_USER`).
    user: Option<String>,
    /// Mật khẩu cho chế độ mạng (fallback: biến môi trường `BOOM_PASSWORD`).
    password: Option<String>,
    /// Đăng ký tài khoản mới thay vì đăng nhập tài khoản có sẵn.
    register: bool,
    /// Chạy chế độ demo UI V1 (đăng nhập → sảnh → phòng → trận → kết quả), không cần server.
    ui_demo: bool,
    /// Nhảy thẳng tới một màn UI rồi chụp ảnh (dùng cho QA ảnh local).
    ui_shot: Option<ShotScreen>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scale: 2,
            snapshot: None,
            ticks: DEFAULT_TICKS,
            seed: DEFAULT_SEED,
            rebinds: Vec::new(),
            options_path: None,
            save_options: false,
            master_volume: None,
            bgm_volume: None,
            sfx_volume: None,
            mute: None,
            bundle: None,
            rules: None,
            l10n: None,
            demo: false,
            demo_death: false,
            death_frame: 0,
            death_hold: false,
            connect: None,
            user: None,
            password: None,
            register: false,
            ui_demo: false,
            ui_shot: None,
        }
    }
}

impl Options {
    fn parse() -> Result<Self, String> {
        let mut opts = Self::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scale" => {
                    opts.scale = args
                        .next()
                        .ok_or("--scale cần 1 hoặc 2")?
                        .parse()
                        .map_err(|_| "--scale cần 1 hoặc 2")?;
                }
                "--snapshot" => {
                    opts.snapshot = Some(args.next().ok_or("--snapshot cần đường dẫn")?.into())
                }
                "--ticks" => {
                    opts.ticks = args
                        .next()
                        .ok_or("--ticks cần số")?
                        .parse()
                        .map_err(|_| "--ticks cần số")?;
                }
                "--seed" => {
                    opts.seed = args
                        .next()
                        .ok_or("--seed cần số")?
                        .parse()
                        .map_err(|_| "--seed cần số")?;
                }
                "--rebind" => {
                    let spec = args.next().ok_or("--rebind cần ACTION=KEY")?;
                    let (action, key) = spec
                        .split_once('=')
                        .ok_or("--rebind cần ACTION=KEY (vd. up=W)")?;
                    opts.rebinds.push((parse_action(action)?, parse_key(key)?));
                }
                "--options" => {
                    opts.options_path = Some(
                        args.next()
                            .ok_or("--options cần đường dẫn file options")?
                            .into(),
                    );
                }
                "--save-options" => opts.save_options = true,
                "--volume" => {
                    let raw = args.next().ok_or("--volume cần số 0-100")?;
                    opts.master_volume = Some(parse_volume(&raw)?);
                }
                "--bgm-volume" => {
                    let raw = args.next().ok_or("--bgm-volume cần số 0-100")?;
                    opts.bgm_volume = Some(parse_volume(&raw)?);
                }
                "--sfx-volume" => {
                    let raw = args.next().ok_or("--sfx-volume cần số 0-100")?;
                    opts.sfx_volume = Some(parse_volume(&raw)?);
                }
                "--mute" => opts.mute = Some(true),
                "--no-mute" => opts.mute = Some(false),
                "--bundle" => {
                    opts.bundle = Some(args.next().ok_or("--bundle cần thư mục bundle")?.into())
                }
                "--rules" => {
                    opts.rules = Some(
                        args.next()
                            .ok_or("--rules cần đường dẫn file rules")?
                            .into(),
                    )
                }
                "--l10n" => {
                    opts.l10n = Some(args.next().ok_or("--l10n cần đường dẫn file chuỗi")?.into())
                }
                "--demo" => opts.demo = true,
                "--demo-death" => opts.demo_death = true,
                "--death-frame" => {
                    opts.death_frame = args
                        .next()
                        .ok_or("--death-frame cần số")?
                        .parse()
                        .map_err(|_| "--death-frame cần số")?;
                }
                "--death-hold" => opts.death_hold = true,
                "--connect" => {
                    opts.connect = Some(args.next().ok_or("--connect cần địa chỉ server")?);
                }
                "--user" => {
                    opts.user = Some(args.next().ok_or("--user cần tên đăng nhập")?);
                }
                "--password" => {
                    opts.password = Some(args.next().ok_or("--password cần mật khẩu")?);
                }
                "--register" => opts.register = true,
                "--ui-demo" => opts.ui_demo = true,
                "--ui-shot" => {
                    let screen = args
                        .next()
                        .ok_or("--ui-shot cần tên màn (login|lobby|room|match|result)")?;
                    opts.ui_shot = Some(parse_shot_screen(&screen)?);
                }
                "-h" | "--help" => {
                    println!(
                        "dùng: boom-client [--scale 1|2] [--snapshot PNG] [--ticks N] [--seed N] [--rebind ACTION=KEY]\n\
                         \t  [--options PATH] [--save-options] [--volume N] [--bgm-volume N] [--sfx-volume N] [--mute|--no-mute]\n\
                         \t  [--bundle DIR] [--rules FILE] [--l10n FILE] [--demo] [--demo-death] [--death-frame N] [--death-hold] [--ui-demo] [--ui-shot SCREEN]\n\
                         \t  [--connect ĐỊA_CHỈ_SERVER] [--user TÊN] [--password MẬT_KHẨU] [--register]\n\
                         \n\
                         \t  --bundle DIR   thư mục bundle content (dựng bằng `boom-assets bundle`)\n\
                         \t  --rules FILE   file rules profile TOML (mặc định: bản nhúng)\n\
                         \t  --l10n FILE    file chuỗi UI V1 TOML (mặc định: bản nhúng)\n\
                         \t  --demo-death   kịch bản offline chết (đặt bom rồi đứng yên) để chụp ảnh\n\
                         \t  --death-frame N  frame của chuỗi chết cần chụp (dùng cùng --demo-death)\n\
                         \t  --death-hold   chụp sau khi chuỗi chết hết (nhân vật giữ khung cuối có ảnh)\n\
                         chế độ mạng cần tên/mật khẩu: --user/--password, hoặc biến môi trường BOOM_USER/BOOM_PASSWORD\n\
                         (nên đặt mật khẩu qua biến môi trường: cờ dòng lệnh lộ trong danh sách tiến trình)."
                    );
                    std::process::exit(0);
                }
                other => return Err(format!("tham số lạ: {other}")),
            }
        }
        if opts.scale != 1 && opts.scale != 2 {
            return Err("--scale chỉ nhận 1 hoặc 2".to_owned());
        }
        Ok(opts)
    }
}

/// Thông tin đăng nhập cho chế độ mạng: cờ dòng lệnh trước, biến môi trường sau.
///
/// Server chặn mọi kind trước `AuthOk` (ADR 0009 Addendum A §A.1) nên chế độ mạng **bắt
/// buộc** có tên/mật khẩu; thiếu thì báo lỗi rõ thay vì đoán. Biến môi trường
/// (`BOOM_USER`/`BOOM_PASSWORD`) an toàn hơn cờ dòng lệnh: cờ lộ trong danh sách tiến trình.
///
/// `env_user`/`env_password` là giá trị biến môi trường do caller đọc và truyền vào để hàm này
/// không phụ thuộc môi trường lúc test.
fn net_credentials(
    opts: &Options,
    env_user: Option<String>,
    env_password: Option<String>,
) -> Result<net::Credentials, String> {
    let username = opts
        .user
        .clone()
        .or(env_user)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or("cần tên đăng nhập: --user TÊN hoặc biến môi trường BOOM_USER")?;
    // Mật khẩu **không** trim: khoảng trắng có thể là phần của mật khẩu.
    let password = opts
        .password
        .clone()
        .or(env_password)
        .filter(|value| !value.is_empty())
        .ok_or("cần mật khẩu: --password MẬT_KHẨU hoặc biến môi trường BOOM_PASSWORD")?;
    Ok(if opts.register {
        net::Credentials::register(username, password)
    } else {
        net::Credentials::login(username, password)
    })
}

fn parse_action(value: &str) -> Result<InputAction, String> {
    match value.to_ascii_lowercase().as_str() {
        "up" => Ok(InputAction::Up),
        "down" => Ok(InputAction::Down),
        "left" => Ok(InputAction::Left),
        "right" => Ok(InputAction::Right),
        "bomb" => Ok(InputAction::Bomb),
        other => Err(format!("action lạ: {other} (up/down/left/right/bomb)")),
    }
}

/// Tên phím cho `--rebind`: dùng chung bảng tên với `options.toml` để hai đường không lệch nhau.
fn parse_key(value: &str) -> Result<KeyCode, String> {
    options::parse_key_name(value).ok_or_else(|| format!("tên phím lạ: {value}"))
}

/// Tên màn cho `--ui-shot` (chụp ảnh QA một màn UI cụ thể).
fn parse_shot_screen(value: &str) -> Result<ShotScreen, String> {
    match value.to_ascii_lowercase().as_str() {
        "login" => Ok(ShotScreen::Login),
        "lobby" => Ok(ShotScreen::Lobby),
        "room" => Ok(ShotScreen::Room),
        "match" => Ok(ShotScreen::Match),
        "result" => Ok(ShotScreen::Result),
        other => Err(format!(
            "màn UI lạ: {other} (login|lobby|room|match|result)"
        )),
    }
}

/// Volume 0–100; miền lấy từ `options` (ADR 0011 §7) để không rải hằng số.
fn parse_volume(value: &str) -> Result<u8, String> {
    let raw: i64 = value
        .parse()
        .map_err(|_| format!("volume không hợp lệ: {value}"))?;
    if !(i64::from(VOLUME_MIN)..=i64::from(VOLUME_MAX)).contains(&raw) {
        return Err(format!(
            "volume ngoài miền {VOLUME_MIN}-{VOLUME_MAX}: {value}"
        ));
    }
    u8::try_from(raw).map_err(|_| format!("volume không hợp lệ: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use boom_content::animation::{FrameTiming, PresentationState};
    use boom_content::bundle::{Clip, EvidenceStatus, Frame, Sprite, SpriteImage};
    use boom_content::rules::Source;
    use boom_core::{Dir, EventKind, SimulationEvent};

    /// Rules profile nhúng trong binary (dùng cho test không cần file ngoài).
    fn embedded_profile() -> RulesProfile {
        RulesProfile::from_toml(include_str!("../../../data/rules/v1-dev.toml")).expect("rules")
    }

    #[test]
    fn rebind_spec_parses_actions_and_keys() {
        assert_eq!(parse_action("up").unwrap(), InputAction::Up);
        assert_eq!(parse_action("bomb").unwrap(), InputAction::Bomb);
        assert!(parse_action("jump").is_err());
        assert_eq!(parse_key("W").unwrap(), KeyCode::W);
        assert_eq!(parse_key("space").unwrap(), KeyCode::Space);
        assert_eq!(parse_key("arrowleft").unwrap(), KeyCode::Left);
        assert!(parse_key("nope").is_err());
    }

    #[test]
    fn volume_flag_rejects_values_outside_domain() {
        assert_eq!(parse_volume("0").unwrap(), 0);
        assert_eq!(parse_volume("100").unwrap(), 100);
        assert!(parse_volume("101").is_err());
        assert!(parse_volume("-1").is_err());
        assert!(parse_volume("toi").is_err());
    }

    #[test]
    fn net_credentials_can_come_from_flags_or_env() {
        // Cờ dòng lệnh trước, biến môi trường sau.
        let opts = Options {
            user: Some("tu-flag".to_owned()),
            password: Some("matkhau-flag".to_owned()),
            ..Options::default()
        };
        let credentials = net_credentials(
            &opts,
            Some("tu-env".to_owned()),
            Some("matkhau-env".to_owned()),
        )
        .expect("lấy được thông tin đăng nhập");
        assert_eq!(credentials.username, "tu-flag");
        assert_eq!(credentials.password, "matkhau-flag");
        assert_eq!(credentials.op, boom_proto::dto::auth::AuthOp::Login);

        // Không có cờ thì lấy từ biến môi trường.
        let credentials = net_credentials(
            &Options::default(),
            Some("tu-env".to_owned()),
            Some("matkhau-env".to_owned()),
        )
        .expect("lấy được thông tin đăng nhập từ biến môi trường");
        assert_eq!(credentials.username, "tu-env");
        assert_eq!(credentials.password, "matkhau-env");
    }

    #[test]
    fn net_credentials_bao_loi_khi_thieu_ten_hoac_mat_khau() {
        // Server chặn mọi kind trước `AuthOk` nên thiếu thông tin là lỗi rõ, không phải đoán.
        assert!(net_credentials(&Options::default(), None, Some("matkhau".to_owned())).is_err());
        assert!(net_credentials(&Options::default(), Some("ten".to_owned()), None).is_err());
        assert!(net_credentials(&Options::default(), None, None).is_err());
    }

    #[test]
    fn net_credentials_dang_ky_theo_co_register() {
        let opts = Options {
            register: true,
            ..Options::default()
        };
        let credentials =
            net_credentials(&opts, Some("ten".to_owned()), Some("matkhau".to_owned()))
                .expect("lấy được thông tin đăng nhập");
        assert_eq!(credentials.op, boom_proto::dto::auth::AuthOp::Register);
    }

    #[test]
    fn placeholder_terrain_has_floor_spawns() {
        let terrain = placeholder_terrain();
        assert_eq!((terrain.width, terrain.height), (MAP_WIDTH, MAP_HEIGHT));
        assert!(!terrain.has_unknown());
        for &(x, y) in &SPAWNS {
            assert_eq!(
                terrain.kind(i32::from(x), i32::from(y)),
                Some(CellKind::Floor)
            );
        }
    }

    #[test]
    fn offline_tick_advances_core_without_window() {
        let profile = embedded_profile();
        let mut game =
            Game::new(7, ClientOptions::default(), None, &profile).expect("scene offline");
        assert_eq!(game.state.tick, 0);
        for _ in 0..10 {
            game.tick().expect("tick core");
        }
        assert_eq!(game.state.tick, 10);
        assert_eq!(game.state.players.len(), 2);
        // Không có `--bundle` thì giữ scene placeholder.
        assert!(game.scene.is_none());
    }

    #[test]
    fn game_uses_keybinds_from_options() {
        let profile = embedded_profile();
        let mut options = ClientOptions::default();
        options.keybinds.up = KeyCode::W;
        let game = Game::new(3, options, None, &profile).expect("scene offline");
        assert_eq!(game.input.binds().up, KeyCode::W);
        assert_eq!(game.input.binds().bomb, KeyCode::Space);
    }

    /// Vụ nổ của core phải vào sổ trình diễn để client phát clip nổ (R-088).
    ///
    /// Chạy `boom-core` thật (không GPU): đặt bom, chờ nổ, rồi kiểm sổ trình diễn có
    /// vụ nổ đúng ô bom và đúng tick event.
    #[test]
    fn bomb_explosion_event_reaches_the_presentation_journal() {
        let profile = embedded_profile();
        let mut game =
            Game::new(7, ClientOptions::default(), None, &profile).expect("scene offline");
        // Ô đặt bom: ô spawn của người chơi đầu.
        let bomb_cell = game
            .world
            .cell_at(game.state.players[0].x, game.state.players[0].y);
        let mut events = Vec::new();
        let inputs = [
            PlayerInput {
                dir: Dir::None,
                place_bomb: true,
            },
            PlayerInput::default(),
        ];
        // Chờ bom nổ (fuse 2500 ms ở 33 Hz ≈ 83 tick).
        let mut exploded_at = None;
        for _ in 0..200 {
            game.state
                .step(&game.world, &inputs, &mut events)
                .expect("tick mô phỏng");
            if let Some(event) = events
                .iter()
                .find(|event| matches!(event.kind, EventKind::BombExploded { .. }))
            {
                exploded_at = Some((event.id.tick, event.kind.fields()));
                break;
            }
        }
        let (tick, fields) = exploded_at.expect("bom phải nổ trong lượt");
        assert_eq!(fields[0], ("bomb", 1), "ID bom do allocator cấp");
        assert_eq!(fields[1], ("owner", 0), "bom của người chơi đầu");
        assert_eq!(fields[2], ("x", i64::from(bomb_cell.x)));
        assert_eq!(fields[3], ("y", i64::from(bomb_cell.y)));
        assert_eq!(
            fields[4],
            ("chained", 0),
            "bom đến hạn thì không dây chuyền"
        );
        // Client đọc event của tick vừa chạy ⇒ vụ nổ vào sổ trình diễn.
        game.events = events;
        game.absorb_events();
        // Sổ trình diễn không biết bundle: caller đưa vào bảng số frame theo clip ID.
        let mut frames = anim::FrameCounts::default();
        frames.set(anim::EXPLOSION_CLIP, 10);
        let clock = anim::PresentationClock::new(game.clock.frame, tick + 1);
        let active = game.journal.active(&clock, &frames);
        assert_eq!(active.len(), 1, "hiệu ứng nổ ở ô bom, frame đầu tiên");
        assert_eq!(active[0].subject, anim::Subject::Cell(bomb_cell));
        assert_eq!(active[0].frame, 0);
        // Hết khoảng clip thì không còn hiệu ứng.
        let clock = anim::PresentationClock::new(game.clock.frame, tick + 33);
        assert!(game.journal.active(&clock, &frames).is_empty());
    }

    /// Nhân vật chết: chuỗi chết vào sổ trình diễn, phát **đủ frame** rồi kết thúc (R-089).
    ///
    /// Chạy `boom-core` thật (không GPU): đặt bom rồi đứng yên ⇒ bị nhốt ⇒ chết; event
    /// `player_died` vào sổ trình diễn và chuỗi chết phát một lần hết đúng khoảng clip.
    #[test]
    fn death_event_plays_the_whole_death_clip_once() {
        let profile = embedded_profile();
        let mut game =
            Game::new(7, ClientOptions::default(), None, &profile).expect("scene offline");
        let inputs = [
            PlayerInput {
                dir: Dir::None,
                place_bomb: true,
            },
            PlayerInput::default(),
        ];
        let mut events = Vec::new();
        let mut died = None;
        for _ in 0..600 {
            game.state
                .step(&game.world, &inputs, &mut events)
                .expect("tick mô phỏng");
            if let Some(event) = events
                .iter()
                .find(|event| matches!(event.kind, EventKind::PlayerDied { .. }))
            {
                died = Some(event.clone());
                break;
            }
        }
        let died = died.expect("nhân vật phải chết trong lượt");
        let tick = died.id.tick;
        game.events = events;
        game.absorb_events();
        // Clip chết của pack có 14 frame.
        let mut frames = anim::FrameCounts::default();
        frames.set("c00.dead.static", 14);
        let subject = anim::Subject::Player(0);
        // Chuỗi chết phát đủ frame, mỗi frame đúng một lần.
        let played: Vec<usize> = (tick..tick + 14 * 4)
            .filter_map(|at| {
                let clock = anim::PresentationClock::new(game.clock.frame, at);
                game.journal
                    .active_for(subject, &clock, &frames)
                    .map(|active| active.frame)
            })
            .collect();
        assert!(
            played.windows(2).all(|pair| pair[0] <= pair[1]),
            "chuỗi chết không lặp về đầu: {played:?}"
        );
        let mut unique = played.clone();
        unique.dedup();
        assert_eq!(
            unique,
            (0..14).collect::<Vec<usize>>(),
            "chuỗi chết phát đủ frame, mỗi frame đúng một lần"
        );
        // Hết khoảng clip thì không còn hiệu ứng để phát.
        let clock = anim::PresentationClock::new(game.clock.frame, tick + 14 * 4);
        assert!(game.journal.active_for(subject, &clock, &frames).is_none());
        // Event lặp (reconcile gửi lại event cũ) không tạo mục mới.
        assert!(!game.journal.record_event(&died));
    }

    /// Bundle tối thiểu cho clip chết: 14 frame, khung cuối trỏ **ảnh rỗng 0×0**.
    ///
    /// Dữ liệu thật của client gốc (R-086): khung cuối của clip chết không có pixel, nên
    /// nhân vật chết phải giữ khung cuối **có ảnh** (khung 12) thay vì ô màu.
    fn death_bundle() -> Bundle {
        let mut frames = Vec::new();
        for _ in 0..14 {
            frames.push(Frame {
                image: 0,
                offset_x: 0,
                offset_y: 39,
                flag: -1,
                x0: 0,
                y0: 0,
                x1: 40,
                y1: 40,
                canvas_width: 40,
                canvas_height: 40,
                pivot_x: 0,
                pivot_y: 39,
            });
        }
        // Khung cuối trỏ ảnh rỗng (0×0) như dữ liệu thật.
        frames[13].image = 1;
        Bundle {
            schema: boom_content::bundle::SCHEMA,
            build: "kr".to_owned(),
            map: "Village00".to_owned(),
            tileset: "Tile00".to_owned(),
            width: 0,
            height: 0,
            spawns: Vec::new(),
            content_hash: String::new(),
            atlas: Vec::new(),
            sprites: vec![Sprite {
                id: "c00".to_owned(),
                entry: "Character/C00-DieLayerStatic".to_owned(),
                role: "character_walk".to_owned(),
                status: EvidenceStatus::Confirmed,
                source_sha256: "00".repeat(32),
                map_id: None,
                object_name: None,
                images: vec![
                    SpriteImage {
                        index: 0,
                        page: Some(0),
                        x: 0,
                        y: 0,
                        width: 40,
                        height: 40,
                        pixel_sha256: "11".repeat(32),
                    },
                    // Ảnh rỗng: 0×0, không đặt trong atlas.
                    SpriteImage {
                        index: 1,
                        page: None,
                        x: 0,
                        y: 0,
                        width: 0,
                        height: 0,
                        pixel_sha256: "22".repeat(32),
                    },
                ],
                frames: Vec::new(),
            }],
            assets: Vec::new(),
            cells: Vec::new(),
            unknown: Vec::new(),
            clips: vec![Clip {
                id: "c00.dead.static".to_owned(),
                state: PresentationState::Dead,
                pack: "FxCharacter".to_owned(),
                entry: "Character/C00-DieLayerStatic".to_owned(),
                layer: None,
                sprite_unk: 0,
                sprite: "c00".to_owned(),
                source: Source::KrStatic,
                status: EvidenceStatus::Confirmed,
                claims: vec!["R-077".to_owned()],
                note: None,
                timing: FrameTiming::Unknown {
                    reason: "chưa có nguồn".to_owned(),
                },
                frames,
            }],
            ui: Vec::new(),
            audio: Vec::new(),
        }
    }

    /// Chuỗi chết khi **lượt kết thúc**: vẫn phát trọn clip rồi giữ khung cuối có ảnh.
    ///
    /// Roster mặc định **hai** người: một người chết thì lượt kết thúc ngay
    /// (`Phase::Ended`) và mô phỏng dừng. Chuỗi chết phải **chạy nốt** nhờ đồng hồ
    /// trình diễn tiến sau khi mô phỏng dừng, rồi nhân vật chết giữ khung cuối **có
    /// ảnh** của clip chết (R-089).
    ///
    /// Test đi qua **đường thật**: `Game::tick()` (update) rồi `scene::presentation_plan`
    /// (render), không truyền tick tương lai vào sổ.
    #[test]
    fn death_chain_finishes_after_the_round_ended() {
        let profile = embedded_profile();
        let mut game =
            Game::new(7, ClientOptions::default(), None, &profile).expect("scene offline");
        // Client thật luôn nạp bundle; test gắn bundle **dựng sẵn** (không có asset game)
        // để sổ trình diễn có clip chết và để bước render biết clip nào có ảnh.
        let bundle = death_bundle();
        game.bundle = Some(bundle.clone());
        // Kịch bản chết: người chơi 0 đặt bom rồi đứng yên ⇒ bị lửa bắt ⇒ chết.
        game.enable_demo(true);
        // Chạy tới khi lượt kết thúc (người chơi 0 chết, chỉ còn một người sống).
        let mut guard = 0;
        while game.state.phase == Phase::Playing {
            game.tick().expect("tick mô phỏng");
            guard += 1;
            assert!(guard < 10_000, "lượt phải kết thúc");
        }
        assert!(
            matches!(game.state.phase, Phase::Ended(_)),
            "một người chết thì lượt hai người phải kết thúc"
        );
        let subject = anim::Subject::Player(0);
        // Khoá event của mục đã xác nhận: dùng để kiểm tra event gửi lại không phát lại.
        let key = game
            .journal
            .item_for(subject)
            .and_then(|item| item.key)
            .expect("mục chết phải có khoá event");
        // Sổ trình diễn không biết bundle: caller đưa vào bảng số frame theo clip ID.
        let frames = scene::frame_counts(&bundle);
        // Chuỗi chết phải chạy nốt sau khi lượt kết thúc: đồng hồ trình diễn tiến mỗi
        // tick client, mô phỏng thì đã dừng.
        let mut played: Vec<usize> = Vec::new();
        for _ in 0..200 {
            game.tick().expect("tick trình diễn");
            if let Some(active) = game.journal.active_for(subject, &game.clock, &frames) {
                played.push(active.frame);
            }
        }
        assert!(
            played.windows(2).all(|pair| pair[0] <= pair[1]),
            "chuỗi chết không lặp về đầu: {played:?}"
        );
        let mut unique = played.clone();
        unique.dedup();
        assert_eq!(
            unique,
            (0..14).collect::<Vec<usize>>(),
            "chuỗi chết phát đủ frame sau khi lượt kết thúc"
        );
        // Hết khoảng clip ⇒ mục bị dọn khỏi sổ.
        assert!(
            game.journal.item_for(subject).is_none(),
            "mục hết khoảng clip phải bị dọn"
        );
        // Render: nhân vật chết giữ **khung cuối có ảnh** của clip chết (khung 12),
        // không vẽ ô màu.
        let scene = scene::Scene::from_bundle(&bundle, &classifier(), 40).expect("dựng scene");
        let plan = scene::presentation_plan(
            &bundle,
            &scene,
            &game.state,
            &game.world,
            &game.clock,
            &game.previous_positions,
            &game.journal,
        );
        let dead_items: Vec<&scene::PlanItem> =
            plan.items
                .iter()
                .filter(|item| match item {
                    scene::PlanItem::Sprite { clip, .. }
                    | scene::PlanItem::Fallback { clip, .. } => clip == "c00.dead.static",
                    scene::PlanItem::Object(_) => false,
                })
                .collect();
        assert_eq!(dead_items.len(), 1, "chỉ một mục cho nhân vật chết");
        assert!(
            matches!(
                dead_items[0],
                scene::PlanItem::Sprite {
                    clip,
                    frame: 12,
                    ..
                } if clip == "c00.dead.static"
            ),
            "nhân vật chết giữ khung cuối có ảnh, không vẽ ô màu: {dead_items:?}"
        );
        // Event gửi lại (reconcile) không tạo mục mới ⇒ không phát lại chuỗi chết.
        let repeated = SimulationEvent {
            id: key.event_id,
            kind: EventKind::PlayerDied {
                player: 0,
                by: None,
            },
        };
        assert!(
            !game.journal.record_event(&repeated),
            "event gửi lại không được tạo mục mới"
        );
        assert!(game.journal.item_for(subject).is_none());
    }

    /// Bảng phân loại ô (bundle test không có ô nào nên bảng mặc định là đủ).
    fn classifier() -> boom_content::map::CellClassifier {
        boom_content::map::CellClassifier::default()
    }

    #[test]
    fn missing_bundle_directory_is_reported_not_panicking() {
        let profile = embedded_profile();
        let error = match Game::new(
            7,
            ClientOptions::default(),
            Some(Path::new("/khong/ton/tai/bundle")),
            &profile,
        ) {
            Ok(_) => panic!("thư mục bundle không tồn tại phải báo lỗi"),
            Err(error) => error,
        };
        let message = error.to_string();
        assert!(message.contains("bundle"));
        // V1-036: thông điệp phải có đường dẫn và cách sửa.
        assert!(
            message.contains("/khong/ton/tai/bundle"),
            "thông điệp phải nêu đường dẫn: {message}"
        );
        assert!(
            message.contains("boom-assets bundle"),
            "thông điệp phải nêu cách dựng lại bundle: {message}"
        );
    }

    /// Thiếu file rules → thông điệp có đường dẫn và cách sửa, không panic.
    #[test]
    fn missing_rules_file_reports_path_and_fix() {
        let error = load_rules_file(Path::new("/khong/ton/tai/rules.toml")).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("/khong/ton/tai/rules.toml"),
            "thông điệp phải nêu đường dẫn: {message}"
        );
        assert!(
            message.contains("sửa:"),
            "thông điệp phải nêu cách sửa: {message}"
        );
    }

    /// File rules sai cú pháp → thông điệp có đường dẫn và cách sửa.
    #[test]
    fn broken_rules_file_reports_path_and_fix() {
        let dir = std::env::temp_dir().join(format!("boom-client-rules-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("tạo thư mục tạm");
        let path = dir.join("rules.toml");
        std::fs::write(&path, "schema = 1\nprofile_id = \"x\"\n").expect("ghi file");
        let error = load_rules_file(&path).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains(path.to_str().unwrap()),
            "thông điệp phải nêu đường dẫn: {message}"
        );
        assert!(
            message.contains("sửa:"),
            "thông điệp phải nêu cách sửa: {message}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Thiếu file chuỗi → thông điệp có đường dẫn và cách sửa, không panic.
    #[test]
    fn missing_l10n_file_reports_path_and_fix() {
        let error = load_l10n_file(Path::new("/khong/ton/tai/vi-v1.toml")).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("/khong/ton/tai/vi-v1.toml"),
            "thông điệp phải nêu đường dẫn: {message}"
        );
        assert!(
            message.contains("sửa:"),
            "thông điệp phải nêu cách sửa: {message}"
        );
    }

    /// Bản nhúng (không có `--rules`/`--l10n`) phải nạp được: gói phát hành không bắt
    /// buộc người dùng có file bên ngoài.
    #[test]
    fn embedded_content_loads_without_files() {
        let profile = load_rules(&None).expect("rules nhúng");
        assert!(!profile.profile_id.is_empty());
        let l10n = load_l10n(&None).expect("chuỗi nhúng");
        assert_eq!(l10n.locale(), "vi-VN");
    }

    /// Gạch phá được phải biến mất khỏi scene sau khi nổ (review #81).
    ///
    /// Chạy thật `boom-core` (không GPU): đặt bom cạnh gạch, chờ nổ, rồi kiểm ô gạch
    /// chuyển từ `Breakable` sang `Floor` và quyết định vẽ của client đổi theo.
    #[test]
    fn destroyed_block_is_not_drawn_anymore() {
        // Bản đồ 3×1: spawn ở (0,0), nền ở (1,0), gạch ở (2,0).
        let terrain = Terrain::from_ascii(&["0.+"]).expect("bản đồ test");
        let profile = RulesProfile::from_toml(include_str!("../../../data/rules/v1-dev.toml"))
            .expect("rules");
        let rules = Rules::from_profile(&profile).expect("rules");
        let mode = mode_by_id("battle_ffa").expect("mode");
        let world = World::new(terrain, rules, mode).expect("world");
        let roster = [PlayerSetup {
            id: 0,
            stats: DEMO_STATS,
        }];
        let mut state = SimulationState::new_round(&world, &roster, 7).expect("lượt");
        let mut events = Vec::new();
        let block = Cell::new(2, 0);

        // Trước khi nổ: ô là gạch phá được → hình gạch được vẽ.
        assert_eq!(state.field.kind(block), Some(CellKind::Breakable));
        assert!(scene::object_visible(
            CellKind::Breakable,
            state.field.kind(block)
        ));

        // Đi tới ô (1,0) rồi đặt bom (tầm 1 → phá gạch ở (2,0)).
        for _ in 0..300 {
            let cell = world.cell_at(state.players[0].x, state.players[0].y);
            let input = PlayerInput {
                dir: if cell.x < 1 { Dir::Right } else { Dir::None },
                place_bomb: cell.x == 1 && state.bombs.is_empty(),
            };
            state
                .step(&world, &[input], &mut events)
                .expect("tick mô phỏng");
            if state.field.kind(block) == Some(CellKind::Floor) {
                break;
            }
        }

        // Sau khi nổ: ô thành nền → hình gạch không vẽ nữa.
        assert_eq!(state.field.kind(block), Some(CellKind::Floor));
        assert!(!scene::object_visible(
            CellKind::Breakable,
            state.field.kind(block)
        ));
        // Vật cản cứng vẫn còn (loại ô khác `Breakable` thì luôn vẽ).
        assert!(scene::object_visible(
            CellKind::Solid,
            state.field.kind(block)
        ));
    }

    #[test]
    fn default_options_are_valid() {
        let opts = Options::default();
        assert_eq!(opts.scale, 2);
        assert_eq!(opts.ticks, DEFAULT_TICKS);
        assert_eq!(opts.seed, DEFAULT_SEED);
        assert!(opts.options_path.is_none());
        assert!(!opts.save_options);
        assert!(opts.bundle.is_none());
        assert!(opts.rules.is_none());
        assert!(opts.l10n.is_none());
    }
}
