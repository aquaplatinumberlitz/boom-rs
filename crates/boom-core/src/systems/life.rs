//! Bị nhốt, chạm địch, hết thời gian nhốt thì chết — V1-020.
//!
//! Thứ tự trong một tick (R-108, BnB `server/game.js:454–474,546–567`):
//! 1. Người `Alive` đứng trong ô có lửa → `Trapped { since: tick }`, phát `player_trapped`
//!    với `by` là chủ của lửa.
//! 2. **Hết thời gian nhốt trước chạm**: mọi người đang bị nhốt — kể cả người vừa bị nhốt ở
//!    bước 1 — giảm timer; hết hạn thì chết, `by = None`.
//! 3. **Chạm địch**: người `Alive` còn lại làm vỡ bong bóng khi lệch với người bị nhốt **nhỏ
//!    hơn** `player.trap_touch_distance_px` (28 px) **trên từng trục**; người bị nhốt chết,
//!    `by` là người chạm.
//!
//! **Timer nhốt tính cả tick bị nhốt** (R-108): người bị nhốt ở tick `T` chết ở tick
//! `T + trap_ticks - 1` — profile V1 hiệu chuẩn 6350ms/191tick ở30Hz (R-160),
//! chết ở T+190 (~6333ms). Cơ chế tính tick tạo giữ từ BnB: giảm `trapTicks` ngay
//! trong tick bị nhốt (`server/game.js:569` chạy sau `applyFlameDamage`), nên tick bị nhốt là
//! tick giảm đầu tiên.
//!
//! Người vừa bị nhốt **có thể bị chọc ngay trong tick đó**: BnB xếp người đã bị nhốt vào danh
//! sách `trapped` ngay sau khi nhốt, rồi vòng chạm chạy cùng tick.
//!
//! V1 giữ thứ tự `id` tăng dần thay thứ tự chèn của `Map` JS (ADR 0012): khi nhiều người
//! chạm cùng một người bị nhốt thì người có `id` nhỏ thắng.
//!
//! **Warmup** (R-100): trong warmup không system nào chạy, nên timer nhốt không giảm; mốc
//! bị nhốt dồn về cuối warmup để người bị nhốt còn đủ timer khi lượt bắt đầu.
//!
//! Người `Trapped` không làm vỡ bong bóng của nhau; `Dead` không đổi trạng thái lần nữa.
//! Mọi giá trị thời gian và khoảng cách lấy từ `Rules`; không có hằng số luật trong file này.
//!
//! Quyền ghi và event được phép: bảng trong `systems/mod.rs`. Kịch bản nghiệm thu:
//! `fixtures/scenarios/life/`. Luật và nguồn: RULES_V1.

use crate::event::{EventKind, EventSink};
use crate::num::Cell;
use crate::scratch::TickScratch;
use crate::sim::{PlayerInput, World};
use crate::state::{Life as PlayerLife, SimulationState};
use crate::system::System;

#[derive(Debug)]
pub struct Life;

pub static LIFE: Life = Life;

