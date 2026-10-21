//! Backend UI nối server thật: đăng nhập → sảnh → phòng → lượt → kết quả.
//!
//! Module này là **adapter** giữa [`crate::ui::model::UiBackend`] (hợp đồng của UI, thuần —
//! kiểm được không cần GPU) và [`NetClient`] (tầng mạng). Nhờ vậy UI chạy được với server
//! thật mà **không** đổi model/layout (ADR 0009 Addendum B §B.6).
//!
//! ## Ranh giới quyền quyết (ADR 0005)
//!
//! - Tên hiển thị do **server** điền (`RoomMember::name`, `RoundMember::name`,
//!   `LobbyRoom.name`): client chỉ hiện, không tự đặt.
//! - Kết quả lượt do **server** quyết (`RoundResult`): client không tự kết luận thắng/thua.
//! - Client **không** khai định danh: mọi trường `player` trên dây là `AuthOk.account_id`.
//! - Tên phòng **không** có trên wire ở V1 (`RoomCommand::Create` không mang tên): UI hiện
//!   `#<room_id>` (ADR 0009 Addendum B §B.6).
//!
//! ## Đồng bộ
//!
//! [`crate::ui::model::UiBackend`] là hợp đồng **đồng bộ**: mỗi hành động gửi lệnh phòng rồi
//! bơm mạng một lần, đọc trạng thái server đã xác nhận. Nhịp bơm thường xuyên do
//! [`crate::ui::model::UiBackend::pump`] (gọi mỗi vòng của [`crate::ui::model::UiModel::tick`]).

use std::collections::BTreeMap;
use std::time::Duration;

use boom_core::Phase;
use boom_net::{TcpTransport, Transport};
use boom_proto::dto::auth::AuthFailReason;
use boom_proto::{LobbyList, RoomAction, RoomCharacter, RoomOutcome, RoomPhase, RoundResult};

use crate::anim::Character;
use crate::ui::model::{
    ChatLine, MatchEntry, MatchOutcome, MatchProgress, MatchResult, RoomInfo, RoomMember,
    RoomPhase as UiRoomPhase, RoomView, UiBackend, UiError,
};

use super::{Credentials, NetClient, NetError};

/// Thời gian chờ message khi **bơm định kỳ** (mỗi vòng của UI).
///
/// **Tham số vận hành** (design), không phải luật chơi: ngắn để vòng vẽ không bị kẹt; message
/// tới trễ sẽ được bơm ở vòng sau.
const POLL_TIMEOUT: Duration = Duration::from_millis(2);

/// Thời gian chờ message khi **chờ một câu trả lời** (vd. danh sách phòng sảnh).
///
/// **Tham số vận hành** (design): đủ rộng để server xử lý lệnh và trả lại trên mạng thật.
const REPLY_TIMEOUT: Duration = Duration::from_millis(500);

/// Số người tối thiểu để bắt đầu lượt theo policy V1 (SCOPE_V1 §1: 2–4 người).
///
/// **Tham số hiển thị** phía client: server kiểm lại khi nhận lệnh `Start`, nên lệch ở đây chỉ
/// làm nút bấm sai trạng thái, không cho phát hành lượt thiếu người.
const MIN_PLAYERS_TO_START: usize = 2;

/// Hàm mở phiên mạng: nhận thông tin đăng nhập rồi trả client đã handshake + `AuthOk`.
type Connector<W> = Box<dyn FnMut(Credentials) -> Result<NetClient<W>, NetError> + Send>;

/// Backend UI nối server thật.
///
/// `W` là transport hai chiều: `TcpTransport` ở chế độ `--connect`, `MemoryTransport` (hoặc
/// transport có lịch giao) trong test end-to-end với server thật.
pub struct NetBackend<W: Transport> {
    /// Client mạng (sau khi đăng nhập thành công).
    client: Option<NetClient<W>>,
    /// Mở phiên khi người chơi đăng nhập; `None` = client đã có sẵn (test).
    connect: Option<Connector<W>>,
    /// Tên tài khoản đang đăng nhập (tên đăng nhập, không phải `account_id`).
    account: Option<String>,
    /// Nhân vật đang chọn: client gửi kèm lệnh tạo/vào phòng.
    character: RoomCharacter,
    /// Round mà kết quả **đã** hiện lên màn kết quả.
    ///
    /// Kết quả của server nằm lại trong luồng phòng, nên backend phải nhớ round nào đã hiện:
    /// không thì lượt sau bị báo "đã xong kết quả" ngay lúc bắt đầu.
    presented_result: Option<u64>,
}

