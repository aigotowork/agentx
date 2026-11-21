//! WebSocket framing over the managed daemon's Unix control socket. The native
//! `proxy` only copies bytes; it does not translate JSONL to WebSocket messages.

use std::io;
use std::net::Shutdown;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use tungstenite::{Error, Message, WebSocket, protocol::WebSocketConfig};

use super::{ConnectionFailure, NativeConnection, Receive, ServiceFailure};

const IO_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

pub(in crate::integrations::codex) struct UnixConnection {
    socket: WebSocket<UnixStream>,
    closed: bool,
}

impl UnixConnection {
    pub(super) fn connect(path: &Path) -> Result<Self, ServiceFailure> {
        let stream = UnixStream::connect(path).map_err(|error| ServiceFailure::Unavailable(format!("connect {}: {error}", path.display())))?;
        Self::handshake_timeouts(&stream)?;
        let (socket, _) = tungstenite::client::client_with_config("ws://localhost/", stream, Some(Self::config())).map_err(|error| ServiceFailure::Protocol(format!("WebSocket handshake on {}: {error}", path.display())))?;
        Self::from_socket(socket)
    }

    pub(in crate::integrations::codex) fn accept(stream: UnixStream) -> Result<Self, ServiceFailure> {
        Self::handshake_timeouts(&stream)?;
        let socket = tungstenite::accept_with_config(stream, Some(Self::config())).map_err(|error| ServiceFailure::Protocol(format!("TUI WebSocket handshake: {error}")))?;
        Self::from_socket(socket)
    }

    fn config() -> WebSocketConfig {
        WebSocketConfig::default().max_message_size(Some(MAX_FRAME_BYTES)).max_frame_size(Some(MAX_FRAME_BYTES)).max_write_buffer_size(2 * MAX_FRAME_BYTES)
    }

    fn handshake_timeouts(stream: &UnixStream) -> Result<(), ServiceFailure> {
        stream.set_read_timeout(Some(IO_TIMEOUT)).and_then(|()| stream.set_write_timeout(Some(IO_TIMEOUT))).map_err(|error| ServiceFailure::Unavailable(error.to_string()))
    }

    fn from_socket(socket: WebSocket<UnixStream>) -> Result<Self, ServiceFailure> {
        socket.get_ref().set_nonblocking(true).map_err(|error| ServiceFailure::Unavailable(error.to_string()))?;
        Ok(Self { socket, closed: false })
    }

    pub(super) fn peer_pid(&self) -> Result<u32, ServiceFailure> {
        let mut peer: libc::ucred = unsafe { std::mem::zeroed() };
        let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        // SAFETY: the socket is live, and both output pointers refer to valid
        // storage of the supplied length for the duration of getsockopt.
        let result = unsafe { libc::getsockopt(self.socket.get_ref().as_raw_fd(), libc::SOL_SOCKET, libc::SO_PEERCRED, (&mut peer as *mut libc::ucred).cast(), &mut length) };
        if result != 0 {
            return Err(ServiceFailure::Unavailable(io::Error::last_os_error().to_string()));
        }
        u32::try_from(peer.pid).ok().filter(|pid| *pid != 0).ok_or_else(|| ServiceFailure::Protocol("native Unix peer has no valid PID".into()))
    }

    /// A deadline covers the whole fragmented frame, rather than restarting
    /// an OS read timeout after every arriving fragment.
    fn ready(&self, events: libc::c_short, timeout: Duration) -> Result<bool, ConnectionFailure> {
        let started = Instant::now();
        loop {
            let remaining = timeout.saturating_sub(started.elapsed());
            let millis = remaining.as_millis().saturating_add(u128::from(!remaining.subsec_nanos().is_multiple_of(1_000_000))).min(i32::MAX as u128) as i32;
            let mut descriptor = libc::pollfd { fd: self.socket.get_ref().as_raw_fd(), events, revents: 0 };
            // SAFETY: descriptor is a valid one-element pollfd array; the
            // borrowed WebSocket keeps its file descriptor alive.
            let result = unsafe { libc::poll(&mut descriptor, 1, millis) };
            if result >= 0 {
                return Ok(result != 0);
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(io_failure(error));
            }
            if started.elapsed() >= timeout {
                return Ok(false);
            }
        }
    }
}

