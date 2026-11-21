//! Exercise production composition with a minimal identity DTO seam.
//! Persistence/authorization are deliberately not simulated by this suite.
#![allow(dead_code)]
mod agents {
    pub mod attachments {
        pub struct ParticipantRef {
            pub canonical_name: String,
            pub incarnation_bits: u64,
        }
    }
}
#[path = "../../src/runtime/agent_context.rs"]
mod subject;
use agents::attachments::ParticipantRef;
use std::cell::RefCell;
use subject::*;

fn ids(sources: &[ContextSource]) -> Vec<&str> {
    sources.iter().map(|source| source.id.as_str()).collect()
}

#[test]
fn compatibility_matrix_preserves_absent_empty_disable_and_file_replacement() {
    let mixed = r#"[{"id":"a","text":"A"},{"id":"builtin","builtin":"agentx"},{"id":"f","file":"facts"},{"id":"b","text":"B"}]"#;
    for raw in ["", " ", "\n"] {
        let (fixed, dynamic) = select_sources(None, raw).unwrap();
        assert_eq!(ids(&fixed), ["agentx"]);
        assert!(dynamic.is_empty());
    }
    let (fixed, dynamic) = select_sources(None, "[]").unwrap();
    assert!(fixed.is_empty() && dynamic.is_empty());
    let (fixed, dynamic) = select_sources(None, mixed).unwrap();
    assert_eq!(ids(&fixed), ["builtin"]);
    assert_eq!(ids(&dynamic), ["a", "f", "b"]);
    let file_only = r#"[{"id":"file","file":"guide"}]"#;
    let (fixed, dynamic) = select_sources(None, file_only).unwrap();
    assert!(fixed.is_empty());
    assert_eq!(ids(&dynamic), ["file"]);
    for raw in ["", " \n"] {
        let (fixed, dynamic) = select_sources(Some(raw), "").unwrap();
        assert_eq!(ids(&fixed), ["agentx"]);
        assert!(dynamic.is_empty());
    }
    let (fixed, dynamic) = select_sources(Some(file_only), mixed).unwrap();
    assert_eq!(ids(&fixed), ["file"]);
    assert_eq!(ids(&dynamic), ["a", "builtin", "f", "b"]);
    let (fixed, dynamic) = select_sources(Some("[]"), "[]").unwrap();
    assert!(fixed.is_empty() && dynamic.is_empty());
    for (fixed, dynamic) in [(Some("null"), "[]"), (Some("[]"), "null"), (None, "{}") ] {
        assert!(select_sources(fixed, dynamic).is_err());
    }
}

struct Contributor { calls: RefCell<Vec<String>> }
impl InstructionContributor for Contributor {
    fn id(&self) -> &str { "agentx" }
    fn instructions(&self, request: &InstructionRequest<'_>) -> anyhow::Result<String> {
        self.calls.borrow_mut().push("fixed".into());
        Ok(format!("fixed:{}:{}", request.participant.canonical_name, request.participant.incarnation_bits))
    }
}
impl ContextContributor for Contributor {
    fn id(&self) -> &str { "agentx" }
    fn snapshot(&self, request: &SnapshotRequest<'_>) -> anyhow::Result<String> {
        self.calls.borrow_mut().push(format!("{:?}:{}:{}:{}", request.reason, request.session_id, request.identity.participant.canonical_name, request.identity.tool));
        Ok(format!("facts:{}", self.calls.borrow().len()))
    }
}