impl<W: Transport> NetBackend<W> {
    /// Backend dùng client **đã có sẵn** (test end-to-end với server thật).
    ///
    /// Client phải đã handshake + `AuthOk`: mọi kind phòng chỉ hợp lệ sau `AuthOk`
    /// (ADR 0009 Addendum A §A.1).
    pub fn with_client(client: NetClient<W>) -> Self {
        let account = client.account().map(|account| account.display_name.clone());
        Self {
            client: Some(client),
            connect: None,
            account,
            character: RoomCharacter::C00,
            presented_result: None,
        }
    }

    /// Backend **mở phiên khi người chơi đăng nhập** (chế độ `--connect`).
    ///
    /// `connect` được gọi với thông tin đăng nhập và phải trả về client đã handshake + `AuthOk`
    /// (hoặc lỗi để hiện cho người chơi).
    pub fn lazy(
        connect: impl FnMut(Credentials) -> Result<NetClient<W>, NetError> + Send + 'static,
    ) -> Self {
        Self {
            client: None,
            connect: Some(Box::new(connect)),
            account: None,
            character: RoomCharacter::C00,
            presented_result: None,
        }
    }

    /// Client mạng hiện tại (nếu đã đăng nhập).
    pub fn client(&self) -> Option<&NetClient<W>> {
        self.client.as_ref()
    }

    /// Cho vòng gameplay mượn client; trả lại trước khi bơm UI.
    pub fn take_client(&mut self) -> Option<NetClient<W>> {
        self.client.take()
    }

    /// Trả client sau bước gameplay.
    pub fn put_client(&mut self, client: NetClient<W>) {
        self.client = Some(client);
    }

    /// Nhân vật đang chọn (mã trên dây).
    pub fn character(&self) -> RoomCharacter {
        self.character
    }

    /// Mở phiên cho tài khoản đã cho (đăng nhập hoặc đăng ký).
    fn open_session(&mut self, credentials: Credentials) -> Result<(), UiError> {
        let Some(connect) = self.connect.as_mut() else {
            // Đã có phiên rồi: một tài khoản chỉ có **một** phiên (ADR 0011 §6) nên không mở
            // phiên thứ hai.
            return Err(UiError::InvalidCredentials);
        };
        let client = connect(credentials).map_err(map_net_error)?;
        self.account = client.account().map(|account| account.display_name.clone());
        self.client = Some(client);
        self.presented_result = None;
        Ok(())
    }

    /// Bơm mạng: đọc message đang chờ để trạng thái server mới tới UI.
    fn pump_with(&mut self, timeout: Duration) -> Result<(), UiError> {
        let Some(client) = self.client.as_mut() else {
            return Err(UiError::Other);
        };
        client.pump(timeout).map_err(map_net_error)
    }

    /// Gửi lệnh phòng rồi bơm mạng một lần (server trả `RoomState`/`RoomRejected`).
    fn room_command(&mut self, action: RoomAction) -> Result<(), UiError> {
        let Some(client) = self.client.as_mut() else {
            return Err(UiError::Other);
        };
        client.room_command(action).map_err(map_net_error)?;
        self.pump_with(POLL_TIMEOUT)
    }

    /// Tên hiển thị của mọi người chơi mà client biết: roster của round (lịch sử — kể cả
    /// người đã rời phòng) và thành viên phòng hiện tại.
    fn display_names(&self) -> BTreeMap<u64, String> {
        let mut names = BTreeMap::new();
        let Some(client) = self.client.as_ref() else {
            return names;
        };
        let flow = client.room();
        if let Some(state) = flow.room() {
            for member in &state.members {
                names.insert(member.player, member.name.clone());
            }
        }
        if let Some(plan) = flow.round() {
            for member in &plan.roster {
                names
                    .entry(member.player)
                    .or_insert_with(|| member.name.clone());
            }
        }
        names
    }