impl NativeConnection for UnixConnection {
    fn send(&mut self, frame: &str) -> Result<(), ConnectionFailure> {
        if self.closed {
            return Err(ConnectionFailure::Closed);
        }
        if frame.len() > MAX_FRAME_BYTES {
            return Err(ConnectionFailure::InvalidFrame("native message exceeds 16 MiB transport limit".into()));
        }
        validate_frame(frame)?;
        // tungstenite retains a frame on WouldBlock. Enqueue once, then only
        // flush; retrying send would duplicate a potentially accepted input.
        match self.socket.write(Message::Text(frame.to_owned().into())) {
            Ok(()) => {}
            Err(Error::Io(error)) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(connection_failure(error)),
        }
        let started = Instant::now();
        loop {
            match self.socket.flush() {
                Ok(()) => return Ok(()),
                Err(Error::Io(error)) if error.kind() == io::ErrorKind::WouldBlock => {
                    let remaining = IO_TIMEOUT.saturating_sub(started.elapsed());
                    if remaining.is_zero() || !self.ready(libc::POLLOUT, remaining)? {
                        return Err(ConnectionFailure::Io("native write timeout; input outcome is unknown".into()));
                    }
                }
                Err(error) => return Err(connection_failure(error)),
            }
        }
    }

    fn receive(&mut self, timeout: Duration) -> Result<Receive, ConnectionFailure> {
        if self.closed {
            return Ok(Receive::Closed);
        }
        let start = Instant::now();
        loop {
            match self.socket.read() {
                Ok(Message::Text(frame)) => {
                    validate_frame(frame.as_str())?;
                    return Ok(Receive::Frame(frame.to_string()));
                }
                Ok(Message::Close(_)) | Err(Error::ConnectionClosed | Error::AlreadyClosed) => {
                    self.closed = true;
                    return Ok(Receive::Closed);
                }
                Ok(Message::Ping(_) | Message::Pong(_)) => {}
                Ok(_) => return Err(ConnectionFailure::InvalidFrame("expected a native JSON text message".into())),
                Err(Error::Io(error)) if error.kind() == io::ErrorKind::WouldBlock => {
                    let remaining = timeout.saturating_sub(start.elapsed());
                    if remaining.is_zero() || !self.ready(libc::POLLIN, remaining)? {
                        return Ok(Receive::TimedOut);
                    }
                }
                Err(error) => return Err(connection_failure(error)),
            }
            if start.elapsed() >= timeout {
                return Ok(Receive::TimedOut);
            }
        }
    }

    fn close(&mut self) -> Result<(), ConnectionFailure> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        let _ = self.socket.close(None);
        self.socket.get_ref().shutdown(Shutdown::Both).map_err(io_failure)
    }
}

impl Drop for UnixConnection {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn validate_frame(frame: &str) -> Result<(), ConnectionFailure> {
    match serde_json::from_str::<Box<serde_json::value::RawValue>>(frame) {
        Ok(value) if value.get().trim_start().starts_with('{') => Ok(()),
        Ok(_) => Err(ConnectionFailure::InvalidFrame("expected one JSON-RPC object".into())),
        Err(error) => Err(ConnectionFailure::InvalidFrame(error.to_string())),
    }
}

fn io_failure(error: io::Error) -> ConnectionFailure {
    ConnectionFailure::Io(error.to_string())
}

fn connection_failure(error: Error) -> ConnectionFailure {
    match error {
        Error::ConnectionClosed | Error::AlreadyClosed => ConnectionFailure::Closed,
        other => ConnectionFailure::Io(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn unix_websocket_preserves_json_and_closes_only_connection() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("control.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            assert_eq!(socket.read().unwrap().into_text().unwrap(), r#"{"id":7,"method":"thread/read"}"#);
            socket.send(Message::Ping(vec![1].into())).unwrap();
            socket.send(Message::Text(r#"{"id":7,"result":{"text":"中文\nline"}}"#.into())).unwrap();
            loop {
                match socket.read() {
                    Ok(Message::Close(_)) | Err(Error::ConnectionClosed) => break,
                    Ok(Message::Pong(_)) => {}
                    other => panic!("unexpected frame: {other:?}"),
                }
            }
            // The listening endpoint remains usable after this client closes.
            assert!(listener.local_addr().is_ok());
        });
        let mut connection = UnixConnection::connect(&path).unwrap();
        connection.send(r#"{"id":7,"method":"thread/read"}"#).unwrap();
        assert_eq!(connection.receive(Duration::from_secs(1)).unwrap(), Receive::Frame(r#"{"id":7,"result":{"text":"中文\nline"}}"#.into()));
        connection.close().unwrap();
        assert_eq!(connection.receive(Duration::ZERO).unwrap(), Receive::Closed);
        server.join().unwrap();
    }
}
