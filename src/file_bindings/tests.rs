use super::document::NormalizedStatus as Status;
use super::*;
use crate::agents::attachments::ParticipantRef;
use std::path::PathBuf;

fn assignment(bits: u64) -> Assignment {
    Assignment { scope: "/scope".into(), generation: "generation".into(), participant: ParticipantRef { canonical_name: "naga".into(), incarnation_bits: bits } }
}
fn source(schema: &str, status: &str) -> String {
    format!("---\nschema: {schema}\nstatus: {status}\nagent: old-name # preserve\nunknown: {{key: [a, b]}}\n---\n# Body\nLiteral content\n")
}

#[test]
fn states_preserve_unknown_words_and_distinguish_work_from_observation() {
    for (words, expected) in [
        (vec!["planned", "TODO", "Not-Started", "not_yet_started", "queued"], Status::NotStarted),
        (vec!["planning", "ACTIVE", "In Progress", "in-progress", "in_progress", "ongoing"], Status::Ongoing),
        (vec!["completed", "Done", "accepted", "succeeded"], Status::Completed),
        (vec!["paused", "On-Hold", "suspended"], Status::Paused),
        (vec!["blocked", "waiting"], Status::Blocked),
        (vec!["failed", "error"], Status::Failed),
        (vec!["cancelled", "canceled", "aborted"], Status::Cancelled),
    ] {
        for word in words {
            let doc = parse(&source("task", &format!("'{word}'"))).unwrap();
            assert_eq!(doc.raw_status.as_deref(), Some(word));
            assert_eq!(doc.status, expected, "{word}");
        }
    }
    let unknown = parse(&source("delivery", "'Awaiting Space Whale'")).unwrap();
    assert_eq!(unknown.status, Status::Unknown);
    assert_eq!(unknown.raw_status.as_deref(), Some("Awaiting Space Whale"));
    assert_eq!(parse("---\nschema: task\n---\n").unwrap().status, Status::Undeclared);
}
#[test]
fn frontmatter_variants_and_relations_are_read_without_filename_inference() {
    let doc = parse("\u{feff}---\r\n'schema': delivery\r\ndescription: >-\r\n  A useful\r\n  delivery\r\nstatus: planning\r\nstages: {one: {tasks: [./one.md]}}\r\nagents: [legacy-name]\r\n...\r\nBody\r\n").unwrap();
    assert_eq!(doc.description.as_deref(), Some("A useful delivery"));
    assert_eq!(doc.schema, "delivery");
    assert!(doc.relations.contains_key("stages"));
    assert!(doc.assignments.is_empty());
    assert!(parse("---\n{schema: task, status: planned}\n---\nbody").is_ok());
    for text in ["no frontmatter", "---\nschema: task\n", "---\nschema: task\nstatus: [bad]\n---\n", "---\nschema: document\n---\n", "---\nschema: task\nstatus: [\n---\n", "---\nschema: task\nagents: wrong\n---\n"] {
        assert!(parse(text).is_err(), "{text}");
    }
}

#[test]
fn launch_message_includes_bound_file_snapshots_and_preserves_the_requested_prompt() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("task.md");
    let contents = "---\nschema: task\ndescription: Inspect the launch flow\nstatus: planned\n---\n# Assignment\nRead the launch code.\n````\nKeep this fence literal.\n````\n";
    std::fs::write(&path, contents).unwrap();

    let message = launch_message(std::slice::from_ref(&path), Some("Implement the assigned task.")).unwrap().unwrap();
    assert!(message.starts_with("Implement the assigned task."));
    assert!(message.contains("Bound file \""));
    assert!(message.contains(contents));
    assert!(message.contains("`````markdown"), "the snapshot fence must not close inside the file");
    assert_eq!(launch_message(&[], Some("Only this prompt.")).unwrap().as_deref(), Some("Only this prompt."));
}

