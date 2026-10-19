//! Giải mã asset KR đã chọn cho spike client.
//!
//! Đọc pack `.idx/.idd` bằng `boom-assets` (PR #28), giải mã sprite `CDataI16Alpha` v2,
//! cắt từng frame theo frame record 48 byte và ghi PNG + `scene.json`.
//! Không ghi asset vào repo: đầu ra mặc định nằm trong `/local/` (đã gitignore).

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use boom_assets::legacy_map::Build;
use boom_assets::pack::Pack;
use serde::{Deserialize, Serialize};

const TILE: u32 = 40;
const MAP_ORIGIN_Y: u32 = 40;
const HUD_H: u32 = 40;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut pack_dir = PathBuf::from("/home/admin/boom-re/boomKR");
    let mut out = PathBuf::from("local/asset-spike");
    let mut map_toml = PathBuf::from("data/maps/village00-kr.toml");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--pack-dir" => pack_dir = args.next().context("--pack-dir cần giá trị")?.into(),
            "--out" => out = args.next().context("--out cần giá trị")?.into(),
            "--map-toml" => map_toml = args.next().context("--map-toml cần giá trị")?.into(),
            "-h" | "--help" => {
                println!("dùng: prepare-assets [--pack-dir DIR] [--out DIR] [--map-toml FILE]");
                return Ok(());
            }
            other => bail!("tham số lạ: {other}"),
        }
    }

    fs::create_dir_all(out.join("sprites"))?;
    fs::create_dir_all(out.join("audio"))?;

    let map_def: MapDef = toml::from_str(
        &fs::read_to_string(&map_toml).with_context(|| map_toml.display().to_string())?,
    )
    .context("parse map TOML")?;

    let map_pack = Pack::open(&pack_dir, "FxMap", Build::Kr).context("mở FxMap")?;
    let tile_pack = Pack::open(&pack_dir, "FxTile", Build::Kr).context("mở FxTile")?;
    let char_pack = Pack::open(&pack_dir, "FxCharacter", Build::Kr).context("mở FxCharacter")?;
    let desktop_pack = Pack::open(&pack_dir, "FxDesktop", Build::Kr).context("mở FxDesktop")?;
    let sound_pack = Pack::open(&pack_dir, "FxSound", Build::Kr).context("mở FxSound")?;

    let map_bytes = map_pack
        .read_named("Map/Village00")
        .context("đọc FxMap/Map/Village00")?;
    let map = boom_assets::legacy_map::parse(&map_bytes, Build::Kr).context("parse Village00")?;
    if map.grid.width != 15 || map.grid.height != 13 {
        bail!(
            "Village00 là {}x{}, cần 15x13",
            map.grid.width,
            map.grid.height
        );
    }

    let mut catalog = Catalog {
        out: out.clone(),
        sprites: BTreeMap::new(),
    };
    let mut sprite_cache: HashMap<String, String> = HashMap::new();

    let mut ground = Vec::new();
    let mut objects = Vec::new();
    for (index, cell) in map.grid.cells.iter().enumerate() {
        let x = u16::try_from(index % usize::from(map.grid.width)).expect("x trong u16");
        let y = u16::try_from(index / usize::from(map.grid.width)).expect("y trong u16");
        let tile_id = cell
            .tile
            .with_context(|| format!("ô ({x},{y}) không có tile nền"))?;
        let tile_name = map_def
            .tiles
            .get(&tile_id.to_string())
            .with_context(|| format!("tile id {tile_id} chưa có trong {map_toml:?}"))?;
        let ground_entry = tile_entry(tile_name)?;
        let key = cache_entry(&mut sprite_cache, &ground_entry, &mut catalog, &tile_pack)?;
        ground.push(key);

        if let Some(obj_id) = cell.obj {
            let obj = map_def
                .objects
                .get(&obj_id.to_string())
                .with_context(|| format!("obj id {obj_id} chưa có trong {map_toml:?}"))?;
            let entry = tile_entry(&obj.name)?;
            let key = cache_entry(&mut sprite_cache, &entry, &mut catalog, &tile_pack)?;
            objects.push(ObjectScene { x, y, sprite: key });
        }
    }
    objects.sort_by_key(|o| (o.y, o.x));

    let mut characters = BTreeMap::new();
    for (dir, name) in [
        ("down", "Down"),
        ("left", "Left"),
        ("right", "Right"),
        ("up", "Up"),
    ] {
        let entry = format!("Character/C00-Walk{name}_ALayerStatic");
        let blob = char_pack
            .read_named(&entry)
            .with_context(|| format!("đọc FxCharacter/{entry}"))?;
        let keys = catalog.add_frames(&entry, &blob, usize::MAX)?;
        characters.insert(dir.to_owned(), keys);
    }

    for (pack, entry, label) in [
        (&desktop_pack, "Bomb/Bomb00", "Bomb/Bomb00"),
        (
            &desktop_pack,
            "Bomb/BombExplosion004",
            "Bomb/BombExplosion004",
        ),
    ] {
        let blob = pack
            .read_named(entry)
            .with_context(|| format!("đọc FxDesktop/{entry}"))?;
        catalog.add_frames(label, &blob, usize::MAX)?;
    }

    let bomb = catalog.keys_matching("Bomb/Bomb00#");
    let explosion = catalog.keys_matching("Bomb/BombExplosion004#");

    write_audio(&sound_pack, "MP3/Village", &out.join("audio/bgm.ogg"))?;
    write_audio(
        &sound_pack,
        "Wave/BombExplosion001",
        &out.join("audio/sfx.ogg"),
    )?;

    let scene = Scene {
        schema: 1,
        tile_size: TILE,
        logic_size: [15 * TILE, 13 * TILE + HUD_H],
        map_origin: [0, MAP_ORIGIN_Y],
        background: [16, 18, 24],
        hud_height: HUD_H,
        sprites: catalog.sprites,
        map: MapScene {
            width: map.grid.width,
            height: map.grid.height,
            ground,
            objects,
        },
        characters,
        bomb,
        explosion,
        character_cell: [1, 11],
        character_dir: "right".to_owned(),
        bomb_cell: [7, 6],
        explosion_cell: [9, 6],
        audio: AudioScene {
            bgm: "audio/bgm.ogg".to_owned(),
            sfx: "audio/sfx.ogg".to_owned(),
        },
        text: "Village00 · C00 đi 4 hướng · Bom nổ · Nhặt Bom, Lửa, Roller · 1×/2×".to_owned(),
    };
    fs::write(
        out.join("scene.json"),
        serde_json::to_string_pretty(&scene)?,
    )?;
    println!(
        "đã ghi {} sprite, {} object, {} frame nhân vật, {} frame bom/thổi vào {}",
        scene.sprites.len(),
        scene.map.objects.len(),
        scene.characters.values().map(Vec::len).sum::<usize>(),
        scene.bomb.len() + scene.explosion.len(),
        out.display()
    );
    Ok(())
}

