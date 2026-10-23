//! Test tích hợp của harness QA: chạy kịch bản headless thật (server + N bot qua
//! `MemoryTransport` có mất/trùng/đảo thứ tự) và kiểm tra state hash hội tụ.
//!
//! Test chạy trong `cargo xtask ci` (qua `cargo test --workspace`); dùng hồ sơ `fast` để
//! lượt ngắn.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use boom_qa::bot::{BotClient, BotConfig, BotError, BotFactory, BotPhase, BotStats, DirectDtoBot};
use boom_qa::harness::{RunConfig, run, run_with};
use boom_qa::profile::QaProfile;

/// Kịch bản nhanh: 2 bot, 1 lượt, hồ sơ `fast` (lượt ngắn, mạng vẫn có lỗi).
#[test]
fn fast_profile_two_bots_converge() {
    let profile = QaProfile::builtin("fast").expect("hồ sơ fast");
    let config = RunConfig {
        profile,
        clients: 2,
        seed: 7,
        rounds: Some(1),
    };
    let report = run(&config).expect("chạy harness");

    assert!(!report.timed_out, "hết ngân sách thời gian: {report:?}");
    assert_eq!(report.rounds.len(), 1, "phải có đúng một lượt");
    assert!(report.converged, "lượt không hội tụ: {report:?}");
    assert_eq!(report.rounds[0].bots, 2, "lượt phải có hai bot");
    assert_eq!(
        report.rounds[0].bot_hashes.len(),
        2,
        "mọi bot phải dựng lại được hash: {report:?}",
    );
    assert!(
        report.latency_ms.p95.is_some(),
        "phải đo được độ trễ p95: {report:?}",
    );
    assert!(report.input.sent > 0, "bot phải gửi input: {report:?}");
}

/// Bốn bot: mọi bot phải **vào phòng** trước khi lượt bắt đầu, và mọi bot phải dựng
/// lại được hash của lượt.
///
/// Đây là ca bắt lỗi "bot kẹt ở sảnh vì lệnh `Join` bị mất": khi đó lượt chỉ có ba
/// bot, bot thứ tư không có hash ⇒ báo cáo báo không hội tụ.
#[test]
fn fast_profile_four_bots_all_join_and_converge() {
    let profile = QaProfile::builtin("fast").expect("hồ sơ fast");
    let config = RunConfig {
        profile,
        clients: 4,
        seed: 7,
        rounds: Some(1),
    };
    let report = run(&config).expect("chạy harness");

    assert!(!report.timed_out, "hết ngân sách thời gian: {report:?}");
    assert_eq!(report.rounds.len(), 1, "phải có đúng một lượt");
    assert_eq!(report.rounds[0].bots, 4, "lượt phải có bốn bot");
    assert_eq!(
        report.rounds[0].bot_hashes.len(),
        4,
        "mọi bot phải dựng lại được hash: {report:?}",
    );
    assert!(report.converged, "lượt không hội tụ: {report:?}");
    assert_eq!(
        report.input.snapshot_missed, 0,
        "không bot nào được bỏ sót state cuối lượt: {report:?}",
    );
}

