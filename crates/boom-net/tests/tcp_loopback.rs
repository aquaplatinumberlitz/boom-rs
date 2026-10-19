//! Transport TCP: loopback nhiều frame, trần kích thước, ngắt kết nối (ADR 0006).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use boom_net::{FrameLimits, NetError, TcpTransport, Transport};

fn loopback() -> (TcpListener, std::net::SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = listener.local_addr().expect("local addr");
    (listener, addr)
}

#[test]
fn loopback_giao_nhieu_frame_va_nhan_dung_thu_tu() {
    let (listener, addr) = loopback();
    const SO_FRAME: u32 = 64;

    let server = thread::spawn(move || {
        let mut conn = TcpTransport::accept(&listener).expect("accept");
        let mut received = Vec::new();
        for _ in 0..SO_FRAME {
            received.push(
                conn.recv_frame()
                    .expect("server nhận frame")
                    .expect("server chưa thấy EOF"),
            );
        }
        // Trả lại từng frame kèm số thứ tự để kiểm cả chiều ngược.
        for (i, frame) in received.iter().enumerate() {
            let mut echo = frame.clone();
            echo.extend_from_slice(&(i as u32).to_le_bytes());
            conn.send_frame(&echo).expect("server gửi frame");
        }
        conn.close().expect("server đóng");
        conn
    });

    let mut client = TcpTransport::connect(addr).expect("connect");
    assert!(
        client.nodelay().expect("đọc nodelay"),
        "TCP_NODELAY phải bật"
    );
    for i in 0..SO_FRAME {
        let mut payload = b"frame-".to_vec();
        payload.extend_from_slice(&i.to_le_bytes());
        client.send_frame(&payload).expect("client gửi frame");
    }
    for i in 0..SO_FRAME {
        let got = client
            .recv_frame()
            .expect("client nhận frame")
            .expect("client chưa thấy EOF");
        let mut expected = b"frame-".to_vec();
        expected.extend_from_slice(&i.to_le_bytes());
        expected.extend_from_slice(&i.to_le_bytes());
        assert_eq!(got, expected, "frame thứ {i} sai");
    }

    let conn = server.join().expect("server thread");
    assert!(conn.is_closed(), "server đã đóng kết nối");
}

#[test]
fn frame_vuot_tran_bi_tu_choi_khi_gui() {
    let (listener, addr) = loopback();
    let server = thread::spawn(move || {
        let mut conn = TcpTransport::accept(&listener).expect("accept");
        // Giữ kết nối mở tới khi client đóng.
        let _ = conn.recv_frame();
    });

    let mut client = TcpTransport::connect(addr)
        .expect("connect")
        .with_limits(FrameLimits::new(16));
    let err = client
        .send_frame(&[0u8; 17])
        .expect_err("frame vượt trần phải bị từ chối");
    assert!(
        matches!(err, NetError::FrameTooLarge { len: 17, max: 16 }),
        "lỗi sai: {err}"
    );
    // Từ chối trước khi ghi: kết nối vẫn dùng được và frame nhỏ vẫn gửi được.
    client.send_frame(b"ok").expect("frame nhỏ vẫn gửi được");

    server.join().expect("server thread");
}

#[test]
fn frame_vuot_tran_bi_tu_choi_khi_nhan() {
    let (listener, addr) = loopback();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        // Ghi thẳng tiền tố độ dài vượt trần, không đi qua transport.
        stream
            .write_all(&1000u32.to_le_bytes())
            .expect("write prefix");
        stream.write_all(&[0u8; 1000]).expect("write payload");
        stream.flush().expect("flush");
        let mut sink = [0u8; 1];
        let _ = stream.read(&mut sink);
    });

    let mut client = TcpTransport::connect(addr)
        .expect("connect")
        .with_limits(FrameLimits::new(16));
    let err = client
        .recv_frame()
        .expect_err("frame vượt trần phải bị từ chối");
    assert!(
        matches!(err, NetError::FrameTooLarge { len: 1000, max: 16 }),
        "lỗi sai: {err}"
    );
    // Dòng byte không còn đáng tin: transport phải đóng.
    assert!(client.is_closed(), "transport phải đóng sau lỗi framing");

    server.join().expect("server thread");
}

#[test]
fn ngat_ket_noi_bao_loi_ro() {
    let (listener, addr) = loopback();
    let server = thread::spawn(move || {
        let conn = TcpTransport::accept(&listener).expect("accept");
        drop(conn); // đối phương biến mất
    });

    let mut client = TcpTransport::connect(addr).expect("connect");
    assert_eq!(
        client.recv_frame().expect("recv không lỗi"),
        None,
        "đối phương đóng thì nhận phải ra EOF"
    );
    assert!(client.is_closed());
    let err = client
        .send_frame(b"x")
        .expect_err("gửi sau khi đóng phải lỗi");
    assert!(matches!(err, NetError::Closed), "lỗi sai: {err}");

    server.join().expect("server thread");
}

#[test]
fn frame_gui_truoc_khi_dong_van_nhan_duoc() {
    let (listener, addr) = loopback();
    let server = thread::spawn(move || {
        let mut conn = TcpTransport::accept(&listener).expect("accept");
        conn.send_frame(b"bye").expect("gửi frame");
        conn.close().expect("đóng");
    });

    let mut client = TcpTransport::connect(addr).expect("connect");
    assert_eq!(
        client.recv_frame().expect("recv"),
        Some(b"bye".to_vec()),
        "frame gửi trước khi đóng phải tới trước EOF"
    );
    assert_eq!(client.recv_frame().expect("recv"), None);

    server.join().expect("server thread");
}

#[test]
fn dong_roi_roi_gui_bao_loi() {
    let (listener, addr) = loopback();
    let server = thread::spawn(move || {
        let mut conn = TcpTransport::accept(&listener).expect("accept");
        let _ = conn.recv_frame();
    });

    let mut client = TcpTransport::connect(addr).expect("connect");
    client.close().expect("đóng");
    assert!(client.is_closed());
    let err = client
        .send_frame(b"x")
        .expect_err("gửi sau khi đóng phải lỗi");
    assert!(matches!(err, NetError::Closed), "lỗi sai: {err}");
    let err = client.recv_frame().expect_err("nhận sau khi đóng phải lỗi");
    assert!(matches!(err, NetError::Closed), "lỗi sai: {err}");

    server.join().expect("server thread");
}