#[test]
fn typed_ports_keep_identity_session_and_boundary_separate() {
    let root = tempfile::tempdir().unwrap();
    let actor = ParticipantRef { canonical_name: "agent-a".into(), incarnation_bits: 42 };
    let contributor = Contributor { calls: RefCell::new(vec![]) };
    let sources = parse_sources("").unwrap();
    let identity = || InstructionRequest { participant: &actor, tool: "codex", cwd: root.path() };
    let fixed = prepare_instructions(&identity(), &sources, &[&contributor]).unwrap();
    assert_eq!(fixed.text, "fixed:agent-a:42");
    for reason in [SnapshotReason::Initial, SnapshotReason::AfterCompact, SnapshotReason::AfterCompact] {
        let request = SnapshotRequest { identity: identity(), session_id: "session-a", reason };
        let output = match reason {
            SnapshotReason::Initial => prepare_initial_context(&request, &sources, &[&contributor]),
            SnapshotReason::AfterCompact => prepare_compact_context(&request, &sources, &[&contributor]),
        }.unwrap();
        assert_eq!(ids(&sources), output.blocks.iter().map(|b| b.source.as_str()).collect::<Vec<_>>());
        assert!(output.text().starts_with("facts:"));
        assert!(!output.text().contains("fixed:"));
        let wrong = match reason {
            SnapshotReason::Initial => prepare_compact_context(&request, &sources, &[&contributor]),
            SnapshotReason::AfterCompact => prepare_initial_context(&request, &sources, &[&contributor]),
        };
        assert!(wrong.is_err());
    }
    assert_eq!(*contributor.calls.borrow(), ["fixed", "Initial:session-a:agent-a:codex", "AfterCompact:session-a:agent-a:codex", "AfterCompact:session-a:agent-a:codex"]);
}

#[test]
fn frozen_value_roundtrips_while_dynamic_file_changes_and_clears() {
    let root = tempfile::tempdir().unwrap();
    let actor = ParticipantRef { canonical_name: "agent-a".into(), incarnation_bits: 42 };
    let identity = || InstructionRequest { participant: &actor, tool: "codex", cwd: root.path() };
    let fixed_path = root.path().join("instructions");
    let dynamic_path = root.path().join("facts");
    std::fs::write(&fixed_path, "FIXED_OLD").unwrap();
    std::fs::write(&dynamic_path, "FACTS_OLD").unwrap();
    let (fixed_sources, dynamic_sources) = select_sources(Some(r#"[{"id":"fixed","file":"instructions"}]"#), r#"[{"id":"facts","file":"facts"}]"#).unwrap();
    let fixed = prepare_instructions(&identity(), &fixed_sources, &[]).unwrap();
    let encoded = serde_json::to_string(&fixed).unwrap();
    let request = |reason| SnapshotRequest { identity: identity(), session_id: "session-a", reason };
    assert_eq!(prepare_initial_context(&request(SnapshotReason::Initial), &dynamic_sources, &[]).unwrap().text(), "FACTS_OLD");
    std::fs::write(&fixed_path, "FIXED_NEW").unwrap();
    std::fs::remove_file(&fixed_path).unwrap();
    std::fs::write(&dynamic_path, "FACTS_NEW").unwrap();
    let restored: FrozenInstructions = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restored.text, "FIXED_OLD");
    assert_eq!(prepare_compact_context(&request(SnapshotReason::AfterCompact), &dynamic_sources, &[]).unwrap().text(), "FACTS_NEW");
    std::fs::write(&dynamic_path, "").unwrap();
    let cleared = prepare_compact_context(&request(SnapshotReason::AfterCompact), &dynamic_sources, &[]).unwrap();
    assert!(cleared.blocks.is_empty());
    assert!(cleared.snapshot_text().contains("No current runtime context."));
    assert!(cleared.snapshot_text().contains("supersedes"));
    assert!(!cleared.snapshot_text().contains("FACTS_"));
    std::fs::remove_file(&dynamic_path).unwrap();
    assert!(prepare_compact_context(&request(SnapshotReason::AfterCompact), &dynamic_sources, &[]).is_err());
    let empty = FrozenInstructions { text: String::new() };
    assert_eq!(serde_json::from_str::<FrozenInstructions>(&serde_json::to_string(&empty).unwrap()).unwrap(), empty);
}
