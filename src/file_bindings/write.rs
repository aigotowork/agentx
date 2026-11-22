//! Locked source writes and assignment-only compensation; no durable index/sidecar.
use super::{
    Assignment,
    document::{MAX_BYTES, frontmatter},
    parse,
};
use anyhow::{Context, Result, bail};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

pub(super) fn read_locked(path: &Path) -> Result<String> {
    if !std::fs::metadata(path)?.is_file() {
        bail!("source is not a regular file");
    }
    let mut file = File::open(path)?;
    file.try_lock_shared()?;
    read(&mut file)
}
fn read(file: &mut File) -> Result<String> {
    if !file.metadata()?.is_file() {
        bail!("source is not a regular file");
    }
    if file.metadata()?.len() > MAX_BYTES {
        bail!("document exceeds 1 MiB");
    }
    let mut text = String::new();
    file.take(MAX_BYTES + 1).read_to_string(&mut text)?;
    if text.len() as u64 > MAX_BYTES {
        bail!("document exceeds 1 MiB");
    }
    Ok(text)
}

/// Resolve against originating cwd, validate every source before launch effects.
pub(crate) fn preflight(paths: &[PathBuf], cwd: &Path) -> Result<Vec<PathBuf>> {
    if paths.len() > 64 {
        bail!("at most 64 --bind files are supported");
    }
    let mut resolved = Vec::new();
    for path in paths {
        let path = if path.is_absolute() { path.clone() } else { cwd.join(path) };
        let path = path.canonicalize().with_context(|| format!("binding source {}", path.display()))?;
        if resolved.contains(&path) {
            continue;
        }
        if !path.metadata()?.is_file() {
            bail!("binding source is not a regular file: {}", path.display());
        }
        let mut file = OpenOptions::new().read(true).write(true).open(&path).with_context(|| format!("binding source must be writable: {}", path.display()))?;
        file.lock()?;
        let original = read(&mut file)?;
        parse(&original).with_context(|| format!("invalid binding source {}", path.display()))?;
        let probe = Assignment { scope: String::new(), generation: String::new(), participant: crate::agents::attachments::ParticipantRef { canonical_name: String::new(), incarnation_bits: 0 } };
        render(&original, &probe, true).with_context(|| format!("unsupported binding layout {}", path.display()))?;
        resolved.push(path);
    }
    Ok(resolved)
}

/// Return only newly changed sources so compensation cannot remove older binds.
pub(crate) fn bind(paths: &[PathBuf], assignment: &Assignment) -> Result<Vec<PathBuf>> {
    let mut changed = Vec::new();
    for path in paths {
        match edit(path, assignment, true) {
            Ok(true) => changed.push(path.clone()),
            Ok(false) => {}
            Err(error) => {
                let recovery = rollback(&changed, assignment);
                bail!("assignment failed for {}: {error:#}{recovery}", path.display());
            }
        }
    }
    Ok(changed)
}

/// Best effort compensation preserves current body/status/other Agents. Errors are actionable.
pub(crate) fn rollback(paths: &[PathBuf], assignment: &Assignment) -> String {
    let failures: Vec<_> = paths.iter().rev().filter_map(|path| edit(path, assignment, false).err().map(|error| format!("{}: {error:#}", path.display()))).collect();
    if failures.is_empty() { String::new() } else { format!("; rollback incomplete; remove exact assignment {assignment:?} from: {}", failures.join("; ")) }
}

fn edit(path: &Path, assignment: &Assignment, add: bool) -> Result<bool> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    file.lock()?;
    let original = read(&mut file)?;
    let Some(updated) = render(&original, assignment, add)? else {
        return Ok(false);
    };
    // Cooperating readers share the same inode lock. Keep permissions/inode,
    // and restore original bytes on a failed write rather than claiming success.
    let write = |file: &mut File, text: &str| -> std::io::Result<()> {
        file.seek(SeekFrom::Start(0))?;
        file.write_all(text.as_bytes())?;
        file.set_len(text.len() as u64)?;
        file.sync_data()
    };
    if let Err(error) = write(&mut file, &updated) {
        match write(&mut file, &original) {
            Ok(()) => bail!("write failed; original restored: {error}"),
            Err(restore) => bail!("write failed: {error}; ORIGINAL RESTORE FAILED: {restore}; inspect source manually"),
        }
    }
    Ok(true)
}

fn render(original: &str, assignment: &Assignment, add: bool) -> Result<Option<String>> {
    let mut doc = parse(original)?;
    let contains = doc.assignments.contains(assignment);
    if contains == add {
        return Ok(None);
    }
    if add {
        doc.agents.push(serde_yaml_ng::to_value(assignment)?);
    } else {
        doc.agents.retain(|value| serde_yaml_ng::from_value::<Assignment>(value.clone()).ok().as_ref() != Some(assignment));
    }
    let range = frontmatter(original)?;
    let yaml = &original[range.clone()];
    // YAML is parsed by the library. This narrow source span locator accepts
    // canonical and quoted block keys; unusual flow-root/alias keys fail safely.
    let key = regex::Regex::new(r#"^(?:agents|'agents'|"agents")\s*:"#)?;
    let mut offset = range.start;
    let mut owned = None;
    let mut end = range.end;
    for line in yaml.split_inclusive('\n') {
        if owned.is_some() && !line.trim().is_empty() && !line.starts_with([' ', '\t', '#', '-']) {
            end = offset;
            break;
        }
        if key.is_match(line) {
            owned = Some(offset);
        }
        offset += line.len();
    }

    // Require the parsed mapping to remain identical apart from our owned field;
    // flow root mappings and complex keys cannot be rewritten accidentally.
    let newline = if original.contains("\r\n") { "\r\n" } else { "\n" };
    let values = serde_json::to_string(&doc.agents)?;
    let replacement = format!("agents: {values}{newline}");
    let span = owned.unwrap_or(range.end)..if owned.is_some() { end } else { range.end };
    let mut updated = original.to_owned();
    updated.replace_range(span, &replacement);
    let before: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml)?;
    let mut expected = before;
    expected.as_mapping_mut().unwrap().insert(serde_yaml_ng::Value::String("agents".into()), serde_yaml_ng::to_value(&doc.agents)?);
    let after = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&updated[frontmatter(&updated)?]);
    if after.as_ref().ok() != Some(&expected) {
        // Flow-root or complex YAML keys cannot be spliced by line. Re-emit
        // only frontmatter, retaining every semantic field and exact body.
        let emitted = serde_yaml_ng::to_string(&expected)?.replace('\n', newline);
        updated = format!("{}{}{}", &original[..range.start], emitted, &original[range.end..]);
        let checked: serde_yaml_ng::Value = serde_yaml_ng::from_str(&updated[frontmatter(&updated)?])?;
        if checked != expected {
            bail!("cannot preserve YAML metadata during assignment");
        }
    }
    if updated.len() as u64 > MAX_BYTES {
        bail!("assignment would exceed 1 MiB");
    }
    Ok(Some(updated))
}
