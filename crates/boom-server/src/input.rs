//! Scheduler input nội bộ — ngữ nghĩa ADR 0009 §3.3 (dedup trước cửa sổ, retire, gap timeout).
//!
//! Đây không phải DTO mạng. Phần 2 sẽ thay `InternalInputBatch`/`InternalInputCommand` bằng DTO
//! `boom-proto` và adapter `boom-sync`, còn scheduler này giữ nguyên ngữ nghĩa.

use std::collections::BTreeMap;
use std::fmt;

use boom_core::{Dir, PlayerInput, Tick};
use boom_proto::{InputCommand, ProtoError};
use boom_room::{PlayerId, RoundId};
use boom_sync::SyncError;

use crate::config::InputTicks;

/// Ack scheduler sau khi xử lý batch/tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputAck {
    pub applied_seq: u32,
    pub server_tick: Tick,
    /// Sequence bị retire trong lần xử lý này (expired hoặc gap), để replay/log.
    pub retired: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputError {
    NotPlaying,
    Proto(ProtoError),
    Sync(SyncError),
    WrongRound {
        expected: RoundId,
        received: RoundId,
    },
    UnknownPlayer(PlayerId),
    EmptyBatch,
    TooManyCommands {
        count: usize,
        max: u8,
    },
    NonContiguousSequence {
        expected: u32,
        received: u32,
    },
    /// Cùng sequence nhưng payload khác bản đang pending (ADR 0009 §3.3 bước 2).
    BadSequence {
        seq: u32,
    },
    /// `target_tick` quá tương lai; batch bị từ chối atomic.
    OutOfWindow {
        seq: u32,
        target_tick: u32,
        server_tick: Tick,
        lead: Tick,
    },
    /// `seq` vượt `applied_seq + MAX_INPUT_GAP`.
    GapTooLarge {
        seq: u32,
        applied_seq: u32,
        max_gap: u32,
    },
    RateLimited {
        max_per_second: u32,
    },
    RoundIdOutOfRange {
        round: RoundId,
    },
    PlayerMismatch {
        expected: u8,
        received: u8,
    },
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPlaying => write!(f, "không ở pha chơi"),
            Self::Proto(e) => write!(f, "batch input sai DTO: {e}"),
            Self::Sync(e) => write!(f, "batch input không đổi được sang PlayerInput: {e}"),
            Self::WrongRound { expected, received } => {
                write!(f, "sai round: cần {expected:?}, nhận {received:?}")
            }
            Self::RoundIdOutOfRange { round } => {
                write!(f, "round {round:?} vượt miền u32 của DTO")
            }
            Self::PlayerMismatch { expected, received } => {
                write!(
                    f,
                    "batch khai player {received}, session là slot {expected}"
                )
            }
            Self::UnknownPlayer(player) => write!(f, "không có người chơi {player:?} trong trận"),
            Self::EmptyBatch => write!(f, "batch rỗng"),
            Self::TooManyCommands { count, max } => {
                write!(f, "batch có {count} lệnh, tối đa {max}")
            }
            Self::NonContiguousSequence { expected, received } => {
                write!(
                    f,
                    "sequence không liên tục: cần {expected}, nhận {received}"
                )
            }
            Self::BadSequence { seq } => write!(f, "sequence {seq} đã có payload khác"),
            Self::OutOfWindow {
                seq,
                target_tick,
                server_tick,
                lead,
            } => write!(
                f,
                "sequence {seq} target {target_tick} quá tương lai (server {server_tick}, lead {lead})"
            ),
            Self::GapTooLarge {
                seq,
                applied_seq,
                max_gap,
            } => write!(
                f,
                "sequence {seq} vượt gap {max_gap} từ applied_seq {applied_seq}"
            ),
            Self::RateLimited { max_per_second } => {
                write!(f, "vượt rate limit {max_per_second} batch/giây")
            }
        }
    }
}

impl From<SyncError> for InputError {
    fn from(value: SyncError) -> Self {
        Self::Sync(value)
    }
}

impl From<ProtoError> for InputError {
    fn from(value: ProtoError) -> Self {
        Self::Proto(value)
    }
}