/// Hồi quy #226: độ trễ của hồ sơ nghiệm thu không được biến gần hết command thành retired.
///
/// Dùng **rules `fast`** (round 2 s, 60 Hz) để test ngắn, nhưng giữ đúng mạng của
/// `scope-v1` (RTT 100 ms, jitter 0–30 ms, reorder window 30 ms). Đây là hồ sơ **chỉ trong
/// test**, không sửa `profiles/*.toml`.
///
/// `resolved`/`retired` là sequence, `sent` là batch (mỗi batch tạo đúng một command mới).
/// `applied = resolved - retired`. Trước fix, target_tick lấy từ state dự đoán nên cũ hơn
/// server khoảng một RTT; gần hết sequence rơi ngoài `INPUT_LATE` (100 ms @60 Hz) và bị
/// retire, dù hash vẫn hội tụ. Ngưỡng retired ở đây chỉ là biên hồi quy cho lỗi này (mạng
/// fixture mất 1 %), **không** phải ngân sách latency/tỉ lệ pass của V1-027.
#[test]
fn scope_latency_fixture_applies_input_instead_of_retiring_it() {
    let mut profile = QaProfile::builtin("fast").expect("hồ sơ fast");
    profile.network.rtt_ms = 100;
    profile.network.jitter_ms = 30;
    profile.network.reorder_window_ms = 30;
    // Cùng lead với `scope-v1` (3 tick ≈ 50 ms): hồ sơ `fast` chỉ ngắn hơn về thời lượng.
    profile.harness.input_lead_ticks = 3;
    let config = RunConfig {
        profile,
        clients: 2,
        seed: 1,
        rounds: Some(1),
    };
    let report = run(&config).expect("chạy harness với mạng scope-v1");

    assert!(!report.timed_out, "hết ngân sách thời gian: {report:?}");
    assert!(report.converged, "lượt không hội tụ: {report:?}");
    assert!(report.input.sent > 0, "bot phải tạo/gửi input: {report:?}");
    assert!(
        report.input.resolved > 0,
        "server phải giải quyết input: {report:?}"
    );
    let applied = report.input.resolved - report.input.retired;
    assert!(
        applied > 0,
        "phải có command thực sự được áp dụng, không chỉ retire: {report:?}"
    );
    assert!(
        report.input.retired * 10 <= report.input.resolved,
        "gần hết command bị retire dù đã đủ cửa sổ input: {report:?}"
    );
    assert!(
        report.latency_ms.count > 0,
        "command được áp dụng phải có mẫu latency: {report:?}"
    );
}

/// Command hợp lệ có hướng và cạnh bom phải được giữ nguyên payload qua resend, và khi
/// merge/step bằng core (cùng hàm server dùng) phải thực sự di chuyển và đặt bom.
#[test]
fn applied_command_moves_and_places_bomb_in_core() {
    use boom_core::{Dir, PlayerInput, SimulationState};
    use boom_proto::InputBits;

    let direction = InputBits::new(boom_proto::dto::DIR_RIGHT, true).expect("bits hợp lệ");
    assert!(direction.place_bomb(), "bit cạnh bom phải được giữ");
    let command = boom_proto::InputCommand {
        seq: 1,
        target_tick: 7,
        bits: direction,
    };
    let merged = boom_sync::merge_commands(&[command]).expect("merge command");
    assert_eq!(
        merged,
        PlayerInput {
            dir: Dir::Right,
            place_bomb: true,
        }
    );

    let profile = boom_content::rules::RulesProfile::from_toml(
        boom_qa::profile::builtin_rules("qa-fast").expect("rules qa-fast"),
    )
    .expect("profile rules");
    let rules = boom_core::Rules::from_profile(&profile).expect("rules");
    let rows = boom_qa::profile::map_rows(
        boom_qa::profile::builtin_map("qa-village").expect("map qa-village"),
    );
    let terrain = boom_content::map::Terrain::from_ascii(&rows).expect("terrain");
    let mode = boom_core::mode_by_id("battle_ffa").expect("mode");
    let world = boom_core::World::new(terrain, rules, mode).expect("world");
    let mut state = SimulationState::new_round(
        &world,
        &[
            boom_core::PlayerSetup {
                id: 0,
                stats: boom_core::CharacterStats {
                    speed: boom_core::Stat::new(1, 5),
                    bombs: boom_core::Stat::new(1, 5),
                    range: boom_core::Stat::new(1, 5),
                },
            },
            boom_core::PlayerSetup {
                id: 1,
                stats: boom_core::CharacterStats {
                    speed: boom_core::Stat::new(1, 5),
                    bombs: boom_core::Stat::new(1, 5),
                    range: boom_core::Stat::new(1, 5),
                },
            },
        ],
        7,
    )
    .expect("round");
    let before = state.players[0].x;
    state
        .step(
            &world,
            &[
                PlayerInput {
                    dir: Dir::Right,
                    place_bomb: true,
                },
                PlayerInput::default(),
            ],
            &mut Vec::new(),
        )
        .expect("step");
    assert!(
        state.players[0].x > before,
        "command Right phải làm người chơi di chuyển"
    );
    assert_eq!(state.bombs.len(), 1, "cạnh bom phải đặt đúng một bom");
}

