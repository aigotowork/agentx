//! Size-bounded serde frames; Unix I/O uses one absolute deadline per frame.

#[cfg(unix)]
use std::io;
use std::time::Instant;

use serde::{Serialize, de::DeserializeOwned};

use super::DaemonError;

pub(crate) const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

#[cfg(any(unix, windows))]
pub(crate) fn read<T: DeserializeOwned>(stream: &mut super::platform::Stream, deadline: Instant) -> Result<T, DaemonError> {
    read_limited(stream, deadline, MAX_FRAME_BYTES)
}

/// Distinguish a clean empty connection from a truncated frame.
#[cfg(any(unix, windows))]
pub(crate) fn read_optional<T: DeserializeOwned>(stream: &mut super::platform::Stream, deadline: Instant) -> Result<Option<T>, DaemonError> {
    read_limited_optional(stream, deadline, MAX_FRAME_BYTES)
}

/// Reject the advertised size before allocating a body for a smaller lane.
#[cfg(any(unix, windows))]
pub(crate) fn read_limited<T: DeserializeOwned>(stream: &mut super::platform::Stream, deadline: Instant, maximum: usize) -> Result<T, DaemonError> {
    read_limited_optional(stream, deadline, maximum)?.ok_or_else(|| DaemonError::Unavailable("peer closed before frame".into()))
}

#[cfg(any(unix, windows))]
fn read_limited_optional<T: DeserializeOwned>(stream: &mut super::platform::Stream, deadline: Instant, maximum: usize) -> Result<Option<T>, DaemonError> {
    let mut prefix = [0; 4];
    if !read_bytes_optional(stream, &mut prefix, deadline)? {
        return Ok(None);
    }
    let length = u32::from_be_bytes(prefix) as usize;
    if length == 0 || length > maximum {
        return Err(DaemonError::Protocol(format!("frame length must be in 1..={maximum}")));
    }
    let mut body = vec![0; length];
    read_bytes(stream, &mut body, deadline)?;
    serde_json::from_slice(&body).map(Some).map_err(|error| DaemonError::Protocol(error.to_string()))
}

#[cfg(any(unix, windows))]
pub(crate) fn write<T: Serialize>(stream: &mut super::platform::Stream, value: &T, deadline: Instant) -> Result<(), DaemonError> {
    write_limited(stream, value, deadline, MAX_FRAME_BYTES)
}

/// Bound the complete serde envelope while encoding, before any socket write.
#[cfg(any(unix, windows))]
pub(crate) fn write_limited<T: Serialize>(stream: &mut super::platform::Stream, value: &T, deadline: Instant, maximum: usize) -> Result<(), DaemonError> {
    let body = encode_limited(value, maximum)?;
    write_encoded(stream, &body, deadline)
}

#[cfg(any(unix, windows))]
pub(super) fn write_encoded(stream: &mut super::platform::Stream, body: &[u8], deadline: Instant) -> Result<(), DaemonError> {
    write_bytes(stream, &(body.len() as u32).to_be_bytes(), deadline)?;
    write_bytes(stream, body, deadline)
}