    /// Dựng bảng kết quả từ `RoundResult` của server.
    ///
    /// Tên lấy theo roster/thành viên phòng (server điền); hạng lấy theo `PlayerResult.rank`
    /// của server — hiện server chưa có quy định thứ hạng (V1-022) nên `rank` là `0` và bảng
    /// dùng **thứ tự server gửi** (1 = dòng đầu), không tự đặt policy hạng.
    fn outcome_of(&self, result: &RoundResult) -> MatchOutcome {
        let names = self.display_names();
        let my_player = self
            .client
            .as_ref()
            .and_then(|client| client.room().my_player());
        let outcome = match result.outcome {
            RoomOutcome::Winner(player) => {
                if Some(player) == my_player {
                    MatchResult::Win
                } else {
                    MatchResult::Lose
                }
            }
            RoomOutcome::Draw => MatchResult::Draw,
        };
        let mut entries: Vec<MatchEntry> = result
            .results
            .iter()
            .enumerate()
            .map(|(index, item)| MatchEntry {
                rank: if item.rank > 0 {
                    u32::from(item.rank)
                } else {
                    u32::try_from(index + 1).unwrap_or(u32::MAX)
                },
                name: names
                    .get(&item.player)
                    .cloned()
                    .unwrap_or_else(|| format!("player{}", item.player)),
                result: outcome,
            })
            .collect();
        // Thứ tự bảng: hạng server (nếu có) rồi thứ tự server gửi (sort ổn định).
        entries.sort_by_key(|entry| entry.rank);
        MatchOutcome {
            entries,
            self_result: outcome,
        }
    }

    /// Dựng màn phòng chờ từ `RoomState` của server.
    fn room_view(&self) -> Option<RoomView> {
        let client = self.client.as_ref()?;
        let flow = client.room();
        let state = flow.room()?;
        let my_player = flow.my_player();
        let members: Vec<RoomMember> = state
            .members
            .iter()
            .map(|member| RoomMember {
                name: member.name.clone(),
                ready: member.ready,
                character: character_to_ui(member.character),
                host: member.player == state.host,
                is_self: Some(member.player) == my_player,
            })
            .collect();
        // Tên chủ phòng: tra trong `members` theo `RoomState.host` (bất biến "host là thành
        // viên" đã kiểm trong `RoomState::check`) — không có field `host_name`
        // (ADR 0009 Addendum B §B.6).
        let host = state
            .members
            .iter()
            .find(|member| member.player == state.host)
            .map(|member| member.name.clone())
            .unwrap_or_default();
        let me = state
            .members
            .iter()
            .find(|member| Some(member.player) == my_player);
        let all_ready = state.members.iter().all(|member| member.ready);
        let enough = state.members.len() >= MIN_PLAYERS_TO_START;
        let is_host = state.host == my_player.unwrap_or(u64::MAX);
        Some(RoomView {
            id: u32::try_from(state.room_id).unwrap_or(u32::MAX),
            // Tên phòng chưa có trên wire ở V1 (ADR 0009 Addendum B §B.6): hiện `#<id>` để
            // người chơi còn phân biệt được các phòng trong sảnh.
            name: format!("#{}", state.room_id),
            host,
            members,
            self_ready: me.is_some_and(|member| member.ready),
            self_character: me.map_or(Character::C00, |member| character_to_ui(member.character)),
            // Nút "bắt đầu" chỉ dùng được khi mình là chủ phòng, mọi người đã sẵn sàng và đủ
            // người; server kiểm lại khi nhận lệnh (SCOPE_V1 §3).
            can_start: is_host && all_ready && enough,
            chat: flow
                .chat()
                .iter()
                .map(|message| ChatLine {
                    author: message.from_name.clone(),
                    text: message.text.clone(),
                })
                .collect(),
        })
    }
}

impl NetBackend<TcpTransport> {
    /// Backend nối server thật bằng TCP; phiên mở khi người chơi đăng nhập ở màn login.
    ///
    /// Địa chỉ là `host:port` của server (cờ `--connect`).
    pub fn tcp(addr: impl Into<String>) -> Self {
        let addr = addr.into();
        Self::lazy(move |credentials| super::connect(&addr, &credentials))
    }
}

/// Đổi mã nhân vật trên dây sang nhân vật của client.
fn character_to_ui(character: RoomCharacter) -> Character {
    match character {
        RoomCharacter::C00 => Character::C00,
        RoomCharacter::C01 => Character::C01,
    }
}

/// Đổi mã nhân vật của client sang mã trên dây.
fn character_to_dto(character: Character) -> RoomCharacter {
    match character {
        Character::C00 => RoomCharacter::C00,
        Character::C01 => RoomCharacter::C01,
    }
}

/// Đổi pha phòng trên dây sang pha hiển thị.
fn phase_to_ui(phase: RoomPhase) -> UiRoomPhase {
    match phase {
        RoomPhase::Waiting => UiRoomPhase::Waiting,
        RoomPhase::Started => UiRoomPhase::Playing,
    }
}

