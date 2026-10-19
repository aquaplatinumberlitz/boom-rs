//! Dự đoán và reconciliation phía client (ADR 0005, ADR 0009 §"Tác động tới prediction/reconciliation").
//!
//! Ba quy tắc của ADR 0005 được cài ở đây:
//!
//! 1. Client giữ **state confirmed** = snapshot mới nhất, cùng danh sách lệnh chưa ack.
//! 2. Khi có snapshot: bỏ lệnh đã ack (`seq <= acks[player].last_seq`), rồi **replay** các lệnh
//!    còn lại bằng `boom-core`: chỉ di chuyển của mình được dự đoán, người khác giữ nguyên
//!    theo snapshot (không ngoại suy).
//! 3. Bom chỉ là "bóng chờ" ở presentation; nổ/nhốt/chết/item chỉ hiện theo event của server.
//!    Vì vậy event sinh ra khi dự đoán **bị bỏ**, không đẩy ra presentation journal.
//!
//! Về `target_tick` (ADR 0009 §3.2): mỗi `seq` được gán `target_tick` **một lần** và không đổi
//! sau đó, nên payload bất biến khi resend. Công thức cụ thể hoá ở đây là
//! `target_tick = tick của state dự đoán hiện tại`; công thức xấp xỉ trong ADR
//! (`snapshot.tick + số tick local kể từ snapshot + 1`) không tính lệnh đang chờ nên dễ sinh
//! `target_tick` trùng hoặc giảm theo `seq` — điều ADR 0009 §3.4(e) cấm.
//!
//! Client tạo **một lệnh mỗi tick** (hướng đang giữ + cạnh bom của tick đó), nên cửa sổ 8 lệnh
//! (ADR 0009 §3.5 `INPUT_REDUNDANCY` ≈ 133 ms) phủ RTT: lệnh đã gửi mà chưa ack vẫn nằm trong
//! cửa sổ và được resend **y nguyên payload** (không đổi `target_tick`).

use std::fmt;

use boom_content::map::CellKind;
use boom_core::{CoreError, Dir, Mode, PlayerInput, SimulationState, World};
use boom_proto::dto::snapshot::CellKind as CellKindDto;
use boom_proto::{InputBits, InputCommand, Snapshot};
use boom_sync::{SyncError, from_snapshot, merge_commands};

/// Lệnh input chưa được server ack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingCommand {
    /// Sequence của lệnh; bắt đầu từ 1 cho mỗi `(round_id, player)` (ADR 0009 §3.1).
    pub seq: u32,
    /// Tick authority mà lệnh được gán **một lần** và không đổi sau đó.
    pub target_tick: u32,
    /// Byte hướng + cạnh bom.
    pub bits: InputBits,
}

impl PendingCommand {
    /// Đổi sang DTO để đóng gói batch.
    pub fn to_command(self) -> InputCommand {
        InputCommand {
            seq: self.seq,
            target_tick: self.target_tick,
            bits: self.bits,
        }
    }
}

/// Kết quả áp snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotOutcome {
    /// Snapshot đã áp: state confirmed thay bằng state mới, lệnh đã ack bị bỏ, lệnh còn lại
    /// được replay.
    Applied {
        /// Sequence cuối cùng server đã giải quyết (đã áp dụng **hoặc** retire).
        acked_seq: u32,
    },
    /// Snapshot cũ hoặc trùng: bỏ qua, không đụng state.
    Stale,
}

/// Lỗi của tầng dự đoán.
#[derive(Debug)]
pub enum PredictError {
    /// Dựng lại state từ snapshot thất bại (bất biến contract hoặc miền giá trị).
    Sync(SyncError),
    /// Lỗi core khi chạy tick dự đoán.
    Core(CoreError),
    /// Chưa có snapshot nào: client chưa biết state để dự đoán.
    NoConfirmedState,
    /// Người chơi của client không có trong state của server.
    PlayerMissing(u8),
    /// Gói loại ô của snapshot không khớp kích thước lưới.
    FieldShape,
}

