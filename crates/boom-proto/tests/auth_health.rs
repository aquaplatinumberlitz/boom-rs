//! Nghiệm thu kind + DTO auth/health (ADR 0009 Addendum A §A.1–§A.2; ADR 0011 §8.2.1).
//!
//! Bao gồm: round-trip mọi DTO mới, golden byte có envelope cho `Auth` và `HealthRequest`,
//! phân loại kind sai kênh, và giới hạn biên.

use boom_proto::dto::auth::{Auth, AuthFail, AuthFailReason, AuthOk, AuthOp, Logout};
use boom_proto::dto::health::{HealthRequest, HealthResponse, HealthStatus};
use boom_proto::kind::{Channel, Role};
use boom_proto::message::decode_frame_on;
use boom_proto::{
    MessageKind, ProtoError, ProtocolLimits, decode_frame, encode_frame, read_envelope,
};

const LIMITS: ProtocolLimits = ProtocolLimits::v1();

fn auth() -> Auth {
    Auth {
        op: AuthOp::Login,
        username: "u".into(),
        password: "p".into(),
    }
}

fn auth_ok() -> AuthOk {
    AuthOk {
        account_id: 7,
        display_name: "player".into(),
        token: "ZG9uZS10b2tlbi1zZWNyZXQtdmFsdWU".into(),
        expires_at_ms: 1_700_000_000_000,
    }
}

fn health_response() -> HealthResponse {
    HealthResponse {
        status: HealthStatus::Healthy,
        uptime_ms: 1_234,
        started_at_ms: 1_700_000_000_000,
        instance_id: "0123456789abcdef".into(),
        listen_addrs: vec!["0.0.0.0:17889".into()],
        admin_addr: "127.0.0.1:17890".into(),
        last_error_ts: None,
        db_ok: true,
        schema_version: 1,
        expected_schema_version: 1,
        integrity_check: "ok".into(),
        rules_profile_id: "kr00967-v1-dev".into(),
        rules_hash: 0x1234_5678_9abc_def0,
        content_sha256: [7u8; 32],
        accounts: 3,
        active_sessions: 1,
        problems: vec![],
    }
}

#[test]
fn round_trip_every_new_message() {
    let auth_frame = encode_frame(&auth(), &LIMITS).unwrap();
    let auth_ok_frame = encode_frame(&auth_ok(), &LIMITS).unwrap();
    let auth_fail_frame = encode_frame(
        &AuthFail {
            reason: AuthFailReason::InvalidCredentials,
        },
        &LIMITS,
    )
    .unwrap();
    let logout_frame = encode_frame(&Logout {}, &LIMITS).unwrap();
    let health_request_frame = encode_frame(&HealthRequest {}, &LIMITS).unwrap();
    let health_response_frame = encode_frame(&health_response(), &LIMITS).unwrap();

    for (kind, bytes) in [
        (MessageKind::Auth, &auth_frame),
        (MessageKind::AuthOk, &auth_ok_frame),
        (MessageKind::AuthFail, &auth_fail_frame),
        (MessageKind::Logout, &logout_frame),
        (MessageKind::HealthRequest, &health_request_frame),
        (MessageKind::HealthResponse, &health_response_frame),
    ] {
        let (envelope, _) = read_envelope(bytes).unwrap();
        assert_eq!(envelope.kind, kind, "kind của frame phải khớp message");
        assert_eq!(envelope.payload_len as usize, bytes.len() - 16);
    }

    assert_eq!(decode_frame::<Auth>(&auth_frame, &LIMITS).unwrap(), auth());
    assert_eq!(
        decode_frame::<AuthOk>(&auth_ok_frame, &LIMITS).unwrap(),
        auth_ok()
    );
    assert_eq!(
        decode_frame::<AuthFail>(&auth_fail_frame, &LIMITS).unwrap(),
        AuthFail {
            reason: AuthFailReason::InvalidCredentials,
        }
    );
    assert_eq!(
        decode_frame::<Logout>(&logout_frame, &LIMITS).unwrap(),
        Logout {}
    );
    assert_eq!(
        decode_frame::<HealthRequest>(&health_request_frame, &LIMITS).unwrap(),
        HealthRequest {}
    );
    assert_eq!(
        decode_frame::<HealthResponse>(&health_response_frame, &LIMITS).unwrap(),
        health_response()
    );
}