/// Ánh xạ lỗi mạng sang lỗi UI (khóa chuỗi đã có trong file chuỗi V1).
fn map_net_error(error: NetError) -> UiError {
    match error {
        NetError::AuthFailed(reason) => match reason {
            AuthFailReason::UsernameUnavailable => UiError::AccountExists,
            AuthFailReason::InvalidCredentials | AuthFailReason::AccountHasActiveSession => {
                UiError::InvalidCredentials
            }
            _ => UiError::Other,
        },
        _ => UiError::Other,
    }
}

impl<W: Transport> UiBackend for NetBackend<W> {
    fn login(&mut self, account: &str, password: &str) -> Result<(), UiError> {
        // `register` vừa mở phiên cho tài khoản này: một tài khoản chỉ có **một** phiên
        // (ADR 0011 §6) nên không mở phiên thứ hai.
        if self.account.as_deref() == Some(account) && self.client.is_some() {
            return Ok(());
        }
        self.open_session(Credentials::login(account, password))
    }

    fn register(&mut self, account: &str, password: &str) -> Result<(), UiError> {
        if self.account.as_deref() == Some(account) && self.client.is_some() {
            return Ok(());
        }
        self.open_session(Credentials::register(account, password))
    }

    fn logout(&mut self) -> Result<(), UiError> {
        // Bỏ client ⇒ đóng kết nối; server thấy kết nối đóng thì thu hồi phiên
        // (ADR 0011 §6.1).
        self.client = None;
        self.account = None;
        self.presented_result = None;
        Ok(())
    }

    fn rooms(&mut self) -> Result<Vec<RoomInfo>, UiError> {
        {
            let Some(client) = self.client.as_mut() else {
                return Err(UiError::Other);
            };
            client.request_lobby().map_err(map_net_error)?;
        }
        // Chờ server trả `LobbyList`: đây là lần chờ **có câu trả lời** nên hạn rộng hơn nhịp
        // bơm thường xuyên.
        self.pump_with(REPLY_TIMEOUT)?;
        let Some(client) = self.client.as_ref() else {
            return Err(UiError::Other);
        };
        let list = client
            .room()
            .lobby()
            .cloned()
            .unwrap_or_else(|| LobbyList { rooms: Vec::new() });
        let max_players = u32::try_from(client.limits().max_players).unwrap_or(u32::MAX);
        Ok(list
            .rooms
            .iter()
            .map(|room| RoomInfo {
                id: u32::try_from(room.room_id).unwrap_or(u32::MAX),
                // Tên phòng chưa có trên wire ở V1 (ADR 0009 Addendum B §B.6).
                name: format!("#{}", room.room_id),
                // `LobbyRoom.name` là **tên hiển thị của chủ phòng** (ADR 0009 Addendum B §B.6).
                host: room.name.clone(),
                players: u32::from(room.member_count),
                max_players,
                phase: phase_to_ui(room.phase),
            })
            .collect())
    }

    fn create_room(&mut self, _name: &str) -> Result<(), UiError> {
        // Tên phòng **không** có trên wire ở V1 (`RoomCommand::Create` không mang tên —
        // ADR 0009 Addendum B §B.6): tên người chơi gõ ở sảnh được bỏ qua, server cấp ID phòng.
        self.room_command(RoomAction::Create {
            character: self.character,
        })
    }

    fn join_room(&mut self, room: u32) -> Result<(), UiError> {
        self.room_command(RoomAction::Join {
            room: u64::from(room),
            character: self.character,
        })
    }

    fn leave_room(&mut self) -> Result<(), UiError> {
        self.room_command(RoomAction::Leave)
    }

    fn set_ready(&mut self, ready: bool) -> Result<(), UiError> {
        self.room_command(RoomAction::SetReady { ready })
    }

    fn choose_character(&mut self, character: Character) -> Result<(), UiError> {
        self.character = character_to_dto(character);
        self.room_command(RoomAction::SelectCharacter {
            character: self.character,
        })
    }

    fn start(&mut self) -> Result<(), UiError> {
        self.room_command(RoomAction::Start)
    }

    fn send_chat(&mut self, message: &str) -> Result<(), UiError> {
        let Some(client) = self.client.as_mut() else {
            return Err(UiError::Other);
        };
        client.send_chat(message).map_err(map_net_error)?;
        self.pump_with(POLL_TIMEOUT)
    }

