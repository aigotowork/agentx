//! TCP injection server — accepts text on a local port and writes to PTY master.

use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::time::Duration;

/// Magic prefix for query commands (not injection)
const QUERY_PREFIX: u8 = 0x00;
// Bound incomplete requests as well as the PTY queue. Larger requests are
// rejected rather than letting a sender hold unbounded memory in the proxy.
const MAX_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_CLIENTS: usize = 32;

/// Result of reading from an inject client
pub enum InjectResult {
    /// Text to inject into PTY
    Inject(String),
    /// Query command — client removed from vec, caller must respond via stream
    Query(QueryClient),
    /// No data ready yet
    Pending,
}

/// A query client removed from the connection pool, ready for response
pub struct QueryClient {
    stream: TcpStream,
    pub command: QueryCommand,
}

#[derive(Debug)]
pub enum QueryCommand {
    Screen,
    GuardedWake(GuardedWake),
    Unknown,
}

/// Fixed internal operations; the guard never grants authority for arbitrary
/// text or approval keys on behalf of the automatic mailbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WakeKey {
    Marker,
    Enter,
    RecoveryMarker,
    RecoveryEnter,
}

impl WakeKey {
    fn wire(self) -> &'static str {
        match self {
            Self::Marker => "marker",
            Self::Enter => "enter",
            Self::RecoveryMarker => "recover-marker",
            Self::RecoveryEnter => "recover-enter",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GuardedWake {
    key: WakeKey,
    frame: u64,
    user_input: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WakeWrite {
    Written,
    Deferred,
    Unknown,
}

impl GuardedWake {
    pub(crate) fn new(key: WakeKey, frame: u64, user_input: u64) -> Self {
        Self { key, frame, user_input }
    }

    fn parse(command: &str) -> Option<Self> {
        let mut words = command.split_ascii_whitespace();
        if words.next()? != "CODEX-WAKE" {
            return None;
        }
        let key = match words.next()? {
            "marker" => WakeKey::Marker,
            "enter" => WakeKey::Enter,
            "recover-marker" => WakeKey::RecoveryMarker,
            "recover-enter" => WakeKey::RecoveryEnter,
            _ => return None,
        };
        let request = Self::new(key, words.next()?.parse().ok()?, words.next()?.parse().ok()?);
        words.next().is_none().then_some(request)
    }

    pub(crate) fn bytes(self, state: &crate::messaging::delivery::ScreenState) -> Option<&'static [u8]> {
        use crate::messaging::delivery::{SUBMIT_SETTLE_COOLDOWN_MS, WAKE_TRIGGER};
        if self.frame != state.codex_frame
            || self.user_input != state.codex_user_input
            || !state.ready
            || state.approval
            || state.codex_overlay
            || state.startup_loading
            || state.last_user_input.elapsed() < Duration::from_millis(super::shared::USER_ACTIVITY_COOLDOWN_MS)
        {
            return None;
        }
        use crate::agents::recovery::PROMPT;
        match self.key {
            WakeKey::Marker | WakeKey::RecoveryMarker if state.prompt_empty && state.last_prompt_submit.is_none_or(|t| t.elapsed() >= Duration::from_millis(SUBMIT_SETTLE_COOLDOWN_MS)) => {
                Some(if self.key == WakeKey::Marker { WAKE_TRIGGER.as_bytes() } else { PROMPT.as_bytes() })
            }
            WakeKey::Enter if state.input_text.as_deref() == Some(WAKE_TRIGGER) => Some(b"\r"),
            WakeKey::RecoveryEnter if state.input_text.as_deref() == Some(PROMPT) => Some(b"\r"),
            _ => None,
        }
    }

    pub(crate) fn send(self, port: u16) -> WakeWrite {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let timeout = Duration::from_millis(500);
        let Ok(mut stream) = TcpStream::connect_timeout(&addr, timeout) else { return WakeWrite::Deferred };
        if stream.set_write_timeout(Some(timeout)).is_err() || stream.set_read_timeout(Some(timeout)).is_err() {
            return WakeWrite::Deferred;
        }
        let key = self.key.wire();
        let mut response = String::new();
        if writeln!(stream, "\0CODEX-WAKE {key} {} {}", self.frame, self.user_input).is_err() || stream.shutdown(std::net::Shutdown::Write).is_err() || stream.take(32).read_to_string(&mut response).is_err() {
            return WakeWrite::Unknown;
        }
        match response.as_str() {
            "written" => WakeWrite::Written,
            "deferred" => WakeWrite::Deferred,
            _ => WakeWrite::Unknown,
        }
    }
}

impl WakeWrite {
    pub(super) fn response(self) -> &'static str {
        match self {
            Self::Written => "written",
            Self::Deferred => "deferred",
            Self::Unknown => "unknown",
        }
    }
}

impl QueryClient {
    /// Send response and close connection
    pub fn respond(mut self, response: &str) {
        let _ = self.stream.write_all(response.as_bytes());
        let _ = self.stream.flush();
        // stream dropped here, connection closed
    }
}

/// TCP server for text injection
pub struct InjectServer {
    listener: TcpListener,
    port: u16,
    clients: Vec<(TcpStream, Vec<u8>)>,
}

impl InjectServer {
    /// Create a new injection server on localhost
    pub fn new() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").context("Failed to bind inject server")?;
        let port = listener.local_addr()?.port();