/// Golden byte của `Auth`: envelope 16 byte + payload postcard (ADR 0009 Addendum A §A.2).
#[test]
fn golden_bytes_of_auth() {
    let frame = encode_frame(&auth(), &LIMITS).unwrap();

    let envelope = [
        0x42, 0x4F, 0x4F, 0x4D, // "BOOM"
        0x01, 0x00, // wire_schema 1
        0x04, 0x00, // kind 0x0004 (Auth)
        0x00, 0x00, // flags 0
        0x05, 0x00, 0x00, 0x00, // payload_len 5
        0x00, 0x00, // reserved 0
    ];
    // op = Login (0), username = "u" (len 1), password = "p" (len 1).
    let payload = [0x00, 0x01, b'u', 0x01, b'p'];

    assert_eq!(&frame[..16], &envelope);
    assert_eq!(&frame[16..], &payload);
    assert_eq!(frame.len(), 21);

    let decoded = decode_frame::<Auth>(&frame, &LIMITS).unwrap();
    assert_eq!(decoded, auth());
    assert_eq!(decoded.op, AuthOp::Login);
}

/// Golden byte của `HealthRequest`: payload rỗng (message không có trường).
#[test]
fn golden_bytes_of_health_request() {
    let frame = encode_frame(&HealthRequest {}, &LIMITS).unwrap();

    let envelope = [
        0x42, 0x4F, 0x4F, 0x4D, // "BOOM"
        0x01, 0x00, // wire_schema 1
        0x50, 0x00, // kind 0x0050 (HealthRequest)
        0x00, 0x00, // flags 0
        0x00, 0x00, 0x00, 0x00, // payload_len 0
        0x00, 0x00, // reserved 0
    ];

    assert_eq!(&frame[..16], &envelope);
    assert_eq!(frame.len(), 16);
    assert_eq!(
        decode_frame::<HealthRequest>(&frame, &LIMITS).unwrap(),
        HealthRequest {}
    );
}

/// Kênh game không nhận kind health; admin listener chỉ nhận `HealthRequest`.
#[test]
fn kind_wrong_channel_is_classified_correctly() {
    let health_request = encode_frame(&HealthRequest {}, &LIMITS).unwrap();
    let health_response = encode_frame(&health_response(), &LIMITS).unwrap();
    let auth = encode_frame(&auth(), &LIMITS).unwrap();
    let auth_ok = encode_frame(&auth_ok(), &LIMITS).unwrap();

    // Kind health trên kênh game: sai kênh xử lý như kind lạ.
    let err =
        decode_frame_on::<HealthRequest>(&health_request, &LIMITS, Channel::Game, Role::Server)
            .unwrap_err();
    assert!(
        matches!(err, ProtoError::UnknownKind(0x0050)),
        "thực tế: {err:?}"
    );

    // Kind game trên kênh admin: cũng là sai kênh.
    let err = decode_frame_on::<Auth>(&auth, &LIMITS, Channel::Admin, Role::Server).unwrap_err();
    assert!(
        matches!(err, ProtoError::UnknownKind(0x0004)),
        "thực tế: {err:?}"
    );

    // Admin listener chỉ **nhận** `HealthRequest`; `HealthResponse` là chiều server→client.
    let err =
        decode_frame_on::<HealthResponse>(&health_response, &LIMITS, Channel::Admin, Role::Server)
            .unwrap_err();
    assert!(
        matches!(err, ProtoError::UnknownKind(0x0051)),
        "thực tế: {err:?}"
    );

    // Client gửi `AuthOk` (chiều server→client) lên kênh game cũng bị từ chối.
    let err =
        decode_frame_on::<AuthOk>(&auth_ok, &LIMITS, Channel::Game, Role::Server).unwrap_err();
    assert!(
        matches!(err, ProtoError::UnknownKind(0x0005)),
        "thực tế: {err:?}"
    );

    // Đúng kênh thì decode được.
    assert!(
        decode_frame_on::<HealthRequest>(&health_request, &LIMITS, Channel::Admin, Role::Server)
            .is_ok(),
        "HealthRequest phải hợp lệ trên kênh admin"
    );
    assert!(
        decode_frame_on::<Auth>(&auth, &LIMITS, Channel::Game, Role::Server).is_ok(),
        "Auth phải hợp lệ trên kênh game"
    );

    // Client nhận `AuthOk` từ server trên kênh game thì hợp lệ.
    assert!(
        decode_frame_on::<AuthOk>(&auth_ok, &LIMITS, Channel::Game, Role::Client).is_ok(),
        "AuthOk phải hợp lệ với client trên kênh game"
    );
}