    fn ack_result(&mut self) -> Result<(), UiError> {
        // Client đã gửi `ResultAck` **ngay khi nhận** `RoundResult` (xem `NetClient::handle`),
        // nên hành động "xem xong kết quả" chỉ cần bơm mạng cho server đưa phòng về chờ.
        self.pump_with(POLL_TIMEOUT)
    }

    fn room(&self) -> Option<RoomView> {
        self.room_view()
    }

    fn match_progress(&mut self) -> Option<MatchProgress> {
        let (result, plan_round_id, phase) = {
            let client = self.client.as_ref()?;
            let flow = client.room();
            (
                flow.result().cloned(),
                flow.round().map(|plan| plan.round_id),
                client.phase(),
            )
        };
        if let Some(result) = result
            && plan_round_id == Some(result.round_id)
        {
            if self.presented_result == Some(result.round_id) {
                // Kết quả của round này đã hiện rồi: không báo lại (tránh màn kết quả nhảy
                // ngược lúc bắt đầu lượt sau).
                return None;
            }
            self.presented_result = Some(result.round_id);
            return Some(MatchProgress::Finished(self.outcome_of(&result)));
        }
        plan_round_id.map(|_| match phase {
            Some(Phase::Playing) => MatchProgress::Playing,
            // Chưa có snapshot đầu của round ⇒ còn ở loading barrier. Client **không** biết
            // hạn của barrier phía server nên không tự đếm ngược (không đoán số).
            _ => MatchProgress::Loading { remaining: 0 },
        })
    }

    fn active_match(&self) -> bool {
        self.client
            .as_ref()
            .and_then(|client| client.room().round())
            .is_some_and(|round| self.presented_result != Some(round.round_id))
    }

    fn pump(&mut self) {
        // Lỗi bơm mạng không làm hỏng vòng vẽ: lần bơm sau sẽ thử lại; UI hiện trạng thái
        // server đã xác nhận gần nhất.
        let _ = self.pump_with(POLL_TIMEOUT);
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};
    use std::thread;
    use std::time::Duration;

    use boom_content::rules::RulesProfile;
    use boom_core::{CharacterStatsTable, Rules, World, mode_by_id};
    use boom_net::{MemoryConfig, MemoryTransport};
    use boom_room::PlayerId;
    use boom_server::auth::{AuthConfig, AuthService};
    use boom_server::identity::{OsRandomSource, SqliteAccountStore, SqliteOptions};
    use boom_server::{Authority, HandshakeConfig, MatchSetup, Server, ServerConfig};

    use super::*;
    use crate::net::fixture::{ACCOUNT_PASSWORD, stats, terrain};
    use crate::net::{ClientSession, hello_for_profile};

    /// Profile rules cố định cho test (cùng schema với `data/rules/v1-dev.toml`).
    const RULES: &str = include_str!("../../../../fixtures/rules/skeleton-test.toml");

    /// IP nguồn giả của kết nối trong test (bucket rate limit theo nguồn, ADR 0011 §5).
    const TEST_IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

    /// Gốc định danh **kết nối** trong test.
    ///
    /// Cố ý khác `account_id` (SQLite cấp từ 1): định danh kết nối **không** phải slot trận
    /// (ADR 0009 Addendum B §B.2).
    const CONNECTION_ID_BASE: u64 = 900;

    /// Fixture: server **thật** (auth SQLite) + client đã đăng nhập qua `MemoryTransport`.
    ///
    /// Client đầu tiên đưa cho backend UI; các client còn lại do fixture bơm, để test kiểm
    /// được luồng nhiều người (vào phòng, ready, start, kết quả) với đúng đường wire.
    struct UiFixture {
        server: Server,
        /// Các client còn lại (client đầu đưa cho backend UI).
        others: Vec<NetClient<MemoryTransport>>,
        /// Đồng hồ ảo của server (ms).
        now_ms: u64,
        /// Thời gian một lượt (ms), lấy từ rules profile.
        round_duration_ms: u64,
        /// Thư mục tạm chứa DB auth: phải sống suốt đời fixture thì file DB còn tồn tại.
        _db_dir: tempfile::TempDir,
    }

    impl UiFixture {
        /// Dựng server + `count` client đã đăng nhập; trả về fixture và **client đầu tiên**.
        fn new(count: usize) -> (Self, NetClient<MemoryTransport>) {
            Self::sessions(count, true)
        }