#[test]
fn launch_message_truncates_at_utf8_boundaries_and_keeps_later_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("large.md");
    let later = tmp.path().join("later.md");
    let contents = format!("---\nschema: task\n---\n{}\nEnd of large file", "任".repeat(MAX_LAUNCH_MESSAGE_BYTES / 2));
    std::fs::write(&path, &contents).unwrap();
    std::fs::write(&later, "---\nschema: task\n---\nAnother assignment").unwrap();
    let paths = preflight(&[path.clone(), later.clone()], tmp.path()).unwrap();
    let message = launch_message(&paths, Some("Implement both tasks.")).unwrap().unwrap();

    assert!(message.starts_with("Implement both tasks."));
    assert!(message.len() <= MAX_LAUNCH_MESSAGE_BYTES);
    assert!(message.contains("任任任") && !message.contains("End of large file"));
    assert!(message.contains("\n```\n[Snapshot truncated."));
    assert!(message.contains(later.file_name().unwrap().to_str().unwrap()));
    assert!(message.contains("[Snapshot truncated. Read the source file for the rest.]"));
    assert_eq!(bind(&paths, &assignment(1)).unwrap(), paths);
    assert!(std::fs::read_to_string(&path).unwrap().ends_with("End of large file"));
}

#[test]
fn additive_exact_assignments_deduplicate_and_preserve_source() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("anything.md");
    let original = source("task", "planning").replace('\n', "\r\n");
    std::fs::write(&path, &original).unwrap();
    let paths = preflight(&[PathBuf::from("anything.md"), path.clone()], tmp.path()).unwrap();
    assert_eq!(paths, vec![path.clone()]);
    assert_eq!(bind(&paths, &assignment(1)).unwrap(), paths);
    assert!(bind(&paths, &assignment(1)).unwrap().is_empty());
    bind(&paths, &assignment(2)).unwrap();
    let mut other_scope = assignment(1);
    other_scope.scope = "/other".into();
    bind(&paths, &other_scope).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with(original.split("---\r\n# Body").next().unwrap()));
    assert!(text.ends_with("---\r\n# Body\r\nLiteral content\r\n"));
    assert_eq!(parse(&text).unwrap().assignments.len(), 3);
    assert!(rollback(&paths, &assignment(1)).is_empty());
    let doc = parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert!(!doc.assignments.contains(&assignment(1)));
    assert!(doc.assignments.contains(&assignment(2)) && doc.assignments.contains(&other_scope));
    assert_eq!(doc.status, Status::Ongoing);
}
#[test]
fn block_and_indentless_agents_keep_unknown_entries() {
    for entries in ["  - old-name\n  - {future: true}\n", "- old-name\n- {future: true}\n"] {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("x.md");
        std::fs::write(&path, format!("---\nschema: delivery\n'agents':\n{entries}status: planned\n---\nBody")).unwrap();
        bind(std::slice::from_ref(&path), &assignment(1)).unwrap();
        let text = std::fs::read_to_string(path).unwrap();
        let doc = parse(&text).unwrap();
        assert_eq!(doc.agents.len(), 3);
        assert_eq!(doc.status, Status::NotStarted);
        assert!(text.ends_with("---\nBody"));
    }
}
#[test]
fn multi_file_failure_compensates_only_new_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let first = tmp.path().join("first.md");
    let missing = tmp.path().join("missing.md");
    std::fs::write(&first, source("task", "active")).unwrap();
    bind(std::slice::from_ref(&first), &assignment(9)).unwrap();
    assert!(bind(&[first.clone(), missing], &assignment(1)).unwrap_err().to_string().contains("assignment failed"));
    let doc = parse(&std::fs::read_to_string(&first).unwrap()).unwrap();
    assert_eq!(doc.assignments, vec![assignment(9)]);
    // Compensation preserves later status/body edits and another participant.
    bind(std::slice::from_ref(&first), &assignment(1)).unwrap();
    let text = std::fs::read_to_string(&first).unwrap().replace("status: active", "status: blocked");
    std::fs::write(&first, format!("{text}Later edit\n")).unwrap();
    assert!(rollback(std::slice::from_ref(&first), &assignment(1)).is_empty());
    let text = std::fs::read_to_string(&first).unwrap();
    assert_eq!(parse(&text).unwrap().status, Status::Blocked);
    assert!(text.ends_with("Later edit\n"));
    std::fs::remove_file(&first).unwrap();
    assert!(rollback(&[first], &assignment(9)).contains("rollback incomplete"));
}
#[test]
fn index_rebuild_retains_last_success_and_separates_missing_malformed_state() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join(".controller/nested");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("arbitrary.md");
    std::fs::write(&path, source("delivery", "blocked")).unwrap();
    bind(std::slice::from_ref(&path), &assignment(1)).unwrap();
    let query = Query { project: tmp.path().to_path_buf(), paths: Vec::new(), refresh: false, agent: None };
    let mut index = Index::default();
    assert_eq!(index.query(&query, tmp.path()).unwrap().files.len(), 1);
    assert_eq!(index.for_assignment(&assignment(1)).len(), 1);
    assert!(index.for_assignment(&assignment(2)).is_empty());
    let mut rebuilt = Index::default();
    assert_eq!(rebuilt.query(&query, tmp.path()).unwrap().files[0].document.as_ref().unwrap().assignments, vec![assignment(1)]);
    std::fs::write(&path, "---\nschema: task\nstatus: [\n---\n").unwrap();
    let refresh = Query { refresh: true, ..query };
    let bad = index.query(&refresh, tmp.path()).unwrap().files.remove(0);
    assert!(bad.diagnostic.is_some() && bad.last_success_at.is_some());
    assert_eq!(bad.document.unwrap().status, Status::Blocked);
    std::fs::remove_file(&path).unwrap();
    let missing = index.query(&refresh, tmp.path()).unwrap().files.remove(0);
    assert!(missing.diagnostic.unwrap().contains("No such file"));
    assert!(missing.last_success_at.is_some());
    let explicit = Query { paths: vec![path], ..refresh };
    let fresh = Index::default().query(&explicit, tmp.path()).unwrap().files.remove(0);
    assert!(fresh.document.is_none() && fresh.last_success_at.is_none() && fresh.diagnostic.is_some());
}
#[cfg(unix)]
#[test]
fn discovery_does_not_follow_directory_or_file_symlinks() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".controller")).unwrap();
    let target = outside.path().join("target.md");
    std::fs::write(&target, source("task", "active")).unwrap();
    std::os::unix::fs::symlink(outside.path(), tmp.path().join(".controller/outside")).unwrap();
    std::os::unix::fs::symlink(target, tmp.path().join(".controller/link.md")).unwrap();
    let query = Query { project: tmp.path().into(), paths: Vec::new(), refresh: true, agent: None };
    assert!(Index::default().query(&query, tmp.path()).unwrap().files.is_empty());
}
#[test]
fn concurrent_binders_serialize_without_losing_participants() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("x.md");
    std::fs::write(&path, source("task", "active")).unwrap();
    std::thread::scope(|threads| {
        for bits in 0..8 {
            let path = &path;
            threads.spawn(move || {
                bind(std::slice::from_ref(path), &assignment(bits)).unwrap();
            });
        }
    });
    assert_eq!(parse(&std::fs::read_to_string(path).unwrap()).unwrap().assignments.len(), 8);
}