        // Set non-blocking
        listener.set_nonblocking(true)?;

        Ok(Self { listener, port, clients: Vec::new() })
    }

    /// Get the port the server is listening on
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get the listener raw file descriptor for polling (Unix poll loop only).
    #[cfg(unix)]
    pub fn listener_raw_fd(&self) -> i32 {
        self.listener.as_raw_fd()
    }

    /// Screen queries remain readable even when the child stops consuming input.
    #[cfg(unix)]
    pub fn pollable_clients(&self, input_budget: usize) -> Vec<(usize, i32)> {
        self.clients
            .iter()
            .enumerate()
            .filter_map(|(index, (stream, buffer))| {
                let mut first = [0u8; 1];
                let peek = stream.peek(&mut first).ok();
                let query = buffer.first().copied().or_else(|| (peek == Some(1)).then_some(first[0])) == Some(QUERY_PREFIX);
                let can_read = if peek == Some(0) && !buffer.is_empty() {
                    let decoded_len = Self::process_inject_data(buffer).len();
                    decoded_len <= input_budget || decoded_len > MAX_REQUEST_BYTES
                } else {
                    buffer.len() < input_budget || buffer.len() >= MAX_REQUEST_BYTES
                };
                // An unclassified idle connection must stay polled so a SCREEN
                // command arriving later wakes the proxy under input backpressure.
                let unknown = buffer.is_empty() && peek.is_none();
                (query || can_read || unknown).then_some((index, stream.as_raw_fd()))
            })
            .collect()
    }

