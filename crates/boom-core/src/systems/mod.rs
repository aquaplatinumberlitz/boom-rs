//! System của Battle V1, mỗi file một phần luật, một owner (contracts-r4, ARCHITECTURE §5).
//!
//! | File | Feature | Ghi được | Phát event |
//! |---|---|---|---|
//! | `movement.rs` | V1-011, V1-012 | `Player.{x,y,facing}`, `Bomb.grace` (chỉ xóa) | `player_turned` |
//! | `bomb_place.rs` | V1-013 | `bombs` (thêm), `ids` | `bomb_placed` |
//! | `explosion.rs` | V1-014, V1-015 | `bombs` (xóa), `flames`, `blasts` | `bomb_exploded` |
//! | `blocks.rs` | V1-016 | `field`, `items` (thêm/xóa do lửa), `ids`, `rng` (đầu lượt), `blasts` (chỉ xóa) | `block_destroyed`, `item_revealed`, `item_destroyed` |
//! | `pickup.rs` | V1-017…V1-019 | `items` (xóa), `Player.{bombs,range,speed}` | `item_picked` |
//! | `life.rs` | V1-020 | `Player.life` | `player_trapped`, `player_died` |
//!
//! Thắng/hòa/hết giờ (V1-021) ở `mode.rs`. Đọc thì system nào cũng đọc được toàn bộ state.

mod blocks;
mod bomb_place;
mod explosion;
mod life;
mod movement;
mod pickup;

pub use blocks::BLOCKS;
pub use bomb_place::BOMB_PLACE;
pub use explosion::EXPLOSION;
pub use life::LIFE;
pub use movement::MOVEMENT;
pub use pickup::PICKUP;
