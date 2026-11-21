//! Older daemon adapter. Queries only existing native reads; legacy actor
//! context operations are deliberately avoided because they can persist aliases.
use crate::{
    agents::attachments::ParticipantRef,
    events::service::{EventQuery, Request as Events, Response as EventReply},
    file_bindings::{self, Assignment},
    management::contract::{AgentRef, FileBindingView, LaunchRelations, Request as Management, Response as ManagementReply},
    service::{
        Request, Response, ServiceError, ServiceResult,
        client::Client,
        queries::{
            self,
            info::{Legacy, Reference, Saved, Target, snapshot_creation},
        },
    },
};
use serde_json::Value;
use std::{collections::BTreeSet, path::PathBuf};

pub(super) struct Record {
    pub saved: Saved,
    reference: AgentRef,
}

impl Record {
    pub(super) fn load(client: &Client, reference: &Reference, legacy: &Legacy) -> ServiceResult<Self> {
        let name = reference.target.name();
        let bits = reference.target.bits().ok_or(ServiceError::StaleFence)?;
        let stopped_at = match legacy {
            Legacy::Exact => None,
            Legacy::StopSnapshot { stopped_at } => Some(*stopped_at),
        };
        let event = match &reference.target {
            Target::Stopped { event, .. } => Some(*event),
            Target::Incarnation { .. } => None,
        };
        if event.is_none() && stopped_at.is_none() {
            let participant = ParticipantRef { canonical_name: name.to_owned(), incarnation_bits: bits };
            match client.call(Request::Queries(queries::Request::Terminal { participant: participant.clone() })) {
                Ok(Response::Queries(queries::Response::Terminal(endpoint)))
                    if endpoint.participant == participant && endpoint.record.name == name && endpoint.record.created_at.to_bits() == bits && endpoint.record.origin_device_id.as_deref().is_none_or(str::is_empty) =>
                {
                    return Ok(Self { saved: Saved::Current(Box::new(endpoint.record)), reference: AgentRef { name: name.to_owned(), incarnation: Some(bits.to_string()), stopped_event: None } });
                }
                Ok(_) | Err(ServiceError::StaleFence | ServiceError::NotFound(_)) => {}
                Err(error) => return Err(error),
            }
        }

        // The old dashboard never sends a stop event ID. The name filter only
        // narrows the search; the event must match the observed incarnation and
        // stop time. An older snapshot's synthetic creation time cannot match.
        let query = EventQuery {
            filters: [("instance".into(), vec![name.to_owned()]), ("action".into(), vec!["stopped".into(), "killed".into()])].into(),
            after: event.and_then(|id| id.checked_sub(1)),
            through: event,
            limit: if event.is_some() { 1 } else { 512 },
            ..EventQuery::default()
        };
        let Response::Events(EventReply::Page(page)) = client.call(Request::Events(Events::Query(query)))? else {
            return Err(ServiceError::Internal("Unexpected stop history reply".into()));
        };
        if page.database_generation != reference.generation || page.has_more {
            return Err(ServiceError::StaleFence);
        }
        let (id, snapshot) = exact_stop(&page.events, name, bits, stopped_at, event).ok_or(ServiceError::StaleFence)?;
        Ok(Self { saved: Saved::Stopped(snapshot.clone()), reference: AgentRef { name: name.to_owned(), incarnation: None, stopped_event: Some(id.to_string()) } })
    }

    pub(super) fn relations(&self, client: &Client, after: Option<String>, limit: u32) -> ServiceResult<LaunchRelations> {
        match client.call(Request::Management(Management::LaunchRelations { agent: self.reference.clone(), after, limit }))? {
            Response::Management(ManagementReply::LaunchRelations(page)) if page.generation == client.info().database_generation => Ok(page),
            Response::Management(ManagementReply::LaunchRelations(_)) => Err(ServiceError::StaleFence),
            _ => Err(ServiceError::Internal("Unexpected launch relations reply".into())),
        }
    }