impl fmt::Display for PredictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sync(e) => write!(f, "lỗi đồng bộ DTO: {e}"),
            Self::Core(e) => write!(f, "lỗi core: {e}"),
            Self::NoConfirmedState => write!(f, "chưa có snapshot nào để dự đoán"),
            Self::PlayerMissing(id) => write!(f, "người chơi {id} không có trong state"),
            Self::FieldShape => write!(f, "gói loại ô của snapshot không khớp lưới"),
        }
    }
}

impl std::error::Error for PredictError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sync(e) => Some(e),
            Self::Core(e) => Some(e),
            _ => None,
        }
    }
}

impl From<SyncError> for PredictError {
    fn from(value: SyncError) -> Self {
        Self::Sync(value)
    }
}

impl From<CoreError> for PredictError {
    fn from(value: CoreError) -> Self {
        Self::Core(value)
    }
}

/// Dự đoán di chuyển của mình và reconciliation khi có snapshot.
pub struct Predictor {
    world: World,
    player_id: u8,
    /// State confirmed: đúng state server gửi trong snapshot mới nhất.
    confirmed: Option<SimulationState>,
    /// State dự đoán: state confirmed + các lệnh chưa ack đã replay.
    predicted: Option<SimulationState>,
    /// Lệnh chưa ack, theo `seq` tăng dần.
    pending: Vec<PendingCommand>,
    /// Sequence kế tiếp cấp; bắt đầu từ 1 cho mỗi `(round_id, player)`.
    next_seq: u32,
    /// `snapshot_seq` lớn nhất đã áp.
    last_snapshot_seq: u32,
    /// Round hiện tại.
    round_id: Option<u32>,
}

impl Predictor {
    /// Dựng predictor cho người chơi `player_id` trong `world`.
    ///
    /// Chưa có state nào cho tới khi snapshot đầu tiên tới (`confirmed`/`predicted` là `None`).
    pub fn new(world: World, player_id: u8) -> Self {
        Self {
            world,
            player_id,
            confirmed: None,
            predicted: None,
            pending: Vec::new(),
            next_seq: 1,
            last_snapshot_seq: 0,
            round_id: None,
        }
    }

    /// State confirmed (snapshot mới nhất), nếu đã có.
    pub fn confirmed_state(&self) -> Option<&SimulationState> {
        self.confirmed.as_ref()
    }

    /// State dự đoán (confirmed + lệnh chưa ack đã replay), nếu đã có.
    pub fn predicted_state(&self) -> Option<&SimulationState> {
        self.predicted.as_ref()
    }

    /// Lệnh chưa ack còn giữ.
    pub fn pending(&self) -> &[PendingCommand] {
        &self.pending
    }

    /// Áp snapshot: đây là mốc **correction** của client.
    ///
    /// - Snapshot cũ/trùng hoặc của round khác thì bỏ qua.
    /// - State confirmed thay bằng state dựng từ snapshot (đã qua `check_invariants` của core).
    /// - Lệnh đã được server giải quyết (`seq <= acks[player].last_seq`) bị bỏ.
    /// - Lệnh còn lại được replay để dự đoán tiếp.
    pub fn apply_snapshot(&mut self, snapshot: &Snapshot) -> Result<SnapshotOutcome, PredictError> {
        if let Some(round) = self.round_id
            && snapshot.round_id != round
        {
            // Round khác: sequence của client bắt đầu lại từ 1 cho mỗi round.
            self.reset_round();
        }
        if snapshot.snapshot_seq <= self.last_snapshot_seq {
            return Ok(SnapshotOutcome::Stale);
        }
        let state = from_snapshot(snapshot, &self.world)?;
        let acked_seq = snapshot
            .acks
            .iter()
            .find(|ack| ack.player == self.player_id)
            .map_or(0, |ack| ack.last_seq);
        // Ack là sequence đã **giải quyết** (áp dụng hoặc retire): bỏ mọi lệnh `seq <=` giá trị này.
        self.pending.retain(|command| command.seq > acked_seq);
        self.confirmed = Some(state);
        self.last_snapshot_seq = snapshot.snapshot_seq;
        self.round_id = Some(snapshot.round_id);
        self.replay()?;
        Ok(SnapshotOutcome::Applied { acked_seq })
    }

