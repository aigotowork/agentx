//! Independent native-transport counterexamples for A6 and A9. These use real
//! Unix sockets/WebSocket peers, without invoking daemon management or a model.

use super::transport::UnixConnection;
use super::{NativeConnection, Receive};
use std::io::Write;
use std::os::unix::net::UnixListener;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tungstenite::Message;

#[test]
fn acceptance_a6_transport_keeps_opaque_json_numbers_in_both_directions() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("native.sock");
    let listener = UnixListener::bind(&socket_path).unwrap();
    let outbound = r#"{"id":1,"method":"future/native","params":{"huge":1e400,"nestedId":123456789012345678901234567890}}"#;
    let inbound = r#"{"id":1,"result":{"huge":1e400,"literal":1.2300e-7}}"#;
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut websocket = tungstenite::accept(stream).unwrap();
        assert_eq!(websocket.read().unwrap().into_text().unwrap(), outbound);
        websocket.send(Message::Text(inbound.into())).unwrap();
    });
    let mut connection = UnixConnection::connect(&socket_path).unwrap();
    connection.send(outbound).expect("valid opaque JSON request was rejected");
    assert_eq!(connection.receive(Duration::from_secs(2)).unwrap(), Receive::Frame(inbound.to_owned()));
    server.join().unwrap();
}

#[test]
fn acceptance_a9_partial_websocket_frame_survives_read_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("native.sock");
    let listener = UnixListener::bind(&socket_path).unwrap();
    let (partial_sent, ready) = mpsc::channel();
    let (continue_frame, released) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut websocket = tungstenite::accept(stream).unwrap();
        let payload = br#"{"method":"fixture/event","params":{"text":"complete"}}"#;
        assert!(payload.len() < 126);
        let stream = websocket.get_mut();
        stream.write_all(&[0x81, payload.len() as u8]).unwrap();
        stream.write_all(&payload[..8]).unwrap();
        stream.flush().unwrap();
        partial_sent.send(()).unwrap();
        released.recv_timeout(Duration::from_secs(2)).unwrap();
        stream.write_all(&payload[8..]).unwrap();
        stream.flush().unwrap();
    });
    let mut connection = UnixConnection::connect(&socket_path).unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let start = Instant::now();
    assert_eq!(connection.receive(Duration::from_millis(30)).unwrap(), Receive::TimedOut);
    assert!(start.elapsed() < Duration::from_secs(1), "bounded receive stalled on a partial frame");
    continue_frame.send(()).unwrap();
    assert_eq!(connection.receive(Duration::from_secs(2)).unwrap(), Receive::Frame(r#"{"method":"fixture/event","params":{"text":"complete"}}"#.into()));
    server.join().unwrap();
}

#[test]
fn acceptance_a6_large_fragmented_websocket_message_is_one_complete_native_frame() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("native.sock");
    let listener = UnixListener::bind(&socket_path).unwrap();
    let text = serde_json::json!({"method":"fixture/long", "params":{"text":"中文😀".repeat(40000)}}).to_string();
    assert!(text.len() > 256 * 1024);
    let expected = text.clone();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut websocket = tungstenite::accept(stream).unwrap();
        let split = text.len() / 2;
        // Split in the middle of UTF-8 if necessary. RFC6455 assembles message
        // fragments before requiring a complete valid UTF-8 text payload.
        for (opcode, bytes) in [(0x01u8, &text.as_bytes()[..split]), (0x80u8, &text.as_bytes()[split..])] {
            let stream = websocket.get_mut();
            stream.write_all(&[opcode, 127]).unwrap();
            stream.write_all(&(bytes.len() as u64).to_be_bytes()).unwrap();
            stream.write_all(bytes).unwrap();
            stream.flush().unwrap();
        }
    });
    let mut connection = UnixConnection::connect(&socket_path).unwrap();
    assert_eq!(connection.receive(Duration::from_secs(2)).unwrap(), Receive::Frame(expected));
    server.join().unwrap();
}

#[test]
fn acceptance_a9_dropping_one_transport_leaves_sibling_connection_usable() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("native.sock");
    let listener = UnixListener::bind(&socket_path).unwrap();
    let (first_closed, closed) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (first, _) = listener.accept().unwrap();
        first.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut first = tungstenite::accept(first).unwrap();
        let (second, _) = listener.accept().unwrap();
        second.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut second = tungstenite::accept(second).unwrap();
        assert!(matches!(first.read().unwrap(), Message::Close(_)));
        first_closed.send(()).unwrap();
        assert_eq!(second.read().unwrap().into_text().unwrap(), r#"{"id":"alive","method":"thread/list"}"#);
        second.send(Message::Text(r#"{"id":"alive","result":{"survived":true}}"#.into())).unwrap();
        assert!(listener.local_addr().is_ok());
    });
    let first = UnixConnection::connect(&socket_path).unwrap();
    let mut second = UnixConnection::connect(&socket_path).unwrap();
    drop(first);
    closed.recv_timeout(Duration::from_secs(2)).unwrap();
    second.send(r#"{"id":"alive","method":"thread/list"}"#).unwrap();
    assert_eq!(second.receive(Duration::from_secs(2)).unwrap(), Receive::Frame(r#"{"id":"alive","result":{"survived":true}}"#.into()));
    server.join().unwrap();
}
