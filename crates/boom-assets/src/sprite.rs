//! Giải mã sprite `CDataI16Alpha` v2 của client gốc.
//!
//! Định dạng: [PACKS §Sprite](../../docs/specification/assets/PACKS.md) và
//! [MAPS §3](../../docs/specification/assets/MAPS.md) (frame 48 byte). Viết lại từ mô tả
//! format và tool tham chiếu `boom-re-l10n/tools/i16.py`; không copy code legacy.
//!
//! Cấu trúc blob:
//!
//! ```text
//! u32 n_frames
//! u32 version (=2)
//! u32 n_images
//! u32 unk
//! n_images × (u32 height, u32 width)
//! n_frames × 48 byte                 (12 × i32)
//! n_images × (u32 len, RLE)
//! ```
//!
//! RLE mỗi ảnh:
//!
//! - byte đầu `& 0xC1 == 0xC0` → màu 24-bit BGR, không có luồng alpha;
//! - byte đầu `& 0xC1 == 0xC1` → màu 24-bit BGR, luồng alpha bắt đầu tại `u32` ngay sau
//!   byte đầu (offset tính từ đầu ảnh), phần màu bắt đầu ở byte 5;
//! - còn lại → màu 16-bit RGB565, không có luồng alpha, phần màu bắt đầu ngay byte 0.
//!
//! Control byte: `run = ctrl & 0x3F`; bit7 = run trong suốt; bit6 = lặp lại một màu cho cả
//! run; còn lại = run pixel liên tiếp, mỗi pixel một màu. Luồng alpha dùng cùng dạng
//! control nhưng giá trị là **mức trong suốt** (alpha thật = `255 - giá trị`).
//!
//! Nghĩa field frame chưa chốt hết (MAPS §3): field giữ nguyên số, tên trong code theo
//! tài liệu, không suy diễn thêm.

use std::fmt;

/// Version duy nhất của `CDataI16Alpha` mà bộ giải mã này hiểu.
pub const SPRITE_VERSION: u32 = 2;

/// Màu 16-bit RGB565 bị coi là trong suốt (khóa màu của client).
pub const TRANSPARENT_RGB565: u16 = 0xF81F;

/// Số byte của một frame (12 × i32).
const FRAME_BYTES: usize = 48;

/// Trần cạnh ảnh (giới hạn kỹ thuật, không phải luật chơi). Ảnh lớn nhất trong dữ liệu KR
/// đã quét là 1198×1024 (`FxDesktop/LobbySquare/MapObject_Img_BigTreeLeaf`); trần này chặn
/// blob hỏng gây cấp phát bộ nhớ vô hạn.
pub const MAX_IMAGE_SIDE: u32 = 4096;

/// Kích thước ảnh theo pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// Hình chữ nhật nguồn trong ảnh, theo thứ tự `(x0, y0, x1, y1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

/// Điểm pivot của frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// Một frame animation (48 byte = 12 × i32).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    /// `f0` — offset đăng ký khung hình theo trục X (MAPS §3; tin cậy trung bình, có thể âm).
    pub offset_x: i32,
    /// `f1` — offset đăng ký khung hình theo trục Y (MAPS §3; tin cậy trung bình).
    pub offset_y: i32,
    /// `f2` — flag/lớp; nghĩa chưa chốt (MAPS §3; tin cậy thấp).
    pub flag: i32,
    /// `f3` — chỉ số ảnh trong blob (MAPS §3; tin cậy cao).
    pub image_index: i32,
    /// `f4..f7` — rect nguồn `(x0, y0, x1, y1)` trong ảnh (MAPS §3; tin cậy cao).
    pub source: Rect,
    /// `f8, f9` — kích thước canvas `(width, height)` (MAPS §3; tin cậy trung bình).
    pub canvas: Size,
    /// `f10, f11` — pivot `(x, y)` (MAPS §3; tin cậy trung bình).
    pub pivot: Point,
}

/// Một ảnh trong blob: kích thước + dữ liệu RLE chưa giải.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    data: Vec<u8>,
}

impl Image {
    /// Dữ liệu RLE thô của ảnh.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// Blob sprite đã parse: frame + ảnh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sprite {
    /// Frame theo thứ tự blob.
    pub frames: Vec<Frame>,
    /// Ảnh theo thứ tự blob; `Frame::image_index` trỏ vào đây.
    pub images: Vec<Image>,
    /// Dword thứ tư của header; chưa biết ý nghĩa, giữ nguyên số.
    pub unk: u32,
}