    /// Bỏ lệnh đã được server giải quyết (dùng khi nhận `InputRejected`, vì reject cũng mang
    /// `applied_seq`).
    pub fn acknowledge(&mut self, applied_seq: u32) {
        self.pending.retain(|command| command.seq > applied_seq);
    }

    /// Dự đoán một bước của mình: sinh lệnh mới và chạy `boom-core` một tick.
    ///
    /// Lệnh mới nhận `seq` kế tiếp và `target_tick` bằng tick hiện tại của state dự đoán; giá trị
    /// này **không đổi** sau đó (payload bất biến khi resend, ADR 0009 §3.2).
    ///
    /// Trả về `seq` của lệnh vừa sinh.
    pub fn predict(&mut self, input: PlayerInput) -> Result<u32, PredictError> {
        let target_tick = self
            .predicted
            .as_ref()
            .ok_or(PredictError::NoConfirmedState)?
            .tick;
        let seq = self.next_seq;
        self.next_seq = self
            .next_seq
            .checked_add(1)
            .ok_or(PredictError::NoConfirmedState)?;
        let bits = bits_of(input);
        self.pending.push(PendingCommand {
            seq,
            target_tick,
            bits,
        });
        // Chỉ di chuyển của mình được dự đoán; người khác giữ theo state hiện tại (ADR 0005).
        self.step(input)?;
        Ok(seq)
    }

    /// Cửa sổ lệnh chưa ack để đóng gói batch.
    ///
    /// Bắt đầu từ lệnh chưa ack nhỏ nhất còn giữ, tối đa `max` lệnh **liên tục** (ADR 0009
    /// §3.2): lệnh đã gửi nhưng chưa ack vẫn nằm trong cửa sổ để resend y nguyên payload.
    pub fn window(&self, max: usize) -> Vec<InputCommand> {
        let mut out: Vec<InputCommand> = Vec::new();
        for command in &self.pending {
            if out.len() >= max {
                break;
            }
            if let Some(last) = out.last()
                && last.seq.checked_add(1) != Some(command.seq)
            {
                // Lỗ hổng sequence: dừng, không gửi batch sai cấu trúc (ADR 0009 §3.1).
                break;
            }
            out.push(command.to_command());
        }
        out
    }

    fn reset_round(&mut self) {
        self.pending.clear();
        self.next_seq = 1;
        self.last_snapshot_seq = 0;
    }

    /// Chạy một tick của state dự đoán với input của mình.
    fn step(&mut self, input: PlayerInput) -> Result<(), PredictError> {
        let world = self.world.clone();
        let state = self
            .predicted
            .as_mut()
            .ok_or(PredictError::NoConfirmedState)?;
        let index = state
            .players
            .iter()
            .position(|player| player.id == self.player_id)
            .ok_or(PredictError::PlayerMissing(self.player_id))?;
        let mut inputs = vec![PlayerInput::default(); state.players.len()];
        inputs[index] = input;
        let mut events = Vec::new();
        // Event sinh ra khi dự đoán **không** được trình bày (ADR 0005): nổ/nhốt/chết chỉ
        // hiện theo event của server.
        state.step(&world, &inputs, &mut events)?;
        Ok(())
    }

