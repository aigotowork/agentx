//! Individual launch responsibility, retained independently of identity and Role.
use serde_json::Value;

pub(crate) const MAX_CHARS: usize = 160;

/// Shared CLI/API/reservation admission; whitespace normalization is performed once.
pub(crate) fn normalize(value: Option<&str>) -> Result<Option<String>, String> {
    let Some(value) = value else { return Ok(None) };
    if value.chars().any(char::is_control) {
        return Err("title must be a single line without control characters".into());
    }
    let text = value.trim();
    if text.is_empty() || text.chars().count() > MAX_CHARS {
        return Err(format!("title must contain 1–{MAX_CHARS} characters"));
    }
    Ok(Some(text.to_owned()))
}

pub(crate) fn from_context(raw: Option<&str>) -> Option<String> {
    raw.and_then(|raw| serde_json::from_str::<Value>(raw).ok()).and_then(|value| value.get("title").and_then(Value::as_str).map(str::to_owned))
}

/// Terminal-only identity projection; never use this label for addressing an Agent.
pub(crate) fn pane_display(name: &str, role: Option<&str>, title: Option<&str>) -> String {
    let mut display = name.to_owned();
    if let Some(role) = role.filter(|role| !role.is_empty()) {
        display.push_str(&format!(" [{role}]"));
    }
    if let Some(title) = title.filter(|title| !title.is_empty()) {
        display.push_str(&format!(" · {title}"));
    }
    display
}

/// Use the adopted role and launch title, including after resume or participant rebind.
pub(crate) fn pane_display_from_record(record: &crate::db::InstanceRow) -> String {
    let role = super::workspaces::collaboration(record).and_then(|snapshot| snapshot.role);
    let title = from_context(record.launch_context.as_deref());
    pane_display(&record.name, role.as_ref().map(|role| role.name.as_str()), title.as_deref())
}

/// Stopped events retain the serialized row context; legacy snapshots may omit it.
pub(crate) fn from_snapshot(snapshot: &Value) -> Option<String> {
    let context = snapshot.get("launch_context")?;
    if let Some(raw) = context.as_str() { from_context(Some(raw)) } else { context.get("title").and_then(Value::as_str).map(str::to_owned) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_display_preserves_identity_with_optional_role_and_title() {
        for (role, title, expected) in [
            (None, None, "saki"),
            (Some(""), Some(""), "saki"),
            (Some("coder"), None, "saki [coder]"),
            (None, Some("优化终端标题"), "saki · 优化终端标题"),
            (Some("coder"), Some("优化终端标题"), "saki [coder] · 优化终端标题"),
            (Some("fronter.e2e"), None, "saki [fronter.e2e]"),
        ] {
            assert_eq!(pane_display("saki", role, title), expected);
        }
    }

    #[test]
    fn title_admission_and_legacy_snapshots() {
        assert_eq!(normalize(None).unwrap(), None);
        assert_eq!(normalize(Some("  原生 API 负责人  ")).unwrap().as_deref(), Some("原生 API 负责人"));
        assert!(normalize(Some(&"界".repeat(MAX_CHARS))).is_ok());
        for invalid in ["".to_owned(), "  ".into(), "two\nlines".into(), "nul\0byte".into(), "界".repeat(MAX_CHARS + 1)] {
            assert!(normalize(Some(&invalid)).is_err());
        }
        for old in [serde_json::json!({}), serde_json::json!({"launch_context":null}), serde_json::json!({"launch_context":"{}"})] {
            assert_eq!(from_snapshot(&old), None);
        }
        assert_eq!(from_snapshot(&serde_json::json!({"launch_context":"{\"title\":\"职责\"}"})).as_deref(), Some("职责"));
    }
}
