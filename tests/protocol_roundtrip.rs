use play_room_core::{PlayerId, RoomId, SessionToken};
use play_room_protocol::{
    decode_client, decode_server, encode_client, encode_server, ClientEnvelope, ClientRequest,
    EnterRoomMode, ServerMessage, ServerResult, PROTOCOL_VERSION,
};

#[test]
fn client_message_round_trips() {
    let msg = ClientEnvelope {
        request_id: 7,
        request: ClientRequest::ListRooms,
    };
    let encoded = encode_client(&msg).unwrap();
    let decoded = decode_client(&encoded).unwrap();
    assert_eq!(msg, decoded);
}

#[test]
fn enter_room_auto_request_round_trips() {
    let msg = ClientEnvelope {
        request_id: 11,
        request: ClientRequest::EnterRoom {
            room_id: RoomId::new("testroom"),
            mode: EnterRoomMode::Auto,
        },
    };

    let encoded = encode_client(&msg).unwrap();
    let decoded = decode_client(&encoded).unwrap();

    assert_eq!(msg, decoded);
}

#[test]
fn update_display_name_request_round_trips() {
    let msg = ClientEnvelope {
        request_id: 12,
        request: ClientRequest::UpdateDisplayName {
            name: "Alicia".to_owned(),
        },
    };

    let encoded = encode_client(&msg).unwrap();
    let decoded = decode_client(&encoded).unwrap();

    assert_eq!(msg, decoded);
}

#[test]
fn update_match_format_request_round_trips() {
    let msg = ClientEnvelope {
        request_id: 13,
        request: ClientRequest::UpdateMatchFormat { target_score: 3 },
    };

    let encoded = encode_client(&msg).unwrap();
    let decoded = decode_client(&encoded).unwrap();

    assert_eq!(msg, decoded);
}

#[test]
fn welcome_response_round_trips_reconnect_metadata() {
    let msg = ServerMessage::Response {
        request_id: 1,
        result: ServerResult::Welcome {
            player_id: PlayerId::new("player-alice"),
            display_name: "Alice".to_owned(),
            reconnect_token: SessionToken::new("session-alice"),
            protocol_version: PROTOCOL_VERSION,
            reconnected: true,
            stale_token_replaced: false,
            room_restored: true,
        },
    };

    let encoded = encode_server(&msg).unwrap();
    let decoded = decode_server(&encoded).unwrap();

    assert_eq!(msg, decoded);
}
