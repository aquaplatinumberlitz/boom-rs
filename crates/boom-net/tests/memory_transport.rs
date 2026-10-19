//! Transport trong bộ nhớ: tái lập theo seed, mất/trùng/đảo thứ tự, ngắt kết nối.

use std::time::{Duration, Instant};

use boom_net::{
    FrameLimits, MemoryConfig, MemoryFaults, MemoryTransport, NetError, Timing, Transport,
};

/// Chạy một phiên: gửi `count` frame đánh số, đóng, rồi gom mọi frame nhận được.
fn chay(seed: u64, faults: MemoryFaults, count: u32) -> Vec<Vec<u8>> {
    let config = MemoryConfig {
        seed,
        faults,
        timing: Timing::Virtual,
    };
    let (mut gui, mut nhan) = MemoryTransport::pair(config);
    for i in 0..count {
        gui.send_frame(&i.to_le_bytes()).expect("gửi frame");
    }
    gui.close().expect("đóng");
    let mut ra = Vec::new();
    while let Some(frame) = nhan.recv_frame().expect("nhận frame") {
        ra.push(frame);
    }
    ra
}

fn chay_voi_fault_mac_dinh(seed: u64, count: u32) -> Vec<Vec<u8>> {
    let faults = MemoryFaults {
        loss_permille: 100,
        duplicate_permille: 100,
        reorder_permille: 100,
        delay_ms: 10,
        jitter_ms: 5,
        reorder_window_ms: 20,
    };
    chay(seed, faults, count)
}

#[test]
fn cung_seed_cung_thu_tu_giao() {
    let a = chay_voi_fault_mac_dinh(20261019, 40);
    let b = chay_voi_fault_mac_dinh(20261019, 40);
    assert_eq!(a, b, "cùng seed phải cho cùng thứ tự giao");
    assert!(!a.is_empty(), "phải có frame đến nơi");
}

#[test]
fn khac_seed_khac_thu_tu_giao() {
    let a = chay_voi_fault_mac_dinh(1, 40);
    let b = chay_voi_fault_mac_dinh(2, 40);
    assert_ne!(a, b, "seed khác phải cho thứ tự khác");
}

#[test]
fn khong_loi_thi_giu_nguyen_thu_tu_gui() {
    let frames = chay(7, MemoryFaults::none(), 16);
    let expected: Vec<Vec<u8>> = (0..16u32).map(|i| i.to_le_bytes().to_vec()).collect();
    assert_eq!(frames, expected);
}

#[test]
fn mat_frame_bi_nuot() {
    let faults = MemoryFaults {
        loss_permille: 1000,
        ..MemoryFaults::none()
    };
    let frames = chay(3, faults, 8);
    assert!(frames.is_empty(), "mất 100% thì không frame nào tới");
}

#[test]
fn trung_frame_gui_hai_lan() {
    let faults = MemoryFaults {
        duplicate_permille: 1000,
        ..MemoryFaults::none()
    };
    let frames = chay(5, faults, 3);
    assert_eq!(frames.len(), 6, "mỗi frame phải tới hai lần");
    // Bản gốc (mốc giao 0) đến trước, bản trùng (mốc giao 1) đến sau.
    assert_eq!(frames[0], 0u32.to_le_bytes());
    assert_eq!(frames[1], 1u32.to_le_bytes());
    assert_eq!(frames[2], 2u32.to_le_bytes());
    assert_eq!(frames[3], 0u32.to_le_bytes());
    assert_eq!(frames[4], 1u32.to_le_bytes());
    assert_eq!(frames[5], 2u32.to_le_bytes());
}

#[test]
fn dao_thu_tu_giao() {
    let faults = MemoryFaults {
        reorder_permille: 1000,
        reorder_window_ms: 100,
        ..MemoryFaults::none()
    };
    let frames = chay(11, faults, 16);
    assert_eq!(frames.len(), 16, "đảo thứ tự không được mất frame");
    let goc: Vec<Vec<u8>> = (0..16u32).map(|i| i.to_le_bytes().to_vec()).collect();
    assert_ne!(frames, goc, "thứ tự giao phải khác thứ tự gửi");
    // Tập frame đến nơi vẫn đủ, chỉ thứ tự đổi.
    let mut den = frames.clone();
    den.sort();
    let mut den_chuan = goc.clone();
    den_chuan.sort();
    assert_eq!(den, den_chuan, "không được mất hay đổi nội dung frame");
}