        fn sessions(count: usize, authenticate: bool) -> (Self, NetClient<MemoryTransport>) {
            let profile = RulesProfile::from_toml(RULES).expect("profile rules");
            let rules = Rules::from_profile(&profile).expect("rules");
            let hz = rules.ticks_per_second;
            let world = World::new(
                terrain(),
                rules.clone(),
                mode_by_id("battle_ffa").expect("mode"),
            )
            .expect("world");
            let authority = Authority::new(
                ServerConfig::v1(),
                MatchSetup {
                    world,
                    stats: CharacterStatsTable {
                        c00: stats(),
                        c01: stats(),
                    },
                    map: "skeleton-test".to_owned(),
                    replay: None,
                },
            );
            let handshake = HandshakeConfig {
                rules_profile_id: profile.profile_id.clone(),
                rules_hash: rules.profile_hash,
                content_sha256: [0; 32],
                content_epoch: 1,
                snapshot_interval_ms: 50,
            };
            // Auth thật: SQLite trên file tạm + `AuthService` của server (ADR 0011 §5–§6, §10).
            let db_dir = tempfile::tempdir().expect("tempdir");
            let store = SqliteAccountStore::open(
                &db_dir.path().join("auth.sqlite3"),
                SqliteOptions::default(),
            )
            .expect("mở store");
            let auth = AuthService::new(
                AuthConfig::default(),
                "ui-backend-instance",
                Box::new(OsRandomSource),
            )
            .expect("dịch vụ auth");
            let mut server = Server::with_auth_and_timers(
                authority,
                handshake,
                auth,
                Box::new(store),
                0,
                u64::MAX,
                u64::MAX,
            )
            .expect("dựng server có auth");
            // Ping/liveness tắt (`u64::MAX`): fixture nhảy thời gian ảo theo bước lớn (hết
            // thời gian lượt), không muốn server đóng phiên chỉ vì client chưa kịp trả `Pong`.
            server.set_auth_deadline_ms(u64::MAX);
            let hello = hello_for_profile(&profile, &rules, [0; 32], "boom-client-test");
            let mut handles = Vec::new();
            for index in 0..count {
                let (server_side, client_side) = MemoryTransport::pair(MemoryConfig::default());
                let (server_writer, client_reader) = MemoryTransport::pair(MemoryConfig::default());
                server
                    .accept(
                        PlayerId(u64::try_from(index).expect("id") + CONNECTION_ID_BASE),
                        u8::try_from(index).expect("id") + 1,
                        TEST_IP,
                        server_side,
                        server_writer,
                    )
                    .expect("accept");
                // Handshake + auth cần server poll (vòng IO của server), nên client chờ ở luồng
                // riêng còn server poll tới khi xong.
                let client_rules = rules.clone();
                let client_hello = hello.clone();
                let credentials =
                    Credentials::register(format!("ui-user-{index}"), ACCOUNT_PASSWORD.to_owned());
                handles.push(thread::spawn(move || {
                    let mut net = NetClient::new(
                        ClientSession::new(client_reader, client_side),
                        client_rules,
                        mode_by_id("battle_ffa").expect("mode"),
                    );
                    net.handshake(&client_hello).expect("handshake");
                    if authenticate {
                        net.authenticate(&credentials).expect("auth");
                    }
                    net
                }));
            }
            while handles.iter().any(|handle| !handle.is_finished()) {
                server
                    .poll(0, Duration::from_millis(1))
                    .expect("server poll lúc handshake");
            }
            let mut nets = Vec::new();
            for handle in handles {
                nets.push(handle.join().expect("join luồng client"));
            }
            let first = nets.remove(0);
            let round_duration_ms = u64::from(rules.round_ticks) * 1_000 / u64::from(hz);
            (
                Self {
                    server,
                    others: nets,
                    now_ms: 0,
                    round_duration_ms,
                    _db_dir: db_dir,
                },
                first,
            )
        }

        /// Bơm `steps` vòng: server xử lý lệnh đã nhận, backend UI bơm message, các client
        /// còn lại bơm message.
        fn pump(&mut self, backend: &mut NetBackend<MemoryTransport>, steps: u32) {
            for _ in 0..steps {
                self.server
                    .poll(self.now_ms, Duration::from_millis(1))
                    .expect("poll server");
                backend.pump();
                for net in &mut self.others {
                    net.pump(Duration::from_millis(1)).expect("bơm message");
                }
            }
        }

