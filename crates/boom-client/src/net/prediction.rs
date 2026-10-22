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
//! **Dự đoán chỉ chạy phần di chuyển** (ADR 0005: "client dự đoán di chuyển của chính mình",
//! "nổ, nhốt, chết, item: chỉ hiển thị theo event server"). State dự đoán = state confirmed
//! **cộng** di chuyển của mình; bom/lửa/gạch/item/nhốt/chết của server giữ nguyên theo
//! snapshot. Nếu dự đoán chạy cả gameplay, client sẽ tự đặt bom thật vào state dự đoán, tự
//! nổ bom của server, tự vỡ gạch, tự nhặt item và tự kết luận mình chết — trước khi server
//! xác nhận. Vì vậy dự đoán dùng `World` riêng với mode chỉ gồm system `movement` của core;
//! đặt bom của mình chỉ là **bóng chờ** ([`BombShadow`]), không vào state dự đoán.
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
//!
//! Về round: sequence input bắt đầu lại từ 1 cho mỗi round, nên chuyển round hợp lệ phải
//! **reset** hàng lệnh chưa ack. Nhưng packet của round cũ tới trễ (sau khi đã sang round mới)
//! **không** được reset: nếu không, lệnh của round mới bị xóa và client lùi về round cũ. Vì vậy
//! [`Predictor::apply_snapshot`] phân biệt hai trường hợp: round mới hơn thì reset, round cũ
//! hơn thì bỏ qua ([`SnapshotOutcome::OldRound`]).
//!
//! Về kết thúc lượt: server là authority về kết quả. Khi snapshot báo lượt đã kết thúc, client
//! **ngừng** sinh lệnh, ngừng gửi và ngừng replay; state confirmed (kết quả) được giữ nguyên và
//! client vẫn nhận message. Client không tự kết luận thắng/thua.

use std::fmt;
use std::sync::OnceLock;

use boom_content::map::CellKind;
use boom_core::{
    CoreError, Dir, Mode, Outcome, Phase, PlayerInput, SimulationState, System, World,
};
use boom_proto::dto::snapshot::CellKind as CellKindDto;
use boom_proto::{InputBits, InputCommand, Snapshot};
use boom_sync::{SyncError, from_snapshot, merge_commands};

/// Tên ổn định của system di chuyển trong core (`boom-core`, `systems/movement.rs`).
///
/// `Mode::systems()` là contract công khai (ARCHITECTURE §5: thứ tự system là luật), nên client
/// lấy đúng system `movement` từ mode thật thay vì tự chép lại luật di chuyển.
const MOVEMENT_SYSTEM: &str = "movement";

/// Alpha nội suy: `ALPHA_ONE` = 1.0 (đúng snapshot mới), `0` = snapshot cũ.
///
/// Dùng số nguyên cố định 16 bit để không đưa float vào tầng dự đoán (ADR 0004).
pub const ALPHA_ONE: u32 = 1 << 16;

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

/// Bóng bom chờ xác nhận — **dữ liệu presentation**, không vào state dự đoán (ADR 0005).
///
/// Bom thật chỉ xuất hiện theo event/snapshot của server; client hiện ngay một bóng chờ ở ô
/// đang đứng. Khi server ack lệnh đặt bom, bóng bị bỏ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BombShadow {
    /// Sequence của lệnh đặt bom (để bỏ bóng khi server ack).
    pub seq: u32,
    /// Toạ độ bóng (mpx), cùng đơn vị với `Player.x/y`.
    pub x: i32,
    /// Toạ độ bóng (mpx), cùng đơn vị với `Player.x/y`.
    pub y: i32,
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
    /// Snapshot cũ hoặc trùng **cùng round**: bỏ qua, không đụng state.
    Stale,
    /// Snapshot thuộc **round cũ** (tới trễ sau khi đã sang round mới): bỏ qua và **không**
    /// reset hàng lệnh, để lệnh của round mới không bị xóa.
    OldRound,
}

/// Lỗi của tầng dự đoán.
#[derive(Debug)]
pub enum PredictError {
    /// Dựng lại state từ snapshot thất bất (bất biến contract hoặc miền giá trị).
    Sync(SyncError),
    /// Lỗi core khi chạy tick dự đoán.
    Core(CoreError),
    /// Chưa có snapshot nào: client chưa biết state để dự đoán.
    NoConfirmedState,
    /// Người chơi của client không có trong state của server.
    PlayerMissing(u8),
    /// Gói loại ô của snapshot không khớp kích thước lưới.
    FieldShape,
    /// Mode của trận không có system di chuyển: không dự đoán được.
    NoMovementSystem,
    /// Client đã dựng mode dự đoán cho mode khác (mỗi tiến trình chỉ một mode).
    ModeMismatch,
}

