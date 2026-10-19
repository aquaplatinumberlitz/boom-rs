//! Spike client stack: macroquad.
//!
//! Một scene V1 tối thiểu: Village00 tĩnh + C00 đi bốn hướng + bom nổ + chữ Việt có dấu
//! + BGM/SFX + integer scale 1×/2×. Chạy `--snapshot` để render một frame xác định ra PNG.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound};
use macroquad::prelude::*;
use serde::Deserialize;

const HUD: f32 = 40.0;

fn main() {
    let opts = match Options::parse() {
        Ok(opts) => opts,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    let conf = window_conf(&opts);
    macroquad::Window::from_config(conf, run(opts));
}

fn window_conf(opts: &Options) -> macroquad::miniquad::conf::Conf {
    macroquad::miniquad::conf::Conf {
        window_title: "boom-rs spike macroquad".to_owned(),
        window_width: (600 * opts.scale) as i32,
        window_height: (560 * opts.scale) as i32,
        window_resizable: false,
        high_dpi: false,
        sample_count: 1,
        ..Default::default()
    }
}

async fn run(opts: Options) {
    let scene = match Scene::load(&opts.assets) {
        Ok(scene) => scene,
        Err(error) => {
            eprintln!("không nạp được scene: {error}");
            std::process::exit(2);
        }
    };
    if opts.audio_smoke {
        match load_audio(&scene, &opts.assets).await {
            Ok(_) => {
                println!(
                    "audio smoke PASS: BGM+SFX load/decode OK; playback/timing chưa đo (không có soundcard)"
                );
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("audio smoke FAIL: {error}");
                std::process::exit(2);
            }
        }
    }
    let textures = match load_textures(&scene, &opts.assets) {
        Ok(textures) => textures,
        Err(error) => {
            eprintln!("không nạp được texture: {error}");
            std::process::exit(2);
        }
    };
    let font = match load_font() {
        Ok(font) => font,
        Err(error) => {
            eprintln!("không nạp được font: {error}");
            std::process::exit(2);
        }
    };
    let mut audio = if opts.no_audio {
        None
    } else {
        match load_audio(&scene, &opts.assets).await {
            Ok(audio) => Some(audio),
            Err(error) => {
                eprintln!("không nạp được audio: {error}");
                std::process::exit(2);
            }
        }
    };
    let target = render_target(scene.logic_size[0], scene.logic_size[1]);
    target.texture.set_filter(FilterMode::Nearest);
    let mut scale = opts.scale;
    let mut explosion_at = if opts.snapshot.is_some() {
        0.0
    } else {
        f64::NEG_INFINITY
    };
    let mut frame = 0u32;

    loop {
        let now = if opts.snapshot.is_some() {
            opts.time_ms / 1000.0
        } else {
            get_time()
        };
        if let Some(audio) = &mut audio
            && !audio.started
            && opts.snapshot.is_none()
        {
            audio.start();
            audio.started = true;
        }
        if opts.snapshot.is_none() {
            if is_key_pressed(KeyCode::Escape) {
                macroquad::miniquad::window::quit();
            }
            if is_key_pressed(KeyCode::S) {
                scale = if scale == 1 { 2 } else { 1 };
                macroquad::miniquad::window::set_window_size(
                    scene.logic_size[0] * scale,
                    scene.logic_size[1] * scale,
                );
            }
            if is_key_pressed(KeyCode::B) {
                explosion_at = now;
                if let Some(audio) = &audio {
                    audio.sfx();
                }
            }
            if is_key_pressed(KeyCode::R) {
                explosion_at = f64::NEG_INFINITY;
            }
        }
        draw_scene(
            &scene,
            &textures,
            &font,
            &target,
            scale,
            now,
            explosion_at,
            &opts,
        );
        frame += 1;

        if let Some(path) = &opts.snapshot
            && frame >= 3
        {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            get_screen_data().export_png(&path.to_string_lossy());
            println!(
                "snapshot {} ({}x{}, scale {scale})",
                path.display(),
                scene.logic_size[0] * scale,
                scene.logic_size[1] * scale
            );
            std::process::exit(0);
        }
        next_frame().await;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_scene(
    scene: &Scene,
    textures: &BTreeMap<String, Texture2D>,
    font: &Font,
    target: &RenderTarget,
    scale: u32,
    now: f64,
    explosion_at: f64,
    opts: &Options,
) {
    let mut camera = Camera2D::from_display_rect(Rect::new(
        0.0,
        0.0,
        scene.logic_size[0] as f32,
        scene.logic_size[1] as f32,
    ));
    camera.render_target = Some(target.clone());
    set_camera(&camera);
    clear_background(Color::from_rgba(
        scene.background[0],
        scene.background[1],
        scene.background[2],
        255,
    ));

    let map_x = scene.map_origin[0] as f32;
    let map_y = scene.map_origin[1] as f32;
    let tile = scene.tile_size as f32;
    let width = scene.map.width as usize;

    for (index, key) in scene.map.ground.iter().enumerate() {
        let x = u16::try_from(index % width).expect("x");
        let y = u16::try_from(index / width).expect("y");
        draw_sprite(
            textures,
            key,
            map_x + f32::from(x) * tile,
            map_y + f32::from(y) * tile,
        );
    }

    let character = character_key(scene, opts, now);
    let bomb = scene
        .bomb
        .get((now * 8.0) as usize % scene.bomb.len().max(1));
    let explosion_frame = scene
        .explosion
        .get(((now - explosion_at) * 14.0).max(0.0) as usize % scene.explosion.len().max(1));
    let explosion_on = now >= explosion_at && now - explosion_at < 1.4;

    let mut draws: Vec<(f32, DrawKind)> = scene
        .map
        .objects
        .iter()
        .map(|object| (f32::from(object.y) * tile + tile, DrawKind::Object(object)))
        .collect();
    draws.push((
        f32::from(scene.character_cell[1]) * tile + tile,
        DrawKind::Character(character),
    ));
    if let Some(key) = bomb {
        draws.push((
            f32::from(scene.bomb_cell[1]) * tile + tile,
            DrawKind::Bomb(key),
        ));
    }
    if explosion_on && let Some(key) = explosion_frame {
        draws.push((
            f32::from(scene.explosion_cell[1]) * tile + tile,
            DrawKind::Explosion(key),
        ));
    }
    draws.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("sort"));

    for (_, kind) in draws {
        match kind {
            DrawKind::Object(object) => {
                let meta = &scene.sprites[object.sprite.as_str()];
                let x = map_x + f32::from(object.x) * tile - meta.anchor[0] as f32;
                let y = map_y + f32::from(object.y) * tile + tile - (meta.anchor[1] + 1) as f32;
                draw_sprite(textures, &object.sprite, x, y);
            }
            DrawKind::Character(key) => {
                let meta = &scene.sprites[key];
                let x = map_x + f32::from(scene.character_cell[0]) * tile + tile / 2.0
                    - meta.w as f32 / 2.0;
                let y = map_y + f32::from(scene.character_cell[1]) * tile + tile
                    - (meta.anchor[1] + 1) as f32;
                draw_sprite(textures, key, x, y);
            }
            DrawKind::Bomb(key) => {
                let meta = &scene.sprites[key];
                let x =
                    map_x + f32::from(scene.bomb_cell[0]) * tile + tile / 2.0 - meta.w as f32 / 2.0;
                let y = map_y + f32::from(scene.bomb_cell[1]) * tile + tile
                    - (meta.anchor[1] + 1) as f32;
                draw_sprite(textures, key, x, y);
            }
            DrawKind::Explosion(key) => {
                let meta = &scene.sprites[key];
                let x = map_x + f32::from(scene.explosion_cell[0]) * tile + tile / 2.0
                    - meta.w as f32 / 2.0;
                let y = map_y + f32::from(scene.explosion_cell[1]) * tile + tile
                    - (meta.anchor[1] + 1) as f32;
                draw_sprite(textures, key, x, y);
            }
        }
    }

    draw_rectangle(
        0.0,
        0.0,
        scene.logic_size[0] as f32,
        HUD,
        Color::new(0.04, 0.05, 0.07, 1.0),
    );
    draw_rectangle(
        0.0,
        HUD - 1.0,
        scene.logic_size[0] as f32,
        1.0,
        Color::new(0.25, 0.45, 0.55, 1.0),
    );
    draw_text_ex(
        &scene.text,
        10.0,
        26.0,
        TextParams {
            font: Some(font),
            font_size: 14,
            color: Color::new(0.92, 0.96, 1.0, 1.0),
            ..Default::default()
        },
    );

    set_default_camera();
    clear_background(Color::new(0.02, 0.02, 0.03, 1.0));
    draw_texture_ex(
        &target.texture,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(
                (scene.logic_size[0] * scale) as f32,
                (scene.logic_size[1] * scale) as f32,
            )),
            flip_y: true,
            ..Default::default()
        },
    );
}

