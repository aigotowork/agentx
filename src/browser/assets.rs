//! Finite compiled-in bundle and optional development override. Browser URLs never name files.

use serde::Deserialize;
use std::{
    collections::HashMap,
    io::Read,
    path::{Component, Path},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    files: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    url: String,
    file: String,
    content_type: String,
}

pub(crate) struct Asset {
    pub bytes: Vec<u8>,
    pub content_type: String,
}
pub(crate) type Assets = HashMap<String, Asset>;

/// Default resources travel with the executable and need no runtime filesystem.
pub(crate) fn from_entries(entries: &[(&str, &str, &[u8])]) -> Assets {
    entries.iter().map(|(url, content_type, bytes)| ((*url).to_owned(), Asset { bytes: bytes.to_vec(), content_type: (*content_type).to_owned() })).collect()
}

pub(crate) fn load(directory: &Path) -> Result<Assets, &'static str> {
    let root = directory.canonicalize().map_err(|_| "asset directory unavailable")?;
    let manifest_path = root.join("manifest.json").canonicalize().map_err(|_| "asset manifest unavailable")?;
    if !manifest_path.starts_with(&root) {
        return Err("asset manifest escapes bundle");
    }
    let meta = manifest_path.metadata().map_err(|_| "asset manifest unavailable")?;
    if !meta.is_file() || meta.len() > 32 * 1024 {
        return Err("invalid asset manifest");
    }
    let manifest: Manifest = serde_json::from_slice(&read_bounded(&manifest_path, 32 * 1024)?).map_err(|_| "invalid asset manifest")?;
    if manifest.files.is_empty() || manifest.files.len() > 128 {
        return Err("asset count out of bounds");
    }
    let mut total = 0_u64;
    let mut result = HashMap::new();
    for entry in manifest.files {
        if !valid_url(&entry.url) || result.contains_key(&entry.url) {
            return Err("invalid or duplicate asset URL");
        }
        if !matches!(entry.content_type.as_str(), "text/html; charset=utf-8" | "text/css; charset=utf-8" | "text/javascript; charset=utf-8" | "image/svg+xml" | "image/png" | "image/webp" | "font/woff2") {
            return Err("unsupported asset type");
        }
        let relative = Path::new(&entry.file);
        if relative.components().any(|part| !matches!(part, Component::Normal(_))) {
            return Err("invalid asset path");
        }
        let file = root.join(relative).canonicalize().map_err(|_| "asset unavailable")?;
        if !file.starts_with(&root) {
            return Err("asset escapes bundle");
        }
        let metadata = file.metadata().map_err(|_| "asset unavailable")?;
        if !metadata.is_file() || metadata.len() > 4 * 1024 * 1024 {
            return Err("asset size out of bounds");
        }
        total = total.checked_add(metadata.len()).ok_or("asset size overflow")?;
        if total > 16 * 1024 * 1024 {
            return Err("bundle size out of bounds");
        }
        let bytes = read_bounded(&file, 4 * 1024 * 1024)?;
        if bytes.len() as u64 != metadata.len() {
            return Err("asset changed during loading");
        }
        result.insert(entry.url, Asset { bytes, content_type: entry.content_type });
    }
    if !result.get("/").is_some_and(|asset| asset.content_type == "text/html; charset=utf-8") {
        return Err("bundle needs HTML at /");
    }
    Ok(result)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, &'static str> {
    let mut bytes = Vec::new();
    std::fs::File::open(path).map_err(|_| "asset unreadable")?.take(limit + 1).read_to_end(&mut bytes).map_err(|_| "asset unreadable")?;
    if bytes.len() as u64 > limit {
        return Err("asset size out of bounds");
    }
    Ok(bytes)
}

fn valid_url(url: &str) -> bool {
    url.starts_with('/') && !url.starts_with("/api/") && !url.contains("..") && url.bytes().all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b))
}
