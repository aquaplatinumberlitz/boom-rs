use boom_content::hash::Fnv1a64;

use crate::mode::Outcome;
use crate::num::Tick;

/// ID ổn định của event: `(tick, thứ tự trong tick)`. Cùng input → cùng ID, nên consumer
/// (âm thanh, thống kê, phần thưởng) dedup được khi replay/reconcile (ARCHITECTURE §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId {
    pub tick: Tick,
    pub seq: u16,
}

/// Loại event. `non_exhaustive`: thêm loại mới không làm vỡ consumer bên ngoài core;
/// consumer phải bỏ qua loại nó không biết.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventKind {
    PlayerTurned { player: u8 },
    RoundEnded { outcome: Outcome },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimulationEvent {
    pub id: EventId,
    pub kind: EventKind,
}

/// Nơi system ghi event trong một tick; cấp ID theo thứ tự ghi.
#[derive(Debug)]
pub struct EventSink<'a> {
    tick: Tick,
    next: u16,
    out: &'a mut Vec<SimulationEvent>,
}

impl<'a> EventSink<'a> {
    pub(crate) fn new(tick: Tick, out: &'a mut Vec<SimulationEvent>) -> Self {
        Self { tick, next: 0, out }
    }

    pub fn emit(&mut self, kind: EventKind) {
        let id = EventId {
            tick: self.tick,
            seq: self.next,
        };
        self.next = self
            .next
            .checked_add(1)
            .expect("quá nhiều event trong một tick");
        self.out.push(SimulationEvent { id, kind });
    }
}

/// Digest của chuỗi event trên encoding chuẩn, để golden replay bắt cả thay đổi event
/// (vd. âm thanh/hiệu ứng sẽ phát) chứ không chỉ state.
pub fn event_digest(events: &[SimulationEvent]) -> u64 {
    let mut h = Fnv1a64::default();
    h.u32(u32::try_from(events.len()).expect("quá nhiều event"));
    for e in events {
        h.u32(e.id.tick).u32(u32::from(e.id.seq));
        match &e.kind {
            EventKind::PlayerTurned { player } => h.u8(2).u8(*player),
            EventKind::RoundEnded { outcome } => match outcome {
                Outcome::Draw => h.u8(4).u8(0),
                Outcome::Winner(id) => h.u8(4).u8(1).u8(*id),
            },
        };
    }
    h.finish()
}
