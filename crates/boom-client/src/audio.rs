//! Quyết định cue âm thanh thuần + phần phát macroquad mỏng (V1-034).
//!
//! Phần [`AudioState`]/[`AudioPlan`] không phụ thuộc cửa sổ hay thiết bị: test được event→cue,
//! dedup `(round_id, EventId)`, gộp cùng entry trong tick, giới hạn 8 voice và volume/mute.
//! [`MacroquadAudio`] chỉ nạp payload thô từ bundle và gọi API macroquad; lỗi file/định dạng bị
//! bỏ qua có log, không panic.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use boom_content::bundle::Bundle;
use boom_core::{EventId, EventKind, Outcome, SimulationEvent};

use crate::options::ClientOptions;

/// Số voice SFX đồng thời tối đa (AUDIO_V1 §4).
pub const MAX_SFX_VOICES: usize = 8;

/// Một quyết định phát trong một tick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AudioPlan {
    /// ID cue theo bảng `AUDIO_V1` (một lần/entry trong tick).
    pub cues: Vec<String>,
    /// Bắt đầu BGM loop nếu chưa chạy.
    pub start_bgm: bool,
}

/// Trạng thái quyết định cue của một client.
#[derive(Debug)]
pub struct AudioState {
    local_slot: u8,
    seen: HashSet<(u32, EventId)>,
    bgm_started: bool,
}

impl AudioState {
    pub fn new(local_slot: u8) -> Self {
        Self {
            local_slot,
            seen: HashSet::new(),
            bgm_started: false,
        }
    }

    /// Lên kế hoạch phát cho event của một round/tick.
    ///
    /// Event đã thấy theo `(round_id, EventId)` bị bỏ qua trước mọi xử lý khác. Cùng một cue
    /// trong tick chỉ còn một lần; tối đa 8 voice, ưu tiên round-end/nổ/trap/bomb-item/UI.
    pub fn plan(
        &mut self,
        bundle: &Bundle,
        round_id: u32,
        events: &[SimulationEvent],
    ) -> AudioPlan {
        let mut candidates: Vec<(u8, EventId, String)> = Vec::new();
        for event in events {
            if !self.seen.insert((round_id, event.id)) {
                continue;
            }
            let Some(cue) = self.cue_for(event) else {
                continue;
            };
            if bundle.audio_by_id(cue).is_none() {
                continue;
            }
            candidates.push((priority(cue), event.id, cue.to_owned()));
        }
        candidates.sort_by_key(|(rank, id, _)| (*rank, *id));
        candidates.truncate(MAX_SFX_VOICES);
        let mut seen_cues = BTreeSet::new();
        let mut cues = Vec::new();
        for (_, _, cue) in candidates {
            if seen_cues.insert(cue.clone()) {
                cues.push(cue);
            }
        }
        let start_bgm = !self.bgm_started
            && bundle.audio_by_id("bgm_loop").is_some()
            && bundle
                .audio_by_id("bgm_loop")
                .is_some_and(|audio| audio.kind == "bgm");
        if start_bgm {
            self.bgm_started = true;
        }
        AudioPlan { cues, start_bgm }
    }

    /// Cue UI/start không đến từ `SimulationEvent` (gọi từ màn hình tương ứng).
    #[allow(dead_code)] // màn hình UI chưa nối trong client offline; giữ API thuần cho V1-034.
    pub fn plan_ui(&self, bundle: &Bundle, cue: &str) -> AudioPlan {
        AudioPlan {
            cues: bundle
                .audio_by_id(cue)
                .map(|audio| {
                    if audio.kind == "sfx" {
                        vec![cue.to_owned()]
                    } else {
                        Vec::new()
                    }
                })
                .unwrap_or_default(),
            start_bgm: false,
        }
    }

    fn cue_for(&self, event: &SimulationEvent) -> Option<&'static str> {
        match &event.kind {
            EventKind::BombPlaced { .. } => Some("bomb_placed"),
            EventKind::BombExploded { .. } => Some("bomb_exploded"),
            EventKind::BlockDestroyed { .. } => None,
            EventKind::ItemRevealed { .. } => Some("item_revealed"),
            EventKind::ItemDestroyed { .. } => None,
            EventKind::ItemPicked { .. } => Some("item_picked"),
            EventKind::PlayerTrapped { .. } => Some("player_trapped"),
            EventKind::PlayerDied { .. } => Some("player_died"),
            EventKind::PlayerTurned { .. } => None,
            EventKind::RoundEnded { outcome } => Some(match outcome {
                Outcome::Winner(slot) if *slot == self.local_slot => "round_ended_win",
                Outcome::Winner(_) => "round_ended_lose",
                Outcome::Draw => "round_ended_draw",
            }),
            _ => None,
        }
    }
}

