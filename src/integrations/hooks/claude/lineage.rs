//! Originating-host transcript evidence. Only bounded structured envelope IDs
//! leave the parser; participant selection and validation belong to agents.
use anyhow::Result;
use serde_json::Value;
use std::collections::BTreeSet;
use std::io::Read;

/// Keep the two provenances separate: an unvalidated incoming top-level ID
/// cannot authenticate itself, while message.session_id is copied ancestry.
#[derive(Debug, Default)]
pub(crate) struct TranscriptSessionIds {
    pub top_level: Vec<String>,
    pub message_envelope: Vec<String>,
    pub bytes_read: usize,
    pub records_read: usize,
}

pub(crate) fn read_transcript_session_ids(path: &str) -> Result<TranscriptSessionIds> {
    const MAX_BYTES: usize = 512 * 1024;
    const MAX_RECORDS: usize = 2048;
    if path.is_empty() {
        return Ok(TranscriptSessionIds::default());
    }
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(TranscriptSessionIds::default()),
        Err(error) => return Err(error.into()),
    };
    // Fork ancestry is copied into the initial records. Reading only that
    // bounded head also keeps hook latency independent of transcript length.
    let mut input = Vec::with_capacity(MAX_BYTES + 1);
    file.take((MAX_BYTES + 1) as u64).read_to_end(&mut input)?;
    let truncated = input.len() > MAX_BYTES;
    input.truncate(MAX_BYTES);
    let mut top_level = BTreeSet::new();
    let mut message_envelope = BTreeSet::new();
    let mut records_read = 0;
    for line in input.split_inclusive(|byte| *byte == b'\n').take(MAX_RECORDS) {
        if truncated && !line.ends_with(b"\n") {
            break;
        }
        records_read += 1;
        let Ok(line) = std::str::from_utf8(line) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        for id in [value.get("sessionId"), value.get("session_id")].into_iter().flatten().filter_map(Value::as_str).filter(|id| !id.is_empty()) {
            top_level.insert(id.to_owned());
        }
        if let Some(id) = value.pointer("/message/session_id").and_then(Value::as_str).filter(|id| !id.is_empty()) {
            message_envelope.insert(id.to_owned());
        }
    }
    Ok(TranscriptSessionIds { top_level: top_level.into_iter().collect(), message_envelope: message_envelope.into_iter().collect(), bytes_read: input.len(), records_read })
}