#[test]
fn tre_lenh_lam_dong_ho_ao_tien() {
    let faults = MemoryFaults {
        delay_ms: 50,
        ..MemoryFaults::none()
    };
    let config = MemoryConfig {
        seed: 1,
        faults,
        timing: Timing::Virtual,
    };
    let (mut gui, mut nhan) = MemoryTransport::pair(config);
    gui.send_frame(b"x").expect("gửi");
    gui.close().expect("đóng");
    assert_eq!(nhan.recv_frame().expect("nhận"), Some(b"x".to_vec()));
    assert!(nhan.clock_ms() >= 50, "đồng hồ ảo phải đi qua độ trễ");
}

#[test]
fn doi_phuong_dong_roi_thi_ra_eof() {
    let (mut gui, mut nhan) = MemoryTransport::pair(MemoryConfig::default());
    gui.close().expect("đóng");
    assert_eq!(nhan.recv_frame().expect("nhận"), None, "phải ra EOF");
    assert!(nhan.is_eof());
    assert!(nhan.is_closed());
}

#[test]
fn gui_sau_khi_dong_bao_loi() {
    let (mut gui, _nhan) = MemoryTransport::pair(MemoryConfig::default());
    gui.close().expect("đóng");
    let err = gui.send_frame(b"x").expect_err("gửi sau khi đóng phải lỗi");
    assert!(matches!(err, NetError::Closed), "lỗi sai: {err}");
}

#[test]
fn frame_vuot_tran_bi_tu_choi() {
    let (mut gui, _nhan) =
        MemoryTransport::pair_with_limits(MemoryConfig::default(), FrameLimits::new(8));
    let err = gui
        .send_frame(&[0u8; 9])
        .expect_err("frame vượt trần phải lỗi");
    assert!(
        matches!(err, NetError::FrameTooLarge { len: 9, max: 8 }),
        "lỗi sai: {err}"
    );
}

#[test]
fn nhan_khong_chan_thi_tra_ngay() {
    let (mut gui, mut nhan) = MemoryTransport::pair(MemoryConfig::default());
    assert_eq!(nhan.try_recv_frame().expect("nhận không chặn"), None);
    gui.send_frame(b"ping").expect("gửi");
    assert_eq!(
        nhan.try_recv_frame().expect("nhận không chặn"),
        Some(b"ping".to_vec())
    );
    gui.close().expect("đóng");
    assert_eq!(nhan.try_recv_frame().expect("nhận không chặn"), None);
    assert!(nhan.is_eof());
}

#[test]
fn hai_dau_dung_chung_dong_ho_ao() {
    let config = MemoryConfig {
        seed: 9,
        faults: MemoryFaults {
            delay_ms: 30,
            ..MemoryFaults::none()
        },
        timing: Timing::Virtual,
    };
    let (mut gui, mut nhan) = MemoryTransport::pair(config);
    gui.send_frame(b"a").expect("gửi");
    assert_eq!(nhan.recv_frame().expect("nhận"), Some(b"a".to_vec()));
    assert_eq!(nhan.clock_ms(), gui.clock_ms(), "hai đầu chung đồng hồ");
}

#[test]
fn dung_duoc_voi_box_dyn_transport() {
    let (mut gui, nhan) = MemoryTransport::pair(MemoryConfig::default());
    let mut nhan: Box<dyn Transport> = Box::new(nhan);
    gui.send_frame(b"x").expect("gửi");
    assert_eq!(nhan.recv_frame().expect("nhận"), Some(b"x".to_vec()));
}

#[test]
fn che_do_thoi_gian_that_ngu_that() {
    let config = MemoryConfig {
        seed: 1,
        faults: MemoryFaults {
            delay_ms: 40,
            ..MemoryFaults::none()
        },
        timing: Timing::Real,
    };
    let (mut gui, mut nhan) = MemoryTransport::pair(config);
    let bat_dau = Instant::now();
    gui.send_frame(b"x").expect("gửi");
    assert_eq!(nhan.recv_frame().expect("nhận"), Some(b"x".to_vec()));
    assert!(
        bat_dau.elapsed() >= Duration::from_millis(30),
        "chế độ thời gian thật phải ngủ theo trễ khai báo"
    );
}