fn priority(cue: &str) -> u8 {
    if cue.starts_with("round_ended") {
        1
    } else if cue == "bomb_exploded" {
        2
    } else if matches!(cue, "player_trapped" | "player_died") {
        3
    } else if matches!(cue, "bomb_placed" | "item_revealed" | "item_picked") {
        4
    } else {
        5
    }
}

/// Volume hiệu lực sau `master` và `mute` (0.0..=1.0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioMix {
    pub bgm: f32,
    pub sfx: f32,
}

pub fn mix(options: &ClientOptions) -> AudioMix {
    if options.mute {
        return AudioMix { bgm: 0.0, sfx: 0.0 };
    }
    let master = f32::from(options.master_volume) / 100.0;
    AudioMix {
        bgm: master * f32::from(options.bgm_volume) / 100.0,
        sfx: master * f32::from(options.sfx_volume) / 100.0,
    }
}

/// Phần phát macroquad: nạp payload thô từ bundle và phát cue đã quyết.
pub struct MacroquadAudio {
    sounds: std::collections::BTreeMap<String, macroquad::audio::Sound>,
    bgm: Option<macroquad::audio::Sound>,
}

impl MacroquadAudio {
    /// Nạp toàn bộ audio có trong bundle; file lỗi/không đọc được bị bỏ qua có log.
    pub async fn load(bundle_dir: &Path, bundle: &Bundle) -> Self {
        let mut sounds = std::collections::BTreeMap::new();
        for asset in &bundle.audio {
            let path = bundle_dir.join(&asset.file);
            let Ok(bytes) = std::fs::read(&path) else {
                eprintln!("audio: không đọc được {}", path.display());
                continue;
            };
            match macroquad::audio::load_sound_from_bytes(&bytes).await {
                Ok(sound) => {
                    sounds.insert(asset.id.clone(), sound);
                }
                Err(error) => {
                    eprintln!("audio: bỏ qua {} ({}): {error}", asset.entry, asset.format)
                }
            }
        }
        let bgm = sounds.remove("bgm_loop");
        Self { sounds, bgm }
    }

    /// Bắt đầu BGM loop một lần; volume lấy từ options hiện tại.
    pub fn start_bgm(&self, options: &ClientOptions) {
        if let Some(bgm) = &self.bgm {
            let mix = mix(options);
            macroquad::audio::play_sound(
                bgm,
                macroquad::audio::PlaySoundParams {
                    looped: true,
                    volume: mix.bgm,
                },
            );
        }
    }