    /// Dựng lại state dự đoán từ state confirmed + các lệnh chưa ack.
    ///
    /// Replay bám theo scheduler của server (ADR 0009 §3.3 bước 7): ở tick `t`, các lệnh đã
    /// chín (`target_tick <= t`) được **gộp** thành một `PlayerInput` cho bước đó; tick không có
    /// lệnh thì input neutral cho mình, người khác giữ theo snapshot.
    fn replay(&mut self) -> Result<(), PredictError> {
        let Some(confirmed) = self.confirmed.clone() else {
            return Ok(());
        };
        let world = self.world.clone();
        let mut state = confirmed;
        let mut tick = state.tick;
        let mut events = Vec::new();
        let mut index = 0;
        while index < self.pending.len() {
            // Lệnh đã chín ở tick hiện tại: gộp thành một input cho bước này.
            let ripe = self.pending[index..]
                .iter()
                .take_while(|command| command.target_tick <= tick)
                .count();
            if ripe > 0 {
                let commands: Vec<InputCommand> = self.pending[index..index + ripe]
                    .iter()
                    .map(|command| command.to_command())
                    .collect();
                index += ripe;
                let merged = merge_commands(&commands)?;
                let inputs = self.inputs_for(&state, merged)?;
                state.step(&world, &inputs, &mut events)?;
                tick += 1;
                continue;
            }
            // Chưa lệnh nào chín: chạy các tick trống tới tick của lệnh kế tiếp.
            let next = self.pending[index].target_tick;
            while tick < next {
                let inputs = self.inputs_for(&state, PlayerInput::default())?;
                state.step(&world, &inputs, &mut events)?;
                tick += 1;
            }
        }
        self.predicted = Some(state);
        Ok(())
    }

    /// Input của mọi người chơi cho một tick: của mình là `own`, người khác giữ theo snapshot.
    fn inputs_for(
        &self,
        state: &SimulationState,
        own: PlayerInput,
    ) -> Result<Vec<PlayerInput>, PredictError> {
        let index = state
            .players
            .iter()
            .position(|player| player.id == self.player_id)
            .ok_or(PredictError::PlayerMissing(self.player_id))?;
        let mut inputs = vec![PlayerInput::default(); state.players.len()];
        inputs[index] = own;
        Ok(inputs)
    }
}

/// Dựng `World` cho client từ snapshot đầu tiên.
///
/// Client chưa có bundle map (P4) nên địa hình lấy từ chính snapshot: kích thước lưới và luật
/// là thứ quyết định việc dựng lại `SimulationState` (`boom-sync` ghi đè loại ô theo snapshot).
pub fn world_from_snapshot(
    snapshot: &Snapshot,
    rules: &boom_core::Rules,
    mode: &'static dyn Mode,
) -> Result<World, PredictError> {
    let kinds = snapshot.field.kinds().ok_or(PredictError::FieldShape)?;
    let kinds = kinds
        .into_iter()
        .map(|kind| match kind {
            CellKindDto::Floor => CellKind::Floor,
            CellKindDto::Solid => CellKind::Solid,
            CellKindDto::Breakable => CellKind::Breakable,
            CellKindDto::Invalid => CellKind::Unknown,
        })
        .collect();
    let terrain = boom_content::map::Terrain {
        width: snapshot.field.width,
        height: snapshot.field.height,
        kinds,
        // Client không dựng lượt nên không cần spawn; spawn thuộc content của server.
        spawns: Vec::new(),
    };
    Ok(World::new(terrain, rules.clone(), mode)?)
}