        /// Nhảy đồng hồ server tới `now_ms` (server chạy tới mốc đó) rồi bơm một vòng.
        fn advance_to(&mut self, backend: &mut NetBackend<MemoryTransport>, now_ms: u64) {
            self.now_ms = self.now_ms.max(now_ms);
            self.server
                .poll(self.now_ms, Duration::from_millis(1))
                .expect("poll server");
            backend.pump();
            for net in &mut self.others {
                net.pump(Duration::from_millis(1)).expect("bơm message");
            }
        }
    }

    #[test]
    fn ui_model_hai_tran_voi_server_that() {
        use crate::ui::model::{ActionId, ActionOutcome, FieldId};
        use crate::ui::{Texts, UiModel};
        use boom_content::bundle::UiScreen;
        let (mut fixture, first) = UiFixture::sessions(2, false);
        let second = fixture.others.remove(0);
        let mut index = 0;
        let mut models = [first, second].map(|client| {
            let name = format!("ui-user-{index}");
            index += 1;
            let mut client = Some(client);
            let backend = NetBackend::lazy(move |credentials| {
                let mut net = client.take().expect("một phiên");
                net.authenticate(&credentials)?;
                Ok(net)
            });
            let mut model = UiModel::new(backend, Texts::embedded());
            model.set_field_text(FieldId::LoginAccount, &name);
            model.set_field_text(FieldId::LoginPassword, ACCOUNT_PASSWORD);
            // Auth đồng bộ của UI chờ server IO trên luồng fixture.
            thread::scope(|scope| {
                let handle = scope.spawn(|| model.activate(ActionId::LoginRegister));
                while !handle.is_finished() {
                    fixture.server.poll(0, Duration::from_millis(1)).unwrap();
                }
                assert_eq!(handle.join().unwrap(), ActionOutcome::Done);
            });
            assert_eq!(model.screen(), UiScreen::Lobby);
            model
        });
        fn pump(fixture: &mut UiFixture, models: &mut [UiModel<NetBackend<MemoryTransport>>; 2]) {
            for _ in 0..64 {
                fixture.server.poll(fixture.now_ms, Duration::ZERO).unwrap();
                for model in models.iter_mut() {
                    model.tick();
                }
            }
        }
        assert_eq!(
            models[0].activate(ActionId::LobbyCreate),
            ActionOutcome::Done
        );
        pump(&mut fixture, &mut models);
        models[1].activate(ActionId::LobbyRefresh);
        // Fixture server được poll riêng; nhận danh sách trước khi chọn phòng.
        pump(&mut fixture, &mut models);
        models[1].activate(ActionId::LobbyRefresh);
        models[1].move_selection(0);
        assert_eq!(models[1].activate(ActionId::LobbyJoin), ActionOutcome::Done);
        pump(&mut fixture, &mut models);
        let mut rounds = Vec::new();
        for _ in 0..2 {
            for model in &mut models {
                assert_eq!(model.activate(ActionId::RoomReady), ActionOutcome::Done);
            }
            pump(&mut fixture, &mut models);
            assert!(models[0].room().unwrap().can_start);
            assert_eq!(models[0].activate(ActionId::RoomStart), ActionOutcome::Done);
            pump(&mut fixture, &mut models);
            fixture.now_ms += 200;
            pump(&mut fixture, &mut models);
            for model in &models {
                assert_eq!(model.screen(), UiScreen::Match);
            }
            let round = models[0]
                .backend_mut()
                .client()
                .unwrap()
                .round_id()
                .unwrap();
            rounds.push(round);
            for model in &mut models {
                let client = model.backend_mut().client.as_mut().unwrap();
                assert_eq!(client.phase(), Some(Phase::Playing));
                client
                    .predict(boom_core::PlayerInput {
                        dir: boom_core::Dir::Right,
                        place_bomb: true,
                    })
                    .unwrap();
                client.resend().unwrap();
                assert!(client.pending_len() > 0);
            }
            pump(&mut fixture, &mut models);
            fixture.now_ms += fixture.round_duration_ms + 10_000;
            pump(&mut fixture, &mut models);
            for model in &mut models {
                assert_eq!(model.screen(), UiScreen::Result);
                assert_eq!(model.outcome().unwrap().entries.len(), 2);
                assert_eq!(
                    model.activate(ActionId::ResultContinue),
                    ActionOutcome::Done
                );
                assert_eq!(model.screen(), UiScreen::Room);
            }
            pump(&mut fixture, &mut models);
            for model in &models {
                assert!(!model.room().unwrap().self_ready);
            }
        }
        assert!(rounds[1] > rounds[0]);
    }