impl System for Life {
    fn name(&self) -> &'static str {
        "life"
    }

    fn run(
        &self,
        world: &World,
        state: &mut SimulationState,
        _inputs: &[PlayerInput],
        events: &mut EventSink<'_>,
        _scratch: &mut TickScratch,
    ) {
        let tick = state.tick;
        let rules = world.rules();
        let cells: Vec<Cell> = state
            .players
            .iter()
            .map(|p| world.cell_at(p.x, p.y))
            .collect();

        // 1. Lửa nhốt người còn sống. Mỗi ô tối đa một flame (invariant), nên chủ lửa là duy nhất.
        //    Chỉ lửa **còn sống** ở tick này mới gây sát thương (`until` là tick cuối còn lửa,
        //    bao gồm): lửa đã hết hạn không nhốt ai — BnB xoá vụ nổ hết TTL trước khi áp
        //    sát thương (`game.js:448–451`).
        for (i, &cell) in cells.iter().enumerate() {
            if state.players[i].life != PlayerLife::Alive {
                continue;
            }
            let Some(owner) = state
                .flames
                .iter()
                .find(|f| f.cell == cell && f.until >= tick)
                .map(|f| f.owner)
            else {
                continue;
            };
            let player = state.players[i].id;
            state.players[i].life = PlayerLife::Trapped { since: tick };
            events.emit(EventKind::PlayerTrapped { player, by: owner });
        }

        // 2. Hết thời gian nhốt (trước chạm). Người vừa bị nhốt ở bước 1 cũng giảm timer:
        // số lần giảm = số tick đã trôi kể từ tick bị nhốt, tính cả tick bị nhốt.
        // Warmup đóng băng nên timer không chạy trong warmup: mốc bị nhốt dồn về cuối warmup.
        let trap_ticks = rules.trap_ticks;
        let warmup_ticks = rules.warmup_ticks;
        for i in 0..state.players.len() {
            let PlayerLife::Trapped { since } = state.players[i].life else {
                continue;
            };
            let since = since.max(warmup_ticks);
            let elapsed = tick.saturating_sub(since).saturating_add(1);
            if elapsed < trap_ticks {
                continue;
            }
            let player = state.players[i].id;
            state.players[i].life = PlayerLife::Dead;
            events.emit(EventKind::PlayerDied { player, by: None });
        }

        // 3. Chạm địch: lệch trên từng trục phải nhỏ hơn ngưỡng (strict) thì mới chạm.
        let touch = rules.trap_touch_distance.0;
        for i in 0..state.players.len() {
            if !matches!(state.players[i].life, PlayerLife::Trapped { .. }) {
                continue;
            }
            let (x, y) = (state.players[i].x.0, state.players[i].y.0);
            let popper = (0..state.players.len()).find(|&j| {
                j != i
                    && state.players[j].life == PlayerLife::Alive
                    && (state.players[j].x.0 - x).abs() < touch
                    && (state.players[j].y.0 - y).abs() < touch
            });
            let Some(j) = popper else {
                continue;
            };
            let player = state.players[i].id;
            let by = state.players[j].id;
            state.players[i].life = PlayerLife::Dead;
            events.emit(EventKind::PlayerDied {
                player,
                by: Some(by),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use boom_content::map::Terrain;
    use boom_content::rules::RulesProfile;

    use crate::event::{EventKind, EventSink, SimulationEvent};
    use crate::mode::mode_by_id;
    use crate::num::Mpx;
    use crate::rules::Rules;
    use crate::scratch::TickScratch;
    use crate::sim::{Dir, PlayerInput, PlayerSetup, World};
    use crate::state::{CharacterStats, EntityId, Flame, Life, SimulationState, Stat};
    use crate::system::System;

    use super::LIFE;

    const PROFILE: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

    fn rules() -> Rules {
        Rules::from_profile(&RulesProfile::from_toml(PROFILE).unwrap()).unwrap()
    }

    fn world_with(rules: Rules, rows: &[&str]) -> World {
        let ffa = mode_by_id("battle_ffa").unwrap();
        World::new(Terrain::from_ascii(rows).unwrap(), rules, ffa).unwrap()
    }

    fn setup(id: u8) -> PlayerSetup {
        PlayerSetup {
            id,
            stats: CharacterStats {
                speed: Stat::new(5, 7),
                bombs: Stat::new(1, 10),
                range: Stat::new(1, 7),
            },
        }
    }

    /// Hai người: `0` ở ô (1,1), `1` ở ô (2,1); `1` bị nhốt ngay từ đầu. Vị trí đặt tay để
    /// không phụ thuộc thứ tự spawn theo seed.
    fn pair(rules: Rules) -> (World, SimulationState) {
        let w = world_with(rules, &["#####", "#0.1#", "#####"]);
        let mut s = SimulationState::new_round(&w, &[setup(0), setup(1)], 1).unwrap();
        place(&mut s, 0, 60_000, 60_000);
        place(&mut s, 1, 100_000, 60_000);
        s.players[1].life = Life::Trapped { since: 0 };
        (w, s)
    }

    fn step(w: &World, s: &mut SimulationState) -> Vec<SimulationEvent> {
        let mut events = Vec::new();
        s.step(w, &[PlayerInput::default(); 2], &mut events)
            .unwrap();
        s.check_invariants(w).unwrap();
        events
    }

    /// Chạy **riêng** system `life`: lửa hết hạn vẫn còn trong state (bước `explosion` mới
    /// xóa lửa theo tick), nên test phải gọi thẳng system để kiểm bước dùng lửa.
    fn run_life(w: &World, s: &mut SimulationState) -> Vec<SimulationEvent> {
        let mut events = Vec::new();
        let mut sink = EventSink::new(s.tick, &mut events);
        let mut scratch = TickScratch::new();
        LIFE.run(w, s, &[PlayerInput::default(); 2], &mut sink, &mut scratch);
        s.check_invariants(w).unwrap();
        events
    }

    /// Đặt người `id` ở toạ độ pixel (mpx).
    fn place(s: &mut SimulationState, id: u8, x: i32, y: i32) {
        let p = s.players.iter_mut().find(|p| p.id == id).unwrap();
        p.x = Mpx(x);
        p.y = Mpx(y);
    }

    /// R-159/R-160: native HELP còn sau 5,2s; V1 hiệu chuẩn bằng endpoint native,
    /// không đổi 200 bước counter VN thành 200 tick Rust.
    #[test]
    fn vn_calibrated_trap_survives_five_seconds_and_expires_at_boundary() {
        let profile =
            RulesProfile::from_toml(include_str!("../../../../data/rules/v1-dev.toml")).unwrap();
        let mut calibrated = Rules::from_profile(&profile).unwrap();
        assert_eq!(calibrated.ticks_per_second, 30);
        assert_eq!(calibrated.trap_ticks, 191);
        // Thử từ mốc nhốt, không cộng warmup vào thời gian này.
        calibrated.warmup_ticks = 0;
        let (world, mut state) = pair(calibrated);
        for tick in 0..190 {
            let events = step(&world, &mut state);
            assert_eq!(
                state.players[1].life,
                Life::Trapped { since: 0 },
                "tick {tick}"
            );
            assert!(
                !events
                    .iter()
                    .any(|event| matches!(event.kind, EventKind::PlayerDied { .. }))
            );
        }
        let events = step(&world, &mut state);
        assert_eq!(state.players[1].life, Life::Dead);
        assert!(events.iter().any(|event| event.kind
            == EventKind::PlayerDied {
                player: 1,
                by: None
            }));
    }

    #[test]
    fn trap_timer_counts_the_trapping_tick() {
        // R-108: bị nhốt ở tick 0 với trap 120 tick thì chết ở tick 119, không phải 120.
        let mut r = rules();
        r.trap_ticks = 120;
        let (w, mut s) = pair(r);

        for _ in 0..119 {
            let events = step(&w, &mut s);
            assert!(
                !events
                    .iter()
                    .any(|e| matches!(e.kind, EventKind::PlayerDied { .. })),
                "chưa tới hạn thì chưa chết"
            );
            assert_eq!(s.players[1].life, Life::Trapped { since: 0 });
        }
        let events = step(&w, &mut s);
        assert!(
            events.iter().any(|e| e.kind
                == EventKind::PlayerDied {
                    player: 1,
                    by: None
                }),
            "tick 119 là tick hết hạn (120 tick tính cả tick bị nhốt)"
        );
        assert_eq!(s.players[1].life, Life::Dead);
    }

    #[test]
    fn touch_needs_less_than_twenty_eight_px_on_each_axis() {
        // R-108: lệch 27 px mỗi trục thì chạm; lệch 28 px thì không chạm.
        let (w, mut s) = pair(rules());
        // Người 1 ở tâm ô (2,1) = (100 px, 60 px); người 0 đặt lệch 27 px theo cả hai trục.
        place(&mut s, 0, 100_000 - 27_000, 60_000 - 27_000);
        let events = step(&w, &mut s);
        assert!(
            events.iter().any(|e| e.kind
                == EventKind::PlayerDied {
                    player: 1,
                    by: Some(0)
                }),
            "lệch 27 px mỗi trục thì chạm"
        );
        assert_eq!(s.players[1].life, Life::Dead);

        // Lệch đúng 28 px: không chạm.
        let (w, mut s) = pair(rules());
        place(&mut s, 0, 100_000 - 28_000, 60_000 - 28_000);
        let events = step(&w, &mut s);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, EventKind::PlayerDied { .. })),
            "lệch 28 px thì không chạm (ngưỡng strict)"
        );
        assert_eq!(s.players[1].life, Life::Trapped { since: 0 });
    }

    #[test]
    fn timeout_wins_over_touch_in_the_same_tick() {
        // Hết hạn đúng tick chạm: BnB giảm timer trước rồi mới cho chạm, nên chết vì hết giờ.
        let mut r = rules();
        r.trap_ticks = 2;
        let (w, mut s) = pair(r);
        // Tick 0 người 0 còn xa (30 px) nên chưa chạm; tick 1 người 0 tới gần.
        place(&mut s, 0, 100_000 - 30_000, 60_000);

        let first = step(&w, &mut s);
        assert!(
            !first
                .iter()
                .any(|e| matches!(e.kind, EventKind::PlayerDied { .. })),
            "tick 0 mới nhốt, chưa hết hạn và chưa chạm"
        );
        place(&mut s, 0, 100_000 - 10_000, 60_000);
        let second = step(&w, &mut s);
        assert!(
            second.iter().any(|e| e.kind
                == EventKind::PlayerDied {
                    player: 1,
                    by: None
                }),
            "hết hạn trước chạm nên `by` là None"
        );
    }

    #[test]
    fn newly_trapped_player_can_be_popped_in_the_same_tick() {
        // BnB: người bị nhốt ở tick này vẫn nằm trong danh sách `trapped` của vòng chạm.
        let (w, mut s) = pair(rules());
        // Người 0 đứng ở ô (1,1) cách người 1 (ô (2,1)) 24 px; lửa phủ ô (2,1) nên người 1 bị
        // nhốt ở tick 0 và bị chọc ngay trong tick đó.
        place(&mut s, 0, 76_000, 60_000);
        s.players[1].life = Life::Alive;
        s.flames.push(Flame {
            cell: crate::num::Cell::new(2, 1),
            until: 10,
            source: EntityId(0),
            owner: 0,
        });
        let events = step(&w, &mut s);
        assert!(
            events
                .iter()
                .any(|e| e.kind == EventKind::PlayerTrapped { player: 1, by: 0 }),
            "người 1 bị nhốt"
        );
        assert!(
            events.iter().any(|e| e.kind
                == EventKind::PlayerDied {
                    player: 1,
                    by: Some(0)
                }),
            "người vừa bị nhốt bị chọc ngay trong tick đó"
        );
    }

    #[test]
    fn trapped_players_do_not_pop_each_other() {
        let (w, mut s) = pair(rules());
        // Cả hai cùng ô (2,1) và cùng bị nhốt: người bị nhốt không làm vỡ bong bóng của nhau.
        place(&mut s, 0, 100_000, 60_000);
        s.players[0].life = Life::Trapped { since: 0 };
        let events = step(&w, &mut s);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, EventKind::PlayerDied { .. })),
            "người bị nhốt không tự chạm"
        );
        assert_eq!(s.players[1].life, Life::Trapped { since: 0 });
    }

    #[test]
    fn expired_flame_does_not_trap() {
        // `until` là tick cuối còn lửa: lửa đã hết hạn không được nhốt ai (BnB xóa vụ nổ
        // hết TTL trước khi áp sát thương, `game.js:448–451`).
        let rows = ["#####", "#0..#", "#####"];
        let w = world_with(rules(), &rows);
        let mut s = SimulationState::new_round(&w, &[setup(0)], 1).unwrap();
        place(&mut s, 0, 100_000, 60_000); // ô (2,1)
        s.tick = 1;
        s.flames.push(Flame {
            cell: crate::num::Cell::new(2, 1),
            until: 0, // hết hạn từ tick 1
            source: EntityId(0),
            owner: 0,
        });

        let events = run_life(&w, &mut s);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, EventKind::PlayerTrapped { .. })),
            "lửa đã hết hạn không nhốt ai"
        );
        assert_eq!(s.players[0].life, Life::Alive);
    }

    #[test]
    fn trapped_player_moves_slowly_before_life_processing() {
        // R-161/R-162: movement trước life; trapped đi chậm theo profile.
        let (w, mut s) = pair(rules());
        let before = (s.players[1].x, s.players[1].y);
        let mut events = Vec::new();
        s.step(
            &w,
            &[
                PlayerInput {
                    dir: Dir::Right,
                    place_bomb: false,
                },
                PlayerInput {
                    dir: Dir::Right,
                    place_bomb: false,
                },
            ],
            &mut events,
        )
        .unwrap();
        assert_eq!(
            (s.players[1].x, s.players[1].y),
            (Mpx(before.0.0 + w.rules().trapped_speed.0), before.1),
        );
    }
}