#[test]
fn flow_root_binding_preserves_metadata_and_body_with_documented_reformatting() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("x.md");
    std::fs::write(&path, "---\n{schema: task, status: planned, unknown: [a, b], agent: old}\n---\nBody stays\n").unwrap();
    preflight(std::slice::from_ref(&path), tmp.path()).unwrap();
    bind(std::slice::from_ref(&path), &assignment(1)).unwrap();
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.ends_with("---\nBody stays\n"));
    let doc = parse(&text).unwrap();
    assert_eq!(doc.assignments, vec![assignment(1)]);
    assert_eq!(doc.status, Status::NotStarted);
    assert_eq!(doc.agent.unwrap().as_str(), Some("old"));
    assert!(text.contains("unknown:"));
}

#[test]
fn busy_source_read_retains_last_success_without_blocking_dispatch() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("arbitrary.resource");
    std::fs::write(&path, source("task", "failed")).unwrap();
    let query = Query { project: tmp.path().into(), paths: vec![path.clone()], refresh: true, agent: None };
    let mut index = Index::default();
    assert_eq!(index.query(&query, tmp.path()).unwrap().files.len(), 1);
    let locked = std::fs::OpenOptions::new().read(true).write(true).open(&path).unwrap();
    locked.lock().unwrap();
    let blocked = index.query(&query, tmp.path()).unwrap().files.remove(0);
    assert!(blocked.diagnostic.is_some());
    assert_eq!(blocked.document.unwrap().status, Status::Failed);
    assert!(blocked.last_success_at.is_some());
}