pub(crate) fn encode_limited<T: Serialize>(value: &T, maximum: usize) -> Result<Vec<u8>, DaemonError> {
    struct Body {
        bytes: Vec<u8>,
        maximum: usize,
    }
    impl std::io::Write for Body {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "encoded frame exceeds lane bound"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut body = Body { bytes: Vec::new(), maximum: maximum.min(u32::MAX as usize) };
    serde_json::to_writer(&mut body, value).map_err(|error| DaemonError::Protocol(error.to_string()))?;
    if body.bytes.is_empty() {
        return Err(DaemonError::Protocol("empty serialized frame".into()));
    }
    Ok(body.bytes)
}

#[cfg(unix)]
pub(super) fn read_bytes(stream: &mut super::platform::Stream, bytes: &mut [u8], deadline: Instant) -> Result<(), DaemonError> {
    read_bytes_optional(stream, bytes, deadline)?.then_some(()).ok_or_else(|| DaemonError::Unavailable("peer closed a partial frame".into()))
}

#[cfg(unix)]
fn read_bytes_optional(stream: &mut super::platform::Stream, mut bytes: &mut [u8], deadline: Instant) -> Result<bool, DaemonError> {
    use std::io::Read;
    let mut read_any = false;
    while !bytes.is_empty() {
        if Instant::now() >= deadline {
            return Err(DaemonError::Deadline);
        }
        match stream.read(bytes) {
            Ok(0) if !read_any => return Ok(false),
            Ok(0) => return Err(DaemonError::Unavailable("peer closed a partial frame".into())),
            Ok(count) => {
                read_any = true;
                bytes = &mut bytes[count..];
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => wait(stream, libc::POLLIN, deadline)?,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(true)
}

#[cfg(unix)]
pub(super) fn write_bytes(stream: &mut super::platform::Stream, mut bytes: &[u8], deadline: Instant) -> Result<(), DaemonError> {
    use std::io::Write;
    while !bytes.is_empty() {
        if Instant::now() >= deadline {
            return Err(DaemonError::Deadline);
        }
        match stream.write(bytes) {
            Ok(0) => return Err(DaemonError::Unavailable("peer closed during frame write".into())),
            Ok(count) => bytes = &bytes[count..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => wait(stream, libc::POLLOUT, deadline)?,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn wait(stream: &super::platform::Stream, events: libc::c_short, deadline: Instant) -> Result<(), DaemonError> {
    use std::os::fd::AsRawFd;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(DaemonError::Deadline);
        }
        let millis = remaining.as_nanos().div_ceil(1_000_000).min(i32::MAX as u128) as i32;
        let mut fd = libc::pollfd { fd: stream.as_raw_fd(), events, revents: 0 };
        // SAFETY: fd is one live descriptor, and the timeout is bounded.
        let result = unsafe { libc::poll(&mut fd, 1, millis) };
        if result == 0 {
            return Err(DaemonError::Deadline);
        }
        if result > 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error.into());
        }
    }
}

#[cfg(windows)]
fn read_bytes(stream: &mut super::platform::Stream, bytes: &mut [u8], deadline: Instant) -> Result<(), DaemonError> {
    stream.read_bytes(bytes, deadline)
}

#[cfg(windows)]
fn read_bytes_optional(stream: &mut super::platform::Stream, bytes: &mut [u8], deadline: Instant) -> Result<bool, DaemonError> {
    stream.read_bytes_optional(bytes, deadline)
}

#[cfg(windows)]
fn write_bytes(stream: &mut super::platform::Stream, bytes: &[u8], deadline: Instant) -> Result<(), DaemonError> {
    stream.write_bytes(bytes, deadline)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    #[test]
    fn hello_only_eof_is_distinct_from_a_partial_operation() {
        let (mut reader, writer) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        drop(writer);
        assert!(read_optional::<serde_json::Value>(&mut reader, Instant::now() + Duration::from_secs(1)).unwrap().is_none());
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        writer.write_all(&[0, 0]).unwrap();
        drop(writer);
        assert!(read_optional::<serde_json::Value>(&mut reader, Instant::now() + Duration::from_secs(1)).is_err());
    }

    #[test]
    fn inspection_limit_rejects_length_before_reading_body() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        writer.write_all(&(64 * 1024u32 + 1).to_be_bytes()).unwrap();
        let result = read_limited::<serde_json::Value>(&mut reader, Instant::now() + Duration::from_secs(1), 64 * 1024);
        assert!(matches!(result, Err(DaemonError::Protocol(_))));
    }

    #[test]
    fn complete_envelope_must_fit_before_any_frame_is_written() {
        let value = serde_json::json!({ "result": { "value": "bounded" } });
        let length = serde_json::to_vec(&value).unwrap().len();
        assert_eq!(encode_limited(&value, length).unwrap().len(), length);
        assert!(matches!(encode_limited(&value, length - 1), Err(DaemonError::Protocol(_))));
        let (mut sender, mut receiver) = UnixStream::pair().unwrap();
        sender.set_nonblocking(true).unwrap();
        receiver.set_nonblocking(true).unwrap();
        assert!(write_limited(&mut sender, &value, Instant::now() + Duration::from_secs(1), length - 1).is_err());
        let mut byte = [0u8; 1];
        assert_eq!(std::io::Read::read(&mut receiver, &mut byte).unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
    }

    #[test]
    fn invalid_lengths_reject_before_body_allocation() {
        for length in [0u32, MAX_FRAME_BYTES as u32 + 1, u32::MAX] {
            let (mut reader, mut writer) = UnixStream::pair().unwrap();
            reader.set_nonblocking(true).unwrap();
            writer.write_all(&length.to_be_bytes()).unwrap();
            let result = read::<serde_json::Value>(&mut reader, Instant::now() + Duration::from_secs(1));
            assert!(matches!(result, Err(DaemonError::Protocol(_))));
        }
    }

    #[test]
    fn partial_fragments_share_one_absolute_deadline() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        writer.write_all(&64u32.to_be_bytes()).unwrap();
        let fragments = std::thread::spawn(move || {
            for _ in 0..5 {
                if writer.write_all(b" ").is_err() {
                    break;
                }
                std::thread::park_timeout(Duration::from_millis(15));
            }
        });
        let started = Instant::now();
        let result = read::<serde_json::Value>(&mut reader, started + Duration::from_millis(35));
        assert!(matches!(result, Err(DaemonError::Deadline)));
        assert!(started.elapsed() < Duration::from_secs(2));
        fragments.join().unwrap();
    }
}
