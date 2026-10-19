//! Nghiệm thu chuyển `InputCommand` → `PlayerInput` (ADR 0009 §3.1, §3.3 bước 7).

use boom_core::{Dir, PlayerInput};
use boom_proto::{InputBits, InputCommand};
use boom_sync::{SyncError, command_input, merge_commands};

fn command(seq: u32, bits: InputBits) -> InputCommand {
    InputCommand {
        seq,
        target_tick: 100 + seq,
        bits,
    }
}

#[test]
fn command_becomes_player_input() {
    let input = command_input(&command(1, InputBits::new(4, true).unwrap())).unwrap();
    assert_eq!(input.dir, Dir::Right);
    assert!(input.place_bomb);
}

#[test]
fn every_direction_code_maps_back() {
    for (code, dir) in [
        (0u8, Dir::None),
        (1, Dir::Up),
        (2, Dir::Down),
        (3, Dir::Left),
        (4, Dir::Right),
    ] {
        let input = command_input(&command(1, InputBits::new(code, false).unwrap())).unwrap();
        assert_eq!(input.dir, dir, "mã hướng {code}");
    }
}

#[test]
fn reserved_bits_are_rejected() {
    let err = command_input(&command(1, InputBits(0b0001_0000))).unwrap_err();
    assert!(matches!(err, SyncError::ReservedBits(16)), "thực tế: {err}");
}

#[test]
fn unknown_direction_is_rejected() {
    let err = command_input(&command(1, InputBits(5))).unwrap_err();
    assert!(matches!(err, SyncError::BadDirection(5)), "thực tế: {err}");
}

#[test]
fn merge_uses_newest_direction_and_any_bomb_edge() {
    // ADR 0009 §3.3 bước 7: hướng của lệnh mới nhất, `place_bomb` nếu có ít nhất một cạnh bom.
    let merged = merge_commands(&[
        command(1, InputBits::new(1, true).unwrap()),
        command(2, InputBits::new(3, false).unwrap()),
    ])
    .unwrap();
    assert_eq!(merged.dir, Dir::Left);
    assert!(merged.place_bomb);
}

#[test]
fn merge_of_empty_list_is_neutral() {
    let merged = merge_commands(&[]).unwrap();
    assert_eq!(merged, PlayerInput::default());
}

#[test]
fn merge_rejects_invalid_command() {
    let err = merge_commands(&[command(1, InputBits(0b0010_0000))]).unwrap_err();
    assert!(matches!(err, SyncError::ReservedBits(32)), "thực tế: {err}");
}