impl Sprite {
    /// Ảnh theo chỉ số.
    #[must_use]
    pub fn image(&self, index: usize) -> Option<&Image> {
        self.images.get(index)
    }

    /// Giải ảnh theo chỉ số ra RGBA (4 byte mỗi pixel, thứ tự R, G, B, A).
    pub fn decode_image(&self, index: usize) -> Result<Vec<u8>, SpriteError> {
        let image = self.images.get(index).ok_or(SpriteError::ImageIndex {
            index: i32::try_from(index).unwrap_or(i32::MAX),
            count: self.images.len(),
        })?;
        decode_image(&image.data, image.width, image.height)
    }
}

/// Lỗi khi parse blob sprite hoặc giải RLE.
#[derive(Debug, PartialEq, Eq)]
pub enum SpriteError {
    /// Blob cụt.
    Truncated { at: usize, need: usize },
    /// Version khác [`SPRITE_VERSION`].
    Version { version: u32 },
    /// Header khai báo nhiều phần tử hơn blob còn chỗ.
    Count {
        what: &'static str,
        count: u32,
        remaining: usize,
    },
    /// Chỉ số ảnh nằm ngoài danh sách ảnh.
    ImageIndex { index: i32, count: usize },
    /// Offset luồng alpha không hợp lệ.
    AlphaOffset { offset: u32, len: usize },
    /// Kích thước ảnh không nhân được thành số pixel.
    SizeOverflow { width: u32, height: u32 },
    /// Kích thước ảnh vượt [`MAX_IMAGE_SIDE`].
    ImageTooLarge { width: u32, height: u32 },
}

impl fmt::Display for SpriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { at, need } => {
                write!(f, "blob sprite cụt tại byte {at}, cần thêm {need} byte")
            }
            Self::Version { version } => {
                write!(
                    f,
                    "version sprite {version} không hỗ trợ (cần {SPRITE_VERSION})"
                )
            }
            Self::Count {
                what,
                count,
                remaining,
            } => write!(
                f,
                "header khai báo {count} {what} nhưng chỉ còn {remaining} byte"
            ),
            Self::ImageIndex { index, count } => {
                write!(f, "chỉ số ảnh {index} nằm ngoài {count} ảnh")
            }
            Self::AlphaOffset { offset, len } => write!(
                f,
                "offset luồng alpha {offset} không hợp lệ với ảnh {len} byte"
            ),
            Self::SizeOverflow { width, height } => {
                write!(f, "kích thước ảnh {width}x{height} quá lớn")
            }
            Self::ImageTooLarge { width, height } => write!(
                f,
                "kích thước ảnh {width}x{height} vượt trần {MAX_IMAGE_SIDE}"
            ),
        }
    }
}

impl std::error::Error for SpriteError {}

/// Parse blob `CDataI16Alpha` v2.
pub fn parse(blob: &[u8]) -> Result<Sprite, SpriteError> {
    let mut cursor = Cursor::new(blob);
    let n_frames = cursor.u32()?;
    let version = cursor.u32()?;
    let n_images = cursor.u32()?;
    let unk = cursor.u32()?;
    if version != SPRITE_VERSION {
        return Err(SpriteError::Version { version });
    }

    let sizes_bytes = (n_images as usize).checked_mul(8);
    if sizes_bytes.is_none_or(|bytes| bytes > cursor.remaining()) {
        return Err(SpriteError::Count {
            what: "ảnh",
            count: n_images,
            remaining: cursor.remaining(),
        });
    }
    let mut sizes = Vec::with_capacity(n_images as usize);
    for _ in 0..n_images {
        let height = cursor.u32()?;
        let width = cursor.u32()?;
        sizes.push(Size { width, height });
    }

    let frames_bytes = (n_frames as usize).checked_mul(FRAME_BYTES);
    if frames_bytes.is_none_or(|bytes| bytes > cursor.remaining()) {
        return Err(SpriteError::Count {
            what: "frame",
            count: n_frames,
            remaining: cursor.remaining(),
        });
    }
    let mut frames = Vec::with_capacity(n_frames as usize);
    for _ in 0..n_frames {
        frames.push(Frame {
            offset_x: cursor.i32()?,
            offset_y: cursor.i32()?,
            flag: cursor.i32()?,
            image_index: cursor.i32()?,
            source: Rect {
                x0: cursor.i32()?,
                y0: cursor.i32()?,
                x1: cursor.i32()?,
                y1: cursor.i32()?,
            },
            canvas: Size {
                width: cursor.u32()?,
                height: cursor.u32()?,
            },
            pivot: Point {
                x: cursor.i32()?,
                y: cursor.i32()?,
            },
        });
    }

    let mut images = Vec::with_capacity(n_images as usize);
    for size in sizes {
        let len = cursor.u32()? as usize;
        let data = cursor.take(len)?.to_vec();
        images.push(Image {
            width: size.width,
            height: size.height,
            data,
        });
    }

    Ok(Sprite {
        frames,
        images,
        unk,
    })
}