impl fmt::Display for PredictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sync(e) => write!(f, "lỗi đồng bộ DTO: {e}"),
            Self::Core(e) => write!(f, "lỗi core: {e}"),
            Self::NoConfirmedState => write!(f, "chưa có snapshot nào để dự đoán"),
            Self::PlayerMissing(id) => write!(f, "người chơi {id} không có trong state"),
            Self::FieldShape => write!(f, "gói loại ô của snapshot không khớp lưới"),
            Self::NoMovementSystem => write!(f, "mode không có system di chuyển để dự đoán"),
            Self::ModeMismatch => write!(f, "đã dựng mode dự đoán cho mode khác"),
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

/// Mode dùng cho **dự đoán**: chỉ chạy system di chuyển của core.
///
/// `SimulationState::step` chạy đúng danh sách system của `world.mode()`, nên dự đoán dùng
/// world riêng với mode này: state dự đoán chỉ khác state confirmed ở **di chuyển của mình**,
/// còn bom/lửa/gạch/item/nhốt/chết giữ nguyên theo snapshot của server.
struct PredictionMode {
    /// ID của mode thật mà bộ system được lấy ra (để chặn nhầm mode).
    source_mode: &'static str,
    /// Chỉ system di chuyển.
    systems: &'static [&'static dyn System],
}

impl Mode for PredictionMode {
    fn id(&self) -> &'static str {
        "client_prediction"
    }

    fn systems(&self) -> &'static [&'static dyn System] {
        self.systems
    }

    /// Client không tự kết luận thắng/thua: kết quả do server quyết (ADR 0005).
    fn outcome(&self, _world: &World, _state: &SimulationState) -> Option<Outcome> {
        None
    }
}

static PREDICTION_MODE: OnceLock<PredictionMode> = OnceLock::new();

/// Mode dự đoán dựng từ mode thật của trận (chỉ system di chuyển).
fn prediction_mode(mode: &'static dyn Mode) -> Result<&'static PredictionMode, PredictError> {
    if let Some(existing) = PREDICTION_MODE.get() {
        if existing.source_mode != mode.id() {
            return Err(PredictError::ModeMismatch);
        }
        return Ok(existing);
    }
    let movement = mode
        .systems()
        .iter()
        .copied()
        .find(|system| system.name() == MOVEMENT_SYSTEM)
        .ok_or(PredictError::NoMovementSystem)?;
    // `Mode::systems` trả slice `'static`, nên slice chỉ chứa system di chuyển cũng phải sống
    // suốt đời tiến trình → cấp phát một lần rồi `leak` (một slice rất nhỏ).
    let systems: &'static [&'static dyn System] = Box::leak(vec![movement].into_boxed_slice());
    let _ = PREDICTION_MODE.set(PredictionMode {
        source_mode: mode.id(),
        systems,
    });
    PREDICTION_MODE.get().ok_or(PredictError::ModeMismatch)
}

/// Dự đoán di chuyển của mình và reconciliation khi có snapshot.
pub struct Predictor {
    /// World thật của trận (dùng để dựng lại state từ snapshot và kiểm bất biến).
    world: World,
    /// World dự đoán: cùng địa hình/luật nhưng mode chỉ có system di chuyển.
    predict_world: World,
    player_id: u8,
    /// State confirmed: đúng state server gửi trong snapshot mới nhất.
    confirmed: Option<SimulationState>,
    /// State dự đoán: state confirmed + di chuyển của mình (lệnh chưa ack đã replay).
    predicted: Option<SimulationState>,
    /// Lệnh chưa ack, theo `seq` tăng dần.
    pending: Vec<PendingCommand>,
    /// Bóng bom chờ (presentation), theo `seq` tăng dần.
    shadows: Vec<BombShadow>,
    /// Sequence kế tiếp cấp; bắt đầu từ 1 cho mỗi `(round_id, player)`.
    next_seq: u32,
    /// `snapshot_seq` lớn nhất đã áp.
    last_snapshot_seq: u32,
    /// Round hiện tại.
    round_id: Option<u32>,
    /// Snapshot cũ hơn một bậc (đầu vào nội suy người chơi khác).
    prev_snapshot: Option<Snapshot>,
    /// Snapshot mới nhất (đầu vào nội suy người chơi khác).
    latest_snapshot: Option<Snapshot>,
}