fn cache_entry(
    cache: &mut HashMap<String, String>,
    entry: &str,
    catalog: &mut Catalog,
    pack: &Pack,
) -> Result<String> {
    if let Some(key) = cache.get(entry) {
        return Ok(key.clone());
    }
    let blob = pack
        .read_named(entry)
        .with_context(|| format!("đọc FxTile/{entry}"))?;
    let keys = catalog.add_frames(entry, &blob, 1)?;
    let key = keys.into_iter().next().context("sprite không có frame")?;
    cache.insert(entry.to_owned(), key.clone());
    Ok(key)
}

fn tile_entry(name: &str) -> Result<String> {
    let (prefix, suffix) = name
        .split_once('/')
        .with_context(|| format!("tên tile/object không có '/': {name}"))?;
    let theme = prefix
        .split('_')
        .next()
        .filter(|s| !s.is_empty())
        .with_context(|| format!("tên tile/object không có theme: {name}"))?;
    Ok(format!("{theme}/{suffix}"))
}

fn write_audio(pack: &Pack, entry: &str, path: &Path) -> Result<()> {
    let bytes = pack
        .read_named(entry)
        .with_context(|| format!("đọc FxSound/{entry}"))?;
    if !bytes.starts_with(b"OggS") {
        bail!(
            "FxSound/{entry} không phải Ogg (header {:02x?})",
            &bytes[..4.min(bytes.len())]
        );
    }
    fs::write(path, bytes).with_context(|| path.display().to_string())?;
    Ok(())
}