/// Giải một ảnh RLE ra RGBA (4 byte mỗi pixel, thứ tự R, G, B, A).
///
/// Pixel trong suốt để nguyên `(0, 0, 0, 0)`: run trong suốt của luồng màu và màu khóa
/// `TRANSPARENT_RGB565` đều không ghi đè pixel.
pub fn decode_image(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, SpriteError> {
    if width > MAX_IMAGE_SIDE || height > MAX_IMAGE_SIDE {
        return Err(SpriteError::ImageTooLarge { width, height });
    }
    let w = usize::try_from(width).map_err(|_| SpriteError::SizeOverflow { width, height })?;
    let h = usize::try_from(height).map_err(|_| SpriteError::SizeOverflow { width, height })?;
    let count = w
        .checked_mul(h)
        .ok_or(SpriteError::SizeOverflow { width, height })?;
    let mut out = vec![
        0u8;
        count
            .checked_mul(4)
            .ok_or(SpriteError::SizeOverflow { width, height })?
    ];

    let first = *data
        .first()
        .ok_or(SpriteError::Truncated { at: 0, need: 1 })?;
    let mut cursor = Cursor::new(data);
    let mut alpha: Option<AlphaStream<'_>> = None;
    let bytes_per_pixel = if first & 0xC1 == 0xC0 {
        // 24-bit BGR không alpha: byte đầu là marker, phần màu bắt đầu ở byte 1.
        cursor.at = 1;
        3
    } else if first & 0xC1 == 0xC1 {
        // 24-bit BGR có alpha: marker + u32 offset alpha, phần màu bắt đầu ở byte 5.
        let bytes = data
            .get(1..5)
            .ok_or(SpriteError::Truncated { at: 1, need: 4 })?;
        let offset = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let start = usize::try_from(offset).map_err(|_| SpriteError::AlphaOffset {
            offset,
            len: data.len(),
        })?;
        if start < 5 || start > data.len() {
            return Err(SpriteError::AlphaOffset {
                offset,
                len: data.len(),
            });
        }
        cursor.at = 5;
        alpha = Some(AlphaStream::new(data, start));
        3
    } else {
        // 16-bit RGB565: phần màu bắt đầu ngay byte 0.
        2
    };
    let has_alpha = alpha.is_some();

    // `run` còn lại của control byte hiện tại. Run rỗng (`run == 0`) kéo dài tới hết ảnh,
    // giống tool tham chiếu; dữ liệu thật không có control byte run rỗng.
    let mut run: i64 = 0;
    let mut ctrl: u8 = 0;
    let mut color: Option<Color> = None;
    for index in 0..count {
        if run == 0 {
            ctrl = cursor.u8()?;
            run = i64::from(ctrl & 0x3F);
            color = if ctrl & 0x80 != 0 {
                // Run trong suốt: không có byte màu.
                None
            } else {
                Some(read_color(&mut cursor, bytes_per_pixel)?)
            };
        } else if ctrl & 0xC0 == 0 {
            // Run pixel liên tiếp: mỗi pixel một màu.
            color = Some(read_color(&mut cursor, bytes_per_pixel)?);
        }
        run -= 1;

        let transparency = match alpha.as_mut() {
            Some(stream) => stream.next_value()?,
            None => 0,
        };
        let Some(color) = color else {
            continue;
        };
        let (r, g, b) = match color {
            Color::Bgr24([r, g, b]) => (r, g, b),
            Color::Rgb565(value) => {
                if value == TRANSPARENT_RGB565 {
                    continue;
                }
                rgb565(value)
            }
        };
        let alpha_byte = if has_alpha { 255 - transparency } else { 255 };
        out[index * 4] = r;
        out[index * 4 + 1] = g;
        out[index * 4 + 2] = b;
        out[index * 4 + 3] = alpha_byte;
    }
    Ok(out)
}

/// Màu của một pixel trong luồng RLE.
#[derive(Clone, Copy)]
enum Color {
    /// 24-bit BGR theo thứ tự byte trong file.
    Bgr24([u8; 3]),
    /// 16-bit RGB565 theo thứ tự byte trong file.
    Rgb565(u16),
}

fn read_color(cursor: &mut Cursor<'_>, bytes_per_pixel: usize) -> Result<Color, SpriteError> {
    if bytes_per_pixel == 3 {
        let b = cursor.u8()?;
        let g = cursor.u8()?;
        let r = cursor.u8()?;
        Ok(Color::Bgr24([r, g, b]))
    } else {
        let low = cursor.u8()?;
        let high = cursor.u8()?;
        Ok(Color::Rgb565(u16::from(low) | (u16::from(high) << 8)))
    }
}

/// Đổi RGB565 sang RGB 8 bit (làm tròn xuống, như tool tham chiếu).
fn rgb565(value: u16) -> (u8, u8, u8) {
    let r = u32::from((value >> 11) & 0x1F) * 255 / 31;
    let g = u32::from((value >> 5) & 0x3F) * 255 / 63;
    let b = u32::from(value & 0x1F) * 255 / 31;
    (r as u8, g as u8, b as u8)
}

/// Luồng alpha: control byte + byte giá trị, cùng dạng RLE màu nhưng giá trị là mức
/// trong suốt (alpha thật = `255 - giá trị`).
struct AlphaStream<'a> {
    cursor: Cursor<'a>,
    ctrl: u8,
    run: u32,
    value: u8,
}

