//! Local binary selector projected from the daemon-owned persistent setting.
//! Ordinary CLI startup must not open SQLite or make a preliminary native call.
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const FILE: &str = ".daemon/dev-root.json";
const MAX_BYTES: u64 = 64 * 1024;

pub(crate) fn read(scope: &Path) -> io::Result<Option<PathBuf>> {
    let file = match std::fs::File::open(scope.join(FILE)) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "dev-root projection is too large"));
    }
    let root: String = serde_json::from_slice(&bytes)?;
    Ok((!root.is_empty()).then(|| root.into()))
}

pub(crate) fn encode(path: &str) -> io::Result<Vec<u8>> {
    let bytes = serde_json::to_vec(path)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "dev-root setting is too large"));
    }
    Ok(bytes)
}

/// The DB retains the setting for old executable readback. Update this private
/// file while the daemon owns the scope; readers see a complete old or new file.
pub(crate) fn publish(scope: &Path, path: Option<&[u8]>) -> io::Result<()> {
    let target = scope.join(FILE);
    if let Some(path) = path {
        let mut file = tempfile::NamedTempFile::new_in(target.parent().expect("dev-root projection has parent"))?;
        file.write_all(path)?;
        file.as_file().sync_all()?;
        file.persist(target).map_err(|error| error.error)?;
        return Ok(());
    }
    match std::fs::remove_file(target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
