//! Báo cáo của harness QA: JSON (máy đọc) + bảng markdown (người đọc).
//!
//! Báo cáo **không** kết luận gì về độ trùng khớp với client/server gốc: nó chỉ đo hành vi
//! của bản Rust hiện tại theo hồ sơ mạng đã chọn (V1-027).

use serde::Serialize;

use crate::metrics::Samples;

/// Schema của báo cáo.
pub const REPORT_SCHEMA: u32 = 1;

/// Thông số mạng đã dùng.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NetworkReport {
    /// RTT mục tiêu của hồ sơ (ms).
    pub rtt_ms: u32,
    /// Trễ một chiều mà transport mô phỏng (ms).
    pub one_way_delay_ms: u32,
    /// Jitter (ms).
    pub jitter_ms: u32,
    /// Mất frame (phần nghìn).
    pub loss_permille: u16,
    /// Trùng frame (phần nghìn).
    pub duplicate_permille: u16,
    /// Đảo thứ tự (phần nghìn).
    pub reorder_permille: u16,
}

/// Phân vị của một tập mẫu (ms).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct LatencyReport {
    /// Số mẫu.
    pub count: usize,
    /// Nhỏ nhất.
    pub min: Option<u64>,
    /// Trung bình.
    pub mean: Option<u64>,
    /// p50.
    pub p50: Option<u64>,
    /// p95.
    pub p95: Option<u64>,
    /// Lớn nhất.
    pub max: Option<u64>,
}

impl LatencyReport {
    /// Chuyển tập mẫu thô thành báo cáo phân vị.
    pub fn from_samples(samples: &Samples) -> Self {
        Self {
            count: samples.count(),
            min: samples.min(),
            mean: samples.mean(),
            p50: samples.percentile(50),
            p95: samples.percentile(95),
            max: samples.max(),
        }
    }
}

/// Số liệu input của một lần chạy.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct InputReport {
    /// Số lệnh input bot đã gửi.
    pub sent: u32,
    /// Số lệnh server đã giải quyết (áp dụng hoặc retire).
    pub resolved: u32,
    /// Trong đó bị retire vì quá hạn (không áp dụng).
    pub retired: u32,
    /// Số batch input bị server từ chối.
    pub rejected: u32,
    /// Số lệnh phòng bị server từ chối.
    pub room_rejected: u32,
    /// Số trận bot không dựng lại được state hash (snapshot kết thúc round không tới).
    pub snapshot_missed: u32,
}

/// Số lần sửa lại state của bot (prediction).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CorrectionReport {
    /// Số lần sửa.
    pub count: u32,
    /// Biên độ sửa lớn nhất (mpx).
    pub max_amplitude_mpx: i32,
    /// Ghi chú: giai đoạn 1 bot không dự đoán nên không có correction.
    pub note: String,
}

/// Kết quả một trận.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RoundReport {
    /// ID trận (public).
    pub round_id: u64,
    /// Tick cuối cùng của trận.
    pub last_tick: u32,
    /// Hash state cuối round phía server.
    pub server_hash: u64,
    /// Hash state bot dựng lại sau resync, theo tên bot.
    pub bot_hashes: Vec<(String, u64)>,
    /// Mọi bot có hash khớp server không.
    pub converged: bool,
    /// Số bot trong trận.
    pub bots: usize,
}

/// Trạng thái một bot khi lần chạy kết thúc (dùng để giải thích lần chạy hết ngân sách).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct BotStatus {
    /// Tên bot.
    pub name: String,
    /// Pha cuối cùng của bot (`Handshake`, `Lobby`, `Room`, `Loading`, `Playing`, `Results`).
    pub phase: String,
    /// Số lần đã mở lại phiên.
    pub reconnects: u32,
    /// Số lượt bot đã biết kết thúc.
    pub rounds_completed: u32,
    /// Số lượt bot đã ack kết quả.
    pub results_acked: u32,
    /// Số lần auth thất bại.
    pub auth_failed: u32,
    /// Số batch input bị server từ chối.
    pub input_rejected: u32,
    /// Số lệnh phòng bị server từ chối.
    pub room_rejected: u32,
    /// Số lượt bot không dựng lại được state hash.
    pub snapshot_missed: u32,
}

/// Báo cáo đầy đủ của một lần chạy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RunReport {
    /// Schema báo cáo.
    pub schema: u32,
    /// Tên hồ sơ mạng.
    pub profile: String,
    /// Tên profile rules.
    pub rules_profile: String,
    /// Tên bản đồ.
    pub map: String,
    /// Seed của lần chạy.
    pub seed: u64,
    /// Số bot.
    pub clients: usize,
    /// Số trận mong đợi.
    pub rounds_target: u32,
    /// Thời gian chạy thật (ms).
    pub duration_ms: u64,
    /// Thông số mạng đã dùng.
    pub network: NetworkReport,
    /// Kết quả từng trận.
    pub rounds: Vec<RoundReport>,
    /// Số liệu input.
    pub input: InputReport,
    /// Độ trễ input (ms).
    pub latency_ms: LatencyReport,
    /// RTT đo bằng Ping/Pong (ms).
    pub rtt_ms: LatencyReport,
    /// Correction của bot.
    pub corrections: CorrectionReport,
    /// Tất cả trận có hội tụ hash không.
    pub converged: bool,
    /// Lần chạy có hết ngân sách thời gian không.
    pub timed_out: bool,
    /// Trạng thái từng bot khi lần chạy kết thúc (rỗng khi mọi bot đã xong).
    pub bots: Vec<BotStatus>,
}

