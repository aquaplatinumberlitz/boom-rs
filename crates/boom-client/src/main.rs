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
//! Cờ `--connect <addr>` mở chế độ mạng: module [`net`] (handshake, gửi input, dự đoán và
//! reconciliation theo ADR 0005/0009). Không có cờ thì giữ chế độ offline.

mod anim;
mod audio;
mod input;
mod net;
mod options;
mod render;
mod scene;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use boom_content::bundle::Bundle;
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

const HUD_HEIGHT: f32 = 40.0;
const MAP_WIDTH: u16 = 15;
const MAP_HEIGHT: u16 = 13;
const DEFAULT_TICKS: u32 = 60;
const DEFAULT_SEED: u64 = 0xC0FFEE;

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
    let mut game = match Game::new(opts.seed, client_options, opts.bundle.as_deref()) {
        Ok(game) => game,
        Err(error) => {
            eprintln!("không dựng được scene offline: {error}");
            std::process::exit(2);
        }
    };
    // Chế độ mạng: nối server và dùng module `net`; không có `--connect` thì giữ offline.
    if let Some(addr) = opts.connect.clone() {
        match net::connect(&addr) {
            Ok(client) => game.net = Some(client),
            Err(error) => {
                eprintln!("không vào được chế độ mạng: {error}");
                std::process::exit(2);
            }
        }
    }
    if opts.demo {
        game.enable_demo();
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

fn window_conf(game: &Game, opts: &Options) -> macroquad::miniquad::conf::Conf {
    macroquad::miniquad::conf::Conf {
        window_title: "boom-rs client — offline".to_owned(),
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
    /// Nhịp phát frame: thời lượng một frame và tần số tick mô phỏng.
    frame_clock: FrameClock,
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
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let profile = RulesProfile::from_toml(include_str!("../../../data/rules/v1-dev.toml"))?;
        let rules = Rules::from_profile(&profile)?;
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

        // Scene: bundle content nếu có, không thì địa hình placeholder.
        let (terrain, scene, bundle) = match bundle_dir {
            Some(dir) => {
                // Phân loại ô nằm ở bảng chuẩn `data/maps/`, không nhân bản trong bundle.
                let map_profile =
                    MapProfile::from_toml(include_str!("../../../data/maps/village00-kr.toml"))?;
                let loaded = scene::load(dir, &map_profile, tile_px)?;
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
                        eprintln!(
                            "cảnh báo bundle: không có clip {clip_id} — sẽ vẽ ô màu thay thế"
                        );
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
            frame_clock,
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
    fn enable_demo(&mut self) {
        self.demo = Some(Demo::new(&self.world, &self.state));
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
            return Ok(());
        }
        if self.state.phase != Phase::Playing {
            return Ok(());
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
        self.events.clear();
        Ok(())
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

/// Kịch bản demo offline: mỗi người chơi đặt một bom rồi đi ra xa.
///
/// Kịch bản **chỉ đẩy input** vào core như người chơi, không sửa state: mọi thứ trong
/// ảnh đều là kết quả mô phỏng thật. Dùng để chụp ảnh có nhân vật + bom + lửa.
///
/// V1 giới hạn mỗi người chơi có **một** bom cùng lúc, nên cần **hai** người và hai
/// bom đặt lệch nhau: bom của người thứ nhất nổ tạo lửa, bom của người thứ hai còn
/// chờ nên ảnh có cả bom lẫn lửa. Người chơi đi ra xa ít nhất hai ô nên không bị lửa
/// của bom mình chạm tới.
struct Demo {
    /// Hướng đi của từng người chơi (chọn theo địa hình: có ô trống phía trước).
    dirs: BTreeMap<u8, Dir>,
    /// Ô đặt bom của từng người chơi.
    bomb_cells: BTreeMap<u8, Cell>,
    /// Ô của từng người chơi ở lần gọi trước (để biết vừa bước sang ô mới).
    last_cells: BTreeMap<u8, Cell>,
    /// Người chơi đặt bom ngay (bom của họ nổ sớm, tạo lửa).
    place_now: BTreeSet<u8>,
}

impl Demo {
    /// Chọn hướng đi cho từng người chơi.
    ///
    /// Người chơi đầu tiên đặt bom ngay; người chơi sau đặt bom khi vừa bước sang ô
    /// mới, nên bom của họ nổ muộn hơn và còn chờ được khi ảnh được chụp.
    fn new(world: &World, state: &SimulationState) -> Self {
        let mut dirs = BTreeMap::new();
        let mut place_now = BTreeSet::new();
        for (index, player) in state.players.iter().enumerate() {
            let cell = world.cell_at(player.x, player.y);
            dirs.insert(player.id, choose_demo_direction(world, cell));
            if index == 0 {
                place_now.insert(player.id);
            }
        }
        Self {
            dirs,
            bomb_cells: BTreeMap::new(),
            last_cells: BTreeMap::new(),
            place_now,
        }
    }

    /// Input của demo cho người chơi `id`.
    ///
    /// Bom đặt một lần: người đặt ngay thì đặt ở tick đầu, người còn lại đặt khi vừa
    /// bước sang ô mới. Sau đó người chơi chỉ đi theo hướng đã chọn.
    fn input_for(&mut self, id: u8, state: &SimulationState, world: &World) -> PlayerInput {
        let Some(player) = state.player(id) else {
            return PlayerInput::default();
        };
        let dir = self.dirs.get(&id).copied().unwrap_or(Dir::Right);
        let cell = world.cell_at(player.x, player.y);
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
        if opts.demo {
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
                game.state.tick
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
            "  net: round {} slot {}  server {} dự đoán {}  chờ ack {}  event {}{}",
            status.round,
            status.slot,
            status.confirmed_tick,
            status.predicted_tick,
            status.pending,
            status.events,
            if status.ended { "  kết quả" } else { "" }
        ),
        None => String::new(),
    };
    draw_text(
        format!(
            "boom-client {}  tick {}  {} Hz  {}x  vol {}/{}/{}{}  {}={:?}  {}={:?}  held {}  F2 focus{net_text}",
            game.scene
                .as_ref()
                .map_or_else(|| "placeholder".to_owned(), |scene| scene.map.clone()),
            game.state.tick,
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
        &game.frame_clock,
        &game.previous_positions,
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
    /// Chạy kịch bản demo offline (đặt bom, đi ra xa, đặt bom thứ hai) để chụp ảnh.
    demo: bool,
    /// Địa chỉ server để nối (chế độ mạng). Không có thì client chạy offline.
    connect: Option<String>,
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
            demo: false,
            connect: None,
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
                "--demo" => opts.demo = true,
                "--connect" => {
                    opts.connect = Some(args.next().ok_or("--connect cần địa chỉ server")?);
                }
                "-h" | "--help" => {
                    println!(
                        "dùng: boom-client [--scale 1|2] [--snapshot PNG] [--ticks N] [--seed N] [--rebind ACTION=KEY]\n\
                         \t  [--options PATH] [--save-options] [--volume N] [--bgm-volume N] [--sfx-volume N] [--mute|--no-mute]\n\
                         \t  [--bundle DIR] [--demo] [--connect ĐỊA_CHỈ_SERVER]"
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
    use boom_core::Dir;

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
        let mut game = Game::new(7, ClientOptions::default(), None).expect("scene offline");
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
        let mut options = ClientOptions::default();
        options.keybinds.up = KeyCode::W;
        let game = Game::new(3, options, None).expect("scene offline");
        assert_eq!(game.input.binds().up, KeyCode::W);
        assert_eq!(game.input.binds().bomb, KeyCode::Space);
    }

    #[test]
    fn missing_bundle_directory_is_reported_not_panicking() {
        let error = match Game::new(
            7,
            ClientOptions::default(),
            Some(Path::new("/khong/ton/tai/bundle")),
        ) {
            Ok(_) => panic!("thư mục bundle không tồn tại phải báo lỗi"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("bundle"));
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
    }
}
