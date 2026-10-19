//! Client V1 trên macroquad (ADR 0010), chế độ offline cho V1-033.
//!
//! Phạm vi task: input (mũi tên + Space, rebind, mất focus nhả phím), integer scale 1×/2× bằng
//! render target + nearest + copy shader, và chạy `boom-core` trực tiếp với tick cố định tách khỏi
//! render. Địa hình là placeholder vẽ theo `CellKind`; khi có `--bundle <dir>` thì nạp bundle
//! content đã validate, dựng địa hình Village00 từ bundle và vẽ tile/vật cản bằng atlas thật.
//!
//! Tùy chọn người dùng đọc/ghi từ `options.toml` (module [`options`], ADR 0011 §7): volume,
//! mute và keybind. Keybind trong file **thay** hằng số phím mặc định; `--rebind` của dòng lệnh
//! vẫn hoạt động và được ghi đè lên trên (đổi sau, thắng trước).

mod input;
mod options;
mod render;
mod scene;

use std::path::{Path, PathBuf};

use boom_content::map::{CellKind, MapProfile, Terrain};
use boom_content::rules::RulesProfile;
use boom_core::{
    Cell, CharacterStats, Life, Phase, Player, PlayerInput, PlayerSetup, Rules, SimulationEvent,
    SimulationState, Stat, World, mode_by_id,
};
use macroquad::material::{
    Material, MaterialParams, gl_use_default_material, gl_use_material, load_material,
};
use macroquad::miniquad::{PipelineParams, ShaderSource};
use macroquad::prelude::*;

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

        // Scene: bundle content nếu có, không thì địa hình placeholder.
        let (terrain, scene) = match bundle_dir {
            Some(dir) => {
                let loaded = scene::load(dir, tile_px)?;
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
                // Phân loại ô nằm ở bảng chuẩn `data/maps/`, không nhân bản trong bundle.
                let map_profile =
                    MapProfile::from_toml(include_str!("../../../data/maps/village00-kr.toml"))?;
                let terrain = scene::terrain(&loaded.bundle, &map_profile)?;
                (terrain, Some(loaded.scene))
            }
            None => (placeholder_terrain(), None),
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
        Ok(Self {
            world,
            state,
            events: Vec::new(),
            input: InputTracker::new(options.keybinds),
            options,
            logic_size,
            scene,
            bundle_dir: bundle_dir.map(Path::to_path_buf),
        })
    }

    fn tick(&mut self) -> Result<(), boom_core::CoreError> {
        if self.state.phase != Phase::Playing {
            return Ok(());
        }
        let input = PlayerInput {
            dir: self.input.direction(),
            place_bomb: self.input.take_bomb(),
        };
        let inputs = [input, PlayerInput::default()];
        self.state.step(&self.world, &inputs, &mut self.events)?;
        // Presentation chưa dùng event; xóa để không phình bộ nhớ trong chế độ offline.
        self.events.clear();
        Ok(())
    }

    fn tile_px(&self) -> f32 {
        self.world.rules().tile.0 as f32 / 1000.0
    }
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

    if let Some(path) = &opts.snapshot {
        for _ in 0..opts.ticks {
            if let Err(error) = game.tick() {
                eprintln!("snapshot: không chạy được tick: {error}");
                std::process::exit(2);
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

    draw_rectangle(
        0.0,
        0.0,
        game.logic_size[0] as f32,
        HUD_HEIGHT,
        Color::new(0.03, 0.04, 0.06, 1.0),
    );
    draw_text(
        format!(
            "boom-client {}  tick {}  {} Hz  {}x  vol {}/{}/{}{}  {}={:?}  {}={:?}  held {}  F2 focus",
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
        let x = player.x.0 as f32 / 1000.0;
        let y = HUD_HEIGHT + player.y.0 as f32 / 1000.0;
        draw_rectangle(x - 12.0, y - 12.0, 24.0, 24.0, player_color(player));
    }
}

/// Scene từ bundle: nền vẽ trước, vật cản và nhân vật theo z-order hàng.
fn draw_map_from_bundle(game: &Game, scene: &Scene, atlas: &Atlas) {
    let origin = (0.0, HUD_HEIGHT);
    for item in &scene.ground {
        atlas.draw(item, origin);
    }

    // Vật cản và nhân vật cùng một thứ tự z-order: hàng lớn hơn vẽ sau.
    let mut layer: Vec<(u16, u16, LayerItem)> =
        Vec::with_capacity(scene.objects.len() + game.state.players.len());
    for item in &scene.objects {
        layer.push((item.row, item.column, LayerItem::Sprite(*item)));
    }
    for player in &game.state.players {
        let cell = game.world.cell_at(player.x, player.y);
        layer.push((
            u16::try_from(cell.y).unwrap_or(u16::MAX),
            u16::try_from(cell.x).unwrap_or(u16::MAX),
            LayerItem::Player {
                x: player.x.0 as f32 / 1000.0,
                y: player.y.0 as f32 / 1000.0,
                color: player_color(player),
            },
        ));
    }
    layer.sort_by_key(|(row, column, _)| (*row, *column));

    for (_, _, item) in layer {
        match item {
            LayerItem::Sprite(item) => atlas.draw(&item, origin),
            LayerItem::Player { x, y, color } => {
                draw_rectangle(origin.0 + x - 12.0, origin.1 + y - 12.0, 24.0, 24.0, color);
            }
        }
    }
}

/// Một lớp vẽ: sprite từ atlas hoặc nhân vật placeholder.
enum LayerItem {
    Sprite(scene::DrawItem),
    Player { x: f32, y: f32, color: Color },
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
                "-h" | "--help" => {
                    println!(
                        "dùng: boom-client [--scale 1|2] [--snapshot PNG] [--ticks N] [--seed N] [--rebind ACTION=KEY]\n\
                         \t  [--options PATH] [--save-options] [--volume N] [--bgm-volume N] [--sfx-volume N] [--mute|--no-mute]\n\
                         \t  [--bundle DIR]"
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