    /// Backend UI mạng đi trọn luồng V1 với server thật: sảnh → phòng → lượt → kết quả.
    ///
    /// Đây là nghiệm thu của #138: tên hiển thị do **server** điền phải tới đúng màn hình
    /// của UI (tên thành viên, tên chủ phòng, tên trong bảng kết quả).
    #[test]
    fn backend_ui_mang_di_tron_luong_v1_voi_server_that() {
        let (mut fixture, client) = UiFixture::new(2);
        let mut backend = NetBackend::with_client(client);

        // Sảnh: chưa có phòng nào.
        assert!(
            backend.rooms().expect("xin danh sách phòng").is_empty(),
            "phòng đầu tiên chưa có ai tạo"
        );

        // Tạo phòng: server trả `RoomState` có **tên hiển thị** của thành viên.
        backend.create_room("phong thu nghiem").expect("tạo phòng");
        fixture.pump(&mut backend, 4);
        let room = backend.room().expect("phải có phòng");
        assert_eq!(room.members.len(), 1);
        assert_eq!(
            room.members[0].name, "ui-user-0",
            "tên thành viên do server điền từ account store"
        );
        assert!(room.members[0].is_self);
        assert!(room.members[0].host);
        assert_eq!(
            room.host, "ui-user-0",
            "tên chủ phòng lấy từ thành viên có `player == RoomState.host`"
        );
        assert_eq!(
            room.name, "#1",
            "tên phòng chưa có trên wire: hiện `#<room_id>`"
        );

        // Sảnh: `LobbyRoom.name` là tên hiển thị **của chủ phòng**.
        let rooms = backend.rooms().expect("làm mới danh sách phòng");
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].host, "ui-user-0");
        assert_eq!(rooms[0].players, 1);
        assert_eq!(rooms[0].name, "#1");

        // Chat phòng: server điền `from_name` từ account store.
        backend.send_chat("xin chao").expect("gửi chat");
        fixture.pump(&mut backend, 4);
        let room = backend.room().expect("phải có phòng");
        assert_eq!(room.chat.len(), 1);
        assert_eq!(room.chat[0].author, "ui-user-0");
        assert_eq!(room.chat[0].text, "xin chao");

        // Người chơi thứ hai vào phòng và sẵn sàng (client thật, cùng đường wire).
        let other = &mut fixture.others[0];
        other
            .room_command(RoomAction::Join {
                room: 1,
                character: RoomCharacter::C01,
            })
            .expect("vào phòng");
        other
            .room_command(RoomAction::SetReady { ready: true })
            .expect("ready");
        fixture.pump(&mut backend, 4);
        backend.set_ready(true).expect("sẵn sàng");
        fixture.pump(&mut backend, 4);
        let room = backend.room().expect("phải có phòng");
        assert_eq!(room.members.len(), 2, "phòng phải có hai người");
        assert_eq!(room.members[1].name, "ui-user-1");
        assert!(room.self_ready);
        assert!(
            room.can_start,
            "chủ phòng đủ điều kiện thì nút bắt đầu dùng được"
        );

        // Bắt đầu lượt: server gửi `RoundStart` cho mọi thành viên.
        backend.start().expect("bắt đầu");
        fixture.pump(&mut backend, 8);
        assert!(
            backend
                .client()
                .and_then(|client| client.room().round())
                .is_some(),
            "client phải nhận `RoundStart`"
        );
        assert!(
            matches!(
                backend.match_progress(),
                Some(MatchProgress::Loading { .. }) | Some(MatchProgress::Playing)
            ),
            "lượt phải ở trạng thái loading hoặc playing"
        );

        // Chạy hết thời gian lượt: server tự quyết kết quả (ADR 0005).
        let end = fixture.now_ms + fixture.round_duration_ms + 10_000;
        fixture.advance_to(&mut backend, end);
        fixture.pump(&mut backend, 4);

        let Some(MatchProgress::Finished(outcome)) = backend.match_progress() else {
            panic!("phải có kết quả lượt do server quyết");
        };
        assert_eq!(
            outcome.entries.len(),
            2,
            "bảng kết quả phải có đủ người chơi"
        );
        let names: Vec<&str> = outcome
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert!(
            names.contains(&"ui-user-0") && names.contains(&"ui-user-1"),
            "bảng kết quả phải có tên hiển thị của mọi người, thấy {names:?}"
        );
    }
}