/// Đổi input của core sang byte `bits` trên dây (ADR 0009 §3.1).
fn bits_of(input: PlayerInput) -> InputBits {
    let direction = match input.dir {
        Dir::None => boom_proto::dto::DIR_NONE,
        Dir::Up => boom_proto::dto::DIR_UP,
        Dir::Down => boom_proto::dto::DIR_DOWN,
        Dir::Left => boom_proto::dto::DIR_LEFT,
        Dir::Right => boom_proto::dto::DIR_RIGHT,
    };
    InputBits::new(direction, input.place_bomb).expect("mã hướng của core luôn hợp lệ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use boom_content::rules::RulesProfile;
    use boom_core::{CharacterStats, Stat, mode_by_id};

    const RULES: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

    fn world() -> World {
        let profile = RulesProfile::from_toml(RULES).expect("profile");
        let rules = boom_core::Rules::from_profile(&profile).expect("rules");
        let rows = ["#######", "#0.1.2#", "#.....#", "#3....#", "#######"];
        let terrain = boom_content::map::Terrain::from_ascii(&rows).expect("terrain");
        World::new(terrain, rules, mode_by_id("battle_ffa").expect("mode")).expect("world")
    }

    fn stats() -> CharacterStats {
        CharacterStats {
            speed: Stat::new(1, 5),
            bombs: Stat::new(1, 5),
            range: Stat::new(1, 5),
        }
    }

    /// Dựng snapshot từ state của core (giống server làm) để test predictor không cần mạng.
    fn snapshot_of(state: &SimulationState, round_id: u32, seq: u32) -> Snapshot {
        let ctx = boom_sync::SnapshotContext {
            snapshot_seq: seq,
            round_id,
            state_schema: 1,
            rules_hash: 0,
            content_epoch: 1,
            acks: Vec::new(),
        };
        boom_sync::to_snapshot(state, &ctx).expect("snapshot")
    }

    #[test]
    fn window_bat_dau_tu_lenh_chua_ack_nho_nhat() {
        let mut predictor = Predictor::new(world(), 0);
        let state = boom_core::SimulationState::new_round(
            &world(),
            &[boom_core::PlayerSetup {
                id: 0,
                stats: stats(),
            }],
            7,
        )
        .expect("round");
        let snapshot = snapshot_of(&state, 1, 1);
        predictor.apply_snapshot(&snapshot).expect("snapshot");
        for _ in 0..3 {
            predictor
                .predict(PlayerInput {
                    dir: Dir::Right,
                    place_bomb: false,
                })
                .expect("predict");
        }
        let window = predictor.window(8);
        assert_eq!(window.len(), 3);
        assert_eq!(window[0].seq, 1);
        assert_eq!(window[2].seq, 3);
        // `target_tick` không giảm theo `seq` (ADR 0009 §3.4(e)).
        assert!(window[0].target_tick <= window[1].target_tick);
        assert!(window[1].target_tick <= window[2].target_tick);
    }

    #[test]
    fn ack_bo_lenh_da_giai_quyet() {
        let mut predictor = Predictor::new(world(), 0);
        let state = boom_core::SimulationState::new_round(
            &world(),
            &[boom_core::PlayerSetup {
                id: 0,
                stats: stats(),
            }],
            7,
        )
        .expect("round");
        let snapshot = snapshot_of(&state, 1, 1);
        predictor.apply_snapshot(&snapshot).expect("snapshot");
        for _ in 0..3 {
            predictor
                .predict(PlayerInput {
                    dir: Dir::Right,
                    place_bomb: false,
                })
                .expect("predict");
        }
        predictor.acknowledge(2);
        assert_eq!(predictor.pending().len(), 1);
        assert_eq!(predictor.pending()[0].seq, 3);
    }

    #[test]
    fn snapshot_cu_bi_bo_qua() {
        let mut predictor = Predictor::new(world(), 0);
        let state = boom_core::SimulationState::new_round(
            &world(),
            &[boom_core::PlayerSetup {
                id: 0,
                stats: stats(),
            }],
            7,
        )
        .expect("round");
        predictor
            .apply_snapshot(&snapshot_of(&state, 1, 5))
            .expect("snapshot");
        assert_eq!(
            predictor
                .apply_snapshot(&snapshot_of(&state, 1, 4))
                .unwrap(),
            SnapshotOutcome::Stale
        );
    }
}