impl<'a> AlphaStream<'a> {
    fn new(data: &'a [u8], at: usize) -> Self {
        Self {
            cursor: Cursor::new_at(data, at),
            ctrl: 0,
            run: 0,
            value: 0,
        }
    }

    fn next_value(&mut self) -> Result<u8, SpriteError> {
        if self.run != 0 {
            if self.ctrl & 0xC0 == 0 {
                self.value = self.cursor.u8()?;
            }
            self.run -= 1;
            return Ok(self.value);
        }
        let ctrl = self.cursor.u8()?;
        self.ctrl = ctrl;
        self.run = u32::from(ctrl & 0x3F);
        if self.run == 0 {
            // Run rỗng: giá trị 0 (opaque) và control tiếp theo đọc ở lần sau.
            self.value = 0;
            return Ok(0);
        }
        self.value = if ctrl & 0x80 != 0 {
            0
        } else {
            self.cursor.u8()?
        };
        self.run -= 1;
        Ok(self.value)
    }
}

struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }

    fn new_at(data: &'a [u8], at: usize) -> Self {
        Self { data, at }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], SpriteError> {
        let end = self
            .at
            .checked_add(count)
            .filter(|&end| end <= self.data.len());
        let Some(end) = end else {
            return Err(SpriteError::Truncated {
                at: self.at,
                need: count,
            });
        };
        let out = &self.data[self.at..end];
        self.at = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, SpriteError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, SpriteError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i32(&mut self) -> Result<i32, SpriteError> {
        let bytes = self.take(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn remaining(&self) -> usize {
        self.data.len() - self.at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dựng blob sprite tổng hợp: 3 ảnh (24-bit không alpha, 24-bit có alpha, 16-bit
    /// RGB565) và 3 frame. Byte dựng theo đúng định dạng để đối chiếu với tool tham
    /// chiếu `i16.py` (giá trị RGBA kỳ vọng lấy từ tool đó).
    fn synthetic_blob() -> Vec<u8> {
        /// Run pixel liên tiếp, mỗi pixel một màu BGR.
        fn literal(colors: &[[u8; 3]]) -> Vec<u8> {
            let mut out = vec![colors.len() as u8];
            for color in colors {
                out.extend_from_slice(color);
            }
            out
        }
        /// Run lặp lại một màu BGR.
        fn repeat(count: u8, color: [u8; 3]) -> Vec<u8> {
            let mut out = vec![0x40 | count];
            out.extend_from_slice(&color);
            out
        }
        /// Run trong suốt.
        fn transparent(count: u8) -> Vec<u8> {
            vec![0x80 | count]
        }

        // Ảnh 0: 24-bit BGR không alpha, 4×3.
        let mut color_0 = literal(&[[10, 20, 30], [40, 50, 60]]);
        color_0.extend(transparent(2));
        color_0.extend(repeat(3, [70, 80, 90]));
        color_0.extend(literal(&[[100, 110, 120]]));
        color_0.extend(literal(&[[1, 1, 1], [2, 2, 2], [3, 3, 3], [4, 4, 4]]));
        let mut image_0 = vec![0xC0];
        image_0.extend(color_0);

        // Ảnh 1: 24-bit BGR có alpha, 2×2. Offset alpha tính từ đầu ảnh.
        let mut color_1 = literal(&[[1, 2, 3], [4, 5, 6]]);
        color_1.extend(transparent(2));
        let alpha_1 = [0x01, 0x40, 0x42, 0x10, 0x00];
        let alpha_offset = 5 + color_1.len() as u32;
        let mut image_1 = vec![0xC1];
        image_1.extend_from_slice(&alpha_offset.to_le_bytes());
        image_1.extend(color_1);
        image_1.extend(alpha_1);

        // Ảnh 2: 16-bit RGB565, 2×2. Màu khóa 0xF81F là pixel trong suốt.
        let mut image_2 = vec![0x02, 0x1F, 0xF8, 0x00, 0xF8];
        image_2.extend(transparent(2));

        let mut blob = Vec::new();
        blob.extend_from_slice(&3u32.to_le_bytes()); // n_frames
        blob.extend_from_slice(&SPRITE_VERSION.to_le_bytes());
        blob.extend_from_slice(&3u32.to_le_bytes()); // n_images
        blob.extend_from_slice(&0x1234u32.to_le_bytes()); // unk
        for (height, width) in [(3u32, 4u32), (2, 2), (2, 2)] {
            blob.extend_from_slice(&height.to_le_bytes());
            blob.extend_from_slice(&width.to_le_bytes());
        }
        let frames: [[i32; 12]; 3] = [
            [-1, 64, 7, 0, 0, 0, 4, 3, 5, 9, 0, 8],
            [0, 0, -1, 1, 0, 0, 2, 2, 2, 2, 0, 0],
            [2, -3, 0, 2, 0, 0, 2, 2, 3, 4, 1, 2],
        ];
        for frame in frames {
            for value in frame {
                blob.extend_from_slice(&value.to_le_bytes());
            }
        }
        for image in [&image_0, &image_1, &image_2] {
            blob.extend_from_slice(&(image.len() as u32).to_le_bytes());
            blob.extend_from_slice(image);
        }
        blob
    }

    /// RGBA kỳ vọng, lấy từ `i16.py` cho blob tổng hợp trên.
    const EXPECTED_0: [u8; 48] = [
        0x1e, 0x14, 0x0a, 0xff, 0x3c, 0x32, 0x28, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x5a, 0x50, 0x46, 0xff, 0x5a, 0x50, 0x46, 0xff, 0x5a, 0x50, 0x46, 0xff, 0x78, 0x6e,
        0x64, 0xff, 0x01, 0x01, 0x01, 0xff, 0x02, 0x02, 0x02, 0xff, 0x03, 0x03, 0x03, 0xff, 0x04,
        0x04, 0x04, 0xff,
    ];
    const EXPECTED_1: [u8; 16] = [
        0x03, 0x02, 0x01, 0xbf, 0x06, 0x05, 0x04, 0xef, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    const EXPECTED_2: [u8; 16] = [
        0x00, 0x00, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];

    #[test]
    fn parses_synthetic_blob_header_frames_and_images() {
        let sprite = parse(&synthetic_blob()).expect("parse blob tổng hợp");
        assert_eq!(sprite.unk, 0x1234);
        assert_eq!(sprite.images.len(), 3);
        assert_eq!(sprite.frames.len(), 3);
        assert_eq!(
            sprite.images[0],
            Image {
                width: 4,
                height: 3,
                data: sprite.images[0].data.clone(),
            }
        );
        assert_eq!(sprite.images[0].width, 4);
        assert_eq!(sprite.images[0].height, 3);
        assert_eq!(
            sprite.frames[0],
            Frame {
                offset_x: -1,
                offset_y: 64,
                flag: 7,
                image_index: 0,
                source: Rect {
                    x0: 0,
                    y0: 0,
                    x1: 4,
                    y1: 3
                },
                canvas: Size {
                    width: 5,
                    height: 9
                },
                pivot: Point { x: 0, y: 8 },
            }
        );
        assert_eq!(sprite.frames[2].image_index, 2);
        assert_eq!(sprite.frames[2].pivot, Point { x: 1, y: 2 });
    }

    #[test]
    fn decodes_synthetic_blob_like_reference_tool() {
        let sprite = parse(&synthetic_blob()).expect("parse blob tổng hợp");
        assert_eq!(sprite.decode_image(0).expect("giải ảnh 0"), EXPECTED_0);
        assert_eq!(sprite.decode_image(1).expect("giải ảnh 1"), EXPECTED_1);
        assert_eq!(sprite.decode_image(2).expect("giải ảnh 2"), EXPECTED_2);
    }

    #[test]
    fn rejects_bad_version_and_truncated_blob() {
        let mut blob = synthetic_blob();
        blob[4..8].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(parse(&blob), Err(SpriteError::Version { version: 3 }));

        let mut blob = synthetic_blob();
        blob[8..12].copy_from_slice(&1_000_000u32.to_le_bytes());
        assert!(matches!(
            parse(&blob),
            Err(SpriteError::Count { what: "ảnh", .. })
        ));

        let mut blob = synthetic_blob();
        blob[0..4].copy_from_slice(&1_000_000u32.to_le_bytes());
        assert!(matches!(
            parse(&blob),
            Err(SpriteError::Count { what: "frame", .. })
        ));

        let blob = synthetic_blob();
        assert!(matches!(
            parse(&blob[..blob.len() - 3]),
            Err(SpriteError::Truncated { .. })
        ));
        assert!(matches!(parse(&[]), Err(SpriteError::Truncated { .. })));
    }

    #[test]
    fn rejects_out_of_range_image_index_and_bad_alpha_offset() {
        let sprite = parse(&synthetic_blob()).expect("parse blob tổng hợp");
        assert_eq!(
            sprite.decode_image(3),
            Err(SpriteError::ImageIndex { index: 3, count: 3 })
        );

        // Offset alpha nằm ngoài ảnh.
        let mut image = sprite.images[1].data().to_vec();
        image[1..5].copy_from_slice(&9999u32.to_le_bytes());
        assert!(matches!(
            decode_image(&image, 2, 2),
            Err(SpriteError::AlphaOffset { .. })
        ));
    }

    #[test]
    fn rejects_rle_running_past_the_end() {
        // Run literal 4 pixel nhưng ảnh chỉ 2 pixel và dữ liệu màu bị cắt.
        let image = [0xC0u8, 0x04, 0x01, 0x02, 0x03];
        assert!(matches!(
            decode_image(&image, 2, 2),
            Err(SpriteError::Truncated { .. })
        ));
    }

    #[test]
    fn rejects_image_larger_than_the_documented_limit() {
        assert!(matches!(
            decode_image(&[0xC0], MAX_IMAGE_SIDE + 1, 1),
            Err(SpriteError::ImageTooLarge { .. })
        ));
        assert!(matches!(
            decode_image(&[0xC0], 1, MAX_IMAGE_SIDE + 1),
            Err(SpriteError::ImageTooLarge { .. })
        ));
    }

    /// Thư mục chứa pack KR thật; test bỏ qua khi chưa đặt `BOOM_KR_PACK_DIR`.
    fn pack_root() -> Option<std::path::PathBuf> {
        let path = std::env::var_os("BOOM_KR_PACK_DIR")?;
        Some(std::path::PathBuf::from(path))
    }

    /// SHA-256 thuần Rust cho test nghiệm thu (cùng cách làm như `tests/pack.rs`: tránh
    /// thêm dependency chỉ để băm một chuỗi).
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

    /// Nghiệm thu trên pack KR thật: RGBA phải khớp từng pixel với tool tham chiếu
    /// `i16.py` (SHA-256 của chuỗi RGBA do tool đó sinh).
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

    #[test]
    fn decodes_real_kr_bomb_sprite_like_reference_tool() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let pack =
            crate::pack::Pack::open(&root, "fx", crate::legacy_map::Build::Kr).expect("mở pack fx");
        let blob = pack.read_named("Item/Bomb").expect("đọc Item/Bomb");
        let sprite = parse(&blob).expect("parse Item/Bomb");

        assert_eq!(sprite.unk, 0);
        assert_eq!(sprite.frames.len(), 1);
        assert_eq!(sprite.images.len(), 1);
        assert_eq!(sprite.images[0].width, 38);
        assert_eq!(sprite.images[0].height, 43);
        assert_eq!(
            sprite.frames[0],
            Frame {
                offset_x: -1,
                offset_y: 64,
                flag: 7,
                image_index: 0,
                source: Rect {
                    x0: 0,
                    y0: 0,
                    x1: 38,
                    y1: 43
                },
                canvas: Size {
                    width: 40,
                    height: 67
                },
                pivot: Point { x: 0, y: 66 },
            }
        );
        assert_eq!(
            sha256_hex(&sprite.decode_image(0).expect("giải Item/Bomb")),
            "611421c7be811d77c3624092289ff706c7e8fc854bb4406321dbc60341243819"
        );
    }

    /// Nghiệm thu trên pack KR thật: 5 frame của C00 đi xuống, RGBA khớp `i16.py`.
    #[test]
    fn decodes_real_kr_character_sprite_like_reference_tool() {
        let Some(root) = pack_root() else {
            eprintln!("bỏ qua: chưa đặt BOOM_KR_PACK_DIR");
            return;
        };
        let pack = crate::pack::Pack::open(&root, "FxCharacter", crate::legacy_map::Build::Kr)
            .expect("mở pack FxCharacter");
        let blob = pack
            .read_named("Character/C00-WalkDown_ALayerStatic")
            .expect("đọc C00-WalkDown");
        let sprite = parse(&blob).expect("parse C00-WalkDown");

        assert_eq!(sprite.unk, 1);
        assert_eq!(sprite.frames.len(), 5);
        assert_eq!(sprite.images.len(), 5);
        let sizes = [(34u32, 32u32), (33, 32), (36, 32), (33, 32), (35, 32)];
        for (index, (width, height)) in sizes.iter().enumerate() {
            assert_eq!(sprite.images[index].width, *width, "ảnh {index}");
            assert_eq!(sprite.images[index].height, *height, "ảnh {index}");
        }
        // Frame dùng ảnh 0..4 theo thứ tự, rect nguồn phủ đúng ảnh.
        for (index, frame) in sprite.frames.iter().enumerate() {
            assert_eq!(frame.image_index, index as i32, "frame {index}");
            assert_eq!(frame.source.x0, 0, "frame {index}");
            assert_eq!(frame.source.y0, 0, "frame {index}");
            assert_eq!(frame.source.x1, sizes[index].0 as i32, "frame {index}");
            assert_eq!(frame.source.y1, sizes[index].1 as i32, "frame {index}");
            assert_eq!(
                frame.canvas,
                Size {
                    width: 56,
                    height: 67
                },
                "frame {index}"
            );
            assert_eq!(frame.pivot, Point { x: 8, y: 66 }, "frame {index}");
        }
        let expected = [
            "b824f74d1cfbd60faae5fa4737ee9901de16c099d83668704f5b9fcf01025ea5",
            "e094a74f3e79f66f9ff1d694869959447c846ea969dbe531a5111353d5761283",
            "1be322f54e7c2f3ee384b86047532987d656521b94fef6262d080dae9b32c7a5",
            "968f578c4ef647e645e063cc4ba144d7da75111be45b6f0d52ea65f11374add4",
            "775cbd4debe04d68046d8a963426588c9bd66cf76b6b872acdd19977f10090c6",
        ];
        for (index, expected_hash) in expected.iter().enumerate() {
            let rgba = sprite.decode_image(index).expect("giải ảnh C00");
            assert_eq!(&sha256_hex(&rgba), expected_hash, "ảnh {index}");
        }
    }
}
