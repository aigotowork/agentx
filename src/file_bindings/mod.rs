//! File-owned Task/Delivery assignments and disposable observations. See AGENTS.md.
use anyhow::Context as _;

mod document;
pub(crate) mod index;
mod write;

pub(crate) use document::{Assignment, Document, parse};
pub(crate) use index::{Index, Query};
pub(crate) use write::{bind, preflight, rollback};

/// Read one linked Task or Delivery using the existing advisory lock and size
/// limit. Return only the body when the reader explicitly opens it.
pub(crate) fn read_document(path: &std::path::Path, include_body: bool) -> anyhow::Result<(Document, Option<String>)> {
    let text = write::read_locked(path)?;
    let document = parse(&text)?;
    let body = if include_body {
        let end = document::frontmatter(&text)?.end;
        Some(text[end..].split_once('\n').map_or("", |(_, body)| body).to_owned())
    } else {
        None
    };
    Ok((document, body))
}

const MAX_LAUNCH_MESSAGE_BYTES: usize = 64 * 1024;
const TRUNCATED_SNAPSHOT: &str = "\n[Snapshot truncated. Read the source file for the rest.]";

/// Include bounded previews of bound files in the launch's first task message.
pub(crate) fn launch_message(paths: &[std::path::PathBuf], prompt: Option<&str>) -> anyhow::Result<Option<String>> {
    if paths.is_empty() {
        return Ok(prompt.map(str::to_owned));
    }

    let preamble = "Use the bound Task/Delivery snapshots below as your assignment context. These copies were read at launch; use the shown paths if you need the latest contents or must update a file.";
    let omitted = "\n\n[More files are bound than can be shown here. Ask the launcher for the remaining paths.]";
    let prompt = prompt.filter(|prompt| !prompt.trim().is_empty());
    let prompt_truncated = "\n[Launch prompt truncated.]\n\n";
    let reserved_prompt = if prompt.is_some() { prompt_truncated.len() } else { 0 };
    let mut remaining = MAX_LAUNCH_MESSAGE_BYTES - preamble.len() - omitted.len() - reserved_prompt;
    let mut files = Vec::with_capacity(paths.len());
    // Reserve each path and a truncation notice before spending bytes on any body.
    for path in paths {
        let quoted_path = serde_json::to_string(path.to_string_lossy().as_ref())?;
        let heading = format!("\n\nBound file {quoted_path}:");
        let required = heading.len() + TRUNCATED_SNAPSHOT.len();
        if required > remaining {
            break;
        }
        remaining -= required;
        files.push((path, heading));
    }
    let has_omitted_paths = files.len() < paths.len();
    if !has_omitted_paths {
        remaining += omitted.len();
    }
    remaining += reserved_prompt;
    let mut message = String::new();
    if let Some(prompt) = prompt {
        let suffix = if prompt.len().saturating_add(2) <= remaining { "\n\n" } else { prompt_truncated };
        message.push_str(utf8_prefix(prompt, remaining - suffix.len()));
        message.push_str(suffix);
        remaining -= message.len();
    }
    message.push_str(preamble);
    for (path, heading) in files {
        message.push_str(&heading);
        if remaining == 0 {
            message.push_str(TRUNCATED_SNAPSHOT);
            continue;
        }
        let text = write::read_locked(path).with_context(|| format!("read bound file {}", path.display()))?;
        parse(&text).with_context(|| format!("invalid bound file {}", path.display()))?;
        let before = message.len();
        append_snapshot(&mut message, &text, remaining);
        remaining = remaining + TRUNCATED_SNAPSHOT.len() - (message.len() - before);
    }
    if has_omitted_paths {
        message.push_str(omitted);
    }
    Ok(Some(message))
}

fn append_snapshot(message: &mut String, text: &str, budget: usize) {
    // An intact snapshot can reuse the space reserved for its truncation notice.
    let available = budget + TRUNCATED_SNAPSHOT.len();
    let text_end = text.find('\0').unwrap_or(text.len());
    let candidate = utf8_prefix(&text[..text_end], available);
    let fence = "`".repeat(longest_backtick_run(candidate).max(2) + 1);
    let overhead = 2 * fence.len() + "\nmarkdown\n\n".len();
    if candidate.len() == text.len() && text.len() + overhead <= available {
        message.push_str(&format!("\n{fence}markdown\n{text}\n{fence}"));
        return;
    }
    if let Some(limit) = budget.checked_sub(overhead) {
        let preview = utf8_prefix(candidate, limit);
        if !preview.is_empty() {
            message.push_str(&format!("\n{fence}markdown\n{preview}\n{fence}"));
        }
    }
    message.push_str(TRUNCATED_SNAPSHOT);
}

fn utf8_prefix(text: &str, limit: usize) -> &str {
    let mut end = limit.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn longest_backtick_run(text: &str) -> usize {
    let mut longest = 0;
    let mut current = 0;
    for character in text.chars() {
        if character == '`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    longest
}

#[cfg(test)]
mod tests;
