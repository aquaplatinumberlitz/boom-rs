//! Spike client stack: winit + softbuffer + fontdue + rodio.
//!
//! Renderer/UI tự dựng trên CPU để đo chi phí kiểm soát pixel so với framework.
//! Cùng scene.json/asset với spike macroquad; chạy `--snapshot` để xuất PNG xác định.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use fontdue::Font;
use quad_snd::{AudioContext, PlaySoundParams, Sound};
use serde::Deserialize;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

const HUD: u32 = 40;

fn main() -> ExitCode {
    let opts = match Options::parse() {
        Ok(opts) => opts,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    match run(opts) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(opts: Options) -> Result<(), Box<dyn std::error::Error>> {
    let scene = Scene::load(&opts.assets)?;
    if opts.audio_smoke {
        Audio::new(&scene, &opts.assets, false)?;
        println!(
            "audio smoke PASS: BGM+SFX load/decode OK; playback/timing chưa đo (không có soundcard)"
        );
        return Ok(());
    }
    let sprites = load_sprites(&scene, &opts.assets)?;
    let font_bytes = load_font_bytes()?;
    let font = Font::from_bytes(font_bytes, fontdue::FontSettings::default())
        .map_err(|error| format!("font: {error:?}"))?;

    if let Some(frames) = opts.bench {
        let start = Instant::now();
        for index in 0..frames {
            let frame = render_scene(&scene, &sprites, &font, f64::from(index) / 60.0, 0.0, &opts);
            let _ = scale_frame(&frame, opts.scale);
        }
        let elapsed = start.elapsed().as_secs_f64();
        println!(
            "bench {frames} frames: {:.3} s, {:.3} ms/frame",
            elapsed,
            elapsed * 1000.0 / f64::from(frames)
        );
        return Ok(());
    }

    if let Some(path) = &opts.snapshot {
        let frame = render_scene(&scene, &sprites, &font, opts.time_ms / 1000.0, 0.0, &opts);
        let frame = scale_frame(&frame, opts.scale);
        let (width, height) = (frame.width(), frame.height());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        frame.save(path)?;
        println!(
            "snapshot {} ({width}x{height}, scale {})",
            path.display(),
            opts.scale
        );
        return Ok(());
    }

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        scene,
        sprites,
        font,
        opts,
        window: None,
        context: None,
        surface: None,
        audio: None,
        started: Instant::now(),
        explosion_at: f64::NEG_INFINITY,
        scale: 1,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct App {
    scene: Scene,
    sprites: BTreeMap<String, Sprite>,
    font: Font,
    opts: Options,
    window: Option<Arc<Window>>,
    context: Option<softbuffer::Context<Arc<Window>>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    audio: Option<Audio>,
    started: Instant,
    explosion_at: f64,
    scale: u32,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        self.scale = self.opts.scale;
        let width = self.scene.logic_size[0] * self.scale;
        let height = self.scene.logic_size[1] * self.scale;
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("boom-rs spike winit+softbuffer")
                .with_inner_size(LogicalSize::new(width, height))
                .with_resizable(false),
        ) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("không tạo được cửa sổ: {error}");
                event_loop.exit();
                return;
            }
        };
        let context = match softbuffer::Context::new(window.clone()) {
            Ok(context) => context,
            Err(error) => {
                eprintln!("softbuffer context: {error}");
                event_loop.exit();
                return;
            }
        };
        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(surface) => surface,
            Err(error) => {
                eprintln!("softbuffer surface: {error}");
                event_loop.exit();
                return;
            }
        };
        if !self.opts.no_audio {
            match Audio::new(&self.scene, &self.opts.assets, true) {
                Ok(audio) => self.audio = Some(audio),
                Err(error) => {
                    eprintln!("không nạp được audio: {error}");
                    std::process::exit(2);
                }
            }
        }
        self.context = Some(context);
        self.surface = Some(surface);
        self.window = Some(window);
        self.started = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
                    PhysicalKey::Code(KeyCode::KeyS) => {
                        self.scale = if self.scale == 1 { 2 } else { 1 };
                        if let Some(window) = &self.window {
                            let _ = window.request_inner_size(LogicalSize::new(
                                self.scene.logic_size[0] * self.scale,
                                self.scene.logic_size[1] * self.scale,
                            ));
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyB) => {
                        self.explosion_at = self.started.elapsed().as_secs_f64();
                        if let Some(audio) = &self.audio {
                            audio.sfx();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyR) => {
                        self.explosion_at = f64::NEG_INFINITY;
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(window) = self.window.clone() else {
                    return;
                };
                let size = window.inner_size();
                let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                else {
                    return;
                };
                let Some(surface) = self.surface.as_mut() else {
                    return;
                };
                if let Err(error) = surface.resize(width, height) {
                    eprintln!("resize: {error}");
                    return;
                }
                let now = self.started.elapsed().as_secs_f64();
                let frame = render_scene(
                    &self.scene,
                    &self.sprites,
                    &self.font,
                    now,
                    self.explosion_at,
                    &self.opts,
                );
                let scaled = scale_frame(&frame, self.scale);
                let mut buffer = match surface.buffer_mut() {
                    Ok(buffer) => buffer,
                    Err(error) => {
                        eprintln!("buffer: {error}");
                        return;
                    }
                };
                let pixels = scaled.as_raw();
                let count = buffer.len().min(pixels.len() / 4);
                for (dst, src) in buffer.iter_mut().take(count).zip(pixels.as_chunks::<4>().0) {
                    *dst = u32::from(src[0]) << 16 | u32::from(src[1]) << 8 | u32::from(src[2]);
                }
                if let Err(error) = buffer.present() {
                    eprintln!("present: {error}");
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

struct Audio {
    ctx: AudioContext,
    _bgm: Sound,
    sfx: Sound,
}

impl Audio {
    fn new(scene: &Scene, root: &Path, play_bgm: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let bgm_path = root.join(&scene.audio.bgm);
        let sfx_path = root.join(&scene.audio.sfx);
        let bgm_bytes = read_ogg("BGM", &bgm_path)?;
        let sfx_bytes = read_ogg("SFX", &sfx_path)?;
        let ctx = AudioContext::new();
        // Sound::load giải mã thật; chỉ log OK sau khi nó trả về.
        let bgm = Sound::load(&ctx, &bgm_bytes);
        println!(
            "audio BGM: load/decode OK ({} byte, {})",
            bgm_bytes.len(),
            bgm_path.display()
        );
        if play_bgm {
            bgm.play(
                &ctx,
                PlaySoundParams {
                    looped: true,
                    volume: 0.25,
                },
            );
        }
        let sfx = Sound::load(&ctx, &sfx_bytes);
        println!(
            "audio SFX: load/decode OK ({} byte, {})",
            sfx_bytes.len(),
            sfx_path.display()
        );
        Ok(Self {
            ctx,
            _bgm: bgm,
            sfx,
        })
    }

    fn sfx(&self) {
        self.sfx.play(
            &self.ctx,
            PlaySoundParams {
                looped: false,
                volume: 0.7,
            },
        );
    }
}

fn read_ogg(label: &str, path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("{label} {}: đọc lỗi: {error}", path.display()))?;
    if !bytes.starts_with(b"OggS") {
        return Err(format!("{label} {}: không phải Ogg", path.display()).into());
    }
    Ok(bytes)
}

fn render_scene(
    scene: &Scene,
    sprites: &BTreeMap<String, Sprite>,
    font: &Font,
    now: f64,
    explosion_at: f64,
    opts: &Options,
) -> image::RgbaImage {
    let (width, height) = (scene.logic_size[0], scene.logic_size[1]);
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    for pixel in buffer.as_chunks_mut::<4>().0 {
        pixel.copy_from_slice(&[
            scene.background[0],
            scene.background[1],
            scene.background[2],
            255,
        ]);
    }

    let map_x = scene.map_origin[0] as i32;
    let map_y = scene.map_origin[1] as i32;
    let tile = scene.tile_size as i32;
    let map_width = scene.map.width as usize;

    for (index, key) in scene.map.ground.iter().enumerate() {
        let x = i32::try_from(index % map_width).expect("x") * tile + map_x;
        let y = i32::try_from(index / map_width).expect("y") * tile + map_y;
        if let Some(sprite) = sprites.get(key) {
            blit(&mut buffer, width, height, sprite, x, y);
        }
    }

    let character = character_key(scene, opts, now);
    let bomb = scene
        .bomb
        .get((now * 8.0) as usize % scene.bomb.len().max(1));
    let explosion_frame = scene
        .explosion
        .get(((now - explosion_at) * 14.0).max(0.0) as usize % scene.explosion.len().max(1));
    let explosion_on = now >= explosion_at && now - explosion_at < 1.4;

    let mut draws: Vec<(i32, DrawKind)> = scene
        .map
        .objects
        .iter()
        .map(|object| (i32::from(object.y) * tile + tile, DrawKind::Object(object)))
        .collect();
    draws.push((
        i32::from(scene.character_cell[1]) * tile + tile,
        DrawKind::Character(character),
    ));
    if let Some(key) = bomb {
        draws.push((
            i32::from(scene.bomb_cell[1]) * tile + tile,
            DrawKind::Bomb(key),
        ));
    }
    if explosion_on && let Some(key) = explosion_frame {
        draws.push((
            i32::from(scene.explosion_cell[1]) * tile + tile,
            DrawKind::Explosion(key),
        ));
    }
    draws.sort_by_key(|(sort, _)| *sort);

    for (_, kind) in draws {
        match kind {
            DrawKind::Object(object) => {
                let meta = &scene.sprites[object.sprite.as_str()];
                let x = map_x + i32::from(object.x) * tile - meta.anchor[0];
                let y = map_y + i32::from(object.y) * tile + tile - (meta.anchor[1] + 1);
                if let Some(sprite) = sprites.get(&object.sprite) {
                    blit(&mut buffer, width, height, sprite, x, y);
                }
            }
            DrawKind::Character(key) => {
                let meta = &scene.sprites[key];
                let x = map_x + i32::from(scene.character_cell[0]) * tile + tile / 2
                    - i32::try_from(meta.w).expect("w") / 2;
                let y =
                    map_y + i32::from(scene.character_cell[1]) * tile + tile - (meta.anchor[1] + 1);
                if let Some(sprite) = sprites.get(key) {
                    blit(&mut buffer, width, height, sprite, x, y);
                }
            }
            DrawKind::Bomb(key) => {
                let meta = &scene.sprites[key];
                let x = map_x + i32::from(scene.bomb_cell[0]) * tile + tile / 2
                    - i32::try_from(meta.w).expect("w") / 2;
                let y = map_y + i32::from(scene.bomb_cell[1]) * tile + tile - (meta.anchor[1] + 1);
                if let Some(sprite) = sprites.get(key) {
                    blit(&mut buffer, width, height, sprite, x, y);
                }
            }
            DrawKind::Explosion(key) => {
                let meta = &scene.sprites[key];
                let x = map_x + i32::from(scene.explosion_cell[0]) * tile + tile / 2
                    - i32::try_from(meta.w).expect("w") / 2;
                let y =
                    map_y + i32::from(scene.explosion_cell[1]) * tile + tile - (meta.anchor[1] + 1);
                if let Some(sprite) = sprites.get(key) {
                    blit(&mut buffer, width, height, sprite, x, y);
                }
            }
        }
    }

    fill_rect(
        &mut buffer,
        width,
        height,
        0,
        0,
        width,
        HUD,
        [10, 12, 17, 255],
    );
    fill_rect(
        &mut buffer,
        width,
        height,
        0,
        HUD.saturating_sub(1),
        width,
        1,
        [63, 114, 140, 255],
    );
    draw_text(&mut buffer, width, height, font, &scene.text, 10, 26);

    image::RgbaImage::from_raw(width, height, buffer).expect("frame RGBA")
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
    &frames[((now * 12.0) as usize) % frames.len().max(1)]
}

struct Sprite {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

fn load_sprites(
    scene: &Scene,
    root: &Path,
) -> Result<BTreeMap<String, Sprite>, Box<dyn std::error::Error>> {
    let mut sprites = BTreeMap::new();
    for (key, meta) in &scene.sprites {
        let bytes = std::fs::read(root.join(&meta.file))?;
        let image = image::load_from_memory(&bytes)?.to_rgba8();
        sprites.insert(
            key.clone(),
            Sprite {
                width: image.width(),
                height: image.height(),
                pixels: image.into_raw(),
            },
        );
    }
    Ok(sprites)
}

fn blit(buffer: &mut [u8], width: u32, height: u32, sprite: &Sprite, x: i32, y: i32) {
    for sy in 0..sprite.height {
        let dy = y + i32::try_from(sy).expect("sy");
        if dy < 0 || dy >= i32::try_from(height).expect("h") {
            continue;
        }
        for sx in 0..sprite.width {
            let dx = x + i32::try_from(sx).expect("sx");
            if dx < 0 || dx >= i32::try_from(width).expect("w") {
                continue;
            }
            let src = ((sy * sprite.width + sx) * 4) as usize;
            let a = u32::from(sprite.pixels[src + 3]);
            if a == 0 {
                continue;
            }
            let dst = ((u32::try_from(dy).expect("dy") * width + u32::try_from(dx).expect("dx"))
                * 4) as usize;
            if a == 255 {
                buffer[dst..dst + 3].copy_from_slice(&sprite.pixels[src..src + 3]);
            } else {
                let inv = 255 - a;
                for channel in 0..3 {
                    buffer[dst + channel] = u8::try_from(
                        (u32::from(sprite.pixels[src + channel]) * a
                            + u32::from(buffer[dst + channel]) * inv
                            + 127)
                            / 255,
                    )
                    .expect("blend");
                }
            }
            buffer[dst + 3] = 255;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_rect(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    color: [u8; 4],
) {
    for row in y..y.saturating_add(h).min(height) {
        for col in x..x.saturating_add(w).min(width) {
            let at = ((row * width + col) * 4) as usize;
            buffer[at..at + 4].copy_from_slice(&color);
        }
    }
}

fn draw_text(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    font: &Font,
    text: &str,
    x: i32,
    baseline: i32,
) {
    let size = 14.0;
    let mut pen = x as f32;
    let mut last: Option<char> = None;
    for ch in text.chars() {
        if let Some(left) = last {
            pen += font.horizontal_kern(left, ch, size).unwrap_or(0.0);
        }
        let (metrics, bitmap) = font.rasterize(ch, size);
        let gx = pen.round() as i32 + metrics.xmin;
        let gy = baseline - metrics.ymin - i32::try_from(metrics.height).expect("height");
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let coverage = u32::from(bitmap[row * metrics.width + col]);
                if coverage == 0 {
                    continue;
                }
                let px = gx + i32::try_from(col).expect("col");
                let py = gy + i32::try_from(row).expect("row");
                if px < 0 || py < 0 {
                    continue;
                }
                let (Ok(px), Ok(py)) = (u32::try_from(px), u32::try_from(py)) else {
                    continue;
                };
                if px >= width || py >= height {
                    continue;
                }
                let at = ((py * width + px) * 4) as usize;
                let alpha = coverage;
                let inv = 255 - alpha;
                for (channel, value) in [235u8, 245, 255].into_iter().enumerate() {
                    buffer[at + channel] = u8::try_from(
                        (u32::from(value) * alpha + u32::from(buffer[at + channel]) * inv + 127)
                            / 255,
                    )
                    .expect("text blend");
                }
                buffer[at + 3] = 255;
            }
        }
        pen += metrics.advance_width;
        last = Some(ch);
    }
}

fn scale_frame(frame: &image::RgbaImage, scale: u32) -> image::RgbaImage {
    if scale == 1 {
        return frame.clone();
    }
    let width = frame.width() * scale;
    let height = frame.height() * scale;
    let mut out = vec![0u8; (width * height * 4) as usize];
    let src = frame.as_raw();
    for y in 0..height {
        for x in 0..width {
            let sx = x / scale;
            let sy = y / scale;
            let from = ((sy * frame.width() + sx) * 4) as usize;
            let to = ((y * width + x) * 4) as usize;
            out[to..to + 4].copy_from_slice(&src[from..from + 4]);
        }
    }
    image::RgbaImage::from_raw(width, height, out).expect("scale RGBA")
}

fn load_font_bytes() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var("BOOM_FONT") {
        candidates.push(PathBuf::from(path));
    }
    candidates.extend(
        [
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
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
            return Ok(bytes);
        }
    }
    Err("không tìm thấy font TTF; đặt BOOM_FONT".into())
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
    bench: Option<u32>,
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
                    opts.snapshot = Some(args.next().ok_or("--snapshot cần đường dẫn")?.into());
                }
                "--time-ms" => {
                    opts.time_ms = args
                        .next()
                        .ok_or("--time-ms cần số")?
                        .parse()
                        .map_err(|_| "--time-ms cần số")?;
                }
                "--dir" => opts.dir = Some(args.next().ok_or("--dir cần down/left/right/up")?),
                "--no-audio" => opts.no_audio = true,
                "--audio-smoke" => opts.audio_smoke = true,
                "--bench" => {
                    opts.bench = Some(
                        args.next()
                            .ok_or("--bench cần số frame")?
                            .parse()
                            .map_err(|_| "--bench cần số frame")?,
                    );
                }
                "-h" | "--help" => {
                    println!(
                        "dùng: spike-winit-softbuffer [--assets DIR] [--scale 1|2] [--snapshot PNG] [--time-ms N] [--dir DIR] [--no-audio] [--audio-smoke] [--bench N]"
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