impl Predictor {
    /// Dựng predictor cho người chơi `player_id` trong `world`.
    ///
    /// Chưa có state nào cho tới khi snapshot đầu tiên tới (`confirmed`/`predicted` là `None`).
    pub fn new(world: World, player_id: u8) -> Result<Self, PredictError> {
        let mode = prediction_mode(world.mode())?;
        let predict_world = World::new(world.terrain().clone(), world.rules().clone(), mode)?;
        Ok(Self {
            world,
            predict_world,
            player_id,
            confirmed: None,
            predicted: None,
            pending: Vec::new(),
            shadows: Vec::new(),
            next_seq: 1,
            last_snapshot_seq: 0,
            round_id: None,
            prev_snapshot: None,
            latest_snapshot: None,
        })
    }

    /// State confirmed (snapshot mới nhất), nếu đã có.
    pub fn confirmed_state(&self) -> Option<&SimulationState> {
        self.confirmed.as_ref()
    }

    /// State dự đoán (confirmed + di chuyển của mình), nếu đã có.
    pub fn predicted_state(&self) -> Option<&SimulationState> {
        self.predicted.as_ref()
    }

    /// Lệnh chưa ack còn giữ.
    pub fn pending(&self) -> &[PendingCommand] {
        &self.pending
    }

    /// Bóng bom chờ xác nhận (presentation; không có trong state dự đoán).
    pub fn bomb_shadows(&self) -> &[BombShadow] {
        &self.shadows
    }

    /// Round hiện tại (từ snapshot mới nhất đã áp).
    pub fn round_id(&self) -> Option<u32> {
        self.round_id
    }

    /// Pha lượt theo snapshot mới nhất.
    pub fn phase(&self) -> Option<Phase> {
        self.confirmed.as_ref().map(|state| state.phase)
    }

    /// Lượt còn chơi được không (chưa có state hoặc đã kết thúc thì không).
    ///
    /// Khi lượt đã kết thúc, client **không** sinh lệnh, **không** gửi và **không** replay:
    /// kết quả do server quyết, client giữ state confirmed và vẫn nhận message.
    pub fn is_playing(&self) -> bool {
        self.predicted
            .as_ref()
            .is_some_and(|state| state.phase == Phase::Playing)
    }

    /// Cặp snapshot liên tiếp của round hiện tại: `(cũ, mới)`.
    ///
    /// Đây là **dữ liệu cho presentation** (nội suy người chơi khác, ADR 0005); state authority
    /// vẫn là snapshot của server.
    pub fn interpolation_pair(&self) -> Option<(&Snapshot, &Snapshot)> {
        match (self.prev_snapshot.as_ref(), self.latest_snapshot.as_ref()) {
            (Some(prev), Some(latest)) => Some((prev, latest)),
            _ => None,
        }
    }

    /// Vị trí người chơi `id` tại `alpha` giữa hai snapshot liên tiếp.
    ///
    /// `alpha` tính bằng [`ALPHA_ONE`] là 1.0 (đúng snapshot mới). Alpha lớn hơn được **clamp** ở
    /// snapshot mới: không ngoại suy (ADR 0005 "nội suy giữa hai snapshot, không ngoại suy").
    pub fn interpolated_position(&self, id: u8, alpha: u32) -> Option<(i32, i32)> {
        let (prev, latest) = self.interpolation_pair()?;
        let alpha = alpha.min(ALPHA_ONE);
        let from = prev.players.iter().find(|player| player.id == id)?;
        let to = latest.players.iter().find(|player| player.id == id)?;
        let lerp = |a: i32, b: i32| {
            let delta = i64::from(b) - i64::from(a);
            (i64::from(a) + delta * i64::from(alpha) / i64::from(ALPHA_ONE)) as i32
        };
        Some((lerp(from.x, to.x), lerp(from.y, to.y)))
    }