enum DrawKind<'a> {
    Object(&'a ObjectScene),
    Character(&'a str),
    Bomb(&'a str),
    Explosion(&'a str),
}

fn character_key<'a>(scene: &'a Scene, opts: &Options, now: f64) -> &'a str {
    let dir = if let Some(dir) = &opts.dir {
        dir.as_str()
    } else if opts.snapshot.is_some() {
        scene.character_dir.as_str()
    } else {
        match ((now / 2.0) as usize) % 4 {
            0 => "right",
            1 => "down",
            2 => "left",
            _ => "up",
        }
    };
    let frames = scene
        .characters
        .get(dir)
        .unwrap_or_else(|| scene.characters.get("right").expect("character right"));
    let step = ((now * 12.0) as usize) % frames.len().max(1);
    &frames[step]
}

fn draw_sprite(textures: &BTreeMap<String, Texture2D>, key: &str, x: f32, y: f32) {
    if let Some(texture) = textures.get(key) {
        draw_texture_ex(
            texture,
            x.round(),
            y.round(),
            WHITE,
            DrawTextureParams::default(),
        );
    }
}

fn load_textures(
    scene: &Scene,
    root: &Path,
) -> Result<BTreeMap<String, Texture2D>, Box<dyn std::error::Error>> {
    let mut textures = BTreeMap::new();
    for (key, meta) in &scene.sprites {
        let path = root.join(&meta.file);
        let bytes = std::fs::read(&path)?;
        let image = image::load_from_memory(&bytes)?.to_rgba8();
        let (w, h) = image.dimensions();
        let texture = Texture2D::from_rgba8(u16::try_from(w)?, u16::try_from(h)?, image.as_raw());
        texture.set_filter(FilterMode::Nearest);
        textures.insert(key.clone(), texture);
    }
    Ok(textures)
}

fn load_font() -> Result<Font, Box<dyn std::error::Error>> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var("BOOM_FONT") {
        candidates.push(PathBuf::from(path));
    }
    candidates.extend(
        [
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            "C:/Windows/Fonts/segoeui.ttf",
            "C:/Windows/Fonts/arial.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
        ]
        .into_iter()
        .map(PathBuf::from),
    );
    for path in candidates {
        if let Ok(bytes) = std::fs::read(&path) {
            return Ok(load_ttf_font_from_bytes(&bytes)?);
        }
    }
    Err("không tìm thấy font TTF; đặt BOOM_FONT".into())
}