    /// Number of connected inject clients (portable; the Unix loop uses
    /// `pollable_clients`, but tests and the Windows loop need this
    /// without raw fds).
    #[cfg(any(test, windows))]
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Accept a new connection.
    ///
    /// Returns `Ok(true)` if a connection was accepted, `Ok(false)` if the accept
    /// queue was empty (WouldBlock). The caller uses this to apply backoff on
    /// macOS, where a non-blocking listener can keep reporting POLLIN via poll()
    /// even after the accept queue is drained.
    pub fn accept(&mut self) -> Result<bool> {
        match self.listener.accept() {
            Ok((stream, _addr)) => {
                stream.set_nonblocking(true)?;
                if self.clients.len() >= MAX_CLIENTS {
                    let _ = (&stream).write_all(b"error: too many pending injection clients\n");
                    return Ok(true);
                }
                self.clients.push((stream, Vec::new()));
                Ok(true)
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// Read from a client by index. Returns InjectResult:
    /// - Inject(text): text to write to PTY
    /// - ScreenQuery(index): caller should dump screen and call respond_query()
    /// - Pending: no data ready yet
    #[cfg(any(windows, test))]
    pub fn read_client(&mut self, index: usize) -> Result<InjectResult> {
        self.read_client_with_budget(index, MAX_REQUEST_BYTES)
    }

    pub fn read_client_with_budget(&mut self, index: usize, input_budget: usize) -> Result<InjectResult> {
        if index >= self.clients.len() {
            return Ok(InjectResult::Pending);
        }

        let (stream, buffer) = &mut self.clients[index];
        let mut buf = [0u8; 8192];

        let mut read_bytes = 0;
        loop {
            let mut first = [0u8; 1];
            let query = buffer.first().copied().or_else(|| (stream.peek(&mut first).ok() == Some(1)).then_some(first[0])) == Some(QUERY_PREFIX);
            let limit = if query { MAX_REQUEST_BYTES } else { input_budget.min(MAX_REQUEST_BYTES) };
            let capacity = limit.saturating_sub(buffer.len());
            let count = buf.len().min(capacity);
            // At the budget boundary, only consume EOF. Leave further bytes
            // in TCP so the sender experiences backpressure until PTY input drains.
            if count == 0 {
                let mut peek = [0u8; 1];
                match stream.peek(&mut peek) {
                    Ok(0) => {}
                    Ok(_) if buffer.len() >= MAX_REQUEST_BYTES => {
                        let _ = stream.write_all(b"error: injection request exceeds 1 MiB\n");
                        crate::log::log_warn("native", "inject.request_too_large", "Rejected injection request exceeding 1 MiB");
                        self.clients.remove(index);
                        return Ok(InjectResult::Pending);
                    }
                    _ => return Ok(InjectResult::Pending),
                }
            }
            // A one-byte read here observes the EOF established by peek above.
            match stream.read(&mut buf[..count.max(1)]) {
                Ok(0) => {
                    // EOF - client closed, process the data
                    let data = buffer.as_slice();

                    // Check for command (starts with \x00)
                    if data.first() == Some(&QUERY_PREFIX) {
                        let cmd = std::str::from_utf8(&data[1..]).unwrap_or("").trim();
                        let command = match cmd {
                            "SCREEN" => QueryCommand::Screen,
                            _ => GuardedWake::parse(cmd).map_or(QueryCommand::Unknown, QueryCommand::GuardedWake),
                        };
                        let (stream, _) = self.clients.remove(index);
                        return Ok(InjectResult::Query(QueryClient { stream, command }));
                    }

                    let text = Self::process_inject_data(data);
                    if text.len() > MAX_REQUEST_BYTES {
                        let _ = stream.write_all(b"error: decoded injection exceeds 1 MiB\n");
                        self.clients.remove(index);
                        return Ok(InjectResult::Pending);
                    }
                    if text.len() > input_budget {
                        return Ok(InjectResult::Pending);
                    }
                    self.clients.remove(index);
                    return Ok(InjectResult::Inject(text));
                }
                Ok(n) => {
                    buffer.extend_from_slice(&buf[..n]);
                    read_bytes += n;
                    if read_bytes >= 64 * 1024 {
                        return Ok(InjectResult::Pending);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => {
                    self.clients.remove(index);
                    return Err(e.into());
                }
            }
        }

        Ok(InjectResult::Pending)
    }

    /// Process injection data: decode and strip trailing LF
    /// Fix #7: Use UTF-8 with Latin-1 fallback instead of lossy (which mangles bytes)
    fn process_inject_data(data: &[u8]) -> String {
        let mut text = match String::from_utf8(data.to_vec()) {
            Ok(s) => s,
            Err(_) => {
                // Fallback to Latin-1 (preserves all byte values as chars)
                data.iter().map(|&b| b as char).collect()
            }
        };

        // Strip single trailing LF (from echo/nc), preserve CR for submit
        if text.ends_with('\n') {
            text.pop();
        }

        text
    }
}

#[cfg(test)]
mod tests {
    use super::{InjectResult, InjectServer};
    use std::io::Write;
    use std::net::{Shutdown, TcpStream};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn accept_returns_false_when_queue_is_empty() {
        let mut server = InjectServer::new().unwrap();

        assert!(!server.accept().unwrap());
        assert_eq!(server.client_count(), 0);
    }

    #[test]
    fn accept_returns_true_when_connection_is_pending() {
        let mut server = InjectServer::new().unwrap();
        let _client = TcpStream::connect(("127.0.0.1", server.port())).unwrap();

        let deadline = Instant::now() + Duration::from_secs(1);
        let accepted = loop {
            if server.accept().unwrap() {
                break true;
            }
            if Instant::now() >= deadline {
                break false;
            }
            thread::sleep(Duration::from_millis(10));
        };

        assert!(accepted);
        assert_eq!(server.client_count(), 1);
    }

    /// Poll `f` until it returns true or a second passes; under load the
    /// loopback connect and its bytes are not visible to the server instantly.
    #[cfg(unix)]
    fn eventually(mut f: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if f() {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(1));
        }
    }

    #[cfg(unix)]
    #[test]
    fn screen_queries_bypass_input_backpressure() {
        let mut server = InjectServer::new().unwrap();
        let mut text = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        text.write_all(b"queued").unwrap();
        text.shutdown(Shutdown::Write).unwrap();
        assert!(eventually(|| server.accept().unwrap()));
        assert!(eventually(|| {
            assert!(matches!(server.read_client_with_budget(0, 3).unwrap(), InjectResult::Pending));
            server.clients[0].1.len() == 3
        }));
        assert_eq!(server.clients[0].1, b"que");
        assert!(server.pollable_clients(0).is_empty());

        let mut query = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        assert!(eventually(|| server.accept().unwrap()));
        // Unclassified connections remain polled even before their first byte.
        assert_eq!(server.pollable_clients(0).len(), 1);
        query.write_all(b"\0SCREEN\n").unwrap();
        query.shutdown(Shutdown::Write).unwrap();
        assert!(eventually(|| matches!(server.read_client_with_budget(1, 0).unwrap(), InjectResult::Query(_))));
        assert!(matches!(server.read_client_with_budget(0, 6).unwrap(), InjectResult::Inject(text) if text == "queued"));
    }

    #[test]
    fn completed_clients_can_be_drained_in_connection_order() {
        let mut server = InjectServer::new().unwrap();
        let mut first = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        first.write_all(b"text").unwrap();
        first.shutdown(Shutdown::Write).unwrap();
        let mut second = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        second.write_all(b"\r").unwrap();
        second.shutdown(Shutdown::Write).unwrap();

        let deadline = Instant::now() + Duration::from_secs(1);
        while server.client_count() < 2 && Instant::now() < deadline {
            let _ = server.accept();
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(server.client_count(), 2);

        let read_next = |server: &mut InjectServer| {
            let deadline = Instant::now() + Duration::from_secs(1);
            loop {
                if let InjectResult::Inject(text) = server.read_client(0).unwrap() {
                    break text;
                }
                assert!(Instant::now() < deadline, "client did not complete");
                thread::sleep(Duration::from_millis(5));
            }
        };
        assert_eq!(read_next(&mut server), "text");
        assert_eq!(read_next(&mut server), "\r");
    }
}

#[cfg(test)]
#[test]
fn guarded_codex_write_checks_current_frame_user_and_exclusive_composer() {
    use crate::messaging::delivery::{ScreenState, WAKE_TRIGGER};
    use std::time::Instant;

    let mut state = ScreenState { ready: true, prompt_empty: true, codex_frame: 4, codex_user_input: 9, last_user_input: Instant::now() - Duration::from_secs(1), ..ScreenState::default() };
    let marker = GuardedWake::parse("CODEX-WAKE marker 4 9").unwrap();
    assert_eq!(marker.bytes(&state), Some(WAKE_TRIGGER.as_bytes()));
    assert!(GuardedWake::parse("CODEX-WAKE arbitrary 4 9").is_none());
    state.codex_frame += 1;
    assert_eq!(marker.bytes(&state), None, "old observations cannot write after a new frame");
    state.prompt_empty = false;
    state.input_text = Some(WAKE_TRIGGER.into());
    let enter = GuardedWake::new(WakeKey::Enter, 5, 9);
    assert_eq!(enter.bytes(&state), Some(b"\r".as_slice()));
    state.input_text = None;
    assert_eq!(enter.bytes(&state), None, "unknown or multiline input cannot own a wake");
    state.input_text = Some(WAKE_TRIGGER.into());
    state.codex_overlay = true;
    assert_eq!(enter.bytes(&state), None);
    state.codex_overlay = false;
    state.codex_user_input += 1;
    assert_eq!(enter.bytes(&state), None, "a new user input invalidates the attempt even after cooldown");
}