/// Biên độ dài `username`/`password` (ADR 0009 Addendum A §A.2).
#[test]
fn auth_over_limit_is_rejected() {
    let mut over = auth();
    over.username = "u".repeat(LIMITS.max_username_bytes + 1);
    let frame = encode_frame(&over, &LIMITS).unwrap();
    let err = decode_frame::<Auth>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(
            err,
            ProtoError::Invalid {
                field: "username",
                ..
            }
        ),
        "thực tế: {err:?}"
    );

    let mut over = auth();
    over.password = "p".repeat(LIMITS.max_password_bytes + 1);
    let frame = encode_frame(&over, &LIMITS).unwrap();
    let err = decode_frame::<Auth>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(
            err,
            ProtoError::Invalid {
                field: "password",
                ..
            }
        ),
        "thực tế: {err:?}"
    );

    // Đúng biên thì hợp lệ.
    let mut at_limit = auth();
    at_limit.username = "u".repeat(LIMITS.max_username_bytes);
    at_limit.password = "p".repeat(LIMITS.max_password_bytes);
    let frame = encode_frame(&at_limit, &LIMITS).unwrap();
    assert!(decode_frame::<Auth>(&frame, &LIMITS).is_ok());
}

/// Miền giá trị của `HealthResponse` trên wire (ADR 0011 §8.2.1).
#[test]
fn health_response_wire_domain_is_checked() {
    // `Healthy` mà `problems` không rỗng: không hợp lệ (exit code 0 đòi `problems` rỗng).
    let mut resp = health_response();
    resp.problems = vec!["db hỏng".into()];
    let frame = encode_frame(&resp, &LIMITS).unwrap();
    let err = decode_frame::<HealthResponse>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(
            err,
            ProtoError::Invalid {
                field: "problems",
                ..
            }
        ),
        "thực tế: {err:?}"
    );

    // `Unreachable`/`Timeout` là kết quả của CLI, không phải trạng thái trên wire.
    for status in [HealthStatus::Unreachable, HealthStatus::Timeout] {
        let mut resp = health_response();
        resp.status = status;
        let frame = encode_frame(&resp, &LIMITS).unwrap();
        let err = decode_frame::<HealthResponse>(&frame, &LIMITS).unwrap_err();
        assert!(
            matches!(
                err,
                ProtoError::Invalid {
                    field: "status",
                    ..
                }
            ),
            "thực tế: {err:?}"
        );
    }

    // `Healthy` + `problems` rỗng: hợp lệ.
    let frame = encode_frame(&health_response(), &LIMITS).unwrap();
    assert!(decode_frame::<HealthResponse>(&frame, &LIMITS).is_ok());

    // `Unhealthy` + `problems` không rỗng: hợp lệ.
    let mut resp = health_response();
    resp.status = HealthStatus::Unhealthy;
    resp.problems = vec!["db hỏng".into(), "integrity check fail".into()];
    let frame = encode_frame(&resp, &LIMITS).unwrap();
    assert!(decode_frame::<HealthResponse>(&frame, &LIMITS).is_ok());
}

/// Số mục `problems` trong trần của `ProtocolLimits`.
#[test]
fn health_response_problems_over_limit_is_rejected() {
    let mut over = health_response();
    over.status = HealthStatus::Unhealthy;
    over.problems = (0..LIMITS.max_problems + 1)
        .map(|i| format!("problem {i}"))
        .collect();
    let frame = encode_frame(&over, &LIMITS).unwrap();
    let err = decode_frame::<HealthResponse>(&frame, &LIMITS).unwrap_err();
    assert!(
        matches!(
            err,
            ProtoError::CountLimit {
                field: "problems",
                len: 33,
                max: 32
            }
        ),
        "thực tế: {err:?}"
    );

    let mut at_limit = health_response();
    at_limit.status = HealthStatus::Unhealthy;
    at_limit.problems = (0..LIMITS.max_problems)
        .map(|i| format!("problem {i}"))
        .collect();
    let frame = encode_frame(&at_limit, &LIMITS).unwrap();
    assert!(decode_frame::<HealthResponse>(&frame, &LIMITS).is_ok());
}

/// `Debug` của `AuthOk` không được chứa token (ADR 0009 Addendum A §A.2).
#[test]
fn auth_ok_debug_hides_token() {
    let text = format!("{:?}", auth_ok());
    assert!(
        !text.contains("ZG9uZS10b2tlbi1zZWNyZXQtdmFsdWU"),
        "thực tế: {text}"
    );
    assert!(text.contains("<redacted>"), "thực tế: {text}");
}