#[derive(Deserialize)]
struct MapDef {
    tiles: BTreeMap<String, String>,
    objects: BTreeMap<String, ObjectDef>,
}

#[derive(Deserialize)]
struct ObjectDef {
    name: String,
    #[allow(dead_code)]
    class: String,
}

#[derive(Serialize)]
struct Scene {
    schema: u32,
    tile_size: u32,
    logic_size: [u32; 2],
    map_origin: [u32; 2],
    background: [u8; 3],
    hud_height: u32,
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

#[derive(Serialize)]
struct MapScene {
    width: u16,
    height: u16,
    ground: Vec<String>,
    objects: Vec<ObjectScene>,
}

#[derive(Serialize)]
struct ObjectScene {
    x: u16,
    y: u16,
    sprite: String,
}

#[derive(Serialize)]
struct AudioScene {
    bgm: String,
    sfx: String,
}

#[derive(Serialize, Clone)]
struct SpriteMeta {
    file: String,
    w: u32,
    h: u32,
    anchor: [i32; 2],
}

struct Catalog {
    out: PathBuf,
    sprites: BTreeMap<String, SpriteMeta>,
}

impl Catalog {
    fn add_frames(&mut self, entry: &str, blob: &[u8], limit: usize) -> Result<Vec<String>> {
        let (images, frames) = decode_i16(blob).with_context(|| format!("giải mã I16 {entry}"))?;
        let mut keys = Vec::new();
        for (index, frame) in frames.iter().take(limit).enumerate() {
            let image = images
                .get(frame.image)
                .with_context(|| format!("{entry}: frame {index} trỏ ảnh {}", frame.image))?;
            let (mut rgba, width, height) =
                crop(image, frame).with_context(|| format!("{entry}: cắt frame {index}"))?;
            // Sheet nổ gốc dùng additive/black-background; chuyển sang alpha theo độ sáng
            // để hai stack so sánh trên cùng dữ liệu (macroquad không có blend per-draw đa nền tảng).
            if entry.contains("BombExplosion") {
                additive_to_alpha(&mut rgba);
            }
            let key = format!("{entry}#{index}");
            let salt = slug(&key);
            let file = format!("sprites/{salt}.png");
            write_png(&self.out.join(&file), width, height, &rgba)
                .with_context(|| format!("ghi {file}"))?;
            self.sprites.insert(
                key.clone(),
                SpriteMeta {
                    file,
                    w: width,
                    h: height,
                    anchor: [frame.ax, frame.ay],
                },
            );
            keys.push(key);
        }
        if keys.is_empty() {
            bail!("{entry}: không có frame nào");
        }
        Ok(keys)
    }

    fn keys_matching(&self, prefix: &str) -> Vec<String> {
        self.sprites
            .keys()
            .filter(|key| key.starts_with(prefix))
            .cloned()
            .collect()
    }
}

fn additive_to_alpha(rgba: &mut [u8]) {
    for pixel in rgba.as_chunks_mut::<4>().0 {
        if pixel[3] == 0 {
            continue;
        }
        pixel[3] = pixel[0].max(pixel[1]).max(pixel[2]);
    }
}

fn slug(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut last = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last = false;
        } else if !last {
            out.push('_');
            last = true;
        }
    }
    out.trim_matches('_').to_owned()
}