struct Audio {
    bgm: Sound,
    sfx: Sound,
    started: bool,
}

impl Audio {
    fn start(&self) {
        play_sound(
            &self.bgm,
            PlaySoundParams {
                looped: true,
                volume: 0.25,
            },
        );
    }
    fn sfx(&self) {
        play_sound(
            &self.sfx,
            PlaySoundParams {
                looped: false,
                volume: 0.7,
            },
        );
    }
}

async fn load_audio(scene: &Scene, root: &Path) -> Result<Audio, Box<dyn std::error::Error>> {
    let bgm = load_ogg("BGM", &root.join(&scene.audio.bgm)).await?;
    let sfx = load_ogg("SFX", &root.join(&scene.audio.sfx)).await?;
    Ok(Audio {
        bgm,
        sfx,
        started: false,
    })
}

async fn load_ogg(label: &str, path: &Path) -> Result<Sound, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("{label} {}: đọc lỗi: {error}", path.display()))?;
    if !bytes.starts_with(b"OggS") {
        return Err(format!("{label} {}: không phải Ogg", path.display()).into());
    }
    let sound = load_sound_from_bytes(&bytes)
        .await
        .map_err(|error| format!("{label} {}: decode lỗi: {error}", path.display()))?;
    println!(
        "audio {label}: load/decode OK ({} byte, {})",
        bytes.len(),
        path.display()
    );
    Ok(sound)
}

