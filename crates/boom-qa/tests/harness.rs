//! Test tích hợp của harness QA: chạy kịch bản headless thật (server + N bot qua
//! `MemoryTransport` có mất/trùng/đảo thứ tự) và kiểm tra state hash hội tụ.
//!
//! Test chạy trong `cargo xtask ci` (qua `cargo test --workspace`); dùng hồ sơ `fast` để
//! lượt ngắn.

use boom_qa::harness::{RunConfig, run};
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