fn crop(image: &Image, frame: &Frame) -> Result<(Vec<u8>, u32, u32)> {
    let sx = usize::try_from(frame.sx.max(0)).expect("sx");
    let sy = usize::try_from(frame.sy.max(0)).expect("sy");
    let ex = usize::try_from(frame.ex.max(0)).expect("ex");
    let ey = usize::try_from(frame.ey.max(0)).expect("ey");
    if ex <= sx || ey <= sy {
        bail!("rect rỗng ({sx},{sy})..({ex},{ey})");
    }
    let ex = ex.min(image.w);
    let ey = ey.min(image.h);
    if ex <= sx || ey <= sy {
        bail!("rect ngoài ảnh {}x{}", image.w, image.h);
    }
    let width = ex - sx;
    let height = ey - sy;
    let mut rgba = vec![0u8; width * height * 4];
    for row in 0..height {
        let src = ((sy + row) * image.w + sx) * 4;
        let dst = row * width * 4;
        rgba[dst..dst + width * 4].copy_from_slice(&image.rgba[src..src + width * 4]);
    }
    Ok((rgba, u32::try_from(width)?, u32::try_from(height)?))
}

struct Image {
    w: usize,
    h: usize,
    rgba: Vec<u8>,
}

struct Frame {
    image: usize,
    sx: i32,
    sy: i32,
    ex: i32,
    ey: i32,
    ax: i32,
    ay: i32,
}

fn decode_i16(blob: &[u8]) -> Result<(Vec<Image>, Vec<Frame>)> {
    let nframes = usize::try_from(read_u32(blob, 0)?)?;
    let version = read_u32(blob, 4)?;
    let nimages = usize::try_from(read_u32(blob, 8)?)?;
    if version != 2 {
        bail!("I16 version {version}, cần 2");
    }
    let mut sizes = Vec::with_capacity(nimages);
    for index in 0..nimages {
        let at = 16 + index * 8;
        sizes.push((
            read_u32(blob, at)? as usize,
            read_u32(blob, at + 4)? as usize,
        ));
    }
    let mut frame_at = 16 + nimages * 8;
    let mut frames = Vec::with_capacity(nframes);
    for _ in 0..nframes {
        let mut values = [0i32; 12];
        for (index, value) in values.iter_mut().enumerate() {
            *value = read_u32(blob, frame_at + index * 4)? as i32;
        }
        frame_at += 48;
        frames.push(Frame {
            image: usize::try_from(values[3])
                .with_context(|| format!("chỉ số ảnh âm: {}", values[3]))?,
            sx: values[4],
            sy: values[5],
            ex: values[6],
            ey: values[7],
            ax: values[0],
            ay: values[1],
        });
    }
    let mut images = Vec::with_capacity(nimages);
    let mut image_at = frame_at;
    for (index, &(h, w)) in sizes.iter().enumerate() {
        let len = usize::try_from(read_u32(blob, image_at)?)?;
        image_at += 4;
        let data = blob
            .get(image_at..image_at + len)
            .with_context(|| format!("ảnh {index} vượt blob"))?;
        image_at += len;
        images.push(decode_image(data, w, h).with_context(|| format!("ảnh {index}"))?);
    }
    Ok((images, frames))
}