    pub(super) fn bindings(&self, client: &Client, owner: &Assignment) -> ServiceResult<Vec<FileBindingView>> {
        let workspace = self.saved.collaboration().and_then(|adopted| adopted.workspace);
        let projects = self
            .saved
            .directory()
            .into_iter()
            .chain(workspace.as_deref())
            .map(PathBuf::from)
            .chain([client.scope_root().to_owned()])
            .filter(|path| path.is_absolute() && path.is_dir())
            .map(|path| file_bindings::index::project_root(&path))
            .collect::<BTreeSet<_>>();
        let mut seen = BTreeSet::new();
        let mut files = Vec::new();
        for project in projects {
            // The older worker's name filter is not exact. Match complete
            // document assignments (scope, generation, incarnation) ourselves.
            let query = file_bindings::Query { project, paths: Vec::new(), refresh: false, agent: None };
            let Response::Queries(queries::Response::FileBindings(projection)) = client.call(Request::Queries(queries::Request::FileBindings(query)))? else {
                return Err(ServiceError::Internal("Unexpected file binding reply".into()));
            };
            for file in projection.files {
                let Some(doc) = file.document.filter(|doc| doc.assignments.contains(owner)) else { continue };
                if seen.insert(file.path.clone()) {
                    files.push(FileBindingView {
                        path: file.path.to_string_lossy().into_owned(),
                        schema: Some(doc.schema),
                        description: doc.description,
                        raw_status: doc.raw_status,
                        status: Some(doc.status.as_str().to_owned()),
                        diagnostic: file.diagnostic,
                        checked_at: file.checked_at,
                        last_success_at: file.last_success_at,
                    });
                }
            }
        }
        Ok(files)
    }
}

fn exact_stop<'a>(events: &'a [Value], name: &str, bits: u64, stopped_at: Option<f64>, event: Option<i64>) -> Option<(i64, &'a Value)> {
    let mut matches = events.iter().filter_map(|row| {
        let id = row["id"].as_i64().filter(|id| *id > 0)?;
        if row["instance"].as_str() != Some(name) || row["type"] != "life" || !matches!(row["data"]["action"].as_str(), Some("stopped" | "killed")) || event.is_some_and(|expected| id != expected) {
            return None;
        }
        if stopped_at.is_some_and(|time| row["ts"].as_str().map(queries::parse_iso_to_epoch) != Some(time)) {
            return None;
        }
        let snapshot = &row["data"]["snapshot"];
        if snapshot.get("origin_device_id").is_some_and(|origin| !origin.is_null() && origin.as_str() != Some("")) || snapshot_creation(snapshot).map(f64::to_bits) != Some(bits) {
            return None;
        }
        Some((id, snapshot))
    });
    let matched = matches.next()?;
    matches.next().is_none().then_some(matched)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn legacy_stop_requires_exact_local_incarnation_and_stop_time() {
        let event = |id, time, creation: Value, origin: Value| {
            json!({
                "id": id, "instance": "same", "type": "life", "ts": time,
                "data": {"action": "stopped", "snapshot": {"created_at": creation, "origin_device_id": origin}}
            })
        };
        let time = "2026-11-22T01:02:03.456789+00:00";
        let events =
            [event(1, time, json!(11.0), Value::Null), event(2, time, json!(12.0), Value::Null), event(3, time, json!(11.0), json!("remote")), event(4, time, Value::Null, Value::Null), event(5, "2026-11-22T01:02:10+00:00", json!(11.0), Value::Null)];
        let bits = 11.0f64.to_bits();
        let observed_time = Some(queries::parse_iso_to_epoch(time));
        assert_eq!(exact_stop(&events, "same", bits, observed_time, None).unwrap().0, 1);
        assert!(exact_stop(&events, "same", bits, None, None).is_none(), "ambiguous stop");
        assert!(exact_stop(&events, "same", bits, None, Some(2)).is_none(), "wrong incarnation");
        assert!(exact_stop(&events, "different", bits, observed_time, None).is_none());
        assert!(exact_stop(&events, "same", 42.0f64.to_bits(), observed_time, None).is_none(), "missing creation must not be inferred");
    }
}