    /// Vị trí của chính mình trong state dự đoán (mpx).
    fn own_position(&self) -> Option<(i32, i32)> {
        let state = self.predicted.as_ref()?;
        let player = state
            .players
            .iter()
            .find(|player| player.id == self.player_id)?;
        Some((player.x.0, player.y.0))
    }

    /// Áp snapshot: đây là mốc **correction** của client.
    ///
    /// - Snapshot của **round cũ** thì bỏ qua, không reset hàng lệnh của round mới.
    /// - Snapshot **mới hơn** (cùng round) thì bỏ qua, không đụng state.
    /// - Snapshot của **round mới** thì reset hàng lệnh (sequence bắt đầu lại từ 1 mỗi round).
    /// - State confirmed thay bằng state dựng từ snapshot (đã qua `check_invariants` của core).
    /// - Lệnh đã được server giải quyết (`seq <= acks[player].last_seq`) bị bỏ.
    /// - Lệnh còn lại được replay để dự đoán tiếp (chỉ khi lượt còn chơi).
    pub fn apply_snapshot(&mut self, snapshot: &Snapshot) -> Result<SnapshotOutcome, PredictError> {
        if let Some(round) = self.round_id
            && snapshot.round_id != round
        {
            if snapshot.round_id < round {
                // Packet của round cũ tới trễ: bỏ qua, **không** reset session.
                return Ok(SnapshotOutcome::OldRound);
            }
            // Round mới: sequence của client bắt đầu lại từ 1 cho mỗi round.
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
        self.shadows.retain(|shadow| shadow.seq > acked_seq);
        self.confirmed = Some(state);
        self.last_snapshot_seq = snapshot.snapshot_seq;
        self.round_id = Some(snapshot.round_id);
        // Cặp snapshot cho nội suy người chơi khác (ADR 0005).
        self.prev_snapshot = self.latest_snapshot.take();
        self.latest_snapshot = Some(snapshot.clone());
        if self.phase() != Some(Phase::Playing) {
            // Lượt đã kết thúc: giữ kết quả của server, dừng dự đoán.
            self.pending.clear();
            self.shadows.clear();
            self.predicted = self.confirmed.clone();
            return Ok(SnapshotOutcome::Applied { acked_seq });
        }
        self.replay()?;
        Ok(SnapshotOutcome::Applied { acked_seq })
    }

    /// Bỏ lệnh đã được server giải quyết (dùng khi nhận `InputRejected`, vì reject cũng mang
    /// `applied_seq`).
    pub fn acknowledge(&mut self, applied_seq: u32) {
        self.pending.retain(|command| command.seq > applied_seq);
        self.shadows.retain(|shadow| shadow.seq > applied_seq);
    }

    /// Dự đoán một bước của mình: sinh lệnh mới và chạy **phần di chuyển** của mình.
    ///
    /// Lệnh mới nhận `seq` kế tiếp và `target_tick` bằng tick hiện tại của state dự đoán; giá trị
    /// này **không đổi** sau đó (payload bất biến khi resend, ADR 0009 §3.2).
    ///
    /// Trả về `Some(seq)` của lệnh vừa sinh, hoặc `None` khi chưa có state hoặc lượt đã kết thúc
    /// (lúc đó **không** sinh lệnh: server đã quyết kết quả).
    pub fn predict(&mut self, input: PlayerInput) -> Result<Option<u32>, PredictError> {
        let Some(state) = self.predicted.as_ref() else {
            return Ok(None);
        };
        if state.phase != Phase::Playing {
            return Ok(None);
        }
        let target_tick = state.tick;
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
        // Bóng bom chờ: presentation, **không** vào state dự đoán (ADR 0005).
        if input.place_bomb {
            let (x, y) = self.own_position().unwrap_or((0, 0));
            self.shadows.push(BombShadow { seq, x, y });
        }
        // Chỉ di chuyển của mình được dự đoán; người khác giữ theo state hiện tại (ADR 0005).
        self.step(input)?;
        if input.place_bomb {
            // Bom được đặt sau khi di chuyển trong cùng tick, nên bóng lấy vị trí sau bước dự đoán.
            if let Some((x, y)) = self.own_position() {
                self.update_shadow(seq, x, y);
            }
        }
        Ok(Some(seq))
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

    /// Chuyển round: sequence input bắt đầu lại từ 1, lệnh và bóng của round cũ hết hiệu lực.
    fn reset_round(&mut self) {
        self.pending.clear();
        self.shadows.clear();
        self.next_seq = 1;
        self.last_snapshot_seq = 0;
        self.prev_snapshot = None;
        self.latest_snapshot = None;
    }

    /// Cập nhật vị trí bóng bom của lệnh `seq`.
    fn update_shadow(&mut self, seq: u32, x: i32, y: i32) {
        if let Some(shadow) = self.shadows.iter_mut().find(|shadow| shadow.seq == seq) {
            shadow.x = x;
            shadow.y = y;
        }
    }

    /// Chạy một tick dự đoán: **chỉ** system di chuyển, **chỉ** với input của mình.
    ///
    /// `SimulationState::step` với world dự đoán chạy đúng một system (`movement`), nên bom,
    /// lửa, gạch, item, nhốt và chết **không** được chạy trong state dự đoán (ADR 0005).
    /// Event sinh ra khi dự đoán **không** được trình bày.
    fn step(&mut self, input: PlayerInput) -> Result<(), PredictError> {
        let world = self.predict_world.clone();
        let state = self
            .predicted
            .as_mut()
            .ok_or(PredictError::NoConfirmedState)?;
        let inputs = inputs_for(self.player_id, state, input)?;
        let mut events = Vec::new();
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
        if confirmed.phase != Phase::Playing {
            // Lượt đã kết thúc: giữ kết quả của server, không replay lệnh chưa ack.
            self.predicted = Some(confirmed);
            return Ok(());
        }
        let world = self.predict_world.clone();
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
                let group: Vec<InputCommand> = self.pending[index..index + ripe]
                    .iter()
                    .map(|command| command.to_command())
                    .collect();
                let merged = merge_commands(&group)?;
                let inputs = inputs_for(self.player_id, &state, merged)?;
                state.step(&world, &inputs, &mut events)?;
                // Bóng bom chờ của lệnh trong nhóm: cập nhật vị trí sau bước dự đoán.
                if merged.place_bomb
                    && let Some(command) = group.iter().find(|command| command.bits.place_bomb())
                    && let Some(player) = state.players.iter().find(|p| p.id == self.player_id)
                {
                    let (x, y) = (player.x.0, player.y.0);
                    self.update_shadow(command.seq, x, y);
                }
                index += ripe;
                tick += 1;
                continue;
            }
            // Chưa lệnh nào chín: chạy các tick trống tới tick của lệnh kế tiếp.
            let next = self.pending[index].target_tick;
            while tick < next {
                let inputs = inputs_for(self.player_id, &state, PlayerInput::default())?;
                state.step(&world, &inputs, &mut events)?;
                tick += 1;
            }
        }
        self.predicted = Some(state);
        Ok(())
    }
}