fn decode_image(data: &[u8], w: usize, h: usize) -> Result<Image> {
    let mut rgba = vec![0u8; w * h * 4];
    if w == 0 || h == 0 {
        return Ok(Image { w, h, rgba });
    }
    let first = *data.first().context("ảnh rỗng")?;
    let (mut p, bpp, end, alpha_at) = if first & 0xC1 == 0xC0 {
        (1, 3, data.len(), None)
    } else if first & 0xC1 == 0xC1 {
        let alpha = usize::try_from(read_u32(data, 1)?)?;
        if alpha > data.len() {
            bail!("alpha offset {alpha} vượt ảnh {} byte", data.len());
        }
        (5, 3, alpha, Some(alpha))
    } else {
        (0, 2, data.len(), None)
    };
    if end > data.len() {
        bail!("color stream kết thúc {end} > {}", data.len());
    }
    let mut alpha = alpha_at.map(|at| AlphaReader {
        data,
        p: at,
        ctrl: 0,
        left: 0,
        value: 0,
    });
    let mut left = 0u32;
    let mut ctrl = 0u8;
    let mut color: Option<[u8; 3]> = None;
    for index in 0..w * h {
        if left == 0 {
            ctrl = *data.get(p).context("hết color stream")?;
            p += 1;
            left = u32::from(ctrl & 0x3F);
            if left == 0 {
                bail!("run 0 ở pixel {index}");
            }
            if ctrl & 0x80 != 0 {
                color = None;
            } else {
                color = Some(read_color(data, &mut p, bpp, end)?);
            }
        } else if ctrl & 0xC0 == 0 {
            color = Some(read_color(data, &mut p, bpp, end)?);
        }
        left -= 1;
        let a = match alpha.as_mut() {
            Some(reader) => 255 - reader.next()?,
            None => 255,
        };
        let Some(color) = color else {
            continue;
        };
        let (r, g, b) = match bpp {
            3 => (color[2], color[1], color[0]),
            2 => {
                let value = u16::from(color[0]) | (u16::from(color[1]) << 8);
                if value == 0xF81F {
                    continue;
                }
                rgb565(value)
            }
            _ => unreachable!(),
        };
        let at = index * 4;
        rgba[at] = r;
        rgba[at + 1] = g;
        rgba[at + 2] = b;
        rgba[at + 3] = a;
    }
    Ok(Image { w, h, rgba })
}

fn read_color(data: &[u8], p: &mut usize, bpp: usize, end: usize) -> Result<[u8; 3]> {
    let slice = data
        .get(*p..*p + bpp)
        .filter(|_| *p + bpp <= end)
        .context("hết color stream khi đọc pixel")?;
    *p += bpp;
    Ok(match bpp {
        2 => [slice[0], slice[1], 0],
        3 => [slice[0], slice[1], slice[2]],
        _ => bail!("bpp {bpp}"),
    })
}

fn rgb565(value: u16) -> (u8, u8, u8) {
    let r = u32::from((value >> 11) & 0x1F) * 255 / 31;
    let g = u32::from((value >> 5) & 0x3F) * 255 / 63;
    let b = u32::from(value & 0x1F) * 255 / 31;
    (
        u8::try_from(r).expect("r"),
        u8::try_from(g).expect("g"),
        u8::try_from(b).expect("b"),
    )
}

struct AlphaReader<'a> {
    data: &'a [u8],
    p: usize,
    ctrl: u8,
    left: u32,
    value: u8,
}

impl AlphaReader<'_> {
    fn next(&mut self) -> Result<u8> {
        if self.left > 0 {
            if self.ctrl & 0xC0 == 0 {
                self.value = *self.data.get(self.p).context("hết alpha stream")?;
                self.p += 1;
            }
            self.left -= 1;
            return Ok(self.value);
        }
        let ctrl = *self.data.get(self.p).context("hết alpha stream")?;
        self.p += 1;
        self.ctrl = ctrl;
        self.left = u32::from(ctrl & 0x3F);
        if self.left == 0 {
            self.value = 0;
            return Ok(0);
        }
        if ctrl & 0x80 != 0 {
            self.value = 0;
        } else {
            self.value = *self.data.get(self.p).context("hết alpha stream")?;
            self.p += 1;
        }
        self.left -= 1;
        Ok(self.value)
    }
}

fn read_u32(data: &[u8], at: usize) -> Result<u32> {
    let bytes = data
        .get(at..at + 4)
        .with_context(|| format!("thiếu u32 tại byte {at}"))?;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("đã kiểm 4 byte"),
    ))
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    Ok(())
}