/// Hồ sơ `scope-v1` là hồ sơ nghiệm thu: RTT 100 ms, jitter 0–30 ms, mất/trùng/đảo 1 %.
///
/// Test này chỉ kiểm tra **cấu hình** hồ sơ (không chạy mạng) để giữ CI nhanh; chạy thật
/// hồ sơ `scope-v1` là việc nghiệm thu V1-027, xem `docs/specification/QA_NETWORK.md`.
#[test]
fn scope_v1_profile_matches_acceptance_network() {
    let profile = QaProfile::builtin("scope-v1").expect("hồ sơ scope-v1");
    assert_eq!(profile.network.rtt_ms, 100, "SCOPE_V1 §7.6: RTT 100 ms");
    assert_eq!(
        profile.network.jitter_ms, 30,
        "SCOPE_V1 §7.6: jitter 0–30 ms"
    );
    assert_eq!(profile.network.loss_permille, 10, "SCOPE_V1 §7.6: mất 1 %");
    assert_eq!(
        profile.network.duplicate_permille, 10,
        "trùng frame có kiểm soát"
    );
    assert_eq!(
        profile.network.reorder_permille, 10,
        "đảo thứ tự có kiểm soát"
    );
    assert_eq!(
        profile.network.one_way_delay_ms(),
        50,
        "trễ một chiều = RTT/2"
    );
}

/// Bot **thay thế** cho `DirectDtoBot`: bọc bot gốc và đếm số lần harness gọi vào bot.
///
/// Đây là implementation "bên ngoài" mà harness phải chạy được: nó chỉ biết trait
/// `BotClient` công khai, không có quyền vào nội bộ của harness.
struct CountingBot {
    inner: DirectDtoBot,
    frames: Arc<AtomicUsize>,
    ticks: Arc<AtomicUsize>,
}

impl BotClient for CountingBot {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn on_frame(&mut self, now_ms: u64, frame: &[u8]) -> Result<Vec<Vec<u8>>, BotError> {
        self.frames.fetch_add(1, Ordering::Relaxed);
        self.inner.on_frame(now_ms, frame)
    }

    fn on_tick(&mut self, now_ms: u64) -> Result<Vec<Vec<u8>>, BotError> {
        self.ticks.fetch_add(1, Ordering::Relaxed);
        self.inner.on_tick(now_ms)
    }

    fn stats(&self) -> &BotStats {
        self.inner.stats()
    }

    fn phase(&self) -> BotPhase {
        self.inner.phase()
    }

    fn reset(&mut self, player: u64) {
        self.inner.reset(player);
    }
}

/// Factory dựng [`CountingBot`] thay cho `DirectDtoBot`.
struct CountingFactory {
    frames: Arc<AtomicUsize>,
    ticks: Arc<AtomicUsize>,
}

impl BotFactory for CountingFactory {
    fn create(&self, config: &BotConfig) -> Box<dyn BotClient> {
        Box::new(CountingBot {
            inner: DirectDtoBot::new(config.clone()),
            frames: Arc::clone(&self.frames),
            ticks: Arc::clone(&self.ticks),
        })
    }
}

/// Harness phải chạy được với implementation bot **bên ngoài** (không phải `DirectDtoBot`):
/// factory được truyền vào từ integration test, và bot đó vẫn chơi trọn lượt.
#[test]
fn harness_runs_an_external_bot_implementation() {
    let profile = QaProfile::builtin("fast").expect("hồ sơ fast");
    let config = RunConfig {
        profile,
        clients: 2,
        seed: 7,
        rounds: Some(1),
    };
    let frames = Arc::new(AtomicUsize::new(0));
    let ticks = Arc::new(AtomicUsize::new(0));
    let factory = CountingFactory {
        frames: Arc::clone(&frames),
        ticks: Arc::clone(&ticks),
    };

    let report = run_with(&config, &factory).expect("chạy harness với bot thay thế");

    assert!(!report.timed_out, "hết ngân sách thời gian: {report:?}");
    assert_eq!(report.rounds.len(), 1, "phải có đúng một lượt");
    assert!(
        report.converged,
        "lượt không hội tụ với bot thay thế: {report:?}"
    );
    assert_eq!(
        report.rounds[0].bot_hashes.len(),
        2,
        "mọi bot thay thế phải dựng lại được hash: {report:?}",
    );
    assert!(
        frames.load(Ordering::Relaxed) > 0,
        "harness phải đẩy frame vào bot thay thế: {report:?}"
    );
    assert!(
        ticks.load(Ordering::Relaxed) > 0,
        "harness phải gọi `on_tick` của bot thay thế: {report:?}"
    );
}
