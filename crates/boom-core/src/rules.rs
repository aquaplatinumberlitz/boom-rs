//! Giá trị luật đã đổi sang đơn vị mô phỏng (contracts-r1).
//!
//! Mỗi field đọc từ một khóa của rules profile (`data/rules/*.toml`); thêm rule mới là thay đổi contract,
//! do integrator thêm vào đây cùng khóa profile và claim RULES_V1.

use boom_content::rules::{ProfileError, RulesProfile};

use crate::num::{Mpx, Tick, ms_to_ticks};
use crate::state::CharacterStats;

/// Hiệu ứng một item tác động lên chỉ số người chơi (ADR 0008 #6: item mới ghép từ hiệu ứng có sẵn).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemEffect {
    BombCapacity,
    BlastRange,
    SpeedLevel,
}

/// Một loại item rơi được. `ItemKind(i)` là chỉ số trong [`Rules::items`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemDef {
    /// Tên ổn định (theo tên item của client: `Bomb`, `Fire`, `Roller`).
    pub key: &'static str,
    pub effect: ItemEffect,
    /// Lượng cộng thêm mỗi lần nhặt, theo đơn vị cấp.
    pub amount: i32,
    /// Trọng số rơi tương đối (không phải phần trăm).
    pub weight: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rules {
    pub ticks_per_second: u32,
    pub tile: Mpx,
    /// Tốc độ một cấp, mpx/tick, làm tròn xuống khi đổi từ px/s.
    pub speed_per_level: Mpx,
    pub round_ticks: Tick,
    /// Từ lúc đặt tới lúc nổ.
    pub fuse_ticks: Tick,
    /// Thời gian một ô lửa tồn tại.
    pub flame_ticks: Tick,
    /// Từ lúc bị nhốt tới lúc tự chết.
    pub trap_ticks: Tick,
    /// Nửa cạnh hộp va chạm của người chơi: hộp `2 × collision_half` px (26×26 khi 13 px, R-107).
    pub collision_half: Mpx,
    /// Lệch tối đa (px) được phép bám góc về tâm hàng/cột khi bị chặn (R-107).
    pub corner_assist: Mpx,
    /// Xác suất một khối mềm có item, phần nghìn.
    pub brick_item_chance_permille: u32,
    /// Bảng item V1, thứ tự cố định: Bomb, Fire, Roller.
    pub items: Vec<ItemDef>,
    pub profile_hash: u64,
}

/// Bảng chỉ số hai nhân vật V1 (C00/C01), đọc từ rules profile (`characters.*`, R-040/R-041).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterStatsTable {
    pub c00: CharacterStats,
    pub c01: CharacterStats,
}

impl CharacterStatsTable {
    /// Bảng chỉ số từ rules profile: `characters.c00.*` / `characters.c01.*`.
    ///
    /// Profile thiếu nhân vật hoặc có cấp hiện tại vượt trần ⇒ lỗi rõ (không tự đặt mặc định).
    pub fn from_profile(p: &RulesProfile) -> Result<Self, ProfileError> {
        let table = p.character_table()?;
        Ok(Self {
            c00: CharacterStats::from_spec(&table.c00),
            c01: CharacterStats::from_spec(&table.c01),
        })
    }
}

impl Rules {
    pub fn from_profile(p: &RulesProfile) -> Result<Self, ProfileError> {
        let hz = u32_of(p.int_in("sim.ticks_per_second", "Hz", 1, 1000)?);
        let tile_px = i32_of(p.int_in("grid.tile_px", "px", 1, 1000)?);
        let speed_px_s = i32_of(p.int_in("player.speed_px_per_s_per_level", "px/s", 1, 10_000)?);
        let ms = |key: &str| -> Result<Tick, ProfileError> {
            Ok(ms_to_ticks(u32_of(p.int_in(key, "ms", 1, 3_600_000)?), hz))
        };
        let item = |key, effect, delta_key: &str, delta_unit: &str, weight_key: &str| {
            Ok::<_, ProfileError>(ItemDef {
                key,
                effect,
                amount: i32_of(p.int_in(delta_key, delta_unit, 0, 100)?),
                weight: u32_of(p.int_in(weight_key, "weight", 0, 1_000_000)?),
            })
        };
        Ok(Self {
            ticks_per_second: hz,
            tile: Mpx::from_px(tile_px),
            speed_per_level: Mpx(speed_px_s * Mpx::PER_PX / i32_of(i64::from(hz))),
            round_ticks: ms("round.duration_ms")?,
            fuse_ticks: ms("bomb.fuse_ms")?,
            flame_ticks: ms("bomb.flame_ms")?,
            trap_ticks: ms("player.trap_to_death_ms")?,
            collision_half: Mpx::from_px(i32_of(p.int_in(
                "player.collision_half_px",
                "px",
                1,
                100,
            )?)),
            corner_assist: Mpx::from_px(i32_of(p.int_in(
                "player.corner_assist_px",
                "px",
                0,
                100,
            )?)),
            brick_item_chance_permille: u32_of(p.int_in(
                "drop.brick_item_chance",
                "permille",
                0,
                1000,
            )?),
            items: vec![
                item(
                    "Bomb",
                    ItemEffect::BombCapacity,
                    "item.bomb_capacity_delta",
                    "bomb",
                    "drop.weight_bomb",
                )?,
                item(
                    "Fire",
                    ItemEffect::BlastRange,
                    "item.fire_range_delta",
                    "cell",
                    "drop.weight_fire",
                )?,
                item(
                    "Roller",
                    ItemEffect::SpeedLevel,
                    "item.roller_speed_delta",
                    "level",
                    "drop.weight_roller",
                )?,
            ],
            profile_hash: p.identity_hash(),
        })
    }
}

fn i32_of(v: i64) -> i32 {
    i32::try_from(v).expect("miền đã kiểm nằm trong i32")
}

fn u32_of(v: i64) -> u32 {
    u32::try_from(v).expect("miền đã kiểm nằm trong u32")
}
