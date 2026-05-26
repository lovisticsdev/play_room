use play_room_protocol::{decode_client, encode_client, ClientEnvelope, ClientRequest};

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
