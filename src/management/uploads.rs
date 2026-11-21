//! Host-owned bounded durable binary storage, outside SQLite dispatch.
use super::contract::{Attachment, StoredAttachment};
use crate::service::ServiceError;
use std::{io::Write, path::Path};
pub(crate) const MAX_FILE: usize = 10 * 1024 * 1024;
pub(crate) const MAX_FILES: usize = 8;
fn invalid(message: &str) -> ServiceError {
    ServiceError::InvalidInput(message.into())
}
/// Persist complete bytes under a host UUID; registration follows persistence.
pub(crate) fn persist(scope: &Path, generation: &str, name: &str, media_type: &str, bytes: &[u8]) -> Result<StoredAttachment, ServiceError> {
    if bytes.len() > MAX_FILE {
        return Err(invalid("file exceeds 10 MiB limit"));
    }
    if name.is_empty() || name.len() > 255 || name.chars().any(char::is_control) {
        return Err(invalid("invalid attachment filename"));
    }
    if media_type.len() > 128 || !media_type.bytes().all(|c| c.is_ascii_alphanumeric() || b"/.-+_".contains(&c)) {
        return Err(invalid("invalid attachment media type"));
    }
    // Dataset generations are native UUIDs, never caller path segments.
    uuid::Uuid::parse_str(generation).map_err(|_| invalid("invalid generation"))?;
    let directory = scope.join("management-attachments").join(generation);
    std::fs::create_dir_all(&directory).map_err(|e| ServiceError::Internal(e.to_string()))?;
    let id = uuid::Uuid::new_v4().to_string();
    let path = directory.join(&id);
    let write = || -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    };
    if let Err(error) = write() {
        let _ = std::fs::remove_file(&path);
        return Err(ServiceError::Internal(error.to_string()));
    }
    Ok(StoredAttachment {
        path: path.to_string_lossy().into(),
        attachment: Attachment {
            url: format!("/api/v1/attachments/{id}?generation={generation}"),
            id,
            name: name.into(),
            media_type: if media_type.is_empty() { "application/octet-stream".into() } else { media_type.into() },
            size: bytes.len() as u64,
        },
    })
}
/// Check real bytes before admission/execution so failed uploads cannot send dangling refs.
pub(crate) fn check(stored: &StoredAttachment) -> Result<(), ServiceError> {
    let metadata = std::fs::metadata(&stored.path).map_err(|_| invalid("attachment file is unavailable on this host"))?;
    if !metadata.is_file() || metadata.len() != stored.attachment.size {
        return Err(invalid("attachment file changed or is unavailable"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upload_boundaries_names_and_missing_bytes() {
        let root = tempfile::tempdir().unwrap();
        let generation = uuid::Uuid::new_v4().to_string();
        let first = persist(root.path(), &generation, "../../same.bin", "application/octet-stream", b"").unwrap();
        let second = persist(root.path(), &generation, "../../same.bin", "application/octet-stream", b"two").unwrap();
        assert_ne!(first.path, second.path);
        assert_eq!(Path::new(&first.path).parent().unwrap(), root.path().join("management-attachments").join(&generation));
        check(&first).unwrap();
        assert!(persist(root.path(), &generation, "bad\nname", "text/plain", b"a").is_err());
        assert!(persist(root.path(), &generation, "file", "text/plain; injected", b"a").is_err());
        assert!(persist(root.path(), "../../escape", "file", "text/plain", b"a").is_err());
        let bytes = vec![0; MAX_FILE + 1];
        assert!(persist(root.path(), &generation, "file", "application/octet-stream", &bytes).is_err());
        let boundary = persist(root.path(), &generation, "file", "application/octet-stream", &bytes[..MAX_FILE]).unwrap();
        check(&boundary).unwrap();
        std::fs::remove_file(first.path.clone()).unwrap();
        assert!(check(&first).is_err());
    }
}