impl std::error::Error for InputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Proto(e) => Some(e),
            Self::Sync(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pending {
    target_tick: u32,
    command: InputCommand,
}

/// Trạng thái scheduler của một `(round, player)`.
#[derive(Clone, Debug)]
pub(crate) struct InputTrack {
    applied_seq: u32,
    pending: BTreeMap<u32, Pending>,
    gap_since_tick: Option<Tick>,
    last_input_tick: Option<Tick>,
    last_held_dir: Dir,
    batch_window_start_ms: u64,
    batches_in_window: u32,
}

impl InputTrack {
    pub(crate) fn applied_seq(&self) -> u32 {
        self.applied_seq
    }

    pub(crate) fn new() -> Self {
        Self {
            applied_seq: 0,
            pending: BTreeMap::new(),
            gap_since_tick: None,
            last_input_tick: None,
            last_held_dir: Dir::None,
            batch_window_start_ms: 0,
            batches_in_window: 0,
        }
    }

    /// Xử lý batch; trả lỗi thì không đổi state (atomic với lệnh mới).
    pub(crate) fn submit(
        &mut self,
        commands: &[InputCommand],
        server_tick: Tick,
        now_ms: u64,
        limits: InputTicks,
    ) -> Result<InputAck, InputError> {
        if commands.is_empty() {
            return Err(InputError::EmptyBatch);
        }
        if commands.len() > usize::from(limits.max_batch_commands) {
            return Err(InputError::TooManyCommands {
                count: commands.len(),
                max: limits.max_batch_commands,
            });
        }
        if let Some(command) = commands.iter().find(|command| command.seq == 0) {
            return Err(InputError::BadSequence { seq: command.seq });
        }
        for pair in commands.windows(2) {
            if pair[1].seq != pair[0].seq + 1 {
                return Err(InputError::NonContiguousSequence {
                    expected: pair[0].seq + 1,
                    received: pair[1].seq,
                });
            }
        }

        // Rate limit theo cửa sổ 1 giây; batch lỗi cấu trúc không tính.
        if now_ms.saturating_sub(self.batch_window_start_ms) >= 1_000 {
            self.batch_window_start_ms = now_ms;
            self.batches_in_window = 0;
        }
        if self.batches_in_window >= limits.max_batches_per_second {
            return Err(InputError::RateLimited {
                max_per_second: limits.max_batches_per_second,
            });
        }

        // Bước 2: dedup theo seq TRƯỚC khi kiểm cửa sổ.
        let mut fresh: Vec<InputCommand> = Vec::new();
        for command in commands {
            if command.seq <= self.applied_seq {
                continue;
            }
            if let Some(existing) = self.pending.get(&command.seq) {
                if existing.command != *command {
                    return Err(InputError::BadSequence { seq: command.seq });
                }
                continue;
            }
            fresh.push(*command);
        }

        // Bước 3: kiểm lệnh mới; batch atomic, không chèn lệnh nào nếu lỗi.
        for command in &fresh {
            if command.target_tick > server_tick.saturating_add(limits.lead) {
                return Err(InputError::OutOfWindow {
                    seq: command.seq,
                    target_tick: command.target_tick,
                    server_tick,
                    lead: limits.lead,
                });
            }
            if command.seq > self.applied_seq.saturating_add(limits.max_gap) {
                return Err(InputError::GapTooLarge {
                    seq: command.seq,
                    applied_seq: self.applied_seq,
                    max_gap: limits.max_gap,
                });
            }
        }

        // Bước 4: chèn.
        for command in fresh {
            self.pending.insert(
                command.seq,
                Pending {
                    target_tick: command.target_tick,
                    command,
                },
            );
        }
        self.batches_in_window += 1;
        self.mark_gap(server_tick);

        // Bước 5: retire đầu hàng quá hạn ngay khi batch được chấp nhận.
        let retired = self.retire_late(server_tick, limits.late);
        Ok(InputAck {
            applied_seq: self.applied_seq,
            server_tick,
            retired,
        })
    }

    /// Chạy trước mỗi tick: gap timeout rồi retire/áp dụng lệnh đã chín (ADR 0009 bước 6–7).
    pub(crate) fn apply_tick(
        &mut self,
        server_tick: Tick,
        limits: InputTicks,
    ) -> Result<(PlayerInput, Vec<u32>), InputError> {
        let mut retired = Vec::new();

        // Bước 6: gap timeout chỉ retire sequence thiếu.
        if let Some(since) = self.gap_since_tick
            && server_tick.saturating_sub(since) >= limits.gap_timeout
        {
            if let Some((&first_pending, _)) = self.pending.first_key_value() {
                let missing_end = first_pending - 1;
                while self.applied_seq < missing_end {
                    self.applied_seq += 1;
                    retired.push(self.applied_seq);
                }
            }
            self.gap_since_tick = None;
        }

        // Bước 5/7: retire lệnh quá hạn ở đầu hàng, rồi gộp các lệnh đã chín.
        retired.extend(self.retire_late(server_tick, limits.late));

        let mut applied_commands: Vec<InputCommand> = Vec::new();
        loop {
            let next = self.applied_seq.saturating_add(1);
            let Some(command) = self.pending.get(&next).copied() else {
                break;
            };
            if command.target_tick > server_tick {
                break;
            }
            if command.target_tick < server_tick.saturating_sub(limits.late) {
                self.pending.remove(&next);
                self.applied_seq = next;
                retired.push(next);
                continue;
            }
            self.pending.remove(&next);
            self.applied_seq = next;
            applied_commands.push(command.command);
        }
        self.mark_gap(server_tick);

        let mut merged = if applied_commands.is_empty() {
            PlayerInput::default()
        } else {
            boom_sync::merge_commands(&applied_commands)?
        };
        if applied_commands.is_empty() {
            if self
                .last_input_tick
                .is_some_and(|tick| server_tick.saturating_sub(tick) < limits.stale)
            {
                merged.dir = self.last_held_dir;
            } else {
                self.last_held_dir = Dir::None;
            }
        } else {
            self.last_input_tick = Some(server_tick);
            self.last_held_dir = merged.dir;
        }
        Ok((merged, retired))
    }

    fn mark_gap(&mut self, server_tick: Tick) {
        if self.pending.is_empty() {
            self.gap_since_tick = None;
            return;
        }
        if self
            .pending
            .contains_key(&self.applied_seq.saturating_add(1))
        {
            self.gap_since_tick = None;
        } else if self.gap_since_tick.is_none() {
            self.gap_since_tick = Some(server_tick);
        }
    }

    fn retire_late(&mut self, server_tick: Tick, late: Tick) -> Vec<u32> {
        let mut retired = Vec::new();
        while let Some(command) = self
            .pending
            .get(&self.applied_seq.saturating_add(1))
            .copied()
        {
            if command.target_tick >= server_tick.saturating_sub(late) {
                break;
            }
            self.applied_seq += 1;
            self.pending.remove(&self.applied_seq);
            retired.push(self.applied_seq);
        }
        retired
    }
}