impl RunReport {
    /// Báo cáo dạng JSON (thụt lề 2).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|err| format!("{{\"error\":\"{err}\"}}"))
    }

    /// Báo cáo dạng bảng markdown.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# QA mạng V1-027 — hồ sơ `{}`\n\n", self.profile));
        out.push_str("| Thông số | Giá trị |\n|---|---|\n");
        out.push_str(&format!("| Profile rules | `{}` |\n", self.rules_profile));
        out.push_str(&format!("| Bản đồ | `{}` |\n", self.map));
        out.push_str(&format!("| Số bot | {} |\n", self.clients));
        out.push_str(&format!("| Số trận | {} |\n", self.rounds_target));
        out.push_str(&format!("| Seed | {} |\n", self.seed));
        out.push_str(&format!("| Thời gian chạy | {} ms |\n", self.duration_ms));
        out.push_str(&format!(
            "| RTT hồ sơ | {} ms (một chiều {} ms, jitter {} ms) |\n",
            self.network.rtt_ms, self.network.one_way_delay_ms, self.network.jitter_ms
        ));
        out.push_str(&format!(
            "| Mất / trùng / đảo | {}‰ / {}‰ / {}‰ |\n",
            self.network.loss_permille,
            self.network.duplicate_permille,
            self.network.reorder_permille
        ));
        out.push_str("\n## Input và độ trễ\n\n");
        out.push_str("| Chỉ số | Giá trị |\n|---|---|\n");
        out.push_str(&format!("| Lệnh input gửi | {} |\n", self.input.sent));
        out.push_str(&format!(
            "| Lệnh server giải quyết | {} (trong đó {} bị retire) |\n",
            self.input.resolved, self.input.retired
        ));
        out.push_str(&format!("| Batch bị từ chối | {} |\n", self.input.rejected));
        out.push_str(&format!(
            "| Lệnh phòng bị từ chối | {} |\n",
            self.input.room_rejected
        ));
        out.push_str(&format!(
            "| Trận thiếu state hash | {} |\n",
            self.input.snapshot_missed
        ));
        out.push_str(&format!(
            "| Độ trễ input p50 | {} |\n",
            fmt_ms(self.latency_ms.p50)
        ));
        out.push_str(&format!(
            "| Độ trễ input p95 | {} |\n",
            fmt_ms(self.latency_ms.p95)
        ));
        out.push_str(&format!(
            "| Độ trễ input max | {} |\n",
            fmt_ms(self.latency_ms.max)
        ));
        out.push_str(&format!(
            "| RTT ping/pong p50 | {} |\n",
            fmt_ms(self.rtt_ms.p50)
        ));
        out.push_str(&format!(
            "| RTT ping/pong p95 | {} |\n",
            fmt_ms(self.rtt_ms.p95)
        ));
        out.push_str(&format!(
            "| Correction | {} (biên độ max {} mpx) — {} |\n",
            self.corrections.count, self.corrections.max_amplitude_mpx, self.corrections.note
        ));
        out.push_str("\n## Hội tụ state hash\n\n");
        out.push_str("| Trận | Tick cuối | Hash server | Hash bot | Hội tụ |\n");
        out.push_str("|---|---|---|---|---|\n");
        for round in &self.rounds {
            let bot_hashes = round
                .bot_hashes
                .iter()
                .map(|(name, hash)| format!("{name}={hash}"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                round.round_id,
                round.last_tick,
                round.server_hash,
                bot_hashes,
                if round.converged { "có" } else { "KHÔNG" }
            ));
        }
        if !self.bots.is_empty() {
            out.push_str("\n## Trạng thái bot\n\n");
            out.push_str("| Bot | Pha | Mở lại phiên | Lượt xong | Lượt ack | Auth lỗi | Input bị từ chối | Lệnh phòng bị từ chối |\n");
            out.push_str("|---|---|---|---|---|---|---|---|---|\n");
            for bot in &self.bots {
                out.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
                    bot.name,
                    bot.phase,
                    bot.reconnects,
                    bot.rounds_completed,
                    bot.results_acked,
                    bot.auth_failed,
                    bot.input_rejected,
                    bot.room_rejected
                ));
            }
        }
        out.push_str(&format!(
            "\nKết luận: {}\n",
            if self.timed_out {
                "lần chạn HẾT ngân sách thời gian — xem bảng trên."
            } else if self.converged {
                "mọi trận hội tụ hash state giữa bot và server."
            } else {
                "CÓ trận KHÔNG hội tụ hash state — xem bảng trên."
            }
        ));
        out
    }
}

fn fmt_ms(value: Option<u64>) -> String {
    match value {
        Some(ms) => format!("{ms} ms"),
        None => "chưa đo được".to_owned(),
    }
}