/// Input của mọi người chơi cho một tick: của mình là `own`, người khác giữ theo snapshot.
fn inputs_for(
    player_id: u8,
    state: &SimulationState,
    own: PlayerInput,
) -> Result<Vec<PlayerInput>, PredictError> {
    let index = state
        .players
        .iter()
        .position(|player| player.id == player_id)
        .ok_or(PredictError::PlayerMissing(player_id))?;
    let mut inputs = vec![PlayerInput::default(); state.players.len()];
    inputs[index] = own;
    Ok(inputs)
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

    /// Lượt có hai người chơi (slot 0 và 1) để kiểm "người khác giữ nguyên".
    fn two_player_state(seed: u64) -> SimulationState {
        boom_core::SimulationState::new_round(
            &world(),
            &[
                boom_core::PlayerSetup {
                    id: 0,
                    stats: stats(),
                },
                boom_core::PlayerSetup {
                    id: 1,
                    stats: stats(),
                },
            ],
            seed,
        )
        .expect("round")
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

    /// Snapshot có ack cho người chơi `player`.
    fn snapshot_with_ack(
        state: &SimulationState,
        round_id: u32,
        seq: u32,
        player: u8,
        last_seq: u32,
    ) -> Snapshot {
        let mut snapshot = snapshot_of(state, round_id, seq);
        snapshot.acks = vec![boom_proto::dto::snapshot::InputAck {
            player,
            last_seq,
            applied_tick: state.tick,
        }];
        snapshot
    }

    #[test]
    fn window_bat_dau_tu_lenh_chua_ack_nho_nhat() {
        let mut predictor = Predictor::new(world(), 0).expect("predictor");
        let state = two_player_state(7);
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
        let mut predictor = Predictor::new(world(), 0).expect("predictor");
        let state = two_player_state(7);
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
        let mut predictor = Predictor::new(world(), 0).expect("predictor");
        let state = two_player_state(7);
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

    // --- Finding 1: dự đoán chỉ di chuyển của mình, gameplay khác theo confirmed. ---

    #[test]
    fn du_doan_chi_di_chuyen_nguoi_choi_khac_giu_nguyen() {
        let mut predictor = Predictor::new(world(), 0).expect("predictor");
        let state = two_player_state(7);
        predictor
            .apply_snapshot(&snapshot_of(&state, 1, 1))
            .expect("snapshot");
        let before = predictor.confirmed_state().expect("confirmed").clone();
        let other_before = before.player(1).expect("người chơi khác").clone();

        predictor
            .predict(PlayerInput {
                dir: Dir::Right,
                place_bomb: false,
            })
            .expect("predict");

        let predicted = predictor.predicted_state().expect("dự đoán");
        // Người khác giữ nguyên đúng như snapshot.
        let other_after = predicted.player(1).expect("người chơi khác");
        assert_eq!(other_after.x, other_before.x);
        assert_eq!(other_after.y, other_before.y);
        assert_eq!(other_after.life, other_before.life);
        // Chính mình thì đã đi.
        let mine = predicted.player(0).expect("mình");
        assert!(mine.x > before.player(0).expect("mình").x);
        // Không có gameplay nào khác chạy trong state dự đoán.
        assert_eq!(predicted.bombs.len(), before.bombs.len());
        assert_eq!(predicted.flames.len(), before.flames.len());
        assert_eq!(predicted.items.len(), before.items.len());
        assert_eq!(predicted.field, before.field, "gạch không đổi khi dự đoán");
    }

    #[test]
    fn nhan_bom_vao_state_du_doan_khong_bom_that() {
        let mut predictor = Predictor::new(world(), 0).expect("predictor");
        let state = two_player_state(7);
        predictor
            .apply_snapshot(&snapshot_of(&state, 1, 1))
            .expect("snapshot");
        let bombs_before = predictor.confirmed_state().expect("confirmed").bombs.len();

        predictor
            .predict(PlayerInput {
                dir: Dir::None,
                place_bomb: true,
            })
            .expect("predict");

        let predicted = predictor.predicted_state().expect("dự đoán");
        assert_eq!(
            predicted.bombs.len(),
            bombs_before,
            "nhấn bom không được thêm bom thật vào state dự đoán"
        );
        // Bóng chờ là dữ liệu presentation, tách khỏi state dự đoán.
        assert_eq!(predictor.bomb_shadows().len(), 1);
        let shadow = predictor.bomb_shadows()[0];
        let mine = predicted.player(0).expect("mình");
        assert_eq!((shadow.x, shadow.y), (mine.x.0, mine.y.0));
        // Server ack lệnh thì bóng bị bỏ.
        predictor.acknowledge(shadow.seq);
        assert!(predictor.bomb_shadows().is_empty());
    }

    #[test]
    fn bom_co_fuse_khong_no_trong_du_doan() {
        let mut predictor = Predictor::new(world(), 0).expect("predictor");
        let mut state = two_player_state(7);
        // Đặt bom thật vào state confirmed (như server đã xác nhận) sát thời điểm nổ.
        let cell = world().cell_at(state.players[1].x, state.players[1].y);
        let bomb_id = state.ids.next_id();
        state.bombs.push(boom_core::Bomb {
            id: bomb_id,
            owner: 1,
            cell,
            range: 1,
            explodes_at: state.tick + 1,
            grace: Vec::new(),
        });
        predictor
            .apply_snapshot(&snapshot_of(&state, 1, 1))
            .expect("snapshot");

        for _ in 0..5 {
            predictor
                .predict(PlayerInput {
                    dir: Dir::Right,
                    place_bomb: false,
                })
                .expect("predict");
        }
        let predicted = predictor.predicted_state().expect("dự đoán");
        assert_eq!(
            predicted.bombs.len(),
            1,
            "bom của server không được nổ trong state dự đoán"
        );
        assert!(
            predicted.flames.is_empty(),
            "lửa không được sinh trong state dự đoán"
        );
    }

    #[test]
    fn item_khong_bi_nhat_trong_du_doan() {
        let mut predictor = Predictor::new(world(), 0).expect("predictor");
        let mut state = two_player_state(7);
        let cell = world().cell_at(state.players[1].x, state.players[1].y);
        let item_id = state.ids.next_id();
        state.items.push(boom_core::FloorItem {
            id: item_id,
            kind: boom_core::ItemKind(0),
            cell,
        });
        predictor
            .apply_snapshot(&snapshot_of(&state, 1, 1))
            .expect("snapshot");
        let bombs_before = state.players[0].bombs;

        predictor
            .predict(PlayerInput {
                dir: Dir::Right,
                place_bomb: false,
            })
            .expect("predict");

        let predicted = predictor.predicted_state().expect("dự đoán");
        assert_eq!(
            predicted.items.len(),
            1,
            "item không được nhặt trong state dự đoán"
        );
        assert_eq!(predicted.players[0].bombs, bombs_before);
    }

    #[test]
    fn du_doan_cho_nguoi_bi_nhot_di_chuyen_nhung_nguoi_chet_dung_yen() {
        for life in [boom_core::Life::Trapped { since: 3 }, boom_core::Life::Dead] {
            let mut predictor = Predictor::new(world(), 0).expect("predictor");
            let mut state = two_player_state(7);
            state.players[0].life = life;
            predictor
                .apply_snapshot(&snapshot_of(&state, 1, 1))
                .expect("snapshot");
            let before = predictor.confirmed_state().expect("confirmed").clone();

            predictor
                .predict(PlayerInput {
                    dir: Dir::Right,
                    place_bomb: false,
                })
                .expect("predict");

            let predicted = predictor.predicted_state().expect("dự đoán");
            assert_eq!(
                predicted.players[0].life, life,
                "life không đổi khi dự đoán"
            );
            if matches!(life, boom_core::Life::Trapped { .. }) {
                assert!(
                    predicted.players[0].x > before.players[0].x,
                    "R-127: người bị nhốt vẫn đi"
                );
                assert_eq!(predicted.players[0].y, before.players[0].y);
            } else {
                assert_eq!(
                    (predicted.players[0].x, predicted.players[0].y),
                    (before.players[0].x, before.players[0].y),
                    "người chết không di chuyển"
                );
            }
        }
    }

    #[test]
    fn replay_giu_nguyen_nguoi_choi_khac() {
        let mut predictor = Predictor::new(world(), 0).expect("predictor");
        let state = two_player_state(7);
        predictor
            .apply_snapshot(&snapshot_of(&state, 1, 1))
            .expect("snapshot");
        let other_before = state.player(1).expect("khác").clone();
        predictor
            .predict(PlayerInput {
                dir: Dir::Right,
                place_bomb: true,
            })
            .expect("predict");
        // Correction: snapshot mới có ack lệnh đã áp.
        let mut next = two_player_state(7);
        next.players[0].x = boom_core::Mpx(next.players[0].x.0 + 40);
        predictor
            .apply_snapshot(&snapshot_with_ack(&next, 1, 2, 0, 1))
            .expect("snapshot");

        let predicted = predictor.predicted_state().expect("dự đoán");
        let other_after = predicted.player(1).expect("khác");
        assert_eq!(other_after.x, other_before.x);
        assert_eq!(other_after.y, other_before.y);
        assert_eq!(other_after.life, other_before.life);
    }

    // --- Finding 2: round cũ không được rollback session. ---

    #[test]
    fn packet_round_cu_khong_reset_session() {
        let mut predictor = Predictor::new(world(), 0).expect("predictor");
        let state = two_player_state(7);
        predictor
            .apply_snapshot(&snapshot_of(&state, 1, 1))
            .expect("snapshot");
        predictor
            .predict(PlayerInput {
                dir: Dir::Right,
                place_bomb: false,
            })
            .expect("predict");
        assert_eq!(predictor.pending().len(), 1);

        // Round mới: sequence bắt đầu lại từ 1 và hàng lệnh cũ bị bỏ.
        let round2 = two_player_state(9);
        predictor
            .apply_snapshot(&snapshot_of(&round2, 2, 1))
            .expect("snapshot");
        assert!(predictor.pending().is_empty());
        assert_eq!(predictor.round_id(), Some(2));

        // Packet của round cũ tới trễ: bỏ qua, **không** xóa lệnh của round mới.
        predictor
            .predict(PlayerInput {
                dir: Dir::Down,
                place_bomb: false,
            })
            .expect("predict");
        assert_eq!(predictor.pending().len(), 1);
        let outcome = predictor
            .apply_snapshot(&snapshot_of(&state, 1, 9))
            .expect("snapshot");
        assert_eq!(outcome, SnapshotOutcome::OldRound);
        assert_eq!(predictor.round_id(), Some(2));
        assert_eq!(
            predictor.pending().len(),
            1,
            "lệnh của round mới không được xóa bởi packet round cũ"
        );
        assert_eq!(
            predictor.pending()[0].seq,
            1,
            "sequence của round mới bắt đầu lại từ 1"
        );
    }

    #[test]
    fn snapshot_cu_cung_round_bi_bo_qua() {
        let mut predictor = Predictor::new(world(), 0).expect("predictor");
        let state = two_player_state(7);
        predictor
            .apply_snapshot(&snapshot_of(&state, 1, 5))
            .expect("snapshot");
        let outcome = predictor
            .apply_snapshot(&snapshot_of(&state, 1, 4))
            .expect("snapshot");
        assert_eq!(outcome, SnapshotOutcome::Stale);
        assert_eq!(predictor.round_id(), Some(1));
    }

    // --- Finding 3: lượt kết thúc thì client không dự đoán, không lỗi. ---

    #[test]
    fn ket_thuc_luot_khong_du_doan_va_khong_loi() {
        let mut predictor = Predictor::new(world(), 0).expect("predictor");
        let state = two_player_state(7);
        predictor
            .apply_snapshot(&snapshot_of(&state, 1, 1))
            .expect("snapshot");
        predictor
            .predict(PlayerInput {
                dir: Dir::Right,
                place_bomb: false,
            })
            .expect("predict");
        assert_eq!(predictor.pending().len(), 1);

        // Server báo kết quả lượt.
        let mut ended = two_player_state(7);
        ended.phase = Phase::Ended(Outcome::Winner(0));
        let outcome = predictor
            .apply_snapshot(&snapshot_of(&ended, 1, 2))
            .expect("snapshot");
        assert_eq!(outcome, SnapshotOutcome::Applied { acked_seq: 0 });
        assert!(!predictor.is_playing());
        assert_eq!(predictor.phase(), Some(Phase::Ended(Outcome::Winner(0))));

        // Tick tiếp sau kết quả: không sinh lệnh mới, không lỗi.
        let seq = predictor
            .predict(PlayerInput {
                dir: Dir::Right,
                place_bomb: false,
            })
            .expect("predict sau kết quả");
        assert_eq!(seq, None);
        assert!(predictor.pending().is_empty());
        // Kết quả của server được giữ nguyên.
        let predicted = predictor.predicted_state().expect("dự đoán");
        assert_eq!(predicted.phase, Phase::Ended(Outcome::Winner(0)));
    }

    // --- Finding 4: dữ liệu nội suy người chơi khác. ---

    #[test]
    fn noi_suy_giua_hai_snapshot_va_clamp() {
        let mut predictor = Predictor::new(world(), 0).expect("predictor");
        let mut first = two_player_state(7);
        first.players[1].x = boom_core::Mpx(1000);
        first.players[1].y = boom_core::Mpx(2000);
        predictor
            .apply_snapshot(&snapshot_of(&first, 1, 1))
            .expect("snapshot");
        let mut second = first.clone();
        second.players[1].x = boom_core::Mpx(5000);
        second.players[1].y = boom_core::Mpx(6000);
        predictor
            .apply_snapshot(&snapshot_of(&second, 1, 2))
            .expect("snapshot");

        // Giữa hai snapshot: đúng nửa đường.
        let (x, y) = predictor
            .interpolated_position(1, ALPHA_ONE / 2)
            .expect("nội suy");
        assert_eq!(x, 3000);
        assert_eq!(y, 4000);
        // Alpha = 1: đúng snapshot mới.
        let (x, y) = predictor
            .interpolated_position(1, ALPHA_ONE)
            .expect("nội suy");
        assert_eq!((x, y), (5000, 6000));
        // Alpha vượt 1: clamp ở snapshot cuối, **không** ngoại suy.
        let (x, y) = predictor
            .interpolated_position(1, ALPHA_ONE * 4)
            .expect("nội suy");
        assert_eq!((x, y), (5000, 6000));
    }
}