#[derive(Default)]
struct Options {
    assets: PathBuf,
    scale: u32,
    snapshot: Option<PathBuf>,
    time_ms: f64,
    dir: Option<String>,
    no_audio: bool,
    audio_smoke: bool,
}

impl Options {
    fn parse() -> Result<Self, String> {
        let mut opts = Options {
            assets: default_assets(),
            scale: 1,
            ..Default::default()
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--assets" => opts.assets = args.next().ok_or("--assets cần giá trị")?.into(),
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
                "--time-ms" => {
                    opts.time_ms = args
                        .next()
                        .ok_or("--time-ms cần số")?
                        .parse()
                        .map_err(|_| "--time-ms cần số")?
                }
                "--dir" => opts.dir = Some(args.next().ok_or("--dir cần down/left/right/up")?),
                "--no-audio" => opts.no_audio = true,
                "--audio-smoke" => opts.audio_smoke = true,
                "-h" | "--help" => {
                    println!(
                        "dùng: spike-macroquad [--assets DIR] [--scale 1|2] [--snapshot PNG] [--time-ms N] [--dir DIR] [--no-audio] [--audio-smoke]"
                    );
                    std::process::exit(0);
                }
                other => return Err(format!("tham số lạ: {other}")),
            }
        }
        if opts.scale != 1 && opts.scale != 2 {
            return Err("--scale chỉ nhận 1 hoặc 2".to_owned());
        }
        if let Some(dir) = &opts.dir
            && !matches!(dir.as_str(), "down" | "left" | "right" | "up")
        {
            return Err("--dir chỉ nhận down/left/right/up".to_owned());
        }
        if opts.audio_smoke && opts.no_audio {
            return Err("--audio-smoke và --no-audio không dùng cùng nhau".to_owned());
        }
        Ok(opts)
    }
}

fn default_assets() -> PathBuf {
    if let Ok(path) = std::env::var("BOOM_ASSET_DIR") {
        return PathBuf::from(path);
    }
    for candidate in [
        "local/asset-spike",
        "../../local/asset-spike",
        "../../../local/asset-spike",
    ] {
        let path = PathBuf::from(candidate);
        if path.join("scene.json").exists() {
            return path;
        }
    }
    PathBuf::from("local/asset-spike")
}

#[derive(Deserialize)]
struct Scene {
    tile_size: u32,
    logic_size: [u32; 2],
    map_origin: [u32; 2],
    background: [u8; 3],
    sprites: BTreeMap<String, SpriteMeta>,
    map: MapScene,
    characters: BTreeMap<String, Vec<String>>,
    bomb: Vec<String>,
    explosion: Vec<String>,
    character_cell: [u16; 2],
    character_dir: String,
    bomb_cell: [u16; 2],
    explosion_cell: [u16; 2],
    audio: AudioScene,
    text: String,
}

#[derive(Deserialize)]
struct SpriteMeta {
    file: String,
    w: u32,
    anchor: [i32; 2],
}

#[derive(Deserialize)]
struct MapScene {
    width: u16,
    height: u16,
    ground: Vec<String>,
    objects: Vec<ObjectScene>,
}

#[derive(Deserialize)]
struct ObjectScene {
    x: u16,
    y: u16,
    sprite: String,
}

#[derive(Deserialize)]
struct AudioScene {
    bgm: String,
    sfx: String,
}

impl Scene {
    fn load(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let text = std::fs::read_to_string(root.join("scene.json"))?;
        let scene: Self = serde_json::from_str(&text)?;
        if scene.map.ground.len() != usize::from(scene.map.width) * usize::from(scene.map.height) {
            return Err("scene.json: số ô ground không khớp kích thước map".into());
        }
        Ok(scene)
    }
}
