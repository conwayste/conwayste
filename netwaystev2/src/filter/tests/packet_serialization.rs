use bincode::serialize;
use crate::filter::PingPong;
use crate::protocol::Packet;

// IMPORTANT: if these two tests break, it's likely the Go registrar is broken as well.
#[test]
fn test_serialize_getstatus() {
    let packet = Packet::GetStatus {
        ping: PingPong {
            nonce: 0x123456789ABCDEF0,
        },
    };
    let bytes = serialize(&packet).unwrap();
    // Keep this in sync with the registrar (packet/packet_test.go)
    let expected = vec![
        4, 0, 0, 0, // 4=GetStatus
        0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12, // ping.nonce
    ];
    assert_eq!(bytes, expected);
}

#[test]
fn test_serialize_status() {
    let packet = Packet::Status {
        pong:           PingPong {
            nonce: 0x123456789ABCDEF0,
        },
        server_version: "ver".to_owned(),
        player_count:   123,
        room_count:     456,
        server_name:    "nm".to_owned(),
    };
    let bytes = serialize(&packet).unwrap();
    // Keep this in sync with the registrar (packet/packet_test.go)
    let expected = vec![
        5, 0, 0, 0, // 5=Status
        0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12, // pong.nonce
        3, 0, 0, 0, 0, 0, 0, 0, 118, 101, 114, // server_version
        123, 0, 0, 0, 0, 0, 0, 0, // player_count
        200, 1, 0, 0, 0, 0, 0, 0, // room_count
        2, 0, 0, 0, 0, 0, 0, 0, 110, 109, // server_name
    ];
    assert_eq!(bytes, expected);
}
