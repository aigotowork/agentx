//! Independent cases derived from the accepted context proposal, not provider fixtures.
#[path = "../../src/runtime/agent_context.rs"]
mod subject;

use std::cell::RefCell;
use subject::*;

struct Probe {
    calls: RefCell<Vec<String>>,
}
impl ContextGenerator for Probe {
    fn id(&self) -> &str { "agentx" }
    fn generate(&self, request: &ContextRequest<'_>) -> anyhow::Result<String> {
        self.calls.borrow_mut().push(format!("{:?}:{}:{}", request.reason, request.participant, request.tool));
        Ok(format!("fresh-{}", self.calls.borrow().len()))
    }
}

#[test]
fn selection_defaults_disable_and_structural_rejections() {
    for raw in ["", "  ", "\n"] {
        let sources = parse_sources(raw).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].builtin.as_deref(), Some("agentx"));
    }
    assert!(parse_sources("[]").unwrap().is_empty());
    for raw in ["null", "{}", "[1]", "[{\"text\":\"x\"}]", r#"[{"id":" " ,"text":"x"}]"#,
        r#"[{"id":"x"}]"#, r#"[{"id":"x","file":""}]"#,
        r#"[{"id":"x","text":"x","builtin":"agentx"}]"#,
        r#"[{"id":"x","text":"x","command":"echo bad"}]"#,
        r#"[{"id":"x","text":"a"},{"id":"x","file":"a"}]"#] {
        assert!(parse_sources(raw).is_err(), "accepted invalid declaration {raw}");
    }
}

#[test]
fn ordered_literal_sources_empty_blocks_and_fresh_reason_facts() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("guide.md");
    std::fs::write(&file, "file-before").unwrap();
    let sources = parse_sources(r#"[{"id":"a","text":"${HOME} {{participant}} $(touch forbidden)"},{"id":"empty","text":""},{"id":"file","file":"guide.md"},{"id":"b","builtin":"agentx"}]"#).unwrap();
    let probe = Probe { calls: RefCell::new(vec![]) };
    for (index, reason) in [ContextReason::Startup, ContextReason::Resume, ContextReason::Compact, ContextReason::Compact].into_iter().enumerate() {
        let value = format!("file-{index}");
        std::fs::write(&file, &value).unwrap();
        let request = ContextRequest { reason, participant: "participant-a", tool: "codex", cwd: root.path() };
        let output = compose(&request, &sources, &[&probe]).unwrap();
        assert_eq!(output.blocks.iter().map(|b| b.source.as_str()).collect::<Vec<_>>(), ["a", "file", "b"]);
        assert_eq!(output.text(), format!("${{HOME}} {{{{participant}}}} $(touch forbidden)\n\n{value}\n\nfresh-{}", index+1));
        assert_eq!(probe.calls.borrow()[index], format!("{reason:?}:participant-a:codex"));
    }
    assert!(!root.path().join("forbidden").exists());
    assert!(compose(&ContextRequest { reason: ContextReason::Compact, participant: "b", tool: "claude", cwd: root.path() }, &[], &[&probe]).unwrap().text().is_empty());
    assert_eq!(probe.calls.borrow().len(), 4, "disabled sources must not invoke a generator");
}

#[test]
fn file_resolution_failure_utf8_and_exact_aggregate_byte_limit() {
    let root = tempfile::tempdir().unwrap();
    let request = ContextRequest { reason: ContextReason::Compact, participant: "p", tool: "codex", cwd: root.path() };
    let file = root.path().join("guide");
    let source = |value: serde_json::Value| parse_sources(&value.to_string()).unwrap();
    let relative = source(serde_json::json!([{"id":"guide","file":"guide"}]));
    assert!(format!("{:#}", compose(&request, &relative, &[]).err().unwrap()).contains("guide"));
    std::fs::write(&file, [0xff, 0xfe]).unwrap();
    assert!(format!("{:#}", compose(&request, &relative, &[]).err().unwrap()).contains("UTF-8"));
    std::fs::write(&file, "é".repeat(32768)).unwrap();
    assert_eq!(compose(&request, &relative, &[]).unwrap().text().len(), 65536);
    let absolute = source(serde_json::json!([{"id":"abs","file":file}]));
    assert_eq!(compose(&request, &absolute, &[]).unwrap().text().len(), 65536);
    std::fs::write(&file, "é".repeat(32769)).unwrap();
    assert!(compose(&request, &relative, &[]).is_err());
    for (first_len, succeeds) in [(65533, true), (65534, false)] {
        let sources = source(serde_json::json!([{"id":"a","text":"x".repeat(first_len)},{"id":"b","text":"y"}]));
        assert_eq!(compose(&request, &sources, &[]).is_ok(), succeeds, "two-byte separator counts toward total");
    }
    let sources = source(serde_json::json!([{"id":"a","text":"safe"},{"id":"missing","file":"missing"}]));
    assert!(compose(&request, &sources, &[]).is_err(), "no partial output after late failure");
    let unknown = parse_sources(r#"[{"id":"unknown","builtin":"unregistered"}]"#).unwrap();
    assert!(compose(&request, &unknown, &[]).is_err(), "unknown IDs fail at generation");
}