    /// Phát plan; BGM không restart mỗi tick.
    pub fn apply(&mut self, plan: &AudioPlan, options: &ClientOptions) {
        let mix = mix(options);
        for cue in &plan.cues {
            if let Some(sound) = self.sounds.get(cue) {
                macroquad::audio::play_sound(
                    sound,
                    macroquad::audio::PlaySoundParams {
                        looped: false,
                        volume: mix.sfx,
                    },
                );
            }
        }
        if plan.start_bgm {
            self.start_bgm(options);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boom_content::bundle::{AudioAsset, Bundle, EvidenceStatus};
    use boom_core::EventId;

    fn bundle_with(cues: &[&str]) -> Bundle {
        let mut bundle = Bundle {
            schema: boom_content::bundle::SCHEMA,
            build: "kr".to_owned(),
            map: "Village00".to_owned(),
            tileset: String::new(),
            width: 1,
            height: 1,
            spawns: vec![],
            content_hash: String::new(),
            atlas: vec![],
            sprites: vec![],
            cells: vec![],
            assets: vec![],
            unknown: vec![],
            clips: vec![],
            ui: vec![],
            audio: vec![],
        };
        for cue in cues {
            bundle.audio.push(AudioAsset {
                id: (*cue).to_owned(),
                entry: format!("Wave/{cue}"),
                kind: if *cue == "bgm_loop" { "bgm" } else { "sfx" }.to_owned(),
                format: "ogg".to_owned(),
                file: format!("audio/{cue}.ogg"),
                bytes: 1,
                sha256: "a".repeat(64),
                status: EvidenceStatus::Design,
            });
        }
        bundle
    }

    fn event(tick: u32, seq: u16, kind: EventKind) -> SimulationEvent {
        SimulationEvent {
            id: EventId { tick, seq },
            kind,
        }
    }

    #[test]
    fn maps_events_to_audio_v1_cues_and_dedups_round_event_id() {
        let bundle = bundle_with(&[
            "bgm_loop",
            "bomb_placed",
            "bomb_exploded",
            "player_trapped",
            "player_died",
        ]);
        let mut state = AudioState::new(0);
        let events = vec![
            event(
                1,
                0,
                EventKind::BombPlaced {
                    bomb: boom_core::EntityId(1),
                    player: 0,
                    cell: boom_core::Cell { x: 1, y: 1 },
                    range: 1,
                },
            ),
            event(
                2,
                0,
                EventKind::BombExploded {
                    bomb: boom_core::EntityId(1),
                    owner: 0,
                    cell: boom_core::Cell { x: 1, y: 1 },
                    chained: false,
                },
            ),
            event(2, 1, EventKind::PlayerTrapped { player: 1, by: 0 }),
            event(
                3,
                0,
                EventKind::PlayerDied {
                    player: 1,
                    by: None,
                },
            ),
        ];
        let first = state.plan(&bundle, 7, &events);
        assert!(first.start_bgm);
        assert!(first.cues.contains(&"bomb_placed".to_owned()));
        assert!(first.cues.contains(&"bomb_exploded".to_owned()));
        let again = state.plan(&bundle, 7, &events);
        assert!(again.cues.is_empty(), "event lặp không được phát lại");
        assert!(!again.start_bgm, "BGM không restart khi reconcile");
        let other_round = state.plan(&bundle, 8, &events);
        assert!(!other_round.cues.is_empty(), "round khác có EventId riêng");
    }

    #[test]
    fn groups_same_cue_and_caps_voices() {
        let cues = ["bomb_exploded"];
        let bundle = bundle_with(&cues);
        let mut state = AudioState::new(0);
        let events: Vec<_> = (0..10)
            .map(|i| {
                event(
                    1,
                    i,
                    EventKind::BombExploded {
                        bomb: boom_core::EntityId(u32::from(i)),
                        owner: 0,
                        cell: boom_core::Cell { x: 1, y: 1 },
                        chained: false,
                    },
                )
            })
            .collect();
        let plan = state.plan(&bundle, 1, &events);
        assert_eq!(plan.cues, vec!["bomb_exploded".to_owned()]);
    }

    #[test]
    fn round_result_uses_local_player_perspective() {
        let bundle = bundle_with(&["round_ended_win", "round_ended_lose", "round_ended_draw"]);
        let mut state = AudioState::new(2);
        let win = state.plan(
            &bundle,
            1,
            &[event(
                9,
                0,
                EventKind::RoundEnded {
                    outcome: Outcome::Winner(2),
                },
            )],
        );
        assert_eq!(win.cues, vec!["round_ended_win".to_owned()]);
        let lose = state.plan(
            &bundle,
            2,
            &[event(
                9,
                0,
                EventKind::RoundEnded {
                    outcome: Outcome::Winner(0),
                },
            )],
        );
        assert_eq!(lose.cues, vec!["round_ended_lose".to_owned()]);
        let draw = state.plan(
            &bundle,
            3,
            &[event(
                9,
                0,
                EventKind::RoundEnded {
                    outcome: Outcome::Draw,
                },
            )],
        );
        assert_eq!(draw.cues, vec!["round_ended_draw".to_owned()]);
    }

    #[test]
    fn ui_cue_only_uses_sfx_entry() {
        let bundle = bundle_with(&["bomb_placed", "bgm_loop"]);
        let state = AudioState::new(0);
        assert_eq!(
            state.plan_ui(&bundle, "bomb_placed").cues,
            vec!["bomb_placed".to_owned()]
        );
        assert!(state.plan_ui(&bundle, "bgm_loop").cues.is_empty());
    }

    #[test]
    fn volume_and_mute_are_applied() {
        let mut options = ClientOptions {
            master_volume: 50,
            bgm_volume: 40,
            sfx_volume: 20,
            ..ClientOptions::default()
        };
        let m = mix(&options);
        assert!((m.bgm - 0.2).abs() < f32::EPSILON);
        assert!((m.sfx - 0.1).abs() < f32::EPSILON);
        options.mute = true;
        assert_eq!(mix(&options), AudioMix { bgm: 0.0, sfx: 0.0 });
    }
}
