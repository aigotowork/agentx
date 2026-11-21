//! Lifecycle envelopes embed the native service declarations directly.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::DaemonError;
use crate::service::{CallerContext, Request, Response, ServiceResult};

/// Compatibility epoch shared by business and inspection IPC. Increment for
/// incompatible native request/response shapes or semantics, not for rebuilds.
/// Epoch 3 removes the obsolete participant grouping contract.
pub(crate) const PROTOCOL_VERSION: u32 = 3;
pub(crate) const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const BUILD_ID_PREFIX: &str = "native-build-v1:";
const BUILD_ID: &str = env!("AGENTX_NATIVE_BUILD_ID");

// A missing generator value fails env! expansion. Invalid output also fails
// compilation rather than creating a shared empty/default compatibility ID.
const _: () = {
    let bytes = BUILD_ID.as_bytes();
    let prefix = BUILD_ID_PREFIX.as_bytes();
    assert!(bytes.len() == prefix.len() + 36, "invalid generated native build identity");
    let (family, value) = bytes.split_at(prefix.len());
    let mut index = 0;
    while index < prefix.len() {
        assert!(family[index] == prefix[index], "invalid generated native build family");
        index += 1;
    }
    match uuid::Uuid::try_parse_ascii(value) {
        Ok(value) => {
            assert!(value.get_version_num() == 4, "native build identity must be UUID v4");
            assert!(matches!(value.get_variant(), uuid::Variant::RFC4122), "invalid native build UUID variant");
        }
        Err(_) => panic!("invalid generated native build UUID"),
    }
};

/// Ready application identity. Database generation is independent of daemon
/// incarnation and binary compatibility; the application owns its durable key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DaemonInfo {
    pub protocol_version: u32,
    pub build_version: String,
    pub build_id: String,
    pub canonical_scope: PathBuf,
    pub daemon_instance: String,
    pub database_generation: String,
    pub pid: u32,
    /// Absent on older epoch-3 daemons. Old clients ignore this optional hint.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub codex_hook: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub agent_names: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub workspace_members: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub message_history: bool,
}

#[derive(Serialize, Deserialize)]
#[expect(clippy::large_enum_variant, reason = "At most 32 connection workers decode this finite native request envelope from frames capped at 8 MiB; retain direct service types.")]
pub(crate) enum ClientFrame {
    Hello { protocol_version: u32, build_version: String, build_id: String, canonical_scope: PathBuf },
    Call { daemon_instance: String, caller: CallerContext, request: Request },
    Shutdown { daemon_instance: String },
}

#[derive(Serialize, Deserialize)]
#[expect(clippy::large_enum_variant, reason = "Finite native replies pass through bounded one-reply channels and frames capped at 8 MiB; retain direct service response/error types.")]
pub(crate) enum ServerFrame {
    Ready(DaemonInfo),
    Result { result: ServiceResult<Response>, database_generation: String },
    Stopping,
    Failure(DaemonError),
}

/// Diagnostic build identity embedded in loaded code by build.rs. It performs no
/// runtime image/path/environment read and survives executable replacement.
/// Full artifact SHA-256 is separate integration/package audit evidence.
pub(crate) fn build_identity() -> &'static str {
    BUILD_ID
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::{self, Read};
    use std::time::Instant;

    #[test]
    fn participant_timestamp_roundtrip_preserves_incarnation() {
        // Gala's actual timestamp used to decode one ULP higher, causing every
        // operation deriving ParticipantRef from a transported row to be stale.
        for bits in [4745314209997103380, 4745314209997103381, 4745314209997103382] {
            let row = serde_json::json!({"created_at": f64::from_bits(bits)});
            let encoded = serde_json::to_vec(&row).unwrap();
            let decoded: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
            assert_eq!(decoded["created_at"].as_f64().unwrap().to_bits(), bits);
        }
    }

    /// Full-byte digest helper is audit-only; no production entry hashes an
    /// executable to discover its native compatibility cohort.
    fn digest_reader(mut reader: impl Read) -> Result<String, DaemonError> {
        let mut digest = aws_lc_rs::digest::Context::new(&aws_lc_rs::digest::SHA256);
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        Ok(super::super::hex_digest(digest.finish().as_ref()))
    }

    struct Fragmented<'a>(&'a [u8]);
    impl Read for Fragmented<'_> {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            let count = output.len().min(17).min(self.0.len());
            output[..count].copy_from_slice(&self.0[..count]);
            self.0 = &self.0[count..];
            Ok(count)
        }
    }

    #[test]
    fn artifact_sha256_matches_vectors_and_existing_backend_at_stream_boundaries() {
        assert_eq!(digest_reader(&b""[..]).unwrap(), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(digest_reader(&b"abc"[..]).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        for length in [1, 63, 64, 65, 65_535, 65_536, 65_537, 131_073] {
            let source: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
            let expected = super::super::hex_digest(&Sha256::digest(&source));
            assert_eq!(digest_reader(source.as_slice()).unwrap(), expected);
            assert_eq!(digest_reader(Fragmented(&source)).unwrap(), expected);
        }
    }

    #[test]
    fn artifact_source_failure_never_returns_a_partial_digest() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("fixture artifact read failed"))
            }
        }
        let source = io::Cursor::new(b"partial artifact").chain(Broken);
        assert!(matches!(digest_reader(source), Err(DaemonError::Io { message, .. }) if message.contains("fixture artifact read failed")));
    }

    #[test]
    #[ignore = "integration-owned attribution probe; requires explicit AGENTX_ACCEPTANCE_BINARY"]
    fn artifact_digest_cost_breakdown() {
        let path = std::env::var_os("AGENTX_ACCEPTANCE_BINARY").expect("select the fixed artifact explicitly");
        let started = Instant::now();
        let bytes = io::copy(&mut std::fs::File::open(&path).unwrap(), &mut io::sink()).unwrap();
        let read_only_ms = started.elapsed().as_secs_f64() * 1000.0;
        // The first read warms pages for a CPU/backend comparison. DD12 keeps
        // its separate fresh-process, unprewarmed end-to-end workload.
        let started = Instant::now();
        let old = {
            let mut file = std::fs::File::open(&path).unwrap();
            let mut hash = Sha256::new();
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let count = file.read(&mut buffer).unwrap();
                if count == 0 {
                    break;
                }
                hash.update(&buffer[..count]);
            }
            super::super::hex_digest(&hash.finalize())
        };
        let sha2_ms = started.elapsed().as_secs_f64() * 1000.0;
        let started = Instant::now();
        let new = digest_reader(std::fs::File::open(&path).unwrap()).unwrap();
        let aws_lc_ms = started.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(old, new);
        assert_eq!(bytes, std::fs::metadata(&path).unwrap().len());
        println!("artifact bytes={bytes} sha256={new} read_only_ms={read_only_ms:.3} sha2_ms={sha2_ms:.3} aws_lc_ms={aws_lc_ms:.3}; warmed-page attribution only, not DD12 cold evidence");
    }
}
